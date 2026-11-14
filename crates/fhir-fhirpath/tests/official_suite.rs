//! Runs the official FHIRPath R5 test suite
//! (`tests/fixtures/r5/fhirpath/tests-fhir-r5.xml`) and reports a pass rate.
//!
//! Each case is parsed and evaluated against its `inputfile` (or an empty
//! context). A case passes when the result matches the expected outputs, or,
//! for an `invalid` expression, when parsing or evaluation fails. Outputs are
//! compared by FHIRPath type and value, in order unless `ordered="false"`.
//! Cases whose mode needs something outside FHIRPath + FHIR R5 (CDA model,
//! terminology server, Java-only document functions) are skipped and counted.
//! There is no static type check: `contextType` is ignored, strict and
//! lenient cases run alike (some pairs cannot both pass), and an `invalid`
//! case also passes when the error is an unimplemented function.
//! Inputs are typed with the R5 `Registry` (`resource_to_typed_value`), as in
//! the validator, with the validator's choice navigation (`Observation.value`
//! finds `valueQuantity`); the input resource is `%resource`.
//! The test fails when the pass count drops below `MIN_PASSED`.
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, Environment, EvalError, Value, evaluate_parsed_with_environment, parse,
    resource_to_typed_value,
};
use fhir_parser::{parse_json, parse_xml};
use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesStart, Event};

/// Regression floor (passed cases on the pinned fixtures); raise it as the
/// evaluator improves. Lowered from 757 when the fake stubs (htmlChecks,
/// precision, now, today) and the `is` operator's empty-input `false` stopped
/// passing cases for the wrong reason.
/// Raised to 817 by the operator semantics fixes, then to 839 by the N1
/// empty-input rules for string functions (F8), then to 865 by the N1
/// conversion rules (F9), then to 877 by the FHIR data model mapping (F10:
/// typed inputs, primitive extensions, hasValue), then to 881 by skip/take
/// with an empty count, then to 884 by the string function singleton rule,
/// then to 911 by the N1 conversions (toQuantity, calendar duration units,
/// Decimal toString), then to 915 by `$total` in aggregate(), then to 916
/// by `$index`, then to 943 by `%` environment variables, with the input as
/// `%resource`, and choice navigation (8 wrong-reason passes lost: 5
/// defineVariable cases now reach the unimplemented function, 3 `...Bad` cases
/// need a static type check; 2 Quantity unit-vs-no-unit `=` cases now reach
/// a real comparison gap), then to 936 because is/as/ofType with a FHIR string
/// subtype (code, id, url...) on a String is an error until `Value` carries the
/// FHIR type (7 testInheritance cases had guessed `false`), then to 948 when
/// values carry their FHIR type and is/as/ofType follow the Registry's type
/// ancestry (11 testInheritance cases, and testType11: an XML `boolean` is
/// text, typed `boolean` by its element), then to 969 by `defineVariable()`
/// (19 cases; defineVariable7 still fails on the XML `boolean` text) and an
/// unknown type name in as/ofType being an error (2 cases), then to 987 when a
/// FHIR Quantity compares as a Quantity and units that cannot be compared give
/// empty (20 cases; testQuantity3 and testNEquality24 lost wrong-reason passes:
/// `g` vs `mg` and `[lb_av]` vs `kg` need UCUM conversion, so `!=` is empty).
const MIN_PASSED: u32 = 987;

/// (run, skipped) case counts on the pinned fixtures, so loader drift or a
/// broken skip filter cannot hide behind the floor.
const EXPECTED_RUN_SKIPPED: (u32, u32) = (1226, 121);

/// Modes that need infrastructure beyond FHIRPath and FHIR R5.
const SKIPPED_MODES: [&str; 3] = ["cda", "tx", "custom"];

fn fixtures_dir() -> PathBuf {
    std::env::var_os("FHIR_FIXTURES_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"),
        PathBuf::from,
    )
}

#[derive(Default)]
struct Case {
    group: String,
    name: String,
    /// Test mode and enclosing group mode.
    modes: Vec<String>,
    input_file: Option<String>,
    /// Path evaluated on the input to give the expression's context.
    context: Option<String>,
    expression: String,
    invalid: bool,
    predicate: bool,
    unordered: bool,
    /// `(type, text)` pairs.
    outputs: Vec<(String, String)>,
}

fn attribute(element: &BytesStart<'_>, key: &str) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == key.as_bytes())
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}

