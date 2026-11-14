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

#[test]
fn non_concrete_resource_type_produces_structure_000() {
    let registry = load_registry();
    // Abstract resources and datatypes have a StructureDefinition but are not
    // valid values of resourceType.
    for rt in ["Resource", "DomainResource", "HumanName"] {
        let resource = parse_json(&format!(r#"{{"resourceType":"{rt}","id":"x"}}"#)).unwrap();
        let diagnostics = validate(&resource, &registry);
        assert_eq!(diagnostics.len(), 1, "{rt}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code.as_ref(), "STRUCTURE_000", "{rt}");
        assert_eq!(
            diagnostics[0].message.as_ref(),
            format!("'{rt}' is not a concrete resource type"),
            "{rt}",
        );
    }
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

// ── Terminology validation ────────────────────────────────────────────────────

fn codes(registry: &Registry, json: &str, code: &str) -> usize {
    let resource = parse_json(json).unwrap();
    validate(&resource, registry)
        .iter()
        .filter(|d| d.code.as_ref() == code)
        .count()
}

#[test]
fn invalid_code_with_required_binding_produces_terminology_001() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Observation","status":"bogus","code":{"text":"x"}}"#;
    assert_eq!(codes(&registry, json, "TERMINOLOGY_001"), 1);
}

#[test]
fn valid_code_with_required_binding_passes_terminology() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Observation","status":"final","code":{"text":"x"}}"#;
    assert_eq!(codes(&registry, json, "TERMINOLOGY_001"), 0);
}

fn severities(registry: &Registry, json: &str, code: &str) -> Vec<(Severity, String)> {
    let resource = parse_json(json).unwrap();
    validate(&resource, registry)
        .into_iter()
        .filter(|d| d.code.as_ref() == code)
        .map(|d| (d.severity, d.path))
        .collect()
}

#[test]
fn text_only_codeable_concept_under_required_binding_produces_terminology_001() {
    // terminologies.html 4.1.4.2.1: text is not a substitute for the required code.
    let registry = load_registry();
    let base =
        r#"{"resourceType":"Condition","subject":{"reference":"Patient/1"},"clinicalStatus":"#;
    let error = vec![(Severity::Error, "Condition.clinicalStatus".to_owned())];
    for bad in [r#"{"text":"whatever"}"#, r#"{"coding":[{"display":"x"}]}"#] {
        let json = format!("{base}{bad}}}");
        assert_eq!(
            severities(&registry, &json, "TERMINOLOGY_001"),
            error,
            "{bad}"
        );
    }
    let coded = concat!(
        r#"{"coding":[{"system":"http://terminology.hl7.org/CodeSystem/condition-clinical","#,
        r#""code":"active"}],"text":"x"}"#,
    );
    let json = format!("{base}{coded}}}");
    assert!(severities(&registry, &json, "TERMINOLOGY_001").is_empty());
}

#[test]
fn code_outside_extensible_binding_produces_terminology_001_warning() {
    // FamilyMemberHistory.sex is bound extensibly to administrative-gender.
    let registry = load_registry();
    let sex = |code: &str| {
        format!(
            concat!(
                r#"{{"resourceType":"FamilyMemberHistory","sex":{{"coding":[{{"#,
                r#""system":"http://hl7.org/fhir/administrative-gender","code":"{code}"}}]}}}}"#,
            ),
            code = code,
        )
    };
    let warning = vec![(Severity::Warning, "FamilyMemberHistory.sex".to_owned())];
    assert_eq!(
        severities(&registry, &sex("bogus"), "TERMINOLOGY_001"),
        warning
    );
    assert!(severities(&registry, &sex("female"), "TERMINOLOGY_001").is_empty());
    // Extensible bindings allow text alone when no code fits.
    let text = r#"{"resourceType":"FamilyMemberHistory","sex":{"text":"x"}}"#;
    assert!(severities(&registry, text, "TERMINOLOGY_001").is_empty());
    // A Coding without system is not a member there either, and the missing system warns.
    let bare = r#"{"resourceType":"FamilyMemberHistory","sex":{"coding":[{"code":"female"}]}}"#;
    assert_eq!(severities(&registry, bare, "TERMINOLOGY_001"), warning);
    assert_eq!(
        severities(&registry, bare, "TERMINOLOGY_002"),
        vec![(
            Severity::Warning,
            "FamilyMemberHistory.sex.coding[0]".to_owned()
        )],
    );
}

#[test]
fn single_xml_coding_is_read_by_terminology_checks() {
    // The XML reshape keeps one <coding> as an object, not a one-item array.
    // DeviceUsage.usageStatus is bound (required) to a code system in the Registry.
    let registry = load_registry();
    let status = |code: &str| {
        format!(
            concat!(
                r#"<DeviceUsage xmlns="http://hl7.org/fhir"><usageStatus><coding>"#,
                r#"<system value="http://hl7.org/fhir/deviceusage-status"/>"#,
                r#"<code value="{code}"/></coding></usageStatus></DeviceUsage>"#,
            ),
            code = code,
        )
    };
    let terminology = |xml: &str| {
        let codes = xml_codes(&registry, xml);
        codes.into_iter().filter(|c| c == "TERMINOLOGY_001").count()
    };
    assert_eq!(terminology(&status("active")), 0);
    assert_eq!(terminology(&status("bogus")), 1);
    let text_only = concat!(
        r#"<DeviceUsage xmlns="http://hl7.org/fhir">"#,
        r#"<usageStatus><text value="x"/></usageStatus></DeviceUsage>"#,
    );
    assert_eq!(terminology(text_only), 1);
}

#[test]
fn coding_without_system_is_not_a_member_and_warns() {
    let registry = load_registry();
    let status = |codings: &str| {
        format!(r#"{{"resourceType":"DeviceUsage","usageStatus":{{"coding":[{codings}]}}}}"#)
    };
    let bare = status(r#"{"code":"active"}"#);
    assert_eq!(
        severities(&registry, &bare, "TERMINOLOGY_001"),
        vec![(Severity::Error, "DeviceUsage.usageStatus".to_owned())],
    );
    assert_eq!(
        severities(&registry, &bare, "TERMINOLOGY_002"),
        vec![(
            Severity::Warning,
            "DeviceUsage.usageStatus.coding[0]".to_owned()
        )],
    );
    // A second, valid Coding makes the concept a member; the bare one still warns.
    let mixed = status(
        r#"{"code":"active"},{"system":"http://hl7.org/fhir/deviceusage-status","code":"active"}"#,
    );
    assert!(severities(&registry, &mixed, "TERMINOLOGY_001").is_empty());
    assert_eq!(
        severities(&registry, &mixed, "TERMINOLOGY_002"),
        vec![(
            Severity::Warning,
            "DeviceUsage.usageStatus.coding[0]".to_owned()
        )],
    );
}

// ── Reference validation ──────────────────────────────────────────────────────

#[test]
fn reference_to_unknown_type_produces_reference_001() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","managingOrganization":{"reference":"Bogus/1"}}"#;
    assert_eq!(codes(&registry, json, "REFERENCE_001"), 1);
    // A ':' that does not end a leading URI scheme leaves the reference relative.
    for reference in ["Bogus/a:b", "Bogus?identifier=sys:1"] {
        let json = format!(
            r#"{{"resourceType":"Patient","managingOrganization":{{"reference":"{reference}"}}}}"#
        );
        assert_eq!(codes(&registry, &json, "REFERENCE_001"), 1, "{reference}");
    }
}

#[test]
fn reference_to_known_type_and_absolute_url_pass() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","generalPractitioner":[{"reference":"Practitioner/1"},{"reference":"http://x.org/anything/2"}]}"#;
    assert_eq!(codes(&registry, json, "REFERENCE_001"), 0);
}

#[test]
fn reference_type_disagreeing_with_fhir_url_produces_reference_003() {
    // references.html 2.1.3.0.1: type and a FHIR URL reference SHALL be consistent.
    let registry = load_registry();
    let org = |reference: &str, kind: &str| {
        format!(
            concat!(
                r#"{{"resourceType":"Patient","managingOrganization":"#,
                r#"{{"reference":"{reference}","type":"{kind}"}}}}"#,
            ),
            reference = reference,
            kind = kind,
        )
    };
    let error = vec![(
        Severity::Error,
        "Patient.managingOrganization.reference".to_owned(),
    )];
    for (reference, kind) in [
        ("Organization/2", "Patient"),
        ("http://example.org/fhir/Organization/2", "Patient"),
        (
            "Organization/2/_history/1",
            "http://hl7.org/fhir/StructureDefinition/Patient",
        ),
        // Reference.type names a resource definition, not a profile: absolute URLs are
        // only allowed for logical models (R5 Reference.type comment).
        (
            "Organization/2",
            "http://example.org/fhir/StructureDefinition/my-organization",
        ),
    ] {
        let json = org(reference, kind);
        assert_eq!(
            severities(&registry, &json, "REFERENCE_003"),
            error,
            "{reference} {kind}"
        );
    }
    for (reference, kind) in [
        ("Organization/2", "Organization"),
        ("https://example.org/fhir/Organization/2", "Organization"),
        (
            "Organization/2",
            "http://hl7.org/fhir/StructureDefinition/Organization",
        ),
        // Not FHIR URLs: the rule does not apply.
        ("Organization?identifier=1", "Patient"),
        ("http://example.org/my_fhir/Organization/2", "Patient"),
        ("urn:uuid:9d5e4b5a-7f2b-4c1e-9a3c-0b6c8f5d2e11", "Patient"),
    ] {
        let json = org(reference, kind);
        assert!(
            severities(&registry, &json, "REFERENCE_003").is_empty(),
            "{reference} {kind}"
        );
    }
}

#[test]
fn malformed_relative_reference_produces_reference_004_warning() {
    // The R5 literal reference regex is not worded as a SHALL: a warning.
    let registry = load_registry();
    let gp = |reference: &str| {
        format!(
            r#"{{"resourceType":"Patient","generalPractitioner":[{{"reference":"{reference}"}}]}}"#
        )
    };
    let warning = vec![(
        Severity::Warning,
        "Patient.generalPractitioner[0].reference".to_owned(),
    )];
    let long_id = format!("Practitioner/{}", "a".repeat(65));
    for bad in [
        "Practitioner",
        "Practitioner/a b",
        "Practitioner/1/_history",
        &long_id,
        // A ':' after a '/' is not a URI scheme (RFC 3986): still a relative reference.
        "Practitioner/a:b",
    ] {
        assert_eq!(
            severities(&registry, &gp(bad), "REFERENCE_004"),
            warning,
            "{bad}"
        );
    }
    for good in [
        "Practitioner/1",
        "Practitioner/a-1.b/_history/2",
        "Practitioner?name=x",
        // Not relative references: the rule does not apply.
        "#",
        "#p1",
        "http://example.org/fhir/Practitioner/a b",
        "urn:uuid:9d5e4b5a-7f2b-4c1e-9a3c-0b6c8f5d2e11",
    ] {
        assert!(
            severities(&registry, &gp(good), "REFERENCE_004").is_empty(),
            "{good}"
        );
    }
}

#[test]
fn unresolved_contained_reference_produces_reference_002() {
    let registry = load_registry();
    let json = r##"{"resourceType":"Patient","managingOrganization":{"reference":"#org"}}"##;
    assert_eq!(codes(&registry, json, "REFERENCE_002"), 1);
}

#[test]
fn bare_hash_reference_at_root_produces_reference_002() {
    // ref-1: '#' is valid only when %rootResource != %resource; the root has no container.
    let registry = load_registry();
    let json = r##"{"resourceType":"Patient","managingOrganization":{"reference":"#"}}"##;
    assert_eq!(
        severities(&registry, json, "REFERENCE_002"),
        vec![(
            Severity::Error,
            "Patient.managingOrganization.reference".to_owned()
        )],
    );
}

#[test]
fn resolved_contained_reference_passes() {
    let registry = load_registry();
    let json = r##"{"resourceType":"Patient","contained":[{"resourceType":"Organization","id":"org"}],"managingOrganization":{"reference":"#org"}}"##;
    assert_eq!(codes(&registry, json, "REFERENCE_002"), 0);
}

// ── Profile validation ────────────────────────────────────────────────────────

#[test]
fn unknown_profile_produces_profile_001_warning() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","meta":{"profile":["http://example.org/p"]}}"#;
    assert_eq!(codes(&registry, json, "PROFILE_001"), 1);
}

/// Heart-rate example from the R5 spec (`observation-example-heart-rate`).
const VITAL_SIGNS_HEART_RATE: &str = r#"{
  "resourceType": "Observation",
  "id": "heart-rate",
  "meta": {"profile": ["http://hl7.org/fhir/StructureDefinition/vitalsigns"]},
  "status": "final",
  "category": [{
    "coding": [{
      "system": "http://terminology.hl7.org/CodeSystem/observation-category",
      "code": "vital-signs",
      "display": "Vital Signs"
    }],
    "text": "Vital Signs"
  }],
  "code": {
    "coding": [{"system": "http://loinc.org", "code": "8867-4", "display": "Heart rate"}],
    "text": "Heart rate"
  },
  "subject": {"reference": "Patient/example"},
  "effectiveDateTime": "1999-07-02",
  "valueQuantity": {
    "value": 44,
    "unit": "beats/minute",
    "system": "http://unitsofmeasure.org",
    "code": "/min"
  }
}"#;

