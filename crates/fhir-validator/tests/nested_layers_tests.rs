//! Invariants, terminology and references below the root (PLAN 11.2-A3),
//! against the bundled R5 definitions.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Diagnostic, Severity, validate};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn registry() -> TestResult<&'static Registry> {
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    let loaded = REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).map_err(|e| e.to_string())
    });
    loaded.as_ref().map_err(|e| e.clone().into())
}

fn diagnostics(json: &str) -> TestResult<Vec<Diagnostic>> {
    Ok(validate(&parse_json(json)?, registry()?))
}

/// `(code, path)` of the diagnostics `keep` selects.
fn pairs(json: &str, keep: impl Fn(&Diagnostic) -> bool) -> TestResult<Vec<(String, String)>> {
    Ok(diagnostics(json)?
        .into_iter()
        .filter(|d| keep(d))
        .map(|d| (d.code.to_string(), d.path))
        .collect())
}

/// `(code, path)` of every finding, `INVARIANT_SKIPPED` left out.
fn findings(json: &str) -> TestResult<Vec<(String, String)>> {
    pairs(json, |d| d.code.as_ref() != "INVARIANT_SKIPPED")
}

/// `(code, path)` of every error.
fn errors(json: &str) -> TestResult<Vec<(String, String)>> {
    pairs(json, |d| d.severity == Severity::Error)
}

fn has(found: &[(String, String)], code: &str, path: &str) -> bool {
    found.iter().any(|(c, p)| c == code && p == path)
}

#[test]
fn empty_patient_contact_violates_pat_1_and_ele_1() -> TestResult {
    let found = diagnostics(r#"{"resourceType":"Patient","contact":[{}]}"#)?;
    for key in ["INVARIANT_pat-1", "INVARIANT_ele-1"] {
        let d = found.iter().find(|d| d.code.as_ref() == key);
        let d = d.ok_or_else(|| format!("no {key}: {found:?}"))?;
        assert_eq!(d.path, "Patient.contact[0]");
        assert_eq!(d.severity, Severity::Error);
        assert!(d.location.is_some(), "{key} has no span");
    }
    Ok(())
}

#[test]
fn null_item_beside_objects_keeps_paths_aligned() -> TestResult {
    // A null has no FHIRPath value, so items are converted one by one here.
    let found =
        findings(r#"{"resourceType":"Patient","contact":[null,{},{"name":{"family":"A"}}]}"#)?;
    let pat_1: Vec<_> = found
        .iter()
        .filter(|(c, _)| c == "INVARIANT_pat-1")
        .map(|(_, p)| p.as_str())
        .collect();
    assert_eq!(pat_1, ["Patient.contact[1]"], "{found:?}");
    Ok(())
}

#[test]
fn patient_contact_with_a_name_satisfies_pat_1() -> TestResult {
    let found = errors(r#"{"resourceType":"Patient","contact":[{"name":{"family":"A"}}]}"#)?;
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}

#[test]
fn nested_quantity_violates_datatype_constraint_qty_3() -> TestResult {
    let found = findings(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},
        "component":[{"code":{"text":"y"},"valueQuantity":{"value":1,"code":"mg"}}]}"#,
    )?;
    let path = "Observation.component[0].valueQuantity";
    assert!(has(&found, "INVARIANT_qty-3", path), "{found:?}");
    Ok(())
}

#[test]
fn nested_quantity_with_system_satisfies_qty_3() -> TestResult {
    let found = errors(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},
        "component":[{"code":{"text":"y"},
        "valueQuantity":{"value":1,"code":"mg","system":"http://unitsofmeasure.org"}}]}"#,
    )?;
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}

#[test]
fn constraint_is_reported_once_when_element_and_type_both_carry_it() -> TestResult {
    // ele-1 sits on Patient.contact.name and on the HumanName root.
    let found = findings(r#"{"resourceType":"Patient","contact":[{"name":{}}]}"#)?;
    let ele_1 = found
        .iter()
        .filter(|(c, p)| c == "INVARIANT_ele-1" && p == "Patient.contact[0].name")
        .count();
    assert_eq!(ele_1, 1, "{found:?}");
    Ok(())
}

#[test]
fn extension_with_value_and_children_violates_ext_1() -> TestResult {
    let found = findings(
        r#"{"resourceType":"Patient","name":[{"family":"A","extension":[{"url":"http://x",
        "valueString":"v","extension":[{"url":"http://y","valueString":"w"}]}]}]}"#,
    )?;
    let path = "Patient.name[0].extension[0]";
    assert!(has(&found, "INVARIANT_ext-1", path), "{found:?}");
    Ok(())
}

