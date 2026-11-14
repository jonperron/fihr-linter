//! `is`, `as` and `ofType` on values that carry their FHIR type, answered
//! through FHIR inheritance from the real R5 `Registry` (`code` is a
//! `string`, `canonical` a `uri`), and the error without that knowledge.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, Environment, EvalError, Value, evaluate_parsed_with_environment, parse,
    resource_to_typed_value,
};
use fhir_parser::{Resource, parse_json, parse_xml};

type TestResult = Result<(), Box<dyn Error>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("R5 definitions")
    })
}

fn is_subtype(child: &str, ancestor: &str) -> bool {
    registry().type_ancestors(child).any(|a| a == ancestor)
}

fn typed(resource: &Resource) -> Value {
    let registry = registry();
    resource_to_typed_value(resource, &|parent, name| {
        registry.child_element(parent, name)
    })
}

fn questionnaire() -> Result<Value, Box<dyn Error>> {
    let resource = parse_json(
        r#"{"resourceType":"Questionnaire","id":"q1","status":"draft",
        "url":"http://example.org/q","title":"http://example.org/q",
        "derivedFrom":["http://example.org/base"],
        "_version":{"extension":[{"url":"http://example.org/x","valueBoolean":true}]}}"#,
    )?;
    Ok(typed(&resource))
}

fn eval(expression: &str, with_hook: bool) -> Result<Collection, Box<dyn Error>> {
    eval_on(questionnaire()?, expression, with_hook)
}

fn eval_on(input: Value, expression: &str, with_hook: bool) -> Result<Collection, Box<dyn Error>> {
    let input = [input];
    let environment = Environment {
        is_subtype: with_hook.then_some(&is_subtype as &dyn Fn(&str, &str) -> bool),
        ..Environment::default()
    };
    Ok(evaluate_parsed_with_environment(
        &parse(expression)?,
        &input,
        &|_, _| false,
        &environment,
    )?)
}

fn strings(result: &Collection) -> Vec<String> {
    result
        .iter()
        .map(|v| v.as_string().map(|s| s.to_string()).unwrap_or_default())
        .collect()
}

#[test]
fn of_type_canonical_finds_the_canonical_and_not_a_string() -> TestResult {
    let result = eval("Questionnaire.descendants().ofType(canonical)", true)?;
    assert_eq!(strings(&result), ["http://example.org/base"]);
    Ok(())
}

#[test]
fn of_type_uri_finds_the_uri_and_its_subtypes() -> TestResult {
    let result = eval("Questionnaire.descendants().ofType(uri)", true)?;
    // url (uri), derivedFrom (canonical) and the extension's url (uri).
    assert_eq!(
        strings(&result),
        [
            "http://example.org/q",
            "http://example.org/base",
            "http://example.org/x",
        ]
    );
    Ok(())
}

#[test]
fn a_code_is_a_string() -> TestResult {
    assert_eq!(
        eval("Questionnaire.status.is(string)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(
        eval("Questionnaire.status is code", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(
        strings(&eval("Questionnaire.status.as(code)", true)?),
        ["draft"]
    );
    Ok(())
}

#[test]
fn a_string_is_not_a_code() -> TestResult {
    assert_eq!(
        eval("Questionnaire.title.is(code)", true)?,
        [Value::Bool(false)]
    );
    assert_eq!(eval("Questionnaire.title as code", true)?, []);
    assert_eq!(
        eval("Questionnaire.url.is(url)", true)?,
        [Value::Bool(false)]
    );
    Ok(())
}

#[test]
fn the_resource_id_carries_its_fhir_type() -> TestResult {
    assert_eq!(eval("Questionnaire.id.is(id)", true)?, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn a_primitive_without_a_value_keeps_its_fhir_type() -> TestResult {
    assert_eq!(
        eval("Questionnaire.version.is(string)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(
        eval("Questionnaire.version.is(code)", true)?,
        [Value::Bool(false)]
    );
    Ok(())
}

#[test]
fn system_type_names_keep_the_system_rules_on_typed_values() -> TestResult {
    assert_eq!(
        eval("Questionnaire.status.is(String)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(eval("Questionnaire.status.ofType(integer)", true)?, []);
    Ok(())
}

#[test]
fn an_xml_primitive_with_extensions_keeps_its_fhir_type() -> TestResult {
    let resource = parse_xml(
        r#"<Questionnaire xmlns="http://hl7.org/fhir"><status value="draft">
        <extension url="http://example.org/x"><valueBoolean value="true"/></extension>
        </status></Questionnaire>"#,
    )?;
    let result = eval_on(typed(&resource), "Questionnaire.status.is(code)", true)?;
    assert_eq!(result, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn a_literal_keeps_the_system_type_rules() -> TestResult {
    assert_eq!(eval("'draft'.is(String)", true)?, [Value::Bool(true)]);
    assert!(eval("'draft'.is(code)", true).is_err());
    Ok(())
}

#[test]
fn without_the_hook_a_fhir_string_subtype_is_an_error() -> TestResult {
    for expression in [
        "Questionnaire.status.is(code)",
        "Questionnaire.title.is(code)",
        "Questionnaire.descendants().ofType(canonical)",
    ] {
        let error = eval(expression, false).err();
        let error = error.as_deref().and_then(|e| e.downcast_ref::<EvalError>());
        assert!(
            matches!(error, Some(EvalError::Type(_))),
            "{expression}: {error:?}"
        );
    }
    Ok(())
}

#[test]
fn an_unknown_type_name_is_an_error() -> TestResult {
    // testFHIRPathAsFunction23/24: `string1` is neither a FHIR nor a System type.
    for expression in [
        "Questionnaire.status.as(string1)",
        "Questionnaire.status.ofType(string1)",
        "Questionnaire.status.is(string1)",
        "Questionnaire.status is string1",
        "Questionnaire as Questionnaire1",
        // The name is checked even when the input is empty.
        "{}.as(string1)",
        "{}.ofType(string1)",
        "{} is string1",
    ] {
        let error = eval(expression, true).err();
        let error = error.as_deref().and_then(|e| e.downcast_ref::<EvalError>());
        assert!(
            matches!(error, Some(EvalError::Type(_))),
            "{expression}: {error:?}"
        );
    }
    // Known types still answer: a resource type and a FHIR complex type.
    assert_eq!(
        eval("Questionnaire.is(Questionnaire)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(eval("Questionnaire.status.ofType(HumanName)", true)?, []);
    assert!(eval("Questionnaire.status.is(Any)", true).is_ok());
    Ok(())
}