#[test]
fn core_vitalsigns_profile_is_known() {
    let registry = load_registry();
    assert_eq!(codes(&registry, VITAL_SIGNS_HEART_RATE, "PROFILE_001"), 0);
}

#[test]
fn valid_vitalsigns_observation_has_no_errors() {
    let registry = load_registry();
    let resource = parse_json(VITAL_SIGNS_HEART_RATE).unwrap();
    let errors: Vec<_> = validate(&resource, &registry)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn vitalsigns_profile_cardinality_is_enforced() {
    // `subject` is optional on Observation but 1..1 in the vitalsigns profile.
    let json =
        VITAL_SIGNS_HEART_RATE.replace(r#""subject": {"reference": "Patient/example"},"#, "");
    let found = codes_for(&json);
    assert!(
        found.contains(&("CARDINALITY_001".into(), "Observation.subject".into())),
        "{found:?}",
    );
}

#[test]
fn vitalsigns_extra_category_is_not_a_cardinality_error() {
    // The profile's `category:VSCat` slice is 1..1, but `category` itself stays 0..*.
    let json = VITAL_SIGNS_HEART_RATE.replace(
        r#""text": "Vital Signs"
  }],"#,
        r#""text": "Vital Signs"
  }, {"coding": [{
    "system": "http://terminology.hl7.org/CodeSystem/observation-category",
    "code": "exam"
  }]}],"#,
    );
    assert_ne!(json, VITAL_SIGNS_HEART_RATE);
    let found = codes_for(&json);
    assert!(
        !found.iter().any(|(c, _)| c.starts_with("CARDINALITY")),
        "{found:?}"
    );
}

#[test]
fn profile_for_other_resource_type_produces_profile_002() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","meta":{"profile":["http://hl7.org/fhir/StructureDefinition/Observation"]}}"#;
    assert_eq!(codes(&registry, json, "PROFILE_002"), 1);
}

#[test]
fn conditional_and_abstract_references() {
    let registry = load_registry();
    let ok = r#"{"resourceType":"Patient","managingOrganization":{"reference":"Organization?identifier=1"}}"#;
    assert_eq!(codes(&registry, ok, "REFERENCE_001"), 0);
    let abs =
        r#"{"resourceType":"Patient","managingOrganization":{"reference":"DomainResource/1"}}"#;
    assert_eq!(codes(&registry, abs, "REFERENCE_001"), 1);
}

// ── Nested validation ─────────────────────────────────────────────────────────

fn codes_for(json: &str) -> Vec<(String, String)> {
    let resource = parse_json(json).unwrap();
    validate(&resource, &load_registry())
        .into_iter()
        .map(|d| (d.code.to_string(), d.path))
        .collect()
}

#[test]
fn unknown_element_in_complex_type_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"family":"A","bogus":1}]}"#);
    assert!(found.contains(&("STRUCTURE_001".into(), "Patient.name[0].bogus".into())));
}

#[test]
fn unknown_element_in_backbone_element_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","contact":[{"bogus":1}]}"#);
    assert!(found.contains(&("STRUCTURE_001".into(), "Patient.contact[0].bogus".into())));
}