#[test]
fn empty_primitive_extension_violates_ele_1() -> TestResult {
    let found = findings(r#"{"resourceType":"Patient","name":[{"_family":{}}]}"#)?;
    let path = "Patient.name[0].family";
    assert!(has(&found, "INVARIANT_ele-1", path), "{found:?}");
    Ok(())
}

#[test]
fn primitive_with_only_an_extension_satisfies_ele_1() -> TestResult {
    let found = errors(
        r#"{"resourceType":"Patient","name":[{"_family":
        {"extension":[{"url":"http://x","valueString":"v"}]}}]}"#,
    )?;
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}

/// An AdministrableProductDefinition whose `property[0].status` (a CodeableConcept bound
/// `required` to publication-status, a core CodeSystem) holds `code`.
fn property_status(code: &str) -> String {
    format!(
        r#"{{"resourceType":"AdministrableProductDefinition","status":"active",
        "property":[{{"type":{{"text":"t"}},"status":{{"coding":[
        {{"system":"http://hl7.org/fhir/publication-status","code":"{code}"}}]}}}}]}}"#
    )
}

#[test]
fn nested_codeable_concept_outside_required_set_is_terminology_001() -> TestResult {
    let found = findings(&property_status("bogus"))?;
    let path = "AdministrableProductDefinition.property[0].status";
    assert!(has(&found, "TERMINOLOGY_001", path), "{found:?}");
    Ok(())
}

#[test]
fn nested_codeable_concept_inside_required_set_passes() -> TestResult {
    let found = findings(&property_status("active"))?;
    let terminology = found.iter().any(|(c, _)| c.starts_with("TERMINOLOGY"));
    assert!(!terminology, "{found:?}");
    Ok(())
}

#[test]
fn code_inside_a_datatype_outside_required_set_is_terminology_001() -> TestResult {
    let found = findings(r#"{"resourceType":"Patient","name":[{"use":"bogus"}]}"#)?;
    assert!(
        has(&found, "TERMINOLOGY_001", "Patient.name[0].use"),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn nested_reference_with_disagreeing_type_is_reference_003() -> TestResult {
    let found = findings(
        r#"{"resourceType":"Patient","contact":[{"organization":
        {"reference":"Organization/1","type":"Patient"}}]}"#,
    )?;
    let path = "Patient.contact[0].organization.reference";
    assert!(has(&found, "REFERENCE_003", path), "{found:?}");
    Ok(())
}

#[test]
fn nested_reference_resolves_root_contained_ids() -> TestResult {
    let found = findings(
        r##"{"resourceType":"Patient","contained":[{"resourceType":"Organization","id":"o1"}],
        "contact":[{"organization":{"reference":"#o1"}}],
        "managingOrganization":{"reference":"#o1"}}"##,
    )?;
    let references = found.iter().any(|(c, _)| c.starts_with("REFERENCE"));
    assert!(!references, "{found:?}");
    let missing = findings(
        r##"{"resourceType":"Patient","contact":[{"organization":{"reference":"#nope"}}]}"##,
    )?;
    let path = "Patient.contact[0].organization.reference";
    assert!(has(&missing, "REFERENCE_002", path), "{missing:?}");
    Ok(())
}

/// REFERENCE_* findings for an XML Patient with `contained` and a contact referencing `target`.
fn xml_reference_findings(contained: &str, target: &str) -> TestResult<Vec<(String, String)>> {
    let xml = format!(
        r#"<Patient xmlns="http://hl7.org/fhir">{contained}
        <contact><organization><reference value="{target}"/></organization></contact>
        </Patient>"#
    );
    Ok(validate(&parse_xml(&xml)?, registry()?)
        .into_iter()
        .filter(|d| d.code.starts_with("REFERENCE"))
        .map(|d| (d.code.to_string(), d.path))
        .collect())
}

#[test]
fn xml_contained_resolves_nested_reference() -> TestResult {
    let one = r#"<contained><Organization><id value="o1"/></Organization></contained>"#;
    let two = r#"<contained><Organization><id value="o0"/></Organization></contained>
        <contained><Organization><id value="o1"/></Organization></contained>"#;
    for contained in [one, two] {
        let found = xml_reference_findings(contained, "#o1")?;
        assert!(found.is_empty(), "{found:?}");
        let missing = xml_reference_findings(contained, "#nope")?;
        assert_eq!(
            missing,
            [(
                "REFERENCE_002".to_owned(),
                "Patient.contact.organization.reference".to_owned(),
            )],
        );
    }
    Ok(())
}

