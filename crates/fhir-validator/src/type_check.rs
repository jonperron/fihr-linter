use fhir_definitions::StructureDefinition;
use fhir_parser::{Resource, Span, Value};

use crate::diagnostic::Diagnostic;

/// Validate primitive type constraints for each resource field.
///
/// Emits `TYPE_001` through `TYPE_005` for format and type mismatches.
pub fn validate(resource: &Resource, sd: &StructureDefinition, diagnostics: &mut Vec<Diagnostic>) {
    let resource_type = resource.resource_type.as_ref();

    for (field_name, node) in &resource.fields {
        let path = format!("{resource_type}.{field_name}");

        let Some(elem) = sd
            .snapshot
            .iter()
            .find(|e| e.path.as_ref() == path.as_str())
        else {
            continue;
        };

        for fhir_type in &elem.types {
            check_type(
                &node.value,
                fhir_type.as_ref(),
                &path,
                node.span,
                diagnostics,
            );
        }
    }
}

fn check_type(
    value: &Value,
    fhir_type: &str,
    path: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Recurse into arrays.
    if let Value::Array(items) = value {
        for (i, item) in items.iter().enumerate() {
            let item_path = format!("{path}[{i}]");
            check_type(&item.value, fhir_type, &item_path, item.span, diagnostics);
        }
        return;
    }

    match fhir_type {
        "boolean" => {
            if !matches!(value, Value::Bool(_)) {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected boolean at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        }
        "integer" => {
            if !matches!(value, Value::Integer(_)) {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected integer at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        }
        "unsignedInt" => match value {
            Value::Integer(n) if *n < 0 => {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_002",
                        format!("Expected non-negative integer at '{path}', got {n}"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
            Value::Integer(_) => {}
            _ => {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected integer at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        },
        "positiveInt" => match value {
            Value::Integer(n) if *n <= 0 => {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_002",
                        format!("Expected positive integer at '{path}', got {n}"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
            Value::Integer(_) => {}
            _ => {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected integer at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        },
        "decimal" => {
            if !matches!(value, Value::Decimal(_) | Value::Integer(_)) {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected decimal number at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        }
        "date" => check_string_format(value, path, span, "date", is_valid_date, diagnostics),
        "dateTime" | "instant" => {
            check_string_format(value, path, span, fhir_type, is_valid_datetime, diagnostics);
        }
        "time" => check_string_format(value, path, span, "time", is_valid_time, diagnostics),
        "code" => {
            if let Value::Str(s) = value {
                if s.chars().any(char::is_whitespace) {
                    diagnostics.push(
                        Diagnostic::error(
                            "TYPE_004",
                            format!("Code at '{path}' must not contain whitespace: '{s}'"),
                            path.to_owned(),
                        )
                        .with_location(span),
                    );
                }
            } else {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected code string at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        }
        "id" => {
            if let Value::Str(s) = value {
                if !is_valid_id(s) {
                    diagnostics.push(
                        Diagnostic::error(
                            "TYPE_005",
                            format!("Invalid id format at '{path}': '{s}'"),
                            path.to_owned(),
                        )
                        .with_location(span),
                    );
                }
            } else {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected id string at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        }
        "string" | "markdown" | "base64Binary" | "xhtml" | "uri" | "url" | "canonical" | "oid"
        | "uuid" => {
            if !matches!(value, Value::Str(_)) {
                diagnostics.push(
                    Diagnostic::error(
                        "TYPE_001",
                        format!("Expected string at '{path}'"),
                        path.to_owned(),
                    )
                    .with_location(span),
                );
            }
        }
        // Complex and unknown types — skip deep validation here.
        _ => {}
    }
}

fn check_string_format(
    value: &Value,
    path: &str,
    span: Span,
    type_name: &str,
    is_valid: fn(&str) -> bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Value::Str(s) = value {
        if !is_valid(s) {
            diagnostics.push(
                Diagnostic::error(
                    "TYPE_003",
                    format!("Invalid {type_name} format at '{path}': '{s}'"),
                    path.to_owned(),
                )
                .with_location(span),
            );
        }
    } else {
        diagnostics.push(
            Diagnostic::error(
                "TYPE_001",
                format!("Expected {type_name} string at '{path}'"),
                path.to_owned(),
            )
            .with_location(span),
        );
    }
}

/// FHIR date: `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`.
fn is_valid_date(s: &str) -> bool {
    match s.len() {
        4 => s.chars().all(|c| c.is_ascii_digit()),
        7 => parse_ym(s),
        10 => parse_ymd(s),
        _ => false,
    }
}

fn parse_ym(s: &str) -> bool {
    let (year, rest) = s.split_at(4);
    let Some(month) = rest.strip_prefix('-') else {
        return false;
    };
    year.chars().all(|c| c.is_ascii_digit())
        && month.parse::<u8>().is_ok_and(|m| (1..=12).contains(&m))
}

fn parse_ymd(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return false;
    }
    let year = &s[..4];
    let month = &s[5..7];
    let day = &s[8..10];
    year.chars().all(|c| c.is_ascii_digit())
        && month.parse::<u8>().is_ok_and(|m| (1..=12).contains(&m))
        && day.parse::<u8>().is_ok_and(|d| (1..=31).contains(&d))
}

/// FHIR dateTime: date with optional `T` followed by a valid time component.
fn is_valid_datetime(s: &str) -> bool {
    if is_valid_date(s) {
        return true;
    }
    if let Some(t_pos) = s.find('T') {
        return is_valid_date(&s[..t_pos]) && is_valid_time_component(&s[t_pos + 1..]);
    }
    false
}

/// Validate the time portion of a FHIR dateTime (`hh:mm:ss[.sss][Z|+hh:mm|-hh:mm]`).
fn is_valid_time_component(s: &str) -> bool {
    // Must start with HH:MM:SS
    if s.len() < 8 {
        return false;
    }
    let b = s.as_bytes();
    if b[2] != b':' || b[5] != b':' {
        return false;
    }
    let hh: u8 = match s[0..2].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mm: u8 = match s[3..5].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let ss: u8 = match s[6..8].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    hh <= 23 && mm <= 59 && ss <= 59
}

/// FHIR time: `HH:MM:SS[.sss]` (no timezone in the `time` data type).
fn is_valid_time(s: &str) -> bool {
    if s.len() < 8 {
        return false;
    }
    let b = s.as_bytes();
    if b[2] != b':' || b[5] != b':' {
        return false;
    }
    let hh: u8 = match s[0..2].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mm: u8 = match s[3..5].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let ss: u8 = match s[6..8].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    hh <= 23 && mm <= 59 && ss <= 59
}

/// FHIR id: `[A-Za-z0-9\-\.]{1,64}`.
fn is_valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use fhir_definitions::{ElementDefinition, StructureDefinition};
    use fhir_parser::{Node, Span};
    use indexmap::IndexMap;

    fn make_elem(path: &str, type_code: &str) -> ElementDefinition {
        ElementDefinition {
            path: Arc::from(path),
            min: 0,
            max: Arc::from("1"),
            types: vec![Arc::from(type_code)],
            constraints: vec![],
            binding: None,
        }
    }

    fn make_sd(resource_type: &str, field: &str, type_code: &str) -> StructureDefinition {
        let path = format!("{resource_type}.{field}");
        let snapshot = vec![make_elem(resource_type, ""), make_elem(&path, type_code)];
        StructureDefinition {
            url: Arc::from(
                format!("http://hl7.org/fhir/StructureDefinition/{resource_type}").as_str(),
            ),
            name: Arc::from(resource_type),
            kind: Arc::from("resource"),
            is_abstract: false,
            base_definition: None,
            snapshot,
        }
    }

    fn make_resource(resource_type: &str, field: &str, value: Value) -> fhir_parser::Resource {
        let mut map: IndexMap<Arc<str>, Node> = IndexMap::new();
        map.insert(
            Arc::from(field),
            Node {
                value,
                span: Span::default(),
            },
        );
        fhir_parser::Resource {
            resource_type: Arc::from(resource_type),
            id: None,
            fields: map,
        }
    }

    #[test]
    fn valid_boolean_produces_no_error() {
        let sd = make_sd("Patient", "active", "boolean");
        let resource = make_resource("Patient", "active", Value::Bool(true));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn string_for_boolean_produces_type_001() {
        let sd = make_sd("Patient", "active", "boolean");
        let resource = make_resource("Patient", "active", Value::Str(Arc::from("true")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_001");
    }

    #[test]
    fn valid_date_produces_no_error() {
        let sd = make_sd("Patient", "birthDate", "date");
        for s in &["1990", "1990-01", "1990-01-15"] {
            let resource = make_resource("Patient", "birthDate", Value::Str(Arc::from(*s)));
            let mut d = vec![];
            validate(&resource, &sd, &mut d);
            assert!(d.is_empty(), "Expected no error for date '{s}'");
        }
    }

    #[test]
    fn invalid_date_produces_type_003() {
        let sd = make_sd("Patient", "birthDate", "date");
        let resource = make_resource("Patient", "birthDate", Value::Str(Arc::from("not-a-date")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_003");
    }

    #[test]
    fn code_with_whitespace_produces_type_004() {
        let sd = make_sd("Obs", "status", "code");
        let resource = make_resource("Obs", "status", Value::Str(Arc::from("fin al")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_004");
    }

    #[test]
    fn valid_code_produces_no_error() {
        let sd = make_sd("Obs", "status", "code");
        let resource = make_resource("Obs", "status", Value::Str(Arc::from("final")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn negative_unsigned_int_produces_type_002() {
        let sd = make_sd("Claim", "priority", "unsignedInt");
        let resource = make_resource("Claim", "priority", Value::Integer(-1));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_002");
    }

    #[test]
    fn zero_positive_int_produces_type_002() {
        let sd = make_sd("Claim", "priority", "positiveInt");
        let resource = make_resource("Claim", "priority", Value::Integer(0));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_002");
    }

    #[test]
    fn valid_id_produces_no_error() {
        let sd = make_sd("Patient", "id2", "id");
        for s in &["abc", "ABC-123", "a.b.c", "x-1"] {
            let resource = make_resource("Patient", "id2", Value::Str(Arc::from(*s)));
            let mut d = vec![];
            validate(&resource, &sd, &mut d);
            assert!(d.is_empty(), "Expected no error for id '{s}'");
        }
    }

    #[test]
    fn id_with_invalid_chars_produces_type_005() {
        let sd = make_sd("Patient", "id2", "id");
        let resource = make_resource("Patient", "id2", Value::Str(Arc::from("hello world")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_005");
    }

    #[test]
    fn array_elements_are_each_checked() {
        let sd = make_sd("Patient", "birthDate", "date");
        let items = vec![
            Node {
                value: Value::Str(Arc::from("1990-01-01")),
                span: Span::default(),
            },
            Node {
                value: Value::Str(Arc::from("not-a-date")),
                span: Span::default(),
            },
        ];
        let resource = make_resource("Patient", "birthDate", Value::Array(items));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_003");
    }
}