#[test]
fn missing_required_nested_element_is_reported() {
    let found = codes_for(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"component":[{"valueString":"v"}]}"#,
    );
    assert!(found.contains(&(
        "CARDINALITY_001".into(),
        "Observation.component[0].code".into()
    )));
}

#[test]
fn bad_primitive_in_nested_element_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"period":{"start":"nope"}}]}"#);
    assert!(
        found
            .iter()
            .any(|(c, p)| c == "TYPE_003" && p == "Patient.name[0].period.start")
    );
}

#[test]
fn valid_nested_resource_has_no_nested_errors() {
    let resource = parse_json(
        r#"{"resourceType":"Patient","text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">A</div>"},"name":[{"family":"A","given":["B"]}],"contact":[{"name":{"family":"C"}}]}"#,
    )
    .unwrap();
    // INVARIANT_SKIPPED reports an unchecked constraint, not a finding about the resource.
    let found: Vec<_> = validate(&resource, &load_registry())
        .into_iter()
        .filter(|d| d.code.as_ref() != "INVARIANT_SKIPPED")
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn unevaluable_invariants_are_reported_as_information() {
    // dgr-1 calls resolve(), which the evaluator does not implement.
    let resource = parse_json(
        r#"{"resourceType":"DiagnosticReport","id":"d1","status":"final","code":{"text":"x"},
        "composition":{"reference":"Composition/c1"}}"#,
    )
    .unwrap();
    let skipped: Vec<_> = validate(&resource, &load_registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == "INVARIANT_SKIPPED")
        .collect();
    assert!(
        !skipped.is_empty(),
        "expected skipped DiagnosticReport invariants"
    );
    for d in &skipped {
        assert_eq!(d.severity, Severity::Information);
        assert_eq!(d.path, "DiagnosticReport");
        assert!(d.message.starts_with("constraint "), "{}", d.message);
    }
}

// bdl-11 uses the function form `resource.is(Composition)`.
fn document_bundle_diagnostics(first_entry: &str) -> Vec<(String, String)> {
    let json = format!(
        r#"{{"resourceType":"Bundle","type":"document","timestamp":"2024-01-01T00:00:00Z",
        "identifier":{{"system":"urn:ietf:rfc:3986","value":"urn:uuid:0c3151bd-1cbf-4d64-b04d-cd9187a4c6e0"}},
        "entry":[{{"fullUrl":"urn:uuid:5d3e8a1c-3b1a-4d2a-9f61-1d2f3c4b5a6e","resource":{first_entry}}}]}}"#
    );
    let resource = parse_json(&json).unwrap();
    validate(&resource, &load_registry())
        .into_iter()
        .map(|d| (d.code.to_string(), d.message))
        .collect()
}

fn dom6_diagnostics(json: &str) -> Vec<String> {
    let resource = parse_json(json).unwrap();
    validate(&resource, &load_registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == "INVARIANT_dom-6" || d.message.contains("dom-6"))
        .map(|d| format!("{:?} {}", d.severity, d.code))
        .collect()
}

#[test]
fn document_bundle_starting_with_patient_violates_bdl_11() {
    let found = document_bundle_diagnostics(r#"{"resourceType":"Patient"}"#);
    assert!(
        found.iter().any(|(c, _)| c == "INVARIANT_bdl-11"),
        "{found:?}"
    );
    assert!(
        !found
            .iter()
            .any(|(_, m)| m.starts_with("constraint bdl-11 not checked")),
        "{found:?}"
    );
}

#[test]
fn document_bundle_starting_with_composition_satisfies_bdl_11() {
    let found = document_bundle_diagnostics(
        r#"{"resourceType":"Composition","status":"final","type":{"text":"Note"},"date":"2024-01-01","author":[{"display":"A"}],"title":"T"}"#,
    );
    let bdl_11 = |(c, m): &(String, String)| c.contains("bdl-11") || m.contains("bdl-11");
    assert!(!found.iter().any(bdl_11), "{found:?}");
}

#[test]
fn dom6_is_evaluated_not_skipped() {
    let narrative = r#"{"resourceType":"Patient","id":"p1","text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">x</div>"}}"#;
    assert!(dom6_diagnostics(narrative).is_empty());
    for json in [
        r#"{"resourceType":"Patient","id":"p1"}"#,
        r#"{"resourceType":"Patient","id":"p1","text":{"status":"generated"}}"#,
    ] {
        assert_eq!(
            dom6_diagnostics(json),
            ["Warning INVARIANT_dom-6"],
            "{json}"
        );
    }
}

// dgr-1 calls `resolve()`, which the evaluator does not implement: it must be
// reported as skipped, never as a violation.
#[test]
fn dgr1_with_unimplemented_resolve_is_skipped_not_violated() {
    let json = r#"{"resourceType":"DiagnosticReport","status":"final","code":{"text":"x"},
        "composition":{"reference":"Composition/c1"}}"#;
    let resource = parse_json(json).unwrap();
    let found: Vec<String> = validate(&resource, &load_registry())
        .into_iter()
        .filter(|d| d.code.contains("dgr-1") || d.message.contains("dgr-1"))
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
        .collect();
    assert_eq!(found, ["Information INVARIANT_SKIPPED DiagnosticReport"]);
}

// docRef-1 is `facilityType.empty() or context.where(resolve() is Encounter).empty()`:
// with no facilityType the left side decides `or`, so the unimplemented
// `resolve()` on the right must not make it skipped.
#[test]
fn docref1_decided_by_left_side_is_checked_not_skipped() {
    let docref1 = |facility_type: &str| {
        let json = format!(
            r#"{{"resourceType":"DocumentReference","status":"current",{facility_type}
            "content":[{{"attachment":{{"url":"http://example.org/doc"}}}}],
            "context":[{{"reference":"Encounter/e1"}}]}}"#
        );
        let resource = parse_json(&json).unwrap();
        validate(&resource, &load_registry())
            .into_iter()
            .filter(|d| d.message.contains("docRef-1"))
            .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
            .collect::<Vec<_>>()
    };
    let found = docref1("");
    assert!(found.is_empty(), "{found:?}");
    // With facilityType the right side must be evaluated: this proves the
    // constraint is loaded and run, so the empty result above is a real pass.
    let found = docref1(r#""facilityType":{"text":"x"},"#);
    assert_eq!(found, ["Information INVARIANT_SKIPPED DocumentReference"]);
}

#[test]
fn content_reference_element_is_not_unknown() {
    let found = codes_for(
        r#"{"resourceType":"Questionnaire","status":"draft","item":[{"linkId":"1","type":"group","item":[{"linkId":"2","type":"string"}]}]}"#,
    );
    assert!(
        !found.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{found:?}"
    );
}

#[test]
fn unknown_or_non_primitive_underscore_key_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","_bogus":{}}"#);
    assert!(
        found.contains(&("STRUCTURE_001".into(), "Patient._bogus".into())),
        "{found:?}"
    );
    // The `_name` form only exists for primitives; `name` is a HumanName.
    let found = codes_for(r#"{"resourceType":"Patient","_name":{}}"#);
    assert!(
        found.contains(&("STRUCTURE_001".into(), "Patient._name".into())),
        "{found:?}"
    );
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"_bogus":{}}]}"#);
    assert!(
        found.contains(&("STRUCTURE_001".into(), "Patient.name[0]._bogus".into())),
        "{found:?}"
    );
}

#[test]
fn primitive_extension_keys_are_accepted() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Patient","birthDate":"2000-01-01","#,
        r#""_birthDate":{"id":"b","extension":[{"url":"http://example.org/x","valueString":"v"}]},"#,
        r#""name":[{"given":["A",null],"_given":[null,{"id":"g"}]}]}"#
    ));
    assert!(
        !found.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{found:?}"
    );
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""_valueString":{"extension":[{"url":"http://example.org/x","valueString":"v"}]}}"#
    ));
    assert!(
        !found.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{found:?}"
    );
}

