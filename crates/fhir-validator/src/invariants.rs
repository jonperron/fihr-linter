use std::sync::Arc;

use fhir_definitions::{Constraint, StructureDefinition};
use fhir_fhirpath::Environment;
use fhir_parser::{Resource, Span};

use crate::diagnostic::{Diagnostic, Severity};

/// Evaluate FHIRPath `constraint.expression` invariants of the root element.
///
/// Constraints below the root run in the single walk (`nested.rs`) through
/// [`check`]. `root` is the resource's FHIRPath value, typed by
/// [`fhir_fhirpath::resource_to_typed_value`] and shared with the walk; it is the
/// context and `%resource`, the rest of `environment` is used as given. `path` is the
/// resource's instance path (`Bundle.entry[0].resource` for an embedded one).
pub fn validate(
    resource: &Resource,
    sd: &StructureDefinition,
    is_choice_variant: &dyn Fn(&str, &str) -> bool,
    root: &fhir_fhirpath::Value,
    environment: &Environment<'_>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let environment = Environment {
        resource: Some(root),
        ..*environment
    };
    let resource_type = resource.resource_type.as_ref();

    // The root element carries the constraints for the whole resource.
    let Some(root_elem) = sd.element(resource_type) else {
        return;
    };

    for constraint in &root_elem.constraints {
        check(
            constraint,
            std::slice::from_ref(root),
            is_choice_variant,
            &environment,
            path,
            None,
            diagnostics,
        );
    }
}

