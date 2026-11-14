//! `fixed[x]` and `pattern[x]` of profile elements (elementdefinition.html), also used to
//! match `value`/`pattern` slice discriminators.
use fhir_definitions::ElementDefinition;
use fhir_fhirpath::Value as FhirPathValue;
use fhir_parser::{Node, Value};
use serde_json::Value as Json;

use crate::diagnostic::Diagnostic;

/// The non-null occurrences of `node` (instance `path`): the items of an array, or the node.
pub(crate) fn occurrences<'a>(node: &'a Node, path: &str) -> Vec<(&'a Node, String)> {
    match &node.value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .filter(|(_, item)| !matches!(item.value, Value::Null))
            .map(|(i, item)| (item, format!("{path}[{i}]")))
            .collect(),
        Value::Null => Vec::new(),
        _ => vec![(node, path.to_owned())],
    }
}

/// Check each occurrence of `node` (instance `path`) against the `fixed[x]` of `elem`, which
/// it SHALL be exactly (`PROFILE_004`), and its `pattern[x]`, which it SHALL follow
/// (`PROFILE_005`).
pub(crate) fn check_value(
    node: &Node,
    elem: &ElementDefinition,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if elem.fixed.is_none() && elem.pattern.is_none() {
        return;
    }
    for (item, item_path) in occurrences(node, path) {
        let values = fhir_fhirpath::from_parser_value(&item.value);
        let checks = [
            (elem.fixed.as_deref(), true, "PROFILE_004", "is not exactly"),
            (
                elem.pattern.as_deref(),
                false,
                "PROFILE_005",
                "does not match",
            ),
        ];
        for (expected, exact, code, verb) in checks {
            let Some(expected) = expected.filter(|e| !matches_all(&values, &e.value, exact)) else {
                continue;
            };
            diagnostics.push(
                Diagnostic::error(
                    code,
                    format!(
                        "Value at '{item_path}' {verb} the {} {} required by '{}'",
                        expected.key, expected.value, elem.path
                    ),
                    item_path.clone(),
                )
                .with_location(item.span),
            );
        }
    }
}

/// Whether the collection `values` equals `expected` (`exact`: arrays item by item) or
/// follows it as a pattern (each pattern array item matching some value). A single expected
/// value needs a single value.
pub(crate) fn matches_all(values: &[FhirPathValue], expected: &Json, exact: bool) -> bool {
    match (values, expected) {
        (values, Json::Array(expected)) if exact => {
            values.len() == expected.len()
                && values
                    .iter()
                    .zip(expected)
                    .all(|(v, e)| matches(v, e, true))
        }
        (values, Json::Array(expected)) => expected
            .iter()
            .all(|e| values.iter().any(|v| matches(v, e, false))),
        ([value], expected) => matches(value, expected, exact),
        _ => false,
    }
}

/// Whether `value` equals `expected` (`exact`: same properties, no extra extension or id on a
/// primitive) or follows it as a pattern (every pattern property present and matching).
/// Numbers compare by value, as FHIR decimals.
///
/// shortcut: both sides arrive as `f64` (parser and serde_json), so decimals beyond 15-17
/// significant digits can compare equal; upgrade when the parser keeps decimal text.
fn matches(value: &FhirPathValue, expected: &Json, exact: bool) -> bool {
    if let (FhirPathValue::Primitive(_, children, _), true) = (value, exact)
        && !children.is_empty()
    {
        return false;
    }
    match (value.primitive_value(), expected) {
        (FhirPathValue::Object(fields), Json::Object(map)) => {
            (!exact || fields.len() == map.len())
                && map.iter().all(|(key, e)| {
                    fields
                        .get(key.as_str())
                        .is_some_and(|values| matches_all(values, e, exact))
                })
        }
        (FhirPathValue::Bool(b), Json::Bool(e)) => b == e,
        (FhirPathValue::Integer(i) | FhirPathValue::Long(i), Json::Number(e)) => match e.as_i64() {
            Some(e) => *i == e,
            None => e.as_f64() == Some(*i as f64),
        },
        (FhirPathValue::Decimal(d), Json::Number(e)) => e.as_f64() == Some(*d),
        (_, Json::String(e)) => value.as_string().is_some_and(|s| *s == **e),
        _ => false,
    }
}