#[test]
fn primitive_extension_content_is_walked_as_element() {
    let found = codes_for(r#"{"resourceType":"Patient","_birthDate":{"bogus":1}}"#);
    assert!(
        found.contains(&("STRUCTURE_001".into(), "Patient._birthDate.bogus".into())),
        "{found:?}"
    );
    let found = codes_for(r#"{"resourceType":"Patient","_birthDate":{"extension":[{}]}}"#);
    assert!(
        found.contains(&(
            "CARDINALITY_001".into(),
            "Patient._birthDate.extension[0].url".into()
        )),
        "{found:?}"
    );
}

#[test]
fn primitive_extension_that_is_not_an_object_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","_birthDate":"x"}"#);
    assert!(
        found.contains(&("TYPE_001".into(), "Patient._birthDate".into())),
        "{found:?}"
    );
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"given":["A"],"_given":[1]}]}"#);
    assert!(
        found.contains(&("TYPE_001".into(), "Patient.name[0]._given[0]".into())),
        "{found:?}"
    );
}

#[test]
fn required_child_present_only_as_primitive_extension_is_not_missing() {
    let found = codes_for(
        r#"{"resourceType":"Questionnaire","status":"draft","item":[{"_linkId":{"extension":[{"url":"http://example.org/x","valueString":"v"}]},"type":"string"}]}"#,
    );
    assert!(
        !found.iter().any(|(code, _)| code == "CARDINALITY_001"),
        "{found:?}"
    );
}

#[test]
fn top_level_choice_value_is_type_checked() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""valueBoolean":"yes"}"#
    ));
    assert!(
        found
            .iter()
            .any(|(code, path)| code == "TYPE_001" && path == "Observation.valueBoolean"),
        "{found:?}"
    );
    // Only the suffix-selected type is checked, not every declared type.
    let valid = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""valueBoolean":true}"#
    ));
    assert!(
        !valid.iter().any(|(_, path)| path.contains("value")),
        "{valid:?}"
    );
}

#[test]
fn choice_suffix_must_be_an_allowed_type() {
    let bad = codes_for(r#"{"resourceType":"Observation","status":"final","valueAddress":{}}"#);
    assert!(
        bad.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{bad:?}"
    );
    let ok =
        codes_for(r#"{"resourceType":"Observation","status":"final","code":{},"valueString":"x"}"#);
    assert!(!ok.iter().any(|(_, path)| path.contains("value")), "{ok:?}");
}

#[test]
fn literal_choice_placeholder_key_is_unknown() {
    let top =
        codes_for(r#"{"resourceType":"Observation","status":"final","code":{},"value[x]":"x"}"#);
    assert!(
        top.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{top:?}"
    );
    let nested = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""component":[{"code":{},"value[x]":"x"}]}"#,
    ));
    assert!(
        nested.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{nested:?}"
    );
}

#[test]
fn dotted_top_level_key_is_unknown() {
    let found = codes_for(
        r#"{"resourceType":"Observation","status":"final","code":{},"component.code":{}}"#,
    );
    assert!(
        found.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{found:?}"
    );
}

#[test]
fn top_level_required_primitive_present_only_as_extension_is_not_missing() {
    let found = codes_for(r#"{"resourceType":"Observation","_status":{"id":"a"},"code":{}}"#);
    assert!(
        !found.iter().any(|(code, _)| code == "CARDINALITY_001"),
        "{found:?}"
    );
}

fn assert_has(found: &[(String, String)], wanted_code: &str, wanted_path: &str) {
    assert!(
        found
            .iter()
            .any(|(code, path)| code == wanted_code && path == wanted_path),
        "{found:?}"
    );
}

#[test]
fn required_complex_element_present_only_as_extension_is_missing() {
    let found = codes_for(r#"{"resourceType":"Observation","status":"final","_code":{"id":"a"}}"#);
    assert_has(&found, "CARDINALITY_001", "Observation.code");
}

#[test]
fn primitive_extension_array_over_max_is_cardinality_002() {
    let found = codes_for(r#"{"resourceType":"Patient","_birthDate":[{},{}]}"#);
    assert_has(&found, "CARDINALITY_002", "Patient.birthDate");
}

#[test]
fn punctuated_underscore_keys_are_unknown() {
    let obs = r#"{"resourceType":"Observation","status":"final","code":{},"#;
    for key in ["_a.b", "_value[x]"] {
        let top = codes_for(&format!(r#"{obs}"{key}":{{}}}}"#));
        assert_has(&top, "STRUCTURE_001", &format!("Observation.{key}"));
        let nested = codes_for(&format!(
            r#"{obs}"component":[{{"code":{{}},"{key}":{{}}}}]}}"#
        ));
        assert_has(
            &nested,
            "STRUCTURE_001",
            &format!("Observation.component[0].{key}"),
        );
    }
    let plain = codes_for(&format!(r#"{obs}"_status":{{}}}}"#));
    assert!(
        !plain.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{plain:?}"
    );
}

#[test]
fn nested_complex_child_present_only_as_extension_is_missing() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""component":[{"_code":{"id":"a"}}]}"#,
    ));
    assert_has(&found, "CARDINALITY_001", "Observation.component[0].code");
}

#[test]
fn nested_primitive_extension_array_over_max_is_cardinality_002() {
    let found = codes_for(r#"{"resourceType":"Patient","contact":[{"_gender":[{},{}]}]}"#);
    assert_has(&found, "CARDINALITY_002", "Patient.contact[0].gender");
}

#[test]
fn nested_integer_beyond_i64_produces_type_001() {
    let found = codes_for(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},
            "component":[{"code":{"text":"y"},"valueInteger":12345678901234567890}]}"#,
    );
    assert_has(&found, "TYPE_001", "Observation.component[0].valueInteger");
}

// ── Resource and element ids ──────────────────────────────────────────────────

#[test]
fn invalid_resource_id_produces_type_005() {
    let found = codes_for(r#"{"resourceType":"Patient","id":"has space!"}"#);
    assert_has(&found, "TYPE_005", "Patient.id");
}

#[test]
fn valid_resource_id_produces_no_type_error() {
    let found = codes_for(r#"{"resourceType":"Patient","id":"p-1.a"}"#);
    assert!(
        !found.iter().any(|(code, _)| code.starts_with("TYPE_")),
        "{found:?}"
    );
}

#[test]
fn non_string_element_id_produces_type_001() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"id":5,"family":"x"}]}"#);
    assert_has(&found, "TYPE_001", "Patient.name[0].id");
}

#[test]
fn element_id_is_a_string_not_an_id() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"id":"n:1","family":"x"}]}"#);
    assert!(
        !found.iter().any(|(code, _)| code.starts_with("TYPE_")),
        "{found:?}"
    );
}

// ── Choice element cardinality ────────────────────────────────────────────────

const IMMUNIZATION: &str = concat!(
    r#""resourceType":"Immunization","status":"completed","#,
    r#""vaccineCode":{"text":"x"},"patient":{"reference":"Patient/1"}"#,
);

fn occurrence_errors(extra: &str) -> Vec<(String, String)> {
    codes_for(&format!("{{{IMMUNIZATION}{extra}}}"))
        .into_iter()
        .filter(|(c, p)| c.starts_with("CARDINALITY_") && p.contains("occurrence"))
        .collect()
}

#[test]
fn missing_required_choice_produces_cardinality_001() {
    assert_eq!(
        occurrence_errors(""),
        [(
            "CARDINALITY_001".to_string(),
            "Immunization.occurrence[x]".to_string()
        )]
    );
}