/// Parse the suite XML into cases.
fn load_cases(xml: &str) -> Vec<Case> {
    let mut reader = Reader::from_str(xml);
    let (mut cases, mut group, mut group_mode) = (Vec::new(), String::new(), None);
    let mut case = Case::default();
    // Text of the open `<expression>` or `<output>`, with the output type.
    let mut text: Option<String> = None;
    let mut output_type = String::new();
    loop {
        let event = reader.read_event().expect("suite XML");
        let is_empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => match e.local_name().as_ref() {
                b"group" => {
                    group = attribute(e, "name").unwrap_or_default();
                    group_mode = attribute(e, "mode");
                }
                b"test" => {
                    case = Case {
                        group: group.clone(),
                        name: attribute(e, "name").unwrap_or_default(),
                        modes: attribute(e, "mode")
                            .into_iter()
                            .chain(group_mode.clone())
                            .collect(),
                        input_file: attribute(e, "inputfile"),
                        context: attribute(e, "context"),
                        predicate: attribute(e, "predicate").as_deref() == Some("true"),
                        unordered: attribute(e, "ordered").as_deref() == Some("false"),
                        ..Case::default()
                    };
                }
                b"expression" => {
                    case.invalid = attribute(e, "invalid").is_some();
                    text = Some(String::new());
                }
                b"output" => {
                    output_type = attribute(e, "type").expect("output type");
                    if is_empty {
                        case.outputs.push((output_type.clone(), String::new()));
                    } else {
                        text = Some(String::new());
                    }
                }
                _ => {}
            },
            Event::Text(ref t) => {
                if let Some(buffer) = text.as_mut() {
                    buffer.push_str(&t.decode().expect("suite text"));
                }
            }
            Event::CData(ref c) => {
                if let Some(buffer) = text.as_mut() {
                    buffer.push_str(std::str::from_utf8(c).expect("suite CDATA"));
                }
            }
            Event::GeneralRef(ref r) => {
                if let Some(buffer) = text.as_mut() {
                    match r.resolve_char_ref().expect("char ref") {
                        Some(c) => buffer.push(c),
                        None => buffer.push_str(
                            resolve_predefined_entity(&r.decode().expect("entity"))
                                .expect("predefined entity"),
                        ),
                    }
                }
            }
            Event::End(ref e) => match e.local_name().as_ref() {
                b"expression" => case.expression = text.take().unwrap_or_default(),
                b"output" => {
                    let value = text.take().unwrap_or_default();
                    case.outputs.push((output_type.clone(), value));
                }
                b"test" => cases.push(std::mem::take(&mut case)),
                b"group" => group_mode = None,
                _ => {}
            },
            Event::Eof => return cases,
            _ => {}
        }
    }
}

/// Canonical `type:value` key for an expected output.
fn expected_key(kind: &str, text: &str) -> String {
    match kind {
        "boolean" | "integer" | "long" => format!("{kind}:{text}"),
        "decimal" => format!("decimal:{}", text.parse::<f64>().expect("decimal output")),
        "date" | "dateTime" | "time" => format!("{kind}:{}", temporal_text(text)),
        "Quantity" => {
            let (number, unit) = text.split_once(' ').expect("Quantity output");
            let number: f64 = number.parse().expect("Quantity number");
            format!("Quantity:{number} {}", unit.trim_matches('\''))
        }
        // FHIR string-like primitives (code, id, uri...) are Strings here.
        _ => format!("string:{text}"),
    }
}

/// Canonical `type:value` key for an evaluated value.
// shortcut: decimals compare by f64 value, so precision ("1.0" vs "1") is not
// checked; upgrade when Value keeps decimal precision.
fn actual_key(value: &Value) -> String {
    match value {
        Value::Bool(b) => format!("boolean:{b}"),
        Value::Integer(n) => format!("integer:{n}"),
        Value::Long(n) => format!("long:{n}"),
        Value::Decimal(d) => format!("decimal:{d}"),
        Value::String(s) => format!("string:{s}"),
        Value::Date(s) => format!("date:{}", temporal_text(s)),
        Value::DateTime(s) => format!("dateTime:{}", temporal_text(s)),
        Value::Time(s) => format!("time:{}", temporal_text(s)),
        Value::Quantity(n, u) => format!("Quantity:{n} {u}"),
        Value::Null => "null".to_owned(),
        Value::Object(_) => "object".to_owned(),
        Value::Primitive(value, _, _) => actual_key(value),
    }
}

fn temporal_text(text: &str) -> &str {
    text.trim_start_matches('@').trim_start_matches('T')
}

