//! Slicing rules of R5 core profiles (profiling.html#slicing): `closed` (`PROFILE_007`),
//! `type` discriminators, slice children applied to their items, and discriminators that
//! cannot be evaluated (`SLICING_SKIPPED`).
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Diagnostic, Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

fn diagnostics(json: &str) -> Vec<Diagnostic> {
    validate(&parse_json(json).expect("test JSON parses"), registry())
}

/// Paths of every diagnostic with `code` and `severity`.
fn paths(json: &str, code: &str, severity: Severity) -> Vec<String> {
    diagnostics(json)
        .into_iter()
        .filter(|d| d.code.as_ref() == code && d.severity == severity)
        .map(|d| d.path.to_string())
        .collect()
}

/// A history Bundle (R5 `history-bundle`, entry slicing closed on `request.method`).
fn history(entries: &str) -> String {
    format!(
        r#"{{"resourceType":"Bundle","id":"b",
        "meta":{{"profile":["http://hl7.org/fhir/StructureDefinition/history-bundle"]}},
        "type":"history","entry":[{entries}]}}"#
    )
}

fn entry(method: &str, extra: &str) -> String {
    format!(
        r#"{{{extra}"request":{{"method":"{method}","url":"Patient/p"}},
        "response":{{"status":"200"}}}}"#
    )
}

const FULL_URL: &str = r#""fullUrl":"http://example.org/fhir/Patient/p","#;

#[test]
fn closed_slicing_rejects_an_item_matching_no_slice() {
    let json = history(&entry("HEAD", FULL_URL));
    assert_eq!(
        paths(&json, "PROFILE_007", Severity::Error),
        ["Bundle.entry[0]"]
    );
}

#[test]
fn closed_slicing_accepts_items_matching_a_slice() {
    let json = history(&[entry("GET", FULL_URL), entry("DELETE", FULL_URL)].join(","));
    assert!(paths(&json, "PROFILE_007", Severity::Error).is_empty());
    assert!(paths(&json, "SLICING_SKIPPED", Severity::Information).is_empty());
}

#[test]
fn slice_children_apply_to_the_items_of_the_slice() {
    // history-bundle: `get` needs fullUrl (min 1); `delete` forbids resource (max 0).
    let delete = entry(
        "DELETE",
        r#""fullUrl":"http://example.org/fhir/Patient/p",
        "resource":{"resourceType":"Patient","id":"p"},"#,
    );
    let json = history(&[entry("GET", ""), delete].join(","));
    assert_eq!(
        paths(&json, "CARDINALITY_001", Severity::Error),
        ["Bundle.entry[0].fullUrl"]
    );
    assert_eq!(
        paths(&json, "CARDINALITY_002", Severity::Error),
        ["Bundle.entry[1].resource"]
    );
}

/// A GuidanceResponse claiming R5 `cdshooksguidanceresponse` (module[x] sliced closed by
/// `type` on `$this`, one slice `moduleUri`).
fn guidance_response(module: &str) -> String {
    format!(
        r#"{{"resourceType":"GuidanceResponse","id":"g",
        "meta":{{"profile":["http://hl7.org/fhir/StructureDefinition/cdshooksguidanceresponse"]}},
        "requestIdentifier":{{"system":"urn:ietf:rfc:3986","value":"urn:uuid:1"}},
        "identifier":[{{"system":"urn:ietf:rfc:3986","value":"urn:uuid:2"}}],
        "extension":[{{"url":"http://hl7.org/fhir/StructureDefinition/cqf-cdsHooksEndpoint",
        "valueUri":"http://example.org/cds-services/s"}}],
        "status":"success",{module}}}"#
    )
}

#[test]
fn type_discriminator_rejects_a_choice_type_outside_the_closed_slices() {
    let json = guidance_response(r#""moduleCanonical":"http://example.org/Library/x""#);
    assert_eq!(
        paths(&json, "PROFILE_007", Severity::Error),
        ["GuidanceResponse.moduleCanonical"]
    );
    assert_eq!(
        paths(&json, "PROFILE_006", Severity::Error),
        ["GuidanceResponse.module[x]"]
    );
}

#[test]
fn type_discriminator_matches_the_slice_type() {
    let json = guidance_response(r#""moduleUri":"http://example.org/service""#);
    assert!(paths(&json, "PROFILE_007", Severity::Error).is_empty());
    assert!(paths(&json, "PROFILE_006", Severity::Error).is_empty());
    assert!(paths(&json, "SLICING_SKIPPED", Severity::Information).is_empty());
}

#[test]
fn unsupported_discriminator_is_reported_not_passed_silently() {
    // lipidprofile slices DiagnosticReport.result on `resolve().code`.
    let json = r#"{"resourceType":"DiagnosticReport","id":"d",
        "meta":{"profile":["http://hl7.org/fhir/StructureDefinition/lipidprofile"]},
        "status":"final",
        "code":{"coding":[{"system":"http://loinc.org","code":"57698-3"}]},
        "result":[{"reference":"Observation/o"}]}"#;
    assert_eq!(
        paths(json, "SLICING_SKIPPED", Severity::Information),
        ["DiagnosticReport.result"]
    );
}
