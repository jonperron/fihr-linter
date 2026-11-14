//! XML representation rules checked against the R5 Registry (xml.html, audit 11.6.4).
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::parse_xml;
use fhir_validator::validate;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// `"<severity> <code> <path>"` for every diagnostic of a Patient with `body`, invariant
/// findings aside (a bare Patient has no narrative).
fn patient_diagnostics(body: &str) -> Vec<String> {
    let source = format!("<Patient xmlns=\"http://hl7.org/fhir\">{body}</Patient>");
    let resource = parse_xml(&source).expect("parses");
    validate(&resource, registry())
        .iter()
        .filter(|d| !d.code.starts_with("INVARIANT_"))
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
        .collect()
}

#[test]
fn xml_attribute_elements_given_as_child_elements_are_reported() {
    let cases = [
        (
            "<active><value value=\"true\"/></active>",
            "Error STRUCTURE_001 Patient.active.value",
        ),
        (
            "<gender><id value=\"g1\"/></gender>",
            "Error STRUCTURE_001 Patient._gender.id",
        ),
        (
            "<extension><url value=\"http://e.org\"/><valueBoolean value=\"true\"/></extension>",
            "Error STRUCTURE_001 Patient.extension.url",
        ),
        (
            "<name><id value=\"n1\"/><family value=\"S\"/></name>",
            "Error STRUCTURE_001 Patient.name.id",
        ),
    ];
    for (body, expected) in cases {
        let found = patient_diagnostics(body);
        assert!(found.iter().any(|d| d == expected), "{body}: {found:?}");
    }
}

#[test]
fn child_elements_given_as_attributes_are_reported() {
    let found = patient_diagnostics("<name family=\"Smith\"/>");
    assert!(
        found
            .iter()
            .any(|d| d == "Error STRUCTURE_001 Patient.name.family"),
        "{found:?}"
    );
}

#[test]
fn value_attribute_on_a_complex_element_is_reported() {
    let found = patient_diagnostics("<identifier value=\"1\"/>");
    assert!(
        found
            .iter()
            .any(|d| d == "Error TYPE_001 Patient.identifier"),
        "{found:?}"
    );
}

#[test]
fn elements_named_value_url_and_id_that_are_child_elements_pass() {
    let found = patient_diagnostics(concat!(
        "<extension url=\"http://e.org\">",
        "<valueQuantity><value value=\"1\"/></valueQuantity>",
        "</extension>",
        "<identifier id=\"i1\"><value value=\"123\"/></identifier>",
        "<birthDate id=\"b1\" value=\"2000-01-01\"/>",
    ));
    assert!(found.is_empty(), "{found:?}");
}

// xml.html: readers SHOULD trim attribute values; datatypes.html: non-string primitives
// SHALL NOT have leading or trailing whitespace. User decision: trim, and warn.
#[test]
fn attribute_whitespace_is_trimmed_with_a_warning() {
    let found = patient_diagnostics(concat!(
        "<active value=\" true \"/>",
        "<gender id=\" g1\" value=\"male \"/>",
        "<birthDate value=\"\t2000-01-01\">",
        "<extension url=\" http://e.org \"><valueBoolean value=\"true\"/></extension>",
        "</birthDate>",
    ));
    assert_eq!(
        found,
        [
            "Warning TYPE_007 Patient.active",
            "Warning TYPE_007 Patient.gender",
            "Warning TYPE_007 Patient._gender.id",
            "Warning TYPE_007 Patient.birthDate",
            "Warning TYPE_007 Patient._birthDate.extension.url",
        ],
    );
}

#[test]
fn schema_instance_namespace_is_a_structure_003_error() {
    let source = concat!(
        "<Patient xmlns=\"http://hl7.org/fhir\" ",
        "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
        "xsi:schemaLocation=\"http://hl7.org/fhir patient.xsd\"><active value=\"true\"/></Patient>",
    );
    let resource = parse_xml(source).expect("parses");
    let found: Vec<String> = validate(&resource, registry())
        .iter()
        .filter(|d| !d.code.starts_with("INVARIANT_"))
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
        .collect();
    assert_eq!(found, ["Error STRUCTURE_003 Patient"]);
}

#[test]
fn every_schema_instance_element_is_reported_at_its_location() {
    let source = concat!(
        "<Patient xmlns=\"http://hl7.org/fhir\" ",
        "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\n",
        "<active xsi:type=\"x\" value=\"true\"/></Patient>",
    );
    let resource = parse_xml(source).expect("parses");
    let lines: Vec<u32> = validate(&resource, registry())
        .iter()
        .filter(|d| d.code.as_ref() == "STRUCTURE_003")
        .filter_map(|d| d.location.map(|span| span.line))
        .collect();
    assert_eq!(lines, [1, 2]);
}

