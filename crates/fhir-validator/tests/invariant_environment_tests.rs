//! Invariants evaluated with the FHIRPath environment: FHIR type inheritance for
//! `is` / `as` / `ofType`, and `%resource` / `%rootResource` at every depth.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Finding = (Severity, String, String);

/// `(severity, code, path)` of every diagnostic about the constraint `key`, skips included.
fn about(json: &str, key: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let registry = Registry::from_definitions_dir(dir)?;
    let resource = parse_json(json)?;
    Ok(validate(&resource, &registry)
        .into_iter()
        .filter(|d| {
            d.code.as_ref() == format!("INVARIANT_{key}")
                || (d.code.as_ref() == "INVARIANT_SKIPPED"
                    && d.message.starts_with(&format!("constraint {key} ")))
        })
        .map(|d| (d.severity, d.code.to_string(), d.path))
        .collect())
}

#[test]
fn contained_resource_referenced_through_a_canonical_satisfies_dom3() -> TestResult {
    // dom-3 asks every descendant `ofType(canonical)` / `ofType(uri)`: answering for FHIR
    // subtypes needs the type inheritance hook.
    let json = r##"{"resourceType":"Patient",
        "extension":[{"url":"http://example.org/x","valueCanonical":"#b"}],
        "contained":[{"resourceType":"Basic","id":"b","code":{"text":"x"}}]}"##;
    assert_eq!(about(json, "dom-3")?, vec![]);
    Ok(())
}

#[test]
fn contained_resources_referenced_through_canonicals_in_a_questionnaire_satisfy_dom3() -> TestResult
{
    // `vs2` points back to the container with the canonical `#`.
    let json = r##"{"resourceType":"Questionnaire","status":"active",
        "contained":[{"resourceType":"ValueSet","id":"vs","status":"active"},
            {"resourceType":"ValueSet","id":"vs2","status":"active",
             "compose":{"include":[{"valueSet":["#"]}]}}],
        "item":[{"linkId":"1","text":"q","type":"coding","answerValueSet":"#vs"}]}"##;
    assert_eq!(about(json, "dom-3")?, vec![]);
    Ok(())
}

#[test]
fn unreferenced_contained_resource_violates_dom3() -> TestResult {
    let json = r#"{"resourceType":"Patient",
        "contained":[{"resourceType":"Basic","id":"b","code":{"text":"x"}}]}"#;
    let expected = (
        Severity::Error,
        "INVARIANT_dom-3".to_owned(),
        "Patient".to_owned(),
    );
    assert_eq!(about(json, "dom-3")?, vec![expected]);
    Ok(())
}

#[test]
fn reference_in_a_contained_resource_resolves_against_the_container() -> TestResult {
    // `#` names the container and `#b` a sibling: both need %rootResource = the Patient.
    let json = r##"{"resourceType":"Patient",
        "contained":[
            {"resourceType":"Observation","id":"o","status":"final","code":{"text":"x"},
             "subject":{"reference":"#"},"focus":[{"reference":"#b"}]},
            {"resourceType":"Basic","id":"b","code":{"text":"x"}}],
        "link":[{"other":{"reference":"#o"},"type":"seealso"}]}"##;
    assert_eq!(about(json, "ref-1")?, vec![]);
    Ok(())
}

#[test]
fn reference_to_a_missing_contained_resource_violates_ref1() -> TestResult {
    let json = r##"{"resourceType":"Patient",
        "contained":[{"resourceType":"Observation","id":"o","status":"final",
            "code":{"text":"x"},"subject":{"reference":"#missing"}}],
        "link":[{"other":{"reference":"#o"},"type":"seealso"}]}"##;
    let path = "Patient.contained[0].subject".to_owned();
    assert_eq!(
        about(json, "ref-1")?,
        vec![(Severity::Error, "INVARIANT_ref-1".to_owned(), path)],
    );
    Ok(())
}

#[test]
fn bundle_entry_is_its_own_root_resource() -> TestResult {
    // Not contained: %rootResource is the entry itself, so `#` names nothing and `#c`
    // resolves against the entry's own `contained`.
    let json = r##"{"resourceType":"Bundle","type":"collection","entry":[
        {"fullUrl":"urn:uuid:0c3151bd-1cbf-4d64-b04d-cd9187a4c6e0","resource":
            {"resourceType":"Observation","status":"final","code":{"text":"x"},
             "contained":[{"resourceType":"Patient","id":"c"}],
             "subject":{"reference":"#c"},"focus":[{"reference":"#"}]}}]}"##;
    let path = "Bundle.entry[0].resource.focus[0]".to_owned();
    assert_eq!(
        about(json, "ref-1")?,
        vec![(Severity::Error, "INVARIANT_ref-1".to_owned(), path)],
    );
    Ok(())
}

#[test]
fn contained_resource_in_a_bundle_entry_has_the_entry_as_root_resource() -> TestResult {
    // %rootResource is the entry's Patient, not the Bundle: `#` and `#b` resolve there.
    let json = r##"{"resourceType":"Bundle","type":"collection","entry":[
        {"fullUrl":"urn:uuid:0c3151bd-1cbf-4d64-b04d-cd9187a4c6e0","resource":
            {"resourceType":"Patient",
             "contained":[
                {"resourceType":"Observation","id":"o","status":"final","code":{"text":"x"},
                 "subject":{"reference":"#"},"focus":[{"reference":"#b"},{"reference":"#z"}]},
                {"resourceType":"Basic","id":"b","code":{"text":"x"}}],
             "link":[{"other":{"reference":"#o"},"type":"seealso"}]}}]}"##;
    let path = "Bundle.entry[0].resource.contained[0].focus[1]".to_owned();
    assert_eq!(
        about(json, "ref-1")?,
        vec![(Severity::Error, "INVARIANT_ref-1".to_owned(), path)],
    );
    Ok(())
}
