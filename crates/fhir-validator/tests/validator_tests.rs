//! Integration tests for the FHIR validator against the bundled R5 definitions.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

fn load_registry() -> Registry {
    Registry::from_definitions_dir(definitions_dir()).expect("failed to load R5 definitions")
}

// ── Unknown resource type ─────────────────────────────────────────────────────

#[test]
fn unknown_resource_type_produces_structure_000() {
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"NonExistentThing","id":"x"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code.as_ref(), "STRUCTURE_000");
    assert_eq!(diagnostics[0].severity, Severity::Error);
}

// ── Structural validation ─────────────────────────────────────────────────────

#[test]
fn unknown_element_produces_structure_001() {
    let registry = load_registry();
    let resource =
        parse_json(r#"{"resourceType":"Patient","id":"p1","unknownFooBarBaz":"value"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "STRUCTURE_001")
        .collect();
    assert!(
        !errors.is_empty(),
        "Expected STRUCTURE_001 for unknown element"
    );
    assert!(
        errors.iter().any(|d| d.path.contains("unknownFooBarBaz")),
        "Error path should mention the unknown field",
    );
}

#[test]
fn valid_patient_produces_no_structural_errors() {
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"Patient","id":"p1","active":true}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let structural_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "STRUCTURE_001")
        .collect();
    assert!(
        structural_errors.is_empty(),
        "Unexpected STRUCTURE_001: {structural_errors:?}",
    );
}

// ── Cardinality validation ─────────────────────────────────────────────────────

#[test]
fn missing_required_field_produces_cardinality_001() {
    let registry = load_registry();
    // Observation requires `status` (min=1) and `code` (min=1)
    let resource = parse_json(r#"{"resourceType":"Observation","id":"o1"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "CARDINALITY_001")
        .collect();
    assert!(
        !errors.is_empty(),
        "Expected CARDINALITY_001 for missing required fields on Observation",
    );
}

#[test]
fn patient_with_no_required_fields_passes_cardinality() {
    // Patient has no required fields at the base level.
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"Patient","id":"p1"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "CARDINALITY_001" && d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Unexpected CARDINALITY_001 for Patient: {errors:?}",
    );
}

// ── Type validation ───────────────────────────────────────────────────────────

#[test]
fn invalid_birth_date_produces_type_003() {
    let registry = load_registry();
    let resource =
        parse_json(r#"{"resourceType":"Patient","id":"p1","birthDate":"not-a-date"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "TYPE_003")
        .collect();
    assert!(
        !errors.is_empty(),
        "Expected TYPE_003 for invalid birthDate"
    );
}

#[test]
fn valid_birth_date_produces_no_type_error() {
    let registry = load_registry();
    let resource =
        parse_json(r#"{"resourceType":"Patient","id":"p1","birthDate":"1990-06-15"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let type_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref().starts_with("TYPE_"))
        .collect();
    assert!(
        type_errors.is_empty(),
        "Unexpected type errors: {type_errors:?}",
    );
}

// ── Combined: valid patient has no errors ─────────────────────────────────────

#[test]
fn validates_valid_patient_returns_no_errors() {
    let registry = load_registry();
    let resource = parse_json(
        r#"{"resourceType":"Patient","id":"example","active":true,"birthDate":"1990-01-01"}"#,
    )
    .unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for valid Patient, got: {errors:#?}",
    );
}

// ── Missing required field has correct path ────────────────────────────────────

#[test]
fn missing_required_field_path_is_correct() {
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"Observation","id":"o1"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let status_error = diagnostics
        .iter()
        .find(|d| d.code.as_ref() == "CARDINALITY_001" && d.path.contains("status"));
    assert!(
        status_error.is_some(),
        "Expected CARDINALITY_001 with path containing 'status'",
    );
}