// xml.html: "Elements must always appear in the order documented".
#[test]
fn sibling_elements_out_of_snapshot_order_are_reported() {
    let cases = [
        (
            "<gender value=\"male\"/><active value=\"true\"/>",
            vec!["Error STRUCTURE_004 Patient.active"],
        ),
        (
            "<name><given value=\"a\"/><family value=\"b\"/><given value=\"c\"/></name>",
            vec!["Error STRUCTURE_004 Patient.name.family"],
        ),
        // A choice variant takes its base element's place (deceased[x] after birthDate).
        (
            "<deceasedBoolean value=\"false\"/><birthDate value=\"2000-01-01\"/>",
            vec!["Error STRUCTURE_004 Patient.birthDate"],
        ),
        // Inside a backbone element.
        (
            "<contact><gender value=\"male\"/><name><family value=\"x\"/></name></contact>",
            vec!["Error STRUCTURE_004 Patient.contact.name"],
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(patient_diagnostics(body), expected, "{body}");
    }
}

#[test]
fn elements_in_snapshot_order_pass() {
    let found = patient_diagnostics(concat!(
        "<meta><versionId value=\"1\"/></meta>",
        "<extension url=\"http://e.org\"><valueString value=\"x\"/></extension>",
        "<identifier><system value=\"http://e.org\"/><value value=\"1\"/></identifier>",
        "<active value=\"true\"/>",
        "<name><use value=\"official\"/><family value=\"S\"/>",
        "<given value=\"a\"/><given value=\"b\"/></name>",
        "<gender value=\"male\"/>",
        "<birthDate id=\"b1\" value=\"2000-01-01\">",
        "<extension url=\"http://e.org/a\"><valueBoolean value=\"true\"/></extension>",
        "<extension url=\"http://e.org/b\"><valueBoolean value=\"true\"/></extension>",
        "</birthDate>",
        "<deceasedBoolean value=\"false\"/>",
        "<multipleBirthInteger value=\"2\"/>",
        "<contact><name><family value=\"x\"/></name><gender value=\"male\"/></contact>",
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn elements_of_a_contained_resource_out_of_order_are_reported() {
    let found = patient_diagnostics(concat!(
        "<contained><Patient><id value=\"c1\"/>",
        "<gender value=\"male\"/><active value=\"true\"/></Patient></contained>",
    ));
    assert!(
        found
            .iter()
            .any(|d| d == "Error STRUCTURE_004 Patient.contained.active"),
        "{found:?}"
    );
}

// The resource id is lifted out of the fields by the parser (#89) but is trimmed like any
// other attribute value (#71).
#[test]
fn trimmed_resource_id_gets_a_warning() {
    let cases = [
        "<id value=\" p1 \"/>",
        "<id value=\"p1 \"><extension url=\"http://e.org\"><valueBoolean value=\"true\"/></extension></id>",
    ];
    for body in cases {
        assert_eq!(
            patient_diagnostics(body),
            ["Warning TYPE_007 Patient.id"],
            "{body}"
        );
    }
}

/// Every diagnostic, invariants included, as `"<code> <path>"`.
fn all_codes(resource: &fhir_parser::Resource) -> Vec<String> {
    validate(resource, registry())
        .iter()
        .filter(|d| d.code.as_ref() != "INVARIANT_dom-6")
        .map(|d| format!("{} {}", d.code, d.path))
        .collect()
}

// xml.html: "Attributes cannot be empty", "FHIR elements are never empty"; json.html:
// "Objects are never empty". ele-1 (every depth) and the type checks already report these,
// once per node.
#[test]
fn empty_elements_objects_and_attributes_are_reported_once() {
    let xml = |body: &str| {
        parse_xml(&format!(
            "<Patient xmlns=\"http://hl7.org/fhir\">{body}</Patient>"
        ))
        .expect("parses")
    };
    let json = |text: &str| fhir_parser::parse_json(text).expect("parses");
    let cases = [
        (xml("<name/>"), "INVARIANT_ele-1 Patient.name"),
        (
            xml("<contact><relationship/></contact>"),
            "INVARIANT_ele-1 Patient.contact.relationship",
        ),
        (
            xml("<name><family value=\"\"/></name>"),
            "TYPE_003 Patient.name.family",
        ),
        (
            json(r#"{"resourceType":"Patient","name":[{}]}"#),
            "INVARIANT_ele-1 Patient.name[0]",
        ),
        (
            json(r#"{"resourceType":"Patient","maritalStatus":{}}"#),
            "INVARIANT_ele-1 Patient.maritalStatus",
        ),
    ];
    for (resource, expected) in cases {
        let found = all_codes(&resource);
        let path = expected.split(' ').nth(1).expect("path");
        let at_node: Vec<_> = found
            .iter()
            .filter(|d| d.split(' ').nth(1) == Some(path))
            .collect();
        assert_eq!(at_node, [expected], "{found:?}");
    }
}