/// FHIRPath predicate conversion of a result.
fn as_predicate(result: &Collection) -> Collection {
    let truth = match result.as_slice() {
        [] => false,
        [v] => !matches!(v.primitive_value(), Value::Bool(false)),
        _ => true,
    };
    vec![Value::Bool(truth)]
}

/// The R5 definitions, loaded once.
fn registry() -> Result<&'static Registry, String> {
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
            Registry::from_definitions_dir(dir).map_err(|e| format!("R5 definitions: {e}"))
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Load an input resource as a FHIRPath context.
fn load_input(file: &str) -> Result<Value, String> {
    // Inputs are bare file names next to the suite; refuse anything else.
    if Path::new(file).file_name() != Some(file.as_ref()) {
        return Err(format!("{file}: not a bare file name"));
    }
    let path = fixtures_dir().join("r5").join(file);
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{file}: {e}"))?;
    let resource = if file.ends_with(".json") {
        parse_json(&source)
    } else {
        parse_xml(&source)
    };
    let registry = registry()?;
    resource
        .map(|r| resource_to_typed_value(&r, &|parent, name| registry.child_element(parent, name)))
        .map_err(|e| format!("{file}: {e}"))
}

/// Run one case; `Err` holds the failure detail.
fn run_case(
    case: &Case,
    inputs: &mut HashMap<String, Result<Value, String>>,
) -> Result<(), String> {
    let context = match &case.input_file {
        Some(file) => {
            let input = inputs
                .entry(file.clone())
                .or_insert_with(|| load_input(file));
            vec![input.clone().map_err(|e| format!("input: {e}"))?]
        }
        None => Vec::new(),
    };
    let registry = registry()?;
    // FHIR type inheritance for is/as/ofType (`code` is a `string`).
    let is_subtype =
        |child: &str, ancestor: &str| registry.type_ancestors(child).any(|code| code == ancestor);
    // The input file is `%resource`, also when `context` selects a node in it.
    let environment = Environment {
        resource: context.first(),
        root_resource: None,
        is_subtype: Some(&is_subtype),
    };
    // Choice elements navigate as in the validator (`Observation.effective`).
    let is_choice_variant = |base: &str, suffix: &str| registry.is_choice_variant(base, suffix);
    let run = |expression: &str, input: &[Value]| {
        parse(expression).map_err(EvalError::from).and_then(|expr| {
            evaluate_parsed_with_environment(&expr, input, &is_choice_variant, &environment)
        })
    };
    let expression_context = match &case.context {
        Some(path) => run(path, &context).map_err(|e| format!("context `{path}`: {e}"))?,
        None => context.clone(),
    };
    let result = run(&case.expression, &expression_context);
    let actual = match (result, case.invalid) {
        // A missing function does not prove the expression invalid.
        (Err(EvalError::UndefinedFunction(name)), true) => {
            return Err(format!(
                "expected an error, got unimplemented function {name}"
            ));
        }
        (Err(_), true) => return Ok(()),
        (Ok(value), true) => return Err(format!("expected an error, got {value:?}")),
        (Err(e), false) => return Err(format!("error: {e}")),
        (Ok(value), false) if case.predicate => as_predicate(&value),
        (Ok(value), false) => value,
    };
    let mut got: Vec<String> = actual.iter().map(actual_key).collect();
    let mut want: Vec<String> = case
        .outputs
        .iter()
        .map(|(kind, text)| expected_key(kind, text))
        .collect();
    if case.unordered {
        got.sort();
        want.sort();
    }
    if got == want {
        Ok(())
    } else {
        Err(format!("expected {want:?}, got {got:?}"))
    }
}

#[test]
fn keys_match_across_expected_and_actual_forms() {
    assert_eq!(
        expected_key("date", "@1974-12-25"),
        actual_key(&Value::Date("1974-12-25".into()))
    );
    assert_eq!(
        expected_key("time", "@T10:30:00.000"),
        actual_key(&Value::Time("10:30:00.000".into()))
    );
    assert_eq!(
        expected_key("decimal", "1.58650000"),
        actual_key(&Value::Decimal(1.5865))
    );
    assert_eq!(
        expected_key("Quantity", "1 '1'"),
        actual_key(&Value::Quantity(1.0, "1".into()))
    );
    assert_eq!(
        expected_key("code", "male"),
        actual_key(&Value::String("male".into()))
    );
    assert_ne!(
        expected_key("date", "@1974-12-25"),
        actual_key(&Value::String("1974-12-25".into()))
    );
}