#[test]
fn present_required_choice_produces_no_cardinality_error() {
    assert!(occurrence_errors(r#","occurrenceDateTime":"2021-01-01""#).is_empty());
}

#[test]
fn required_primitive_choice_present_only_as_extension_is_not_missing() {
    let ext = r#","_occurrenceString":{"extension":[{"url":"http://e","valueBoolean":true}]}"#;
    assert!(occurrence_errors(ext).is_empty());
}

#[test]
fn malformed_choice_extension_does_not_satisfy_required_choice() {
    assert_eq!(
        occurrence_errors(r#","_occurrenceString":5"#),
        [(
            "CARDINALITY_001".to_string(),
            "Immunization.occurrence[x]".to_string()
        )]
    );
}

#[test]
fn malformed_primitive_extension_does_not_satisfy_required_element() {
    let found = codes_for(r#"{"resourceType":"Observation","code":{"text":"x"},"_status":5}"#);
    assert_has(&found, "CARDINALITY_001", "Observation.status");
}

#[test]
fn extension_array_without_objects_does_not_satisfy_required_element() {
    for ext in ["[]", "[null]"] {
        let found = codes_for(&format!(
            r#"{{"resourceType":"Observation","code":{{"text":"x"}},"_status":{ext}}}"#
        ));
        assert_has(&found, "CARDINALITY_001", "Observation.status");
        let choice = occurrence_errors(&format!(r#","_occurrenceString":{ext}"#));
        assert_eq!(choice.len(), 1, "{choice:?}");
    }
}

#[test]
fn two_variants_of_one_choice_produce_cardinality_002() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""valueString":"a","valueBoolean":true}"#,
    ));
    assert!(
        found.contains(&("CARDINALITY_002".into(), "Observation.valueBoolean".into())),
        "{found:?}"
    );
}

#[test]
fn missing_nested_required_choice_produces_cardinality_001() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Claim","status":"active","type":{"text":"x"},"#,
        r#""use":"claim","patient":{"reference":"Patient/1"},"#,
        r#""created":"2021-01-01","diagnosis":[{"sequence":1}]}"#,
    ));
    assert!(
        found.contains(&(
            "CARDINALITY_001".into(),
            "Claim.diagnosis[0].diagnosis[x]".into()
        )),
        "{found:?}"
    );
}

#[test]
fn value_and_extension_of_one_variant_count_once() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""valueString":"a","_valueString":{"id":"e"}}"#,
    ));
    assert!(
        !found.iter().any(|(c, _)| c == "CARDINALITY_002"),
        "{found:?}"
    );
}

#[test]
fn two_variants_of_a_nested_choice_produce_cardinality_002() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""component":[{"code":{"text":"c"},"valueString":"a","#,
        r#""valueBoolean":true}]}"#,
    ));
    assert!(
        found.contains(&(
            "CARDINALITY_002".into(),
            "Observation.component[0].valueBoolean".into()
        )),
        "{found:?}"
    );
}

#[test]
fn complex_choice_present_only_as_extension_is_missing() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Claim","status":"active","type":{"text":"x"},"#,
        r#""use":"claim","patient":{"reference":"Patient/1"},"#,
        r#""created":"2021-01-01","diagnosis":[{"sequence":1,"#,
        r#""_diagnosisCodeableConcept":{}}]}"#,
    ));
    assert!(
        found.contains(&(
            "CARDINALITY_001".into(),
            "Claim.diagnosis[0].diagnosis[x]".into()
        )),
        "{found:?}"
    );
}

#[test]
fn variants_of_different_choices_do_not_conflict() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""valueString":"a","effectiveDateTime":"2021-01-01"}"#,
    ));
    assert!(
        !found.iter().any(|(c, _)| c == "CARDINALITY_002"),
        "{found:?}"
    );
}

#[test]
fn top_level_extension_keys_are_not_unknown() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Patient","extension":[],"modifierExtension":[],"#,
        r#""_birthDate":{"id":"a"}}"#,
    ));
    assert!(
        !found.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{found:?}"
    );
}

// ── Extensions ───────────────────────────────────────────────────────────────

/// Patient JSON placing `ext` (an extension array or value) at the top level, in a
/// backbone element and, for `extension` (datatypes have no `modifierExtension`), in a
/// complex type.
fn extension_placements(key: &str, ext: &str) -> Vec<(String, String)> {
    let mut placements = vec![
        (
            format!(r#"{{"resourceType":"Patient","{key}":{ext}}}"#),
            format!("Patient.{key}"),
        ),
        (
            format!(r#"{{"resourceType":"Patient","contact":[{{"{key}":{ext}}}]}}"#),
            format!("Patient.contact[0].{key}"),
        ),
    ];
    if key == "extension" {
        placements.push((
            format!(r#"{{"resourceType":"Patient","name":[{{"{key}":{ext}}}]}}"#),
            format!("Patient.name[0].{key}"),
        ));
    }
    placements
}

#[test]
fn modifier_extension_on_a_datatype_is_unknown() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"modifierExtension":[]}]}"#);
    assert_has(&found, "STRUCTURE_001", "Patient.name[0].modifierExtension");
}

#[test]
fn extension_on_a_resource_without_one_is_unknown() {
    // Bundle derives from Resource, not DomainResource, so it has no `extension`.
    let found = codes_for(r#"{"resourceType":"Bundle","type":"collection","extension":[]}"#);
    assert_has(&found, "STRUCTURE_001", "Bundle.extension");
}

#[test]
fn extension_without_url_is_cardinality_001() {
    for key in ["extension", "modifierExtension"] {
        for (json, path) in extension_placements(key, r#"[{"valueString":"x"}]"#) {
            assert_has(
                &codes_for(&json),
                "CARDINALITY_001",
                &format!("{path}[0].url"),
            );
        }
    }
    let found = codes_for(concat!(
        r#"{"resourceType":"Patient","extension":[{"url":"http://e","#,
        r#""extension":[{"valueString":"x"}]}]}"#,
    ));
    assert_has(
        &found,
        "CARDINALITY_001",
        "Patient.extension[0].extension[0].url",
    );
}

#[test]
fn extension_that_is_not_an_object_is_type_001() {
    for key in ["extension", "modifierExtension"] {
        for ext in [r#""oops""#, r#"["oops"]"#] {
            for (json, path) in extension_placements(key, ext) {
                let path = if ext.starts_with('[') {
                    format!("{path}[0]")
                } else {
                    path
                };
                assert_has(&codes_for(&json), "TYPE_001", &path);
            }
        }
    }
}

#[test]
fn scalar_for_a_complex_element_is_type_001() {
    let resource = parse_json(r#"{"resourceType":"Patient","meta":null,"name":[{},42]}"#).unwrap();
    let found: Vec<_> = validate(&resource, &load_registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == "TYPE_001")
        .map(|d| (d.path, d.message))
        .collect();
    assert!(
        found.contains(&(
            "Patient.meta".into(),
            "Expected Meta object at 'Patient.meta'".into()
        )),
        "{found:?}"
    );
    assert!(
        found.iter().any(|(path, _)| path == "Patient.name[1]"),
        "{found:?}"
    );
    assert!(
        !found.iter().any(|(path, _)| path == "Patient.name[0]"),
        "{found:?}"
    );
}

#[test]
fn extension_value_is_type_checked() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Patient","extension":[{"url":"http://e","valueInteger":"x"},"#,
        r#"{"url":"http://e","valueBogus":1}]}"#,
    ));
    assert_has(&found, "TYPE_001", "Patient.extension[0].valueInteger");
    assert_has(&found, "STRUCTURE_001", "Patient.extension[1].valueBogus");
}

#[test]
fn valid_extensions_have_no_diagnostics() {
    let ext = concat!(
        r#"[{"url":"http://e","valueString":"x"},"#,
        r#"{"url":"http://e","valueCodeableConcept":{"coding":[{"code":"a"}]}},"#,
        r#"{"url":"http://e","extension":[{"url":"n","valueInteger":1}]}]"#,
    );
    for key in ["extension", "modifierExtension"] {
        for (json, path) in extension_placements(key, ext) {
            let found = codes_for(&json);
            assert!(
                !found.iter().any(|(_, p)| p.starts_with(&path)),
                "{json}: {found:?}"
            );
        }
    }
    let found = codes_for(
        r#"{"resourceType":"Patient","_birthDate":{"extension":[{"url":"http://e","valueString":"x"}]}}"#,
    );
    assert!(
        !found.iter().any(|(_, p)| p.contains("birthDate")),
        "{found:?}"
    );
}

#[test]
fn top_level_value_array_over_max_is_cardinality_002() {
    let found = codes_for(r#"{"resourceType":"Patient","active":[true,false]}"#);
    assert_has(&found, "CARDINALITY_002", "Patient.active");
}

#[test]
fn top_level_choice_array_over_max_is_cardinality_002() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""valueString":["a","b"]}"#,
    ));
    assert_has(&found, "CARDINALITY_002", "Observation.valueString");
}

