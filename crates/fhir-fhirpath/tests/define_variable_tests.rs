//! `defineVariable(name [, expr])` (FHIRPath 3.0, used by the R5 suite): the
//! variable is visible to the rest of its expression chain only, and
//! redefining a variable in scope, or a system variable, is an error.
use std::error::Error;

use fhir_fhirpath::{Collection, EvalError, Value, evaluate, resource_to_value};
use fhir_parser::parse_json;

type TestResult = Result<(), Box<dyn Error>>;

fn patient() -> Result<[Value; 1], Box<dyn Error>> {
    Ok([resource_to_value(&parse_json(
        r#"{"resourceType":"Patient","id":"example","active":true,"name":[
        {"use":"official","family":"Chalmers","given":["Peter","James"]},
        {"use":"usual","given":["Jim"]},
        {"use":"maiden","family":"Windsor","given":["Peter","James"]}]}"#,
    )?)])
}

fn eval(expression: &str) -> Result<Collection, Box<dyn Error>> {
    Ok(evaluate(expression, &patient()?)?)
}

fn strings(expression: &str) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(eval(expression)?
        .iter()
        .map(|v| v.as_string().map(|s| s.to_string()).unwrap_or_default())
        .collect())
}

fn assert_error(expression: &str) -> TestResult {
    let error = eval(expression).err();
    let error = error.as_deref().and_then(|e| e.downcast_ref::<EvalError>());
    assert!(
        matches!(error, Some(EvalError::Type(_))),
        "{expression}: {error:?}"
    );
    Ok(())
}

#[test]
fn a_variable_is_visible_to_the_rest_of_the_chain() -> TestResult {
    assert_eq!(
        strings("defineVariable('v1', 'value1').select(%v1)")?,
        ["value1"]
    );
    assert_eq!(
        strings("defineVariable('n1', name.first()).select(%n1.given)")?,
        ["Peter", "James"]
    );
    // defineVariable8: through later steps, next to a second variable.
    assert_eq!(
        strings(
            "defineVariable('v1', 'value1').select(%v1).trace('data')\
             .defineVariable('v2', 'value2').select($this & ':' & %v1 & '-' & %v2)"
        )?,
        ["value1:value1-value2"]
    );
    Ok(())
}

#[test]
fn define_variable_returns_its_input_and_defaults_to_it() -> TestResult {
    assert_eq!(
        eval("defineVariable('n1', name.first()).active")?,
        [Value::Bool(true)]
    );
    // defineVariable13: the value is computed on the whole input collection.
    assert_eq!(
        strings(
            "Patient.name.defineVariable('n2', skip(1).first())\
             .defineVariable('res', %n2.given + %n2.given).select(%res)"
        )?,
        ["JimJim", "JimJim", "JimJim"]
    );
    assert_eq!(
        strings("name.first().defineVariable('n').select(%n.family)")?,
        ["Chalmers"]
    );
    Ok(())
}

#[test]
fn each_branch_and_argument_has_its_own_scope() -> TestResult {
    // defineVariable4: the same name in two union branches.
    assert_eq!(
        strings(
            "defineVariable('n1', name.first()).select(%n1.given) \
             | defineVariable('n1', name.skip(1).first()).select(%n1.given)"
        )?,
        ["Peter", "James", "Jim"]
    );
    // defineVariable15: outer variables are visible inside arguments.
    assert_eq!(
        strings(
            "defineVariable('root', 'r1-').select(defineVariable('v1', 'v1')\
             .defineVariable('v2', 'v2').select(%v1 | %v2)).select(%root & $this)"
        )?,
        ["r1-v1", "r1-v2"]
    );
    // defineVariable19: the name is an expression; arguments do not collide.
    assert_eq!(
        strings(
            "defineVariable(defineVariable('param','ppp').select(%param), \
             defineVariable('param','value').select(%param)).select(%ppp)"
        )?,
        ["value"]
    );
    assert_eq!(
        strings(
            "'aaa'.replace(defineVariable('param', 'aaa').select(%param), \
             defineVariable('param','bbb').select(%param))"
        )?,
        ["bbb"]
    );
    Ok(())
}

#[test]
fn a_variable_out_of_scope_is_an_error() -> TestResult {
    // defineVariable9: defined in the other union branch.
    assert_error(
        "defineVariable('n1', name.first()).active \
         | defineVariable('n2', name.skip(1).first()).select(%n1.given)",
    )?;
    // defineVariable16: defined inside an argument, used after it.
    assert_error(
        "defineVariable('root', 'r1-').select(defineVariable('v1', 'v1')\
         .defineVariable('v2', 'v2').select(%v1 | %v2)).select(%root & $this & %v1)",
    )
}

#[test]
fn redefining_a_variable_in_scope_is_an_error() -> TestResult {
    assert_error("defineVariable('v1').defineVariable('v1').select(%v1)")?;
    assert_error("defineVariable('context', 'oops')")?;
    assert_error("defineVariable('resource')")?;
    assert_error("defineVariable('vs-x')")
}

#[test]
fn the_name_must_be_a_single_string() -> TestResult {
    assert_error("defineVariable(1)")?;
    assert_error("defineVariable({})")?;
    assert_error("defineVariable(name.given)")
}

#[test]
fn a_variable_is_visible_inside_iterating_arguments() -> TestResult {
    assert_eq!(
        eval("defineVariable('x', 1).select((1 | 2).aggregate($total + %x, 0))")?,
        [Value::Integer(2)]
    );
    assert_eq!(
        strings("defineVariable('f', 'Jim').name.where(given = %f).use")?,
        ["usual"]
    );
    Ok(())
}

#[test]
fn define_variable_takes_one_or_two_arguments() {
    assert!(matches!(
        eval("defineVariable()")
            .err()
            .as_deref()
            .and_then(|e| e.downcast_ref::<EvalError>()),
        Some(EvalError::Arity { .. })
    ));
    assert!(eval("defineVariable('a', 1, 2)").is_err());
}

#[test]
fn a_variable_survives_an_indexer() -> TestResult {
    assert_eq!(
        strings("defineVariable('n', name[1]).name[0].select(%n.given)")?,
        ["Jim"]
    );
    assert_eq!(strings("defineVariable('i', 1).name[%i].given")?, ["Jim"]);
    Ok(())
}
