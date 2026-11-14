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
use crate::slicing::{self, Assigned, Item};

/// Validate the profiles declared in `meta.profile`.
///
/// Profiles known to the registry are checked for resource-type match
/// (`PROFILE_002`) and their cardinality, `fixed[x]`/`pattern[x]` and value-slice rules are
/// applied at every depth (`check_profile_object`); `environment` evaluates discriminators and
/// slice invariants. Unknown profiles
/// produce a `PROFILE_001` warning because they cannot be validated against. A `|version`
/// other than the loaded definition's gives a `PROFILE_003` warning.
/// `resource_path` is the resource's instance path.
pub fn validate(
    resource: &Resource,
    base: &StructureDefinition,
    registry: &Registry,
    environment: &Environment<'_>,
    resource_path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let layer = Layer {
        registry,
        environment,
    };
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
                layer,
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
        let layer = Layer {
            registry,
            environment,
        };
        check_profile_object(fields, profile, run, &root.path, path, layer, &mut found);
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
/// depth: child cardinality, `fixed[x]`/`pattern[x]` and slicing. The items a slice matches
/// also get the slice's own `fixed[x]`/`pattern[x]`, invariants and children.
pub(crate) fn check_profile_object(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    run: &[ElementDefinition],
    def_path: &str,
    path: &str,
    layer: Layer<'_, '_>,
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
        let start = diagnostics.len();
        let subtree = subtree(run, k);
        let (present, items): (Vec<(&Arc<str>, &Node)>, Vec<Item>) = match rest.strip_suffix("[x]")
        {
            Some(base) => {
                // Every variant the base element allows is an item of the slicing; the rules
                // below apply to the variants the profile keeps.
                let types = choice_types(elem, layer.registry);
                let variants: Vec<(&Arc<str>, &Node, &str)> = fields
                    .iter()
                    .filter_map(|(key, node)| {
                        let suffix = key.strip_prefix(base)?;
                        let ty = types.iter().find(|t| capitalise(t) == suffix)?;
                        Some((key, node, ty.as_ref()))
                    })
                    .collect();
                let present = variants
                    .iter()
                    .filter(|(_, _, ty)| elem.types.iter().any(|t| &**t == *ty))
                    .map(|&(key, node, _)| (key, node))
                    .collect();
                let items = variants
                    .iter()
                    .map(|&(key, node, ty)| Item {
                        node,
                        path: format!("{path}.{key}"),
                        choice_type: Some(ty),
                    })
                    .collect();
                (present, items)
            }
            None => {
                let child_path = format!("{path}.{rest}");
                let items = fields
                    .get(rest)
                    .map(|node| occurrences(node, &child_path))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(node, path)| Item {
                        node,
                        path,
                        choice_type: None,
                    })
                    .collect();
                (fields.get_key_value(rest).into_iter().collect(), items)
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
                    check_profile_object(
                        fields,
                        sd,
                        own,
                        &elem.path,
                        &item_path,
                        layer,
                        diagnostics,
                    );
                }
            }
        }
        let child_path = format!("{path}.{rest}");
        let mut found = Vec::new();
        check_slices(&items, (elem, subtree), &child_path, sd, layer, &mut found);
        // A slice repeats rules of the sliced element: keep each finding once.
        found.retain(|f| {
            !diagnostics[start..]
                .iter()
                .any(|d| d.code == f.code && d.path == f.path)
        });
        diagnostics.extend(found);
    }
}

/// What the profile layer reads besides the profile: the registry, and the FHIRPath
/// environment of the resource for discriminators and slice invariants.
#[derive(Clone, Copy)]
pub(crate) struct Layer<'a, 'e> {
    pub registry: &'a Registry,
    pub environment: &'a Environment<'e>,
}

/// Assign the `items` of the sliced element `elem` (snapshot `run` below it, instance `path`)
/// to their slices ([`slicing::assign`]), then apply to each item its slice's own
/// `fixed[x]`/`pattern[x]`, the invariants the sliced element does not already have, and the
/// slice's children.
fn check_slices(
    items: &[Item<'_>],
    (elem, run): (&ElementDefinition, &[ElementDefinition]),
    path: &str,
    sd: &StructureDefinition,
    layer: Layer<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let conforms = |fields: &IndexMap<Arc<str>, Node>, url: &str| {
        let profile = layer.registry.structure_definition(url)?;
        let root = profile.snapshot.first()?;
        let run = own_descendants(profile, &root.path);
        let mut found = Vec::new();
        check_profile_object(fields, profile, run, &root.path, path, layer, &mut found);
        Some(!found.iter().any(|d| d.severity == Severity::Error))
    };
    let assigned = slicing::assign(
        items,
        elem,
        run,
        path,
        layer.registry,
        layer.environment,
        &conforms,
        diagnostics,
    );
    let is_choice_variant =
        |base: &str, suffix: &str| layer.registry.is_choice_variant(base, suffix);
    for (item, assigned) in items.iter().zip(assigned) {
        let Some(Assigned {
            slice,
            children,
            value,
        }) = assigned
        else {
            continue;
        };
        fixed_pattern::check_value(item.node, slice, &item.path, diagnostics);
        let known = |key: &str| elem.constraints.iter().any(|c| *c.key == *key);
        for constraint in slice.constraints.iter().filter(|c| !known(&c.key)) {
            invariants::check(
                constraint,
                &value,
                &is_choice_variant,
                layer.environment,
                &item.path,
                Some(item.node.span),
                diagnostics,
            );
        }
        if let Value::Object(fields) = &item.node.value {
            check_profile_object(
                fields,
                sd,
                children,
                &slice.path,
                &item.path,
                layer,
                diagnostics,
            );
        }
    }
}

/// The types the base definition allows for the choice element `elem` (a profile may keep
/// fewer); the profile's own types when the base is not found.
fn choice_types<'a>(elem: &'a ElementDefinition, registry: &'a Registry) -> &'a [Arc<str>] {
    let root = elem.path.split('.').next().unwrap_or_default();
    registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{root}"))
        .and_then(|base| base.element(&elem.path))
        .map_or(&elem.types, |base| &base.types)
}