#[test]
fn each_unknown_top_level_key_is_reported_once() {
    let found = codes_for(r#"{"resourceType":"Patient","foo":1,"bar":2}"#);
    let unknown: Vec<_> = found
        .iter()
        .filter(|(code, _)| code == "STRUCTURE_001")
        .map(|(_, path)| path.as_str())
        .collect();
    assert_eq!(unknown, ["Patient.foo", "Patient.bar"]);
}

// ── Strict temporal and integer primitives ───────────────────────────────────

/// Type codes reported at `path` for `{"resourceType":<rt>,<field_json>}`.
fn type_codes_at(registry: &Registry, rt: &str, field_json: &str, path: &str) -> Vec<String> {
    let resource = parse_json(&format!(r#"{{"resourceType":"{rt}",{field_json}}}"#)).unwrap();
    validate(&resource, registry)
        .into_iter()
        .filter(|d| d.code.starts_with("TYPE_") && d.path == path)
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn temporal_primitives_follow_the_r5_regexes() {
    let registry = load_registry();
    let cases: &[(&str, &str, &str, bool)] = &[
        // (resource, field, value, valid)
        ("Patient", "birthDate", "2020", true),
        ("Patient", "birthDate", "2020-02-29", true),
        ("Patient", "birthDate", "2000-02-29", true),
        ("Patient", "birthDate", "2021-02-29", false),
        ("Patient", "birthDate", "1900-02-29", false),
        ("Patient", "birthDate", "2020-02-31", false),
        ("Patient", "birthDate", "2020-04-31", false),
        ("Patient", "birthDate", "0000", false),
        ("Patient", "deceasedDateTime", "2020-02-31", false),
        ("Patient", "deceasedDateTime", "2020-01-01T10:00:00Z", true),
        (
            "Patient",
            "deceasedDateTime",
            "2020-01-01T10:00:00.123+02:00",
            true,
        ),
        (
            "Patient",
            "deceasedDateTime",
            "2020-01-01T10:00:00-14:00",
            true,
        ),
        ("Patient", "deceasedDateTime", "2020-01-01T23:59:60Z", true),
        (
            "Patient",
            "deceasedDateTime",
            "2020-01-01T10:00:00garbage",
            false,
        ),
        ("Patient", "deceasedDateTime", "2020-01-01T10:00:00", false),
        (
            "Patient",
            "deceasedDateTime",
            "2020-01-01T10:00:00.Z",
            false,
        ),
        (
            "Patient",
            "deceasedDateTime",
            "2020-01-01T10:00:00.1234567890Z",
            false,
        ),
        (
            "Patient",
            "deceasedDateTime",
            "2020-01-01T10:00:00+14:30",
            false,
        ),
        (
            "Patient",
            "deceasedDateTime",
            "2020-01-01T10:00:00+0200",
            false,
        ),
        ("Patient", "deceasedDateTime", "2020-02-30T10:00:00Z", false),
        // The R5 regex allows a zone on a year-month or full date (PLAN 11.2-A5).
        ("Patient", "deceasedDateTime", "2020-01-01Z", true),
        ("Patient", "deceasedDateTime", "2020-01Z", true),
        ("Patient", "deceasedDateTime", "2020-01-01+02:00", true),
        // Year-month with a `-05:00` offset, as the regex reads it.
        ("Patient", "deceasedDateTime", "2020-01-05:00", true),
        ("Patient", "deceasedDateTime", "2020Z", false),
        ("Patient", "deceasedDateTime", "2020-01-01+", false),
        ("Patient", "deceasedDateTime", "2020-01-01Zjunk", false),
        // A multi-byte char across the year-month / full-date boundary: rejected, no panic.
        ("Patient", "deceasedDateTime", "2020-0\u{e9}Z", false),
        ("Patient", "deceasedDateTime", "2020-01-0\u{e9}Z", false),
        ("Patient", "deceasedDateTime", "2020-01\u{e9}Z", false),
        ("Patient", "deceasedDateTime", "2020-01-01T10:00:00+", false),
        ("Observation", "issued", "2020-01-01T10:00:00.5Z", true),
        ("Observation", "issued", "2020", false),
        ("Observation", "issued", "2020-01-01", false),
        ("Observation", "issued", "2020-01-01T10:00:00", false),
        ("Observation", "valueTime", "10:00:00", true),
        ("Observation", "valueTime", "10:00:00.123456789", true),
        ("Observation", "valueTime", "10:00:60", true),
        ("Observation", "valueTime", "10:00:00Z", false),
        ("Observation", "valueTime", "10:00:00junk", false),
        ("Observation", "valueTime", "10:00:61", false),
        ("Observation", "valueTime", "10:00", false),
    ];
    for (rt, field, value, valid) in cases {
        let found = type_codes_at(
            &registry,
            rt,
            &format!(r#""{field}":"{value}""#),
            &format!("{rt}.{field}"),
        );
        let expected: Vec<String> = if *valid {
            vec![]
        } else {
            vec!["TYPE_003".into()]
        };
        assert_eq!(found, expected, "{rt}.{field} = {value:?}");
    }
}

#[test]
fn code_follows_the_r5_regex() {
    // R5 `code` regex: `[^\s]+( [^\s]+)*`.
    let registry = load_registry();
    let cases: &[(&str, bool)] = &[
        ("text/plain; charset=utf-8", true),
        ("final", true),
        ("a  b", false),
        (" a", false),
        ("a ", false),
        ("a\\tb", false),
        ("", false),
    ];
    for (value, valid) in cases {
        let found = type_codes_at(
            &registry,
            "Observation",
            &format!(r#""valueAttachment":{{"contentType":"{value}"}}"#),
            "Observation.valueAttachment.contentType",
        );
        let expected: Vec<String> = if *valid {
            vec![]
        } else {
            vec!["TYPE_004".into()]
        };
        assert_eq!(found, expected, "contentType = {value:?}");
    }
}

#[test]
fn integer_primitives_are_32_bit() {
    let registry = load_registry();
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        // (resource, field, raw JSON number, expected code)
        ("Observation", "valueInteger", "2147483647", None),
        ("Observation", "valueInteger", "-2147483648", None),
        (
            "Observation",
            "valueInteger",
            "2147483648",
            Some("TYPE_002"),
        ),
        (
            "Observation",
            "valueInteger",
            "-2147483649",
            Some("TYPE_002"),
        ),
        ("Bundle", "total", "2147483647", None),
        ("Bundle", "total", "2147483648", Some("TYPE_002")),
        ("Bundle", "total", "-1", Some("TYPE_002")),
    ];
    for (rt, field, raw, expected) in cases {
        let path = format!("{rt}.{field}");
        let found = type_codes_at(&registry, rt, &format!(r#""{field}":{raw}"#), &path);
        let expected: Vec<String> = expected.iter().map(|c| c.to_string()).collect();
        assert_eq!(found, expected, "{path} = {raw}");
    }
    // positiveInt, nested in SampledData.
    for (raw, expected) in [("2147483647", 0), ("2147483648", 1), ("0", 1)] {
        let found = type_codes_at(
            &registry,
            "Observation",
            &format!(r#""valueSampledData":{{"dimensions":{raw}}}"#),
            "Observation.valueSampledData.dimensions",
        );
        assert_eq!(found.len(), expected, "dimensions = {raw}: {found:?}");
        assert!(found.iter().all(|c| c == "TYPE_002"), "{found:?}");
    }
    // integer64 is a string and is not capped at 32 bits.
    let found = type_codes_at(
        &registry,
        "Patient",
        r#""photo":[{"size":"9223372036854775807"}]"#,
        "Patient.photo[0].size",
    );
    assert!(found.is_empty(), "{found:?}");
}

// ── Leading type name in invariants (csd-4) ──────────────────────────────────

#[test]
fn codesystem_csd_4_uses_leading_type_name() {
    let registry = load_registry();
    let official = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator/cs-stds-status.json");
    match std::fs::read_to_string(official) {
        Ok(json) => assert_eq!(codes(&registry, &json, "INVARIANT_csd-4"), 0),
        Err(_) => assert!(
            std::env::var_os("CI").is_none(),
            "fixtures submodule missing in CI: run `git submodule update --init`"
        ),
    }
    let base = r#"{"resourceType":"CodeSystem","status":"draft","content":"supplement"}"#;
    assert_eq!(codes(&registry, base, "INVARIANT_csd-4"), 1);
    let with_supplements = r#"{"resourceType":"CodeSystem","status":"draft",
        "content":"supplement","supplements":"http://x"}"#;
    assert_eq!(codes(&registry, with_supplements, "INVARIANT_csd-4"), 0);
}

// ── XML input shape ───────────────────────────────────────────────────────────

fn xml_codes(registry: &Registry, xml: &str) -> Vec<String> {
    let resource = fhir_parser::parse_xml(xml).unwrap();
    validate(&resource, registry)
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.code.to_string())
        .collect()
}

fn xml_errors(registry: &Registry, xml: &str) -> Vec<String> {
    let resource = fhir_parser::parse_xml(xml).unwrap();
    validate(&resource, registry)
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| format!("{} {}", d.code, d.path))
        .collect()
}

#[test]
fn xml_typed_primitives_pass_type_checks() {
    let registry = load_registry();
    let codes = xml_codes(
        &registry,
        concat!(
            r#"<Patient xmlns="http://hl7.org/fhir"><active value="true"/>"#,
            r#"<multipleBirthInteger value="2"/></Patient>"#,
        ),
    );
    assert!(!codes.iter().any(|c| c.starts_with("TYPE_")), "{codes:?}");
}

#[test]
fn xml_typed_integer_reaches_range_check() {
    // A value that is dropped or left as text would not give the 32-bit range error.
    let registry = load_registry();
    let errors = xml_errors(
        &registry,
        concat!(
            r#"<Patient xmlns="http://hl7.org/fhir">"#,
            r#"<multipleBirthInteger value="2147483648"/></Patient>"#,
        ),
    );
    assert_eq!(errors, ["TYPE_002 Patient.multipleBirthInteger"]);
}

#[test]
fn xml_unsigned_int_is_capped_at_i32_max() {
    // R5 unsignedInt has maxValueInteger 2147483647, the same cap as JSON input.
    let registry = load_registry();
    let study = |n: &str| {
        format!(
            r#"<ImagingStudy xmlns="http://hl7.org/fhir"><status value="available"/>
            <subject><reference value="Patient/1"/></subject>
            <numberOfSeries value="{n}"/></ImagingStudy>"#
        )
    };
    assert!(xml_errors(&registry, &study("2147483647")).is_empty());
    assert_eq!(
        xml_errors(&registry, &study("2147483648")),
        ["TYPE_002 ImagingStudy.numberOfSeries"],
    );
}

#[test]
fn xml_bad_boolean_still_fails() {
    let registry = load_registry();
    let codes = xml_codes(
        &registry,
        r#"<Patient xmlns="http://hl7.org/fhir"><active value="yes"/></Patient>"#,
    );
    assert!(codes.iter().any(|c| c == "TYPE_001"), "{codes:?}");
}

fn active_with_extension(value: &str) -> String {
    format!(
        r#"<Patient xmlns="http://hl7.org/fhir"><active value="true">
        <extension url="http://example.org/x"><valueBoolean value="{value}"/></extension>
        </active></Patient>"#
    )
}

#[test]
fn xml_primitive_extension_value_is_typed() {
    let registry = load_registry();
    let errors = xml_errors(&registry, &active_with_extension("true"));
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn xml_primitive_extension_bad_value_fails() {
    // Proves the extension is kept under `_active` and validated.
    let registry = load_registry();
    let errors = xml_errors(&registry, &active_with_extension("maybe"));
    assert_eq!(errors, ["TYPE_001 Patient._active.extension.valueBoolean"],);
}

#[test]
fn xml_primitive_unknown_attribute_fails() {
    let registry = load_registry();
    let errors = xml_errors(
        &registry,
        r#"<Patient xmlns="http://hl7.org/fhir"><active value="true" foo="x"/></Patient>"#,
    );
    assert!(
        errors.iter().any(|e| e.starts_with("STRUCTURE_001")),
        "{errors:?}"
    );
}

#[test]
fn xml_empty_primitive_is_reported() {
    let registry = load_registry();
    let errors = xml_errors(
        &registry,
        r#"<Patient xmlns="http://hl7.org/fhir"><active/></Patient>"#,
    );
    // ele-1: "All FHIR elements must have a @value or children".
    assert_eq!(
        errors,
        ["TYPE_001 Patient.active", "INVARIANT_ele-1 Patient.active"],
    );
}

#[test]
fn official_valid_xml_cases_have_no_errors() {
    let registry = load_registry();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/validator");
    if !dir.is_dir() {
        assert!(
            std::env::var_os("CI").is_none(),
            "fixtures submodule missing in CI: run `git submodule update --init`"
        );
        return;
    }
    for name in [
        "patient-animal.xml",
        "group-choice-good.xml",
        "list-extension.xml",
        "xml_v11.xml",
        "ext-ctxt-good-ext.xml",
    ] {
        let codes = xml_codes(&registry, &std::fs::read_to_string(dir.join(name)).unwrap());
        assert!(codes.is_empty(), "{name}: {codes:?}");
    }
}

fn xml_diags(registry: &Registry, xml: &str) -> Vec<String> {
    let resource = fhir_parser::parse_xml(xml).unwrap();
    validate(&resource, registry)
        .iter()
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.message))
        .collect()
}

#[test]
fn xml_nested_questionnaire_item_boolean_is_typed() {
    let registry = load_registry();
    let out = xml_diags(
        &registry,
        r#"<Questionnaire xmlns="http://hl7.org/fhir"><status value="draft"/><item><linkId value="1"/><type value="group"/><item><linkId value="1.1"/><type value="string"/><required value="true"/></item></item></Questionnaire>"#,
    );
    assert!(
        !out.iter()
            .any(|d| d.contains("TYPE_") || d.contains("CARDINALITY")),
        "{out:?}"
    );
}

#[test]
fn xml_bundle_entry_resource_boolean_is_typed() {
    let registry = load_registry();
    let out = xml_diags(
        &registry,
        r#"<Bundle xmlns="http://hl7.org/fhir"><type value="collection"/><entry><resource><Patient><active value="true"/></Patient></resource></entry></Bundle>"#,
    );
    assert!(
        !out.iter()
            .any(|d| d.contains("TYPE_") || d.contains("CARDINALITY")),
        "{out:?}"
    );
}

#[test]
fn xml_contained_resource_boolean_is_typed() {
    let registry = load_registry();
    let out = xml_diags(
        &registry,
        r#"<Patient xmlns="http://hl7.org/fhir"><contained><Patient><id value="a"/><active value="true"/></Patient></contained></Patient>"#,
    );
    assert!(
        !out.iter()
            .any(|d| d.contains("TYPE_") || d.contains("CARDINALITY")),
        "{out:?}"
    );
}

// ── JSON array shape ──────────────────────────────────────────────────────────

fn shape_paths(found: &[(String, String)]) -> Vec<&str> {
    found
        .iter()
        .filter(|(code, _)| code == "STRUCTURE_002")
        .map(|(_, path)| path.as_str())
        .collect()
}

#[test]
fn json_repeating_element_as_object_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","name":{"family":"x"}}"#);
    assert_eq!(shape_paths(&found), ["Patient.name"], "{found:?}");
}

#[test]
fn json_single_element_as_array_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","gender":["male"]}"#);
    assert_eq!(shape_paths(&found), ["Patient.gender"], "{found:?}");
}

#[test]
fn json_array_shape_is_checked_in_nested_and_choice_elements() {
    let found = codes_for(
        r#"{"resourceType":"Patient","name":[{"given":"A"}],
            "extension":[{"url":"http://example.org/x","valueString":["a"]}]}"#,
    );
    assert_eq!(
        shape_paths(&found),
        ["Patient.name[0].given", "Patient.extension[0].valueString"],
        "{found:?}"
    );
}

#[test]
fn json_well_shaped_elements_pass() {
    let found = codes_for(
        r#"{"resourceType":"Patient","gender":"male","name":[{"given":["A"]}],
            "extension":[{"url":"http://example.org/x","valueString":"a"}]}"#,
    );
    assert!(shape_paths(&found).is_empty(), "{found:?}");
}

