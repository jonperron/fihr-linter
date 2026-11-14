//! `=`, `~`, `<`, `>` on Quantities (N1 §6.1, §6.2): a FHIR Quantity (typed
//! by the real R5 `Registry`) and a Quantity literal are the same kind, the
//! unit is the UCUM code (or the unit text when there is no code), and units
//! that cannot be compared give empty (`~`: false).
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{Value, evaluate, resource_to_typed_value};
use fhir_parser::{Resource, parse_json, parse_xml};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("R5 definitions")
    })
}

fn typed(resource: &Resource) -> Vec<Value> {
    let registry = registry();
    vec![resource_to_typed_value(resource, &|parent, name| {
        registry.child_element(parent, name)
    })]
}

/// The result of `expr` as `Some(bool)`, or `None` when empty.
fn boolean(expr: &str, context: &[Value]) -> Option<bool> {
    match evaluate(expr, context).unwrap().as_slice() {
        [] => None,
        [Value::Bool(b)] => Some(*b),
        other => panic!("{expr}: not a Boolean: {other:?}"),
    }
}

const OBSERVATION_JSON: &str = r#"{"resourceType":"Observation","status":"final",
    "code":{"text":"weight"},"valueQuantity":{"value":185,"unit":"lbs",
    "system":"http://unitsofmeasure.org","code":"[lb_av]"}}"#;

const OBSERVATION_XML: &str = r#"<Observation xmlns="http://hl7.org/fhir">
    <status value="final"/><code><text value="weight"/></code>
    <valueQuantity><value value="185"/><unit value="lbs"/>
    <system value="http://unitsofmeasure.org"/><code value="[lb_av]"/></valueQuantity>
    </Observation>"#;

#[test]
fn fhir_quantity_compares_with_a_literal_by_its_ucum_code() {
    for context in [
        typed(&parse_json(OBSERVATION_JSON).unwrap()),
        typed(&parse_xml(OBSERVATION_XML).unwrap()),
    ] {
        let check = |expr: &str| boolean(expr, &context);
        assert_eq!(
            check("Observation.valueQuantity = 185 '[lb_av]'"),
            Some(true)
        );
        assert_eq!(
            check("185 '[lb_av]' = Observation.valueQuantity"),
            Some(true)
        );
        assert_eq!(
            check("Observation.valueQuantity = 186 '[lb_av]'"),
            Some(false)
        );
        assert_eq!(
            check("Observation.valueQuantity ~ 185 '[lb_av]'"),
            Some(true)
        );
        assert_eq!(
            check("Observation.valueQuantity < 200 '[lb_av]'"),
            Some(true)
        );
        assert_eq!(
            check("Observation.valueQuantity <= 200 '[lb_av]'"),
            Some(true)
        );
        assert_eq!(
            check("Observation.valueQuantity > 100 '[lb_av]'"),
            Some(true)
        );
        assert_eq!(
            check("Observation.valueQuantity >= 100 '[lb_av]'"),
            Some(true)
        );
        // The unit text is not the unit when there is a code.
        assert_eq!(check("Observation.valueQuantity = 185 'lbs'"), None);
        assert_eq!(check("Observation.valueQuantity ~ 185 'lbs'"), Some(false));
    }
}

const QUANTITIES: &str = r#"{"resourceType":"Parameters","parameter":[
    {"name":"no-unit","valueQuantity":{"value":1}},
    {"name":"no-unit-2","valueQuantity":{"value":1}},
    {"name":"unit-text","valueQuantity":{"value":1,"unit":"mg/dL"}},
    {"name":"ucum-code","valueQuantity":{"value":2,"unit":"mg/dL",
        "system":"http://unitsofmeasure.org","code":"mg/dL"}},
    {"name":"other-system","valueQuantity":{"value":1,"unit":"tab",
        "system":"http://snomed.info/sct","code":"385055001"}},
    {"name":"comparator","valueQuantity":{"value":1,"comparator":"<",
        "system":"http://unitsofmeasure.org","code":"mg/dL"}}]}"#;

#[test]
fn a_quantity_with_a_unit_and_one_without_are_not_comparable() {
    let context = typed(&parse_json(QUANTITIES).unwrap());
    let check = |expr: &str| boolean(expr, &context);
    let p = |i: usize| format!("Parameters.parameter[{i}].valueQuantity");
    assert_eq!(check(&format!("{} = {}", p(0), p(1))), Some(true));
    assert_eq!(check(&format!("{} ~ {}", p(0), p(1))), Some(true));
    assert_eq!(check(&format!("{} = {}", p(2), p(0))), None);
    assert_eq!(check(&format!("{} = {}", p(0), p(2))), None);
    assert_eq!(check(&format!("{} ~ {}", p(2), p(0))), Some(false));
    assert_eq!(check(&format!("{} < {}", p(0), p(2))), None);
    // Unit text and a UCUM code with the same text are the same unit.
    assert_eq!(check(&format!("{} < {}", p(2), p(3))), Some(true));
    assert_eq!(check(&format!("{} = 1 'mg/dL'", p(2))), Some(true));
}

#[test]
fn quantities_outside_the_mapping_keep_structural_comparison() {
    let context = typed(&parse_json(QUANTITIES).unwrap());
    let check = |expr: &str| boolean(expr, &context);
    // A non-UCUM code or a comparator is not a System.Quantity value.
    assert_eq!(
        check("Parameters.parameter[4].valueQuantity = 1 '385055001'"),
        Some(false)
    );
    assert_eq!(
        check("Parameters.parameter[5].valueQuantity = 1 'mg/dL'"),
        Some(false)
    );
    assert_eq!(
        check("Parameters.parameter[4].valueQuantity = Parameters.parameter[4].valueQuantity"),
        Some(true)
    );
}

#[test]
fn literal_quantities_with_units_that_cannot_be_compared_give_empty() {
    let check = |expr: &str| boolean(expr, &[]);
    assert_eq!(check("1 year = 1 'a'"), None);
    assert_eq!(check("1 year != 1 'a'"), None);
    assert_eq!(check("1 year ~ 1 'a'"), Some(true));
    assert_eq!(check("1 'kg' ~ 1 'm'"), Some(false));
    assert_eq!(check("1 'kg' < 2 'm'"), None);
    assert_eq!(check("1 year > 1 'a'"), None);
    assert_eq!(check("1 second = 1 's'"), Some(true));
    assert_eq!(check("1 year < 2 years"), Some(true));
}

#[test]
fn a_code_without_a_value_falls_back_to_the_unit_text() {
    let json = OBSERVATION_JSON.replace(
        r#""code":"[lb_av]""#,
        r#""_code":{"extension":[{"url":"http://example.org/x","valueString":"y"}]}"#,
    );
    let context = typed(&parse_json(&json).unwrap());
    assert_eq!(
        boolean("Observation.valueQuantity = 185 'lbs'", &context),
        Some(true)
    );
}

#[test]
fn an_xml_quantity_value_that_is_not_a_finite_decimal_is_not_a_quantity() {
    for text in ["NaN", "inf"] {
        let xml = OBSERVATION_XML.replace(
            r#"<value value="185"/>"#,
            &format!(r#"<value value="{text}"/>"#),
        );
        let context = typed(&parse_xml(&xml).unwrap());
        let result = evaluate("Observation.valueQuantity <= 200 '[lb_av]'", &context);
        assert!(
            !matches!(result.as_deref(), Ok([Value::Bool(true)])),
            "{text}: {result:?}"
        );
    }
}
