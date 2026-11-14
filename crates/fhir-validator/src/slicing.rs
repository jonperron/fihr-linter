//! Slicing of profile elements (profiling.html#slicing): the slice each repetition belongs to,
//! found by evaluating the discriminators with FHIRPath, then the slice counts and the
//! `closed`, `openAtEnd` and `ordered` rules.
use std::sync::Arc;

use fhir_definitions::{Discriminator, ElementDefinition, Registry};
use fhir_fhirpath::{Collection, Environment, Expr, Value as FhirPathValue};
use fhir_parser::Node;
use indexmap::IndexMap;
use serde_json::Value as Json;

use crate::diagnostic::{Diagnostic, Severity};
use crate::embedded::child_element;
use crate::fixed_pattern::matches_all;

/// One repetition of a sliced element: its node, instance path and, for a choice element,
/// the type its key selects (`valueQuantity` gives `Quantity`).
pub(crate) struct Item<'a> {
    pub node: &'a Node,
    pub path: String,
    pub choice_type: Option<&'a str>,
}

/// The slice an item belongs to: the slice element, the snapshot elements below it, and the
/// item's FHIRPath value (for the slice's invariants).
pub(crate) struct Assigned<'r> {
    pub slice: &'r ElementDefinition,
    pub children: &'r [ElementDefinition],
    pub value: Collection,
}

/// Whether an object conforms to the profile at a URL; `None` when the profile is unknown.
pub(crate) type Conforms<'a> = dyn Fn(&IndexMap<Arc<str>, Node>, &str) -> Option<bool> + 'a;

