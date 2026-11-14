use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
use fhir_fhirpath::{Environment, Value as FhirPathValue};
use fhir_parser::{Node, Resource, Span, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::cardinality::{check_run_cardinality, own_descendants, subtree, without_own_slices};
use crate::diagnostic::{Diagnostic, Severity};
use crate::fixed_pattern::{self, occurrences};
use crate::invariants;
use crate::resolve::{capitalise, complex_type};

/// Validate the profiles declared in `meta.profile`.
///
/// Profiles known to the registry are checked for resource-type match
/// (`PROFILE_002`) and their cardinality, `fixed[x]`/`pattern[x]` and value-slice rules are
/// applied at every depth (`check_profile_object`). Unknown profiles
/// produce a `PROFILE_001` warning because they cannot be validated against. A `|version`
/// other than the loaded definition's gives a `PROFILE_003` warning.
/// `resource_path` is the resource's instance path.
pub fn validate(
    resource: &Resource,
    base: &StructureDefinition,
    registry: &Registry,
    resource_path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();
    let Some(Value::Object(meta)) = resource.fields.get("meta").map(|n| &n.value) else {
        return;
    };
    // The XML reshape keeps a single `<profile>` scalar; it is still the first profile.
    let profiles = match meta.get("profile") {
        Some(Node {
            value: Value::Array(items),
            ..
        }) => items.as_slice(),
        Some(node) => std::slice::from_ref(node),
        None => return,
    };

    for (i, node) in profiles.iter().enumerate() {
        let Value::Str(url) = &node.value else {
            continue;
        };
        let path = format!("{resource_path}.meta.profile[{i}]");
        // references.html canonical form: `url|version#fragment`.
        let (canonical, version) = match url.split_once('|') {
            Some((canonical, rest)) => (canonical, rest.split('#').next()),
            None => (url.as_ref(), None),
        };
        let Some(sd) = registry.structure_definition(canonical) else {
            diagnostics.push(
                Diagnostic::warning("PROFILE_001", format!("Profile '{url}' is not known"), path)
                    .with_location(node.span),
            );
            continue;
        };
        // The spec is silent on a version mismatch: warn, then validate against the loaded
        // version (user decision, PLAN 11.7 V10).
        if let Some(version) = version.filter(|v| sd.version.as_deref() != Some(*v)) {
            let loaded = sd.version.as_deref().unwrap_or("none");
            diagnostics.push(
                Diagnostic::warning(
                    "PROFILE_003",
                    format!(
                        "Profile '{url}' asks for version '{version}' but version '{loaded}' is \
                         loaded; validated against the loaded version"
                    ),
                    path.clone(),
                )
                .with_location(node.span),
            );
        }
        let profile_type = sd.snapshot.first().map(|e| e.path.as_ref());
        // Profiles on abstract parents (e.g. DomainResource) apply to every subtype.
        if profile_type != Some(resource_type) && !sd.is_abstract {
            diagnostics.push(
                Diagnostic::error(
                    "PROFILE_002",
                    format!("Profile '{url}' does not apply to resource type '{resource_type}'"),
                    path,
                )
                .with_location(node.span),
            );
        } else if sd.url != base.url {
            // Keep only findings the base check did not already report.
            let mut found = Vec::new();
            check_profile_object(
                &resource.fields,
                sd,
                own_descendants(sd, resource_type),
                resource_type,
                resource_path,
                &mut found,
            );
            found.retain(|f| {
                !diagnostics
                    .iter()
                    .any(|d| d.code == f.code && d.path == f.path)
            });
            diagnostics.extend(found);
        }
    }
}

/// Apply the profiles `elem` declares for type `ty` (`type.profile`, such as SimpleQuantity) to
/// one object (`fields`, its FHIRPath value and span): the profile root's own invariants
/// (`sqty-1`) and its rules below the root ([`check_profile_object`]). The value must conform
/// to one of them; findings the base walk of the node (from `start`) already made are dropped.
/// `environment` evaluates the invariants.
#[allow(clippy::too_many_arguments)] // the node, its definition, its place and the walk state
pub(crate) fn check_type_profiles(
    (fields, value, span): (&IndexMap<Arc<str>, Node>, &[FhirPathValue], Span),
    elem: &ElementDefinition,
    ty: &str,
    path: &str,
    registry: &Registry,
    environment: &Environment<'_>,
    start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(urls) = elem.type_profiles.iter().find(|t| t.code.as_ref() == ty) else {
        return;
    };
    let base = complex_type(ty, registry)
        .and_then(|type_sd| type_sd.element(ty))
        .map(|root| root.constraints.as_slice())
        .unwrap_or_default();
    let mut results = Vec::with_capacity(urls.profiles.len());
    for url in &urls.profiles {
        let Some((profile, root)) = registry
            .structure_definition(url)
            .and_then(|profile| Some((profile, profile.snapshot.first()?)))
        else {
            let message = format!("Profile '{url}' of '{path}' is not known");
            diagnostics.push(Diagnostic::warning("PROFILE_001", message, path.to_owned()));
            continue;
        };
        let mut found = Vec::new();
        let known = |key: &str| base.iter().chain(&elem.constraints).any(|c| *c.key == *key);
        for constraint in root.constraints.iter().filter(|c| !known(&c.key)) {
            invariants::check(
                constraint,
                value,
                &|base, suffix| registry.is_choice_variant(base, suffix),
                environment,
                path,
                Some(span),
                &mut found,
            );
        }
        let run = own_descendants(profile, &root.path);
        check_profile_object(fields, profile, run, &root.path, path, &mut found);
        found.retain(|f| {
            !diagnostics[start..]
                .iter()
                .any(|d| d.code == f.code && d.path == f.path)
        });
        results.push(found);
    }
    // type.profile: conforming to any one of the profiles is enough.
    let is_error = |d: &Diagnostic| d.severity == Severity::Error;
    let chosen = match results.iter().position(|r| !r.iter().any(is_error)) {
        Some(i) => results.swap_remove(i),
        None => results.into_iter().next().unwrap_or_default(),
    };
    diagnostics.extend(chosen);
}

/// Apply the rules of profile `sd` below `def_path` to one object (instance `path`), at every
/// depth: child cardinality, `fixed[x]`/`pattern[x]` and value-discriminated slices.
pub(crate) fn check_profile_object(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    run: &[ElementDefinition],
    def_path: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_run_cardinality(fields, sd, run, def_path, path, diagnostics);
    let prefix = format!("{def_path}.");
    for (k, elem) in run.iter().enumerate() {
        let Some(rest) = elem.path.strip_prefix(&prefix).filter(|r| !r.contains('.')) else {
            continue;
        };
        if elem.slice_name.is_some() {
            continue;
        }
        let subtree = subtree(run, k);
        let present: Vec<(&Arc<str>, &Node)> = match rest.strip_suffix("[x]") {
            Some(base) => fields
                .iter()
                .filter(|(key, _)| {
                    key.strip_prefix(base)
                        .is_some_and(|suffix| elem.types.iter().any(|t| capitalise(t) == suffix))
                })
                .collect(),
            None => {
                let child_path = format!("{path}.{rest}");
                let node = fields.get(rest);
                fixed_pattern::check_slices(node, elem, subtree, &child_path, diagnostics);
                fields.get_key_value(rest).into_iter().collect()
            }
        };
        let own = without_own_slices(subtree, &elem.path);
        for (name, node) in present {
            let child_path = format!("{path}.{name}");
            fixed_pattern::check_value(node, elem, &child_path, diagnostics);
            if elem.content_reference.is_some() {
                continue;
            }
            for (item, item_path) in occurrences(node, &child_path) {
                if let Value::Object(fields) = &item.value {
                    check_profile_object(fields, sd, own, &elem.path, &item_path, diagnostics);
                }
            }
        }
    }
}