#[test]
fn json_array_shape_checks_content_reference_elements() {
    let found = codes_for(
        r#"{"resourceType":"Questionnaire","status":"draft",
            "item":[{"linkId":"1","type":"group","item":{"linkId":"1.1","type":"string"}}]}"#,
    );
    assert_eq!(
        shape_paths(&found),
        ["Questionnaire.item[0].item"],
        "{found:?}"
    );
}

#[test]
fn json_array_shape_skips_null_values() {
    let found = codes_for(r#"{"resourceType":"Patient","name":null,"gender":null}"#);
    assert!(shape_paths(&found).is_empty(), "{found:?}");
}

#[test]
fn json_too_many_values_as_array_reports_shape_and_cardinality() {
    let found = codes_for(r#"{"resourceType":"Patient","gender":["male","female"]}"#);
    assert_eq!(shape_paths(&found), ["Patient.gender"], "{found:?}");
    assert_has(&found, "CARDINALITY_002", "Patient.gender");
}

#[test]
fn json_primitive_extension_is_not_shape_checked_but_its_content_is() {
    let found = codes_for(
        r#"{"resourceType":"Patient","_birthDate":[{}],
            "name":[{"given":["A"],"_given":{"extension":{"url":"http://e","valueString":"x"}}}]}"#,
    );
    assert_eq!(
        shape_paths(&found),
        ["Patient.name[0]._given.extension"],
        "{found:?}"
    );
}

#[test]
fn xml_single_repeating_element_is_not_a_shape_error() {
    let registry = load_registry();
    let codes = xml_codes(
        &registry,
        r#"<Patient xmlns="http://hl7.org/fhir"><name><given value="A"/></name><gender value="male"/></Patient>"#,
    );
    assert!(!codes.iter().any(|c| c == "STRUCTURE_002"), "{codes:?}");
}

// ── Empty arrays (11.2-A1) ────────────────────────────────────────────────────

#[test]
fn empty_or_null_array_on_required_element_produces_cardinality_001() {
    for empty in ["[]", "[null]"] {
        for (json, path) in [
            (
                format!(
                    concat!(
                        r#"{{"resourceType":"ValueSet","status":"draft","#,
                        r#""compose":{{"include":{empty}}}}}"#,
                    ),
                    empty = empty,
                ),
                "ValueSet.compose.include",
            ),
            (
                format!(r#"{{"resourceType":"Composition","author":{empty}}}"#),
                "Composition.author",
            ),
        ] {
            let found = codes_for(&json);
            assert!(
                found.contains(&("CARDINALITY_001".into(), path.into())),
                "{json}: {found:?}"
            );
            assert!(
                !found.contains(&("CARDINALITY_003".into(), path.into())),
                "{json}: {found:?}"
            );
        }
    }
}

#[test]
fn empty_array_on_optional_element_produces_cardinality_003_warning() {
    for (json, path) in [
        (
            r#"{"resourceType":"Patient","identifier":[]}"#,
            "Patient.identifier",
        ),
        (
            r#"{"resourceType":"Patient","identifier":[null]}"#,
            "Patient.identifier",
        ),
        (
            r#"{"resourceType":"Patient","name":[{"given":[null]}]}"#,
            "Patient.name[0].given",
        ),
        (
            r#"{"resourceType":"Patient","name":[{"given":[]}]}"#,
            "Patient.name[0].given",
        ),
    ] {
        let resource = parse_json(json).unwrap();
        let found: Vec<_> = validate(&resource, &load_registry())
            .into_iter()
            .filter(|d| d.code.as_ref() == "CARDINALITY_003")
            .collect();
        assert_eq!(found.len(), 1, "{json}: {found:?}");
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(found[0].path, path);
    }
}

#[test]
fn cardinality_003_location_is_the_array_span() {
    let json = r#"{"resourceType":"Patient","identifier":[]}"#;
    let resource = parse_json(json).unwrap();
    let found = validate(&resource, &load_registry())
        .into_iter()
        .find(|d| d.code.as_ref() == "CARDINALITY_003")
        .expect("CARDINALITY_003");
    let location = found.location.expect("location");
    assert_eq!(location.offset as usize, json.find("[]").unwrap());
}

#[test]
fn empty_array_on_max_one_element_gets_structure_002_and_cardinality_003() {
    let found = codes_for(r#"{"resourceType":"Patient","gender":[]}"#);
    for code in ["STRUCTURE_002", "CARDINALITY_003"] {
        assert!(
            found.contains(&(code.into(), "Patient.gender".into())),
            "{code}: {found:?}"
        );
    }
}

#[test]
fn profile_requiring_optional_element_reports_base_003_and_profile_001() {
    // `subject` is 0..1 on Observation but 1..1 in the vitalsigns profile.
    let json = VITAL_SIGNS_HEART_RATE.replace(
        r#""subject": {"reference": "Patient/example"},"#,
        r#""subject": [],"#,
    );
    assert_ne!(json, VITAL_SIGNS_HEART_RATE);
    let found = codes_for(&json);
    for code in ["CARDINALITY_003", "CARDINALITY_001"] {
        assert!(
            found.contains(&(code.into(), "Observation.subject".into())),
            "{code}: {found:?}"
        );
    }
}

#[test]
fn partly_null_arrays_and_choice_variants_get_no_cardinality_003() {
    for (json, path) in [
        (
            r#"{"resourceType":"Patient","name":[{"given":[null,"a"]}]}"#,
            "Patient.name[0].given",
        ),
        (
            concat!(
                r#"{"resourceType":"Observation","status":"final","#,
                r#""code":{"text":"x"},"valueQuantity":[]}"#,
            ),
            "Observation.valueQuantity",
        ),
    ] {
        let found = codes_for(json);
        assert!(
            !found.iter().any(|(code, p)| {
                code == "CARDINALITY_003" || (code.starts_with("CARDINALITY") && p == path)
            }),
            "{json}: {found:?}"
        );
    }
}

#[test]
fn null_values_backed_by_primitive_extensions_are_not_empty() {
    for json in [
        concat!(
            r#"{"resourceType":"Patient","identifier":[{"value":"1"}],"#,
            r#""name":[{"given":[null],"_given":[{"id":"a"}]}]}"#,
        ),
        concat!(
            r#"{"resourceType":"Observation","code":{"text":"x"},"#,
            r#""status":[null],"_status":[{"id":"a"}]}"#,
        ),
    ] {
        let found = codes_for(json);
        assert!(
            !found
                .iter()
                .any(|(code, _)| code.starts_with("CARDINALITY")),
            "{json}: {found:?}"
        );
    }
}

// ── FHIRPath choice navigation (R5 §2.1.9.1.1: `value` finds `valueQuantity`) ─

#[test]
fn fhs_2_sees_age_given_as_a_choice_variant() {
    let registry = load_registry();
    let base = r#"{"resourceType":"FamilyMemberHistory","status":"completed",
        "patient":{"reference":"Patient/1"},"relationship":{"text":"father"},
        "estimatedAge":true"#;
    let with_age = format!(r#"{base},"ageAge":{{"value":60}}}}"#);
    assert_eq!(codes(&registry, &with_age, "INVARIANT_fhs-2"), 0);
    assert_eq!(codes(&registry, &format!("{base}}}"), "INVARIANT_fhs-2"), 1);
}

#[test]
fn mrp_2_sees_stratum_value_given_as_a_choice_variant() {
    let registry = load_registry();
    let report = |stratum: &str| {
        format!(
            r#"{{"resourceType":"MeasureReport","status":"complete","type":"summary",
            "period":{{"start":"2020-01-01"}},
            "group":[{{"stratifier":[{{"stratum":[{stratum}]}}]}}]}}"#
        )
    };
    let value = r#""valueCodeableConcept":{"text":"a"}"#;
    let component = r#""component":[{"code":{"text":"c"},"valueCodeableConcept":{"text":"v"}}]"#;
    assert_eq!(
        codes(
            &registry,
            &report(&format!("{{{value}}}")),
            "INVARIANT_mrp-2"
        ),
        0
    );
    let both = format!("{{{value},{component}}}");
    assert_eq!(codes(&registry, &report(&both), "INVARIANT_mrp-2"), 1);
}

#[test]
fn obs_6_fires_on_data_absent_reason_with_a_value_variant() {
    let registry = load_registry();
    let base = r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},
        "dataAbsentReason":{"text":"unknown"}"#;
    let with_value = format!(r#"{base},"valueQuantity":{{"value":1}}}}"#);
    assert_eq!(codes(&registry, &with_value, "INVARIANT_obs-6"), 1);
    assert_eq!(codes(&registry, &format!("{base}}}"), "INVARIANT_obs-6"), 0);
}
