use std::sync::Arc;

use fhir_definitions::StructureDefinition;
use fhir_parser::Resource;

use crate::diagnostic::{Diagnostic, Severity};

/// Evaluate FHIRPath `constraint.expression` invariants from the root element.
///
/// Emits `INVARIANT_<key>` for every constraint whose expression evaluates to
/// anything other than `[true]`. Evaluation errors are silently skipped.
pub fn validate(resource: &Resource, sd: &StructureDefinition, diagnostics: &mut Vec<Diagnostic>) {
    let resource_type = resource.resource_type.as_ref();

    // The root element carries the constraints for the whole resource.
    let Some(root_elem) = sd
        .snapshot
        .iter()
        .find(|e| e.path.as_ref() == resource_type)
    else {
        return;
    };

    let context_value = fhir_fhirpath::resource_to_value(resource);
    let context = vec![context_value];

    for constraint in &root_elem.constraints {
        let Some(expression) = &constraint.expression else {
            continue;
        };

        let result = match fhir_fhirpath::evaluate(expression, &context) {
            Ok(r) => r,
            // Evaluation errors are non-fatal; skip this constraint.
            Err(_) => continue,
        };

        // A constraint is satisfied when the expression returns exactly [true].
        // An empty collection or [false] counts as a violation.
        let satisfied = matches!(result.as_slice(), [fhir_fhirpath::Value::Bool(true)]);

        if !satisfied {
            let severity = if constraint.severity.as_ref() == "warning" {
                Severity::Warning
            } else {
                Severity::Error
            };
            diagnostics.push(Diagnostic {
                severity,
                code: Arc::from(format!("INVARIANT_{}", constraint.key).as_str()),
                message: constraint.human.to_string(),
                path: resource_type.to_owned(),
                location: None,
                rule: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use fhir_definitions::{Constraint, ElementDefinition, StructureDefinition};
    use indexmap::IndexMap;

    fn make_constraint(key: &str, severity: &str, expression: &str) -> Constraint {
        Constraint {
            key: Arc::from(key),
            severity: Arc::from(severity),
            human: Arc::from(format!("Constraint {key} violated").as_str()),
            expression: Some(Arc::from(expression)),
        }
    }

    fn make_sd_with_constraints(
        resource_type: &str,
        constraints: Vec<Constraint>,
    ) -> StructureDefinition {
        StructureDefinition {
            url: Arc::from(
                format!("http://hl7.org/fhir/StructureDefinition/{resource_type}").as_str(),
            ),
            name: Arc::from(resource_type),
            kind: Arc::from("resource"),
            is_abstract: false,
            base_definition: None,
            snapshot: vec![ElementDefinition {
                path: Arc::from(resource_type),
                min: 0,
                max: Arc::from("*"),
                types: vec![],
                constraints,
                binding: None,
            }],
        }
    }

    fn make_empty_resource(resource_type: &str) -> fhir_parser::Resource {
        fhir_parser::Resource {
            resource_type: Arc::from(resource_type),
            id: None,
            fields: IndexMap::new(),
        }
    }

    #[test]
    fn satisfied_constraint_produces_no_diagnostic() {
        // "true" always evaluates to [true]
        let sd =
            make_sd_with_constraints("Patient", vec![make_constraint("test-1", "error", "true")]);
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn violated_error_constraint_produces_invariant_error() {
        // "false" evaluates to [false] — constraint violated
        let sd =
            make_sd_with_constraints("Patient", vec![make_constraint("test-2", "error", "false")]);
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_test-2");
        assert_eq!(d[0].severity, Severity::Error);
    }

    #[test]
    fn violated_warning_constraint_produces_invariant_warning() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint("test-3", "warning", "false")],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Severity::Warning);
    }

    #[test]
    fn absent_expression_is_skipped() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![Constraint {
                key: Arc::from("test-4"),
                severity: Arc::from("error"),
                human: Arc::from("no expression"),
                expression: None,
            }],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn evaluation_error_is_silently_skipped() {
        // This expression references an undefined function — should not crash.
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint(
                "test-5",
                "error",
                "undefinedFunction99999()",
            )],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn empty_expression_result_is_treated_as_violation() {
        // Expression that returns empty: evaluating a path that doesn't exist
        // on an empty resource returns [] which is a violation.
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint(
                "test-6",
                "error",
                "contained.contained.exists()",
            )],
        );
        // Empty resource: no contained → contained.contained → [] → .exists() → [false]
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        // [false] → violated
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn multiple_constraints_are_evaluated_independently() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![
                make_constraint("pass-1", "error", "true"),
                make_constraint("fail-1", "error", "false"),
                make_constraint("pass-2", "warning", "true"),
                make_constraint("fail-2", "warning", "false"),
            ],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 2);
        let codes: Vec<&str> = d.iter().map(|x| x.code.as_ref()).collect();
        assert!(codes.contains(&"INVARIANT_fail-1"));
        assert!(codes.contains(&"INVARIANT_fail-2"));
    }
}