/// What a discriminator asks of the value at its path, for one slice.
enum Rule<'r> {
    /// `value`/`pattern`: the fixed (exact) or pattern value of the slice at the path.
    Value(&'r Json, bool),
    /// `value` on `url` of an extension slice: the URL of the slice's extension profile.
    Url(&'r str),
    /// `exists`: whether the path must be present (`min > 0`) or absent (`max = 0`).
    Exists(bool),
    /// `type`: the types the slice allows at the path.
    Type(Vec<&'r str>),
    /// `profile` on `$this`: the profiles the slice's type must conform to (any one).
    Profile(&'r [Arc<str>]),
}

/// Assign each item of the sliced element `elem` (instance `path`; `run` is the snapshot
/// below it, slices included) to the first slice it matches, then check the slice counts
/// (`PROFILE_006`), `closed` (`PROFILE_007`), `ordered` (`PROFILE_008`) and `openAtEnd`
/// (`PROFILE_009`). A discriminator that cannot be evaluated gives one Information
/// `SLICING_SKIPPED` and no assignment. `conforms` answers `profile` discriminators.
///
/// shortcut: discriminator paths must name an element of the slice (`$this` or a plain
/// path), so `resolve()`, `ofType()` and `extension('url')` paths are skipped; reslices count
/// as slices. Revisit when profiles with such paths are bundled.
#[allow(clippy::too_many_arguments)] // the element, its items and the walk state
pub(crate) fn assign<'r>(
    items: &[Item<'_>],
    elem: &ElementDefinition,
    run: &'r [ElementDefinition],
    path: &str,
    registry: &Registry,
    environment: &Environment<'_>,
    conforms: &Conforms<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Option<Assigned<'r>>> {
    let unassigned = || items.iter().map(|_| None).collect();
    // No discriminator: repetitions cannot be told apart, so slices are not evaluated.
    let Some(slicing) = elem
        .slicing
        .as_ref()
        .filter(|s| !s.discriminators.is_empty())
    else {
        return unassigned();
    };
    let starts: Vec<usize> = (0..run.len())
        .filter(|&i| run[i].slice_name.is_some() && run[i].path == elem.path)
        .collect();
    let slices: Vec<(&ElementDefinition, &[ElementDefinition])> = starts
        .iter()
        .enumerate()
        .map(|(k, &start)| {
            let end = starts.get(k + 1).copied().unwrap_or(run.len());
            (&run[start], &run[start + 1..end])
        })
        .collect();
    if slices.is_empty() && slicing.rules.as_ref() != "closed" {
        return unassigned();
    }
    let skip = |reason: String, diagnostics: &mut Vec<Diagnostic>| {
        // Nothing to check when no item is present and no slice is required.
        if !items.is_empty() || slices.iter().any(|(slice, _)| slice.min > 0) {
            diagnostics.push(Diagnostic {
                severity: Severity::Information,
                code: Arc::from("SLICING_SKIPPED"),
                message: format!("slicing of '{}' not checked: {reason}", elem.path),
                path: path.to_owned(),
                location: items.first().map(|i| i.node.span),
                rule: None,
            });
        }
    };
    let mut rules = Vec::with_capacity(slices.len());
    for (slice, children) in &slices {
        let closed = slicing.rules.as_ref() == "closed";
        let mut slice_rules = Vec::with_capacity(slicing.discriminators.len());
        let mut no_value = None;
        for discriminator in &slicing.discriminators {
            let found = parse(discriminator).and_then(|expr| {
                Ok(rule(slice, children, discriminator)?.map(|rule| (expr, rule)))
            });
            match found {
                Ok(Some(found)) => slice_rules.push(found),
                Ok(None) => {
                    let name = slice.slice_name.as_deref().unwrap_or_default();
                    let (kind, path) = (&discriminator.kind, &discriminator.path);
                    no_value = Some(format!(
                        "slice '{name}' gives no '{kind}' value at '{path}'"
                    ));
                }
                Err(reason) => {
                    skip(reason, diagnostics);
                    return unassigned();
                }
            }
        }
        match no_value {
            None => rules.push(Some(slice_rules)),
            // R5 history-bundle declares slice `patch` (max 0) with no method pattern. In a
            // closed slicing an item it would hold is reported as matching no slice instead,
            // so letting the slice match nothing hides no error.
            Some(_) if closed && &*slice.max == "0" => rules.push(None),
            Some(reason) => {
                skip(reason, diagnostics);
                return unassigned();
            }
        }
    }

    let is_choice_variant = |base: &str, suffix: &str| registry.is_choice_variant(base, suffix);
    let mut found: Vec<Option<(usize, Collection)>> = Vec::with_capacity(items.len());
    for item in items {
        let value = item_value(item, elem, registry);
        let mut matched = None;
        for (k, slice_rules) in rules.iter().enumerate() {
            let Some(slice_rules) = slice_rules else {
                continue;
            };
            let mut all = true;
            for (expr, rule) in slice_rules {
                let selected = match expr {
                    None => Ok(value.clone()),
                    Some(expr) => fhir_fhirpath::evaluate_parsed_with_environment(
                        expr,
                        &value,
                        &is_choice_variant,
                        environment,
                    ),
                };
                let selected = match selected {
                    Ok(selected) => selected,
                    Err(e) => {
                        skip(format!("cannot evaluate a discriminator: {e}"), diagnostics);
                        return unassigned();
                    }
                };
                let at_this = expr.is_none();
                match test(rule, &selected, item, at_this, registry, conforms) {
                    Ok(true) => {}
                    Ok(false) => {
                        all = false;
                        break;
                    }
                    Err(reason) => {
                        skip(reason, diagnostics);
                        return unassigned();
                    }
                }
            }
            if all {
                matched = Some(k);
                break;
            }
        }
        found.push(matched.map(|k| (k, value)));
    }

    check_counts(&slices, &found, items, elem, path, diagnostics);
    check_order(
        &slices,
        &found,
        items,
        slicing.rules.as_ref(),
        slicing.ordered,
        diagnostics,
    );
    found
        .into_iter()
        .map(|f| {
            f.map(|(k, value)| Assigned {
                slice: slices[k].0,
                children: slices[k].1,
                value,
            })
        })
        .collect()
}

/// The parsed discriminator path; `None` for `$this`, which selects the item itself.
fn parse(discriminator: &Discriminator) -> Result<Option<Expr>, String> {
    match discriminator.path.as_ref() {
        "$this" => Ok(None),
        path => fhir_fhirpath::parse(path)
            .map(Some)
            .map_err(|e| format!("cannot parse discriminator path '{path}': {e}")),
    }
}

/// What `discriminator` asks of the value at its path for `slice` (whose snapshot elements
/// below it are `children`); `None` when the slice gives no value there, `Err` with the
/// reason when the discriminator cannot be evaluated at all.
fn rule<'r>(
    slice: &'r ElementDefinition,
    children: &'r [ElementDefinition],
    discriminator: &Discriminator,
) -> Result<Option<Rule<'r>>, String> {
    let path = discriminator.path.as_ref();
    let target = match path {
        "$this" => Some(slice),
        path if path.contains(['(', '[', '$', ' ']) => {
            return Err(format!(
                "discriminator path '{path}' is not an element path"
            ));
        }
        path => {
            let full = format!("{}.{path}", slice.path);
            let choice = format!("{full}[x]");
            children
                .iter()
                .find(|e| e.slice_name.is_none() && (*e.path == *full || *e.path == *choice))
        }
    };
    let kind = discriminator.kind.as_ref();
    Ok(match (kind, target) {
        ("value" | "pattern", Some(target)) => match (&target.fixed, &target.pattern) {
            (Some(fixed), _) => Some(Rule::Value(&fixed.value, true)),
            (None, Some(pattern)) => Some(Rule::Value(&pattern.value, false)),
            (None, None) => url_rule(slice, kind, path),
        },
        ("value" | "pattern", None) => url_rule(slice, kind, path),
        ("exists", Some(target)) if target.min > 0 => Some(Rule::Exists(true)),
        ("exists", Some(target)) if &*target.max == "0" => Some(Rule::Exists(false)),
        ("type", Some(target)) if !target.types.is_empty() => Some(Rule::Type(
            target.types.iter().map(|t| t.as_ref()).collect(),
        )),
        ("profile", Some(target)) if path == "$this" => match target.type_profiles.as_slice() {
            [t] if !t.profiles.is_empty() => Some(Rule::Profile(&t.profiles)),
            _ => None,
        },
        ("exists" | "type" | "profile", _) => None,
        _ => return Err(format!("discriminator type '{kind}' is not supported")),
    })
}

/// profiling.html: an extension slice names its extension by its type profile, so a `value`
/// discriminator on `url` reads that profile's URL.
fn url_rule<'r>(slice: &'r ElementDefinition, kind: &str, path: &str) -> Option<Rule<'r>> {
    match slice.type_profiles.as_slice() {
        [t] if kind == "value" && path == "url" && &*t.code == "Extension" => {
            match t.profiles.as_slice() {
                [url] => Some(Rule::Url(url)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether the values `selected` at a discriminator path follow `rule`; `at_this` when the
/// path is `$this`. `Err` with the reason when the answer cannot be told.
fn test(
    rule: &Rule<'_>,
    selected: &[FhirPathValue],
    item: &Item<'_>,
    at_this: bool,
    registry: &Registry,
    conforms: &Conforms<'_>,
) -> Result<bool, String> {
    Ok(match rule {
        // Any one value at the path is enough (`code.coding.code` reaches every coding).
        Rule::Value(expected, exact) => selected
            .iter()
            .any(|v| matches_all(std::slice::from_ref(v), expected, *exact)),
        Rule::Url(url) => selected
            .iter()
            .any(|v| v.as_string().is_some_and(|s| *s == **url)),
        Rule::Exists(present) => selected.is_empty() != *present,
        Rule::Type(types) => {
            let [value] = selected else {
                return Ok(false);
            };
            let actual = match (item.choice_type.filter(|_| at_this), value) {
                (Some(ty), _) => ty,
                (None, FhirPathValue::Primitive(_, _, Some(ty))) => ty.as_ref(),
                (None, FhirPathValue::Object(fields)) => match fields.get("resourceType") {
                    Some(v) => match v.as_slice() {
                        [FhirPathValue::String(rt)] => rt.as_ref(),
                        _ => return Err("the type of a discriminated value is unknown".into()),
                    },
                    None => return Err("the type of a discriminated value is unknown".into()),
                },
                _ => return Err("the type of a discriminated value is unknown".into()),
            };
            // The type itself; an abstract slice type (Resource) takes its specialisations,
            // but a concrete one does not: `canonical` specialises `uri` yet is another type.
            let is_abstract = |t: &str| {
                registry
                    .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{t}"))
                    .is_some_and(|sd| sd.is_abstract)
            };
            types.iter().any(|t| {
                actual == *t || is_abstract(t) && registry.type_ancestors(actual).any(|a| a == *t)
            })
        }
        Rule::Profile(urls) => {
            let fhir_parser::Value::Object(fields) = &item.node.value else {
                return Ok(false);
            };
            let mut any = false;
            for url in urls.iter() {
                match conforms(fields, url) {
                    Some(c) => any |= c,
                    None => return Err(format!("profile '{url}' is not known")),
                }
            }
            any
        }
    })
}

/// The typed FHIRPath value of an item of `elem`.
fn item_value(item: &Item<'_>, elem: &ElementDefinition, registry: &Registry) -> Collection {
    let ty = item
        .choice_type
        .or_else(|| elem.types.first().map(|t| t.as_ref()))
        .unwrap_or("BackboneElement");
    let definition = match ty {
        "BackboneElement" | "Element" => Arc::clone(&elem.path),
        _ => Arc::from(ty),
    };
    fhir_fhirpath::node_to_typed_value(
        Some(&item.node.value),
        None,
        Some(&(definition, Arc::from(ty))),
        &|parent, name| child_element(registry, parent, name),
    )
}

/// `PROFILE_006` for each slice matched by fewer than `min` or more than `max` items, and
/// `PROFILE_007` for each item of a `closed` slicing that matches no slice.
fn check_counts(
    slices: &[(&ElementDefinition, &[ElementDefinition])],
    found: &[Option<(usize, Collection)>],
    items: &[Item<'_>],
    elem: &ElementDefinition,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let location = items.first().map(|i| i.node.span);
    for (k, (slice, _)) in slices.iter().enumerate() {
        let count = found.iter().flatten().filter(|(i, _)| *i == k).count();
        let name = slice.slice_name.as_deref().unwrap_or_default();
        let message = if count < slice.min as usize {
            format!(
                "Slice '{name}' of '{path}' needs at least {} matching value(s), found {count}",
                slice.min
            )
        } else if slice.max.parse::<usize>().is_ok_and(|max| count > max) {
            format!(
                "Slice '{name}' of '{path}' allows at most {} matching value(s), found {count}",
                slice.max
            )
        } else {
            continue;
        };
        let mut diagnostic = Diagnostic::error("PROFILE_006", message, path.to_owned());
        if let Some(location) = location {
            diagnostic = diagnostic.with_location(location);
        }
        diagnostics.push(diagnostic);
    }
    let closed = elem.slicing.as_ref().is_some_and(|s| &*s.rules == "closed");
    for (item, _) in items
        .iter()
        .zip(found)
        .filter(|(_, f)| closed && f.is_none())
    {
        diagnostics.push(
            Diagnostic::error(
                "PROFILE_007",
                format!(
                    "Value at '{}' matches no slice of '{}', whose slicing is closed",
                    item.path, elem.path
                ),
                item.path.clone(),
            )
            .with_location(item.node.span),
        );
    }
}

/// `PROFILE_008` for an item of an `ordered` slicing whose slice is declared before the slice
/// of an earlier item, and `PROFILE_009` for an item of an `openAtEnd` slicing that matches a
/// slice after an item that matched none.
fn check_order(
    slices: &[(&ElementDefinition, &[ElementDefinition])],
    found: &[Option<(usize, Collection)>],
    items: &[Item<'_>],
    rules: &str,
    ordered: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (mut last, mut unmatched_before) = (None::<usize>, false);
    for (item, f) in items.iter().zip(found) {
        let Some((k, _)) = f else {
            unmatched_before = true;
            continue;
        };
        let name = |k: usize| slices[k].0.slice_name.as_deref().unwrap_or_default();
        if let Some(previous) = last.filter(|&previous| ordered && *k < previous) {
            diagnostics.push(
                Diagnostic::error(
                    "PROFILE_008",
                    format!(
                        "Value at '{}' matches slice '{}', declared before slice '{}' of an \
                         earlier value; the slicing is ordered",
                        item.path,
                        name(*k),
                        name(previous)
                    ),
                    item.path.clone(),
                )
                .with_location(item.node.span),
            );
        }
        if rules == "openAtEnd" && unmatched_before {
            diagnostics.push(
                Diagnostic::error(
                    "PROFILE_009",
                    format!(
                        "Value at '{}' matches slice '{}' after a value that matches no slice; \
                         the slicing is openAtEnd",
                        item.path,
                        name(*k)
                    ),
                    item.path.clone(),
                )
                .with_location(item.node.span),
            );
        }
        last = Some(last.map_or(*k, |previous| previous.max(*k)));
    }
}

#[cfg(test)]
#[path = "slicing_tests.rs"]
mod tests;