#[test]
fn reference_inside_a_datatype_is_checked() -> TestResult {
    let found = findings(
        r#"{"resourceType":"Patient","identifier":[{"assigner":{"reference":"Bogus/1"}}]}"#,
    )?;
    let path = "Patient.identifier[0].assigner.reference";
    assert!(has(&found, "REFERENCE_001", path), "{found:?}");
    Ok(())
}

/// TERMINOLOGY_* `(code, path)` for a CodeSystem whose `versionAlgorithmCoding` (a choice
/// variant bound `extensible` to version-algorithm) holds `code`.
fn version_algorithm(code: &str) -> TestResult<Vec<(String, String)>> {
    let json = format!(
        r#"{{"resourceType":"CodeSystem","status":"draft","content":"complete",
        "versionAlgorithmCoding":{{"system":"http://hl7.org/fhir/version-algorithm",
        "code":"{code}"}}}}"#
    );
    pairs(&json, |d| d.code.starts_with("TERMINOLOGY"))
}

#[test]
fn bound_choice_variant_is_checked_against_its_binding() -> TestResult {
    let path = "CodeSystem.versionAlgorithmCoding".to_owned();
    assert_eq!(
        version_algorithm("bogus")?,
        [("TERMINOLOGY_001".to_owned(), path)]
    );
    assert_eq!(version_algorithm("semver")?, []);
    Ok(())
}

#[test]
fn primitive_values_pair_with_their_extensions_by_index() -> TestResult {
    // given[0] has a value, given[1] only an extension: both satisfy ele-1.
    let paired = findings(
        r#"{"resourceType":"Patient","name":[{"given":["A",null],
        "_given":[null,{"extension":[{"url":"http://x","valueString":"v"}]}]}]}"#,
    )?;
    let ele_1 = paired.iter().any(|(c, _)| c == "INVARIANT_ele-1");
    assert!(!ele_1, "{paired:?}");
    // A longer `_given` adds an item with neither value nor children.
    let longer =
        findings(r#"{"resourceType":"Patient","name":[{"given":["A"],"_given":[null,{}]}]}"#)?;
    let ele_1: Vec<_> = longer
        .iter()
        .filter(|(c, _)| c == "INVARIANT_ele-1")
        .map(|(_, p)| p.as_str())
        .collect();
    assert_eq!(ele_1, ["Patient.name[0].given[1]"], "{longer:?}");
    Ok(())
}

#[test]
fn reference_choice_variant_in_an_extension_is_checked() -> TestResult {
    let extension = |target: &str| {
        format!(
            r##"{{"resourceType":"Patient",
            "contained":[{{"resourceType":"Organization","id":"o1"}}],
            "extension":[{{"url":"http://x","valueReference":{{"reference":"{target}"}}}}]}}"##
        )
    };
    let references = |d: &Diagnostic| d.code.starts_with("REFERENCE");
    let path = "Patient.extension[0].valueReference.reference".to_owned();
    assert_eq!(
        pairs(&extension("#missing"), references)?,
        [("REFERENCE_002".to_owned(), path)],
    );
    assert_eq!(pairs(&extension("#o1"), references)?, []);
    Ok(())
}

#[test]
fn nested_skipped_constraint_names_the_node_path() -> TestResult {
    // txt-1 calls htmlChecks(), which the evaluator does not implement, so it is
    // skipped on the nested Narrative.div, at its path.
    let found = diagnostics(
        r#"{"resourceType":"Patient","text":{"status":"generated",
        "div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">x</div>"}}"#,
    )?;
    let txt_1 = found
        .iter()
        .find(|d| d.message.starts_with("constraint txt-1 not checked"))
        .ok_or_else(|| format!("no txt-1 skip: {found:?}"))?;
    assert_eq!(txt_1.code.as_ref(), "INVARIANT_SKIPPED");
    assert_eq!(txt_1.severity, Severity::Information);
    assert_eq!(txt_1.path, "Patient.text.div");
    assert!(txt_1.location.is_some(), "{txt_1:?}");
    Ok(())
}

#[test]
fn root_id_extension_pairs_with_the_lifted_id() -> TestResult {
    // The parser lifts `id` out of the fields; `_id` must still see its value.
    let found = errors(r#"{"resourceType":"Patient","id":"p1","_id":{}}"#)?;
    assert!(found.is_empty(), "{found:?}");
    let alone = findings(r#"{"resourceType":"Patient","_id":{}}"#)?;
    assert!(has(&alone, "INVARIANT_ele-1", "Patient.id"), "{alone:?}");
    Ok(())
}