#[test]
fn load_cases_reads_group_mode_entities_and_invalid() {
    let xml = r#"<tests><group name="g" mode="tx">
        <test name="t1" inputfile="p.xml" ordered="false"><expression>1 &lt; 2</expression>
        <output type="boolean">true</output><output type="string">a</output></test>
        <test name="t2" mode="strict"><expression invalid="syntax">(</expression></test>
        </group></tests>"#;
    let cases = load_cases(xml);
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].expression, "1 < 2");
    assert_eq!(cases[0].modes, ["tx"]);
    assert!(cases[0].unordered);
    assert_eq!(cases[0].outputs.len(), 2);
    assert_eq!(cases[1].modes, ["strict", "tx"]);
    assert!(cases[1].invalid);
    assert!(cases[1].outputs.is_empty());
}

#[test]
fn run_case_judges_invalid_predicate_and_unordered_cases() {
    let case = |expression: &str, outputs: &[(&str, &str)]| Case {
        expression: expression.to_owned(),
        outputs: outputs
            .iter()
            .map(|(kind, text)| ((*kind).to_owned(), (*text).to_owned()))
            .collect(),
        ..Case::default()
    };
    let mut inputs = HashMap::new();
    let mut run = |c: &Case| run_case(c, &mut inputs).is_ok();
    assert!(run(&case("1 + 1", &[("integer", "2")])));
    assert!(!run(&case("1 + 1", &[("integer", "3")])));
    assert!(!run(&case("1 + 1", &[])));
    let invalid = Case {
        invalid: true,
        ..case("(", &[])
    };
    assert!(run(&invalid));
    let invalid_but_ok = Case {
        invalid: true,
        ..case("1", &[])
    };
    assert!(!run(&invalid_but_ok));
    let invalid_unimplemented = Case {
        invalid: true,
        ..case("1.noSuchFunction()", &[])
    };
    assert!(!run(&invalid_unimplemented));
    let escaping_input = Case {
        input_file: Some("../r5/patient-example.xml".to_owned()),
        ..case("1", &[("integer", "1")])
    };
    assert!(!run(&escaping_input));
    let predicate = Case {
        predicate: true,
        ..case("{}", &[("boolean", "false")])
    };
    assert!(run(&predicate));
    let ordered = case("2 | 1", &[("integer", "1"), ("integer", "2")]);
    assert!(!run(&ordered));
    let unordered = Case {
        unordered: true,
        ..ordered
    };
    assert!(run(&unordered));
}

#[test]
fn official_fhirpath_suite_pass_rate() {
    let path = fixtures_dir().join("r5/fhirpath/tests-fhir-r5.xml");
    let xml = match std::fs::read_to_string(&path) {
        Ok(xml) => xml,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            assert!(
                std::env::var_os("CI").is_none(),
                "fixtures submodule missing in CI: run `git submodule update --init`"
            );
            eprintln!("fixtures submodule not checked out; skipping");
            return;
        }
        Err(e) => panic!("{}: {e}", path.display()),
    };
    let cases = load_cases(&xml);
    let mut inputs = HashMap::new();
    let (mut run, mut passed, mut skipped) = (0u32, 0u32, 0u32);
    let mut failing = Vec::new();
    for case in &cases {
        if case
            .modes
            .iter()
            .any(|m| SKIPPED_MODES.contains(&m.as_str()))
        {
            skipped += 1;
            continue;
        }
        run += 1;
        let outcome = catch_unwind(AssertUnwindSafe(|| run_case(case, &mut inputs)))
            .unwrap_or_else(|_| Err("panic".to_owned()));
        match outcome {
            Ok(()) => passed += 1,
            Err(detail) => failing.push(format!(
                "FAIL {}/{} {:?}: {}",
                case.group,
                case.name,
                case.expression,
                detail.replace('\n', " "),
            )),
        }
    }
    eprintln!("{}", failing.join("\n"));
    eprintln!(
        "fhirpath suite: {passed}/{run} passed ({:.1}%), skipped (mode {SKIPPED_MODES:?}) \
         {skipped}, total {}",
        f64::from(passed) * 100.0 / f64::from(run.max(1)),
        cases.len(),
    );
    assert_eq!(
        (run, skipped),
        EXPECTED_RUN_SKIPPED,
        "case counts drifted (fixture bump or loader change): update the floors"
    );
    assert!(
        passed >= MIN_PASSED,
        "FHIRPath suite passed {passed}, below floor {MIN_PASSED}"
    );
}