/// Evaluate one constraint on `context`, the node at instance `path`.
///
/// Emits `INVARIANT_<key>` when the expression gives anything other than
/// `[true]`. An expression that fails to parse or evaluate emits one
/// `Information` `INVARIANT_SKIPPED` naming the key and the reason, so a clean
/// result never hides an unchecked constraint. `is_choice_variant(base, suffix)` makes
/// choice elements navigable (`value` finds `valueQuantity`); `environment` sets
/// `%resource`, `%rootResource` and the FHIR type inheritance of `is` / `as` /
/// `ofType`. The expression is parsed once and cached on the constraint.
pub(crate) fn check(
    constraint: &Constraint,
    context: &[fhir_fhirpath::Value],
    is_choice_variant: &dyn Fn(&str, &str) -> bool,
    environment: &Environment<'_>,
    path: &str,
    location: Option<Span>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(parsed) = constraint.parsed_expression() else {
        return;
    };
    let result = parsed
        .as_ref()
        .map_err(|e| fhir_fhirpath::EvalError::from(e.clone()))
        .and_then(|expr| {
            fhir_fhirpath::evaluate_parsed_with_environment(
                expr,
                context,
                is_choice_variant,
                environment,
            )
        });
    let (severity, code, message) = match result {
        // Satisfied only by exactly [true]; an empty collection or [false] is a violation.
        Ok(r) if matches!(r.as_slice(), [value] if value.is_truthy()) => return,
        // R5 constraint-severity is error | warning (required binding):
        // only "warning" downgrades, any other code stays an error.
        Ok(_) => (
            match constraint.severity.as_ref() {
                "warning" => Severity::Warning,
                _ => Severity::Error,
            },
            format!("INVARIANT_{}", constraint.key),
            constraint.human.to_string(),
        ),
        // Parse/evaluation errors are non-fatal: report the skip, then move on.
        Err(e) => (
            Severity::Information,
            "INVARIANT_SKIPPED".to_owned(),
            format!("constraint {} not checked: {e}", constraint.key),
        ),
    };
    diagnostics.push(Diagnostic {
        severity,
        code: Arc::from(code),
        message,
        path: path.to_owned(),
        location,
        rule: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use fhir_definitions::ElementDefinition;
    use indexmap::IndexMap;

    fn make_constraint(key: &str, severity: &str, expression: &str) -> Constraint {
        Constraint::new(
            Arc::from(key),
            Arc::from(severity),
            Arc::from(format!("Constraint {key} violated").as_str()),
            Some(Arc::from(expression)),
        )
    }

    fn make_sd_with_constraints(
        resource_type: &str,
        constraints: Vec<Constraint>,
    ) -> StructureDefinition {
        StructureDefinition::new(
            Arc::from(format!("http://hl7.org/fhir/StructureDefinition/{resource_type}").as_str()),
            Arc::from(resource_type),
            Arc::from("resource"),
            false,
            vec![ElementDefinition {
                path: Arc::from(resource_type),
                min: 0,
                max: Arc::from("*"),
                types: vec![],
                constraints,
                binding: None,
                slice_name: None,
                xml_attr: false,
                content_reference: None,
                type_profiles: vec![],
                fixed: None,
                pattern: None,
                slicing: None,
                max_length: None,
                min_value: None,
                max_value: None,
            }],
        )
    }

    fn make_empty_resource(resource_type: &str) -> fhir_parser::Resource {
        fhir_parser::Resource {
            resource_type: Arc::from(resource_type),
            id: None,
            from_xml: false,
            fields: IndexMap::new(),
            trimmed_attributes: Vec::new(),
            schema_instance: Vec::new(),
        }
    }

    /// The FHIRPath value of the empty Patient every test validates.
    fn empty_root() -> fhir_fhirpath::Value {
        fhir_fhirpath::resource_to_value(&make_empty_resource("Patient"))
    }

    #[test]
    fn satisfied_constraint_produces_no_diagnostic() {
        // "true" always evaluates to [true]
        let sd =
            make_sd_with_constraints("Patient", vec![make_constraint("test-1", "error", "true")]);
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(
            &resource,
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert!(d.is_empty());
    }

    #[test]
    fn violated_error_constraint_produces_invariant_error() {
        // "false" evaluates to [false] — constraint violated
        let sd =
            make_sd_with_constraints("Patient", vec![make_constraint("test-2", "error", "false")]);
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(
            &resource,
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
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
        validate(
            &resource,
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Severity::Warning);
    }

    #[test]
    fn absent_expression_is_skipped() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![Constraint::new(
                Arc::from("test-4"),
                Arc::from("error"),
                Arc::from("no expression"),
                None,
            )],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(
            &resource,
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert!(d.is_empty());
    }

    #[test]
    fn evaluation_error_reports_invariant_skipped_information() {
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
        validate(
            &resource,
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_SKIPPED");
        assert_eq!(d[0].severity, Severity::Information);
        assert!(d[0].message.contains("test-5"), "{}", d[0].message);
    }

    #[test]
    fn oversized_collection_reports_invariant_skipped_information() {
        // 2^20 items: past the evaluator's collection cap.
        let doubling = format!("1{}", ".select($this.combine($this))".repeat(20));
        let expression = format!("{doubling}.exists()");
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint("test-8", "error", &expression)],
        );
        let mut d = vec![];
        validate(
            &make_empty_resource("Patient"),
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_SKIPPED");
        assert_eq!(d[0].severity, Severity::Information);
        assert!(
            d[0].message.contains("collection larger than"),
            "{}",
            d[0].message
        );
    }

    #[test]
    fn oversized_string_reports_invariant_skipped_information() {
        // 2^21 characters: past the evaluator's string cap.
        let doubling = format!("'a'{}", ".select($this + $this)".repeat(21));
        let expression = format!("{doubling}.exists()");
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint("test-9", "error", &expression)],
        );
        let mut d = vec![];
        validate(
            &make_empty_resource("Patient"),
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_SKIPPED");
        assert_eq!(d[0].severity, Severity::Information);
        assert!(
            d[0].message.contains("string longer than"),
            "{}",
            d[0].message
        );
    }

    #[test]
    fn unparsable_expression_reports_invariant_skipped_information() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint("test-7", "error", "name.where(")],
        );
        let mut d = vec![];
        validate(
            &make_empty_resource("Patient"),
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_SKIPPED");
        assert_eq!(d[0].severity, Severity::Information);
        assert!(d[0].message.contains("test-7"), "{}", d[0].message);
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
        validate(
            &resource,
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
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
        validate(
            &resource,
            &sd,
            &|_, _| false,
            &empty_root(),
            &Environment::default(),
            "Patient",
            &mut d,
        );
        assert_eq!(d.len(), 2);
        let codes: Vec<&str> = d.iter().map(|x| x.code.as_ref()).collect();
        assert!(codes.contains(&"INVARIANT_fail-1"));
        assert!(codes.contains(&"INVARIANT_fail-2"));
    }

    #[test]
    fn severity_outside_constraint_severity_stays_error() {
        // R5 constraint-severity is error | warning (case-sensitive codes);
        // any other code is not a downgrade.
        for code in ["information", "Warning"] {
            let sd = make_sd_with_constraints(
                "Patient",
                vec![make_constraint("test-10", code, "false")],
            );
            let mut d = vec![];
            validate(
                &make_empty_resource("Patient"),
                &sd,
                &|_, _| false,
                &empty_root(),
                &Environment::default(),
                "Patient",
                &mut d,
            );
            assert_eq!(d[0].severity, Severity::Error, "{code}");
        }
    }
}
