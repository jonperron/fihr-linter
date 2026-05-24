/// Unit tests for the FHIRPath parser and evaluator.
use std::sync::Arc;

use indexmap::IndexMap;

use fhir_fhirpath::{Collection, Value, evaluate};

// ── Helpers ────────────────────────────────────────────────────────────────

fn make_patient() -> Collection {
    // Minimal Patient resource as a FHIRPath Value::Object
    let mut root: IndexMap<Arc<str>, Collection> = IndexMap::new();
    root.insert(
        Arc::from("resourceType"),
        vec![Value::String(Arc::from("Patient"))],
    );
    root.insert(Arc::from("id"), vec![Value::String(Arc::from("patient-1"))]);

    // name array: [{family: "Doe", given: ["John", "James"]}]
    let mut name_fields: IndexMap<Arc<str>, Collection> = IndexMap::new();
    name_fields.insert(Arc::from("family"), vec![Value::String(Arc::from("Doe"))]);
    name_fields.insert(
        Arc::from("given"),
        vec![
            Value::String(Arc::from("John")),
            Value::String(Arc::from("James")),
        ],
    );
    root.insert(
        Arc::from("name"),
        vec![Value::Object(Arc::new(name_fields))],
    );

    // active: true
    root.insert(Arc::from("active"), vec![Value::Bool(true)]);

    // birthDate
    root.insert(
        Arc::from("birthDate"),
        vec![Value::Date(Arc::from("1990-06-15"))],
    );

    // telecom: [{system: "phone", value: "+33123456789"}]
    let mut tel_fields: IndexMap<Arc<str>, Collection> = IndexMap::new();
    tel_fields.insert(Arc::from("system"), vec![Value::String(Arc::from("phone"))]);
    tel_fields.insert(
        Arc::from("value"),
        vec![Value::String(Arc::from("+33123456789"))],
    );
    root.insert(
        Arc::from("telecom"),
        vec![Value::Object(Arc::new(tel_fields))],
    );

    vec![Value::Object(Arc::new(root))]
}

fn eval(expr: &str) -> Collection {
    let ctx = make_patient();
    evaluate(expr, &ctx).expect("evaluation failed")
}

fn eval_bool(expr: &str) -> bool {
    match eval(expr).as_slice() {
        [Value::Bool(b)] => *b,
        other => panic!("expected single Bool, got: {other:?}"),
    }
}

fn eval_int(expr: &str) -> i64 {
    match eval(expr).as_slice() {
        [Value::Integer(n)] => *n,
        other => panic!("expected Integer, got: {other:?}"),
    }
}

fn eval_str(expr: &str) -> String {
    match eval(expr).as_slice() {
        [Value::String(s)] => s.to_string(),
        other => panic!("expected String, got: {other:?}"),
    }
}

// ── Literal tests ────────────────────────────────────────────────────────────

#[test]
fn literal_integer() {
    assert_eq!(evaluate("42", &[]).unwrap(), vec![Value::Integer(42)]);
}

#[test]
#[allow(clippy::approx_constant)] // Testing FHIRPath literal parsing of "3.14", not π
fn literal_decimal() {
    assert_eq!(evaluate("3.14", &[]).unwrap(), vec![Value::Decimal(3.14)]);
}

#[test]
fn literal_bool_true() {
    assert_eq!(evaluate("true", &[]).unwrap(), vec![Value::Bool(true)]);
}

#[test]
fn literal_bool_false() {
    assert_eq!(evaluate("false", &[]).unwrap(), vec![Value::Bool(false)]);
}

#[test]
fn literal_string() {
    assert_eq!(
        evaluate("'hello'", &[]).unwrap(),
        vec![Value::String(Arc::from("hello"))],
    );
}

#[test]
fn literal_null_yields_empty() {
    assert_eq!(evaluate("{}", &[]).unwrap(), vec![]);
}

// ── Path navigation ──────────────────────────────────────────────────────────

#[test]
fn path_resource_type() {
    let result = eval("resourceType");
    assert_eq!(result, vec![Value::String(Arc::from("Patient"))]);
}

#[test]
fn path_active() {
    assert_eq!(eval("active"), vec![Value::Bool(true)]);
}

#[test]
fn path_name_family() {
    assert_eq!(eval("name.family"), vec![Value::String(Arc::from("Doe"))]);
}

#[test]
fn path_name_given_multiple() {
    let result = eval("name.given");
    assert_eq!(
        result,
        vec![
            Value::String(Arc::from("John")),
            Value::String(Arc::from("James")),
        ],
    );
}

#[test]
fn path_nonexistent_yields_empty() {
    assert!(eval("gender").is_empty());
}

// ── Existence functions ──────────────────────────────────────────────────────

#[test]
fn exists_true_when_field_present() {
    assert!(eval_bool("name.exists()"));
}

#[test]
fn exists_false_when_field_absent() {
    assert!(!eval_bool("gender.exists()"));
}

#[test]
fn empty_true_when_absent() {
    assert!(eval_bool("gender.empty()"));
}

#[test]
fn empty_false_when_present() {
    assert!(!eval_bool("name.empty()"));
}

#[test]
fn count_name() {
    assert_eq!(eval_int("name.count()"), 1);
}

#[test]
fn count_given_names() {
    assert_eq!(eval_int("name.given.count()"), 2);
}

// ── where() filtering ────────────────────────────────────────────────────────

#[test]
fn where_filters_given_names() {
    let result = eval("name.given.where($this = 'John')");
    assert_eq!(result, vec![Value::String(Arc::from("John"))]);
}

#[test]
fn where_no_match_yields_empty() {
    let result = eval("name.given.where($this = 'Alice')");
    assert!(result.is_empty());
}

// ── Boolean operators ────────────────────────────────────────────────────────

#[test]
fn boolean_and_true() {
    assert!(eval_bool("true and true"));
}

#[test]
fn boolean_and_false() {
    assert!(!eval_bool("true and false"));
}

#[test]
fn boolean_or_true() {
    assert!(eval_bool("false or true"));
}

#[test]
fn boolean_not() {
    assert!(eval_bool("active.not().not()"));
}

#[test]
fn boolean_implies_true() {
    assert!(eval_bool("true implies true"));
}

#[test]
fn boolean_implies_false_antecedent() {
    // false implies anything = true
    assert!(eval_bool("false implies false"));
}

// ── Arithmetic ───────────────────────────────────────────────────────────────

#[test]
fn arith_addition() {
    assert_eq!(evaluate("2 + 3", &[]).unwrap(), vec![Value::Integer(5)]);
}

#[test]
fn arith_subtraction() {
    assert_eq!(evaluate("10 - 4", &[]).unwrap(), vec![Value::Integer(6)]);
}

#[test]
fn arith_multiplication() {
    assert_eq!(evaluate("3 * 4", &[]).unwrap(), vec![Value::Integer(12)]);
}

#[test]
fn arith_division_decimal() {
    match evaluate("7 / 2", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!((d - 3.5).abs() < 1e-10),
        other => panic!("expected Decimal, got: {other:?}"),
    }
}

#[test]
fn arith_div_integer() {
    assert_eq!(evaluate("7 div 2", &[]).unwrap(), vec![Value::Integer(3)]);
}

#[test]
fn arith_modulo() {
    assert_eq!(evaluate("7 mod 3", &[]).unwrap(), vec![Value::Integer(1)]);
}

#[test]
fn arith_unary_neg() {
    assert_eq!(evaluate("-5", &[]).unwrap(), vec![Value::Integer(-5)]);
}

#[test]
fn string_concat() {
    assert_eq!(
        evaluate("'foo' & 'bar'", &[]).unwrap(),
        vec![Value::String(Arc::from("foobar"))],
    );
}

// ── Comparisons ──────────────────────────────────────────────────────────────

#[test]
fn comparison_eq_true() {
    assert!(eval_bool("1 = 1"));
}

#[test]
fn comparison_eq_false() {
    assert!(!eval_bool("1 = 2"));
}

#[test]
fn comparison_neq() {
    assert!(eval_bool("1 != 2"));
}

#[test]
fn comparison_lt() {
    assert!(eval_bool("1 < 2"));
}

#[test]
fn comparison_gt() {
    assert!(eval_bool("2 > 1"));
}

#[test]
fn comparison_lte_equal() {
    assert!(eval_bool("2 <= 2"));
}

#[test]
fn comparison_gte_greater() {
    assert!(eval_bool("3 >= 2"));
}

#[test]
fn equivalent_strings_case_insensitive() {
    assert!(eval_bool("'Hello' ~ 'hello'"));
}

// ── String functions ─────────────────────────────────────────────────────────

#[test]
fn string_length() {
    assert_eq!(
        evaluate("'hello'.length()", &[]).unwrap(),
        vec![Value::Integer(5)],
    );
}

#[test]
fn string_starts_with() {
    assert!(eval_bool("'foobar'.startsWith('foo')"));
}

#[test]
fn string_ends_with() {
    assert!(eval_bool("'foobar'.endsWith('bar')"));
}

#[test]
fn string_contains() {
    assert!(eval_bool("'foobar'.contains('oba')"));
}

#[test]
fn string_upper() {
    assert_eq!(eval_str("'hello'.upper()"), "HELLO");
}

#[test]
fn string_lower() {
    assert_eq!(eval_str("'HELLO'.lower()"), "hello");
}

#[test]
fn string_trim() {
    assert_eq!(
        evaluate("'  hi  '.trim()", &[]).unwrap(),
        vec![Value::String(Arc::from("hi"))],
    );
}

#[test]
fn string_substring() {
    assert_eq!(
        evaluate("'hello'.substring(1, 3)", &[]).unwrap(),
        vec![Value::String(Arc::from("ell"))],
    );
}

#[test]
fn string_matches_digit_pattern() {
    assert!(matches!(
        evaluate("'abc123'.matches('[0-9]+')", &[])
            .unwrap()
            .as_slice(),
        [Value::Bool(true)]
    ));
}

#[test]
fn string_to_string() {
    assert_eq!(
        evaluate("42.toString()", &[]).unwrap(),
        vec![Value::String(Arc::from("42"))],
    );
}

// ── Type conversion ──────────────────────────────────────────────────────────

#[test]
fn to_integer_from_string() {
    assert_eq!(
        evaluate("'42'.toInteger()", &[]).unwrap(),
        vec![Value::Integer(42)],
    );
}

#[test]
#[allow(clippy::approx_constant)] // Testing FHIRPath toDecimal() of literal "3.14", not π
fn to_decimal_from_integer() {
    assert_eq!(
        evaluate("'3.14'.toDecimal()", &[]).unwrap(),
        vec![Value::Decimal(3.14)],
    );
}

#[test]
fn converts_to_integer_true() {
    assert!(eval_bool("'42'.convertsToInteger()"));
}

#[test]
fn converts_to_integer_false() {
    assert!(!eval_bool("'abc'.convertsToInteger()"));
}

// ── Math functions ────────────────────────────────────────────────────────────

#[test]
fn math_abs() {
    assert_eq!(
        evaluate("(-5).abs()", &[]).unwrap(),
        vec![Value::Integer(5)]
    );
}

#[test]
fn math_ceiling() {
    assert_eq!(
        evaluate("3.2.ceiling()", &[]).unwrap(),
        vec![Value::Integer(4)]
    );
}

#[test]
fn math_floor() {
    assert_eq!(
        evaluate("3.9.floor()", &[]).unwrap(),
        vec![Value::Integer(3)]
    );
}

#[test]
fn math_round() {
    assert_eq!(
        evaluate("3.567.round(2)", &[]).unwrap(),
        vec![Value::Decimal(3.57)]
    );
}

#[test]
fn math_sqrt() {
    match evaluate("4.sqrt()", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!((d - 2.0).abs() < 1e-10),
        other => panic!("expected Decimal(2.0), got: {other:?}"),
    }
}

// ── Collection functions ─────────────────────────────────────────────────────

#[test]
fn first_returns_first_element() {
    let result = eval("name.given.first()");
    assert_eq!(result, vec![Value::String(Arc::from("John"))]);
}

#[test]
fn last_returns_last_element() {
    let result = eval("name.given.last()");
    assert_eq!(result, vec![Value::String(Arc::from("James"))]);
}

#[test]
fn tail_skips_first() {
    let result = eval("name.given.tail()");
    assert_eq!(result, vec![Value::String(Arc::from("James"))]);
}

#[test]
fn skip_n_items() {
    assert_eq!(evaluate("(1 | 2 | 3).skip(1)", &[]).unwrap().len(), 2);
}

#[test]
fn take_n_items() {
    assert_eq!(evaluate("(1 | 2 | 3).take(2)", &[]).unwrap().len(), 2);
}

#[test]
fn distinct_removes_duplicates() {
    let col = vec![Value::Integer(1), Value::Integer(2), Value::Integer(1)];
    let result = fhir_fhirpath::evaluate("$this.distinct()", &col).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn is_distinct_false_for_duplicates() {
    let col = vec![Value::Integer(1), Value::Integer(1)];
    assert_eq!(
        fhir_fhirpath::evaluate("$this.isDistinct()", &col).unwrap(),
        vec![Value::Bool(false)],
    );
}

// ── Type operators ────────────────────────────────────────────────────────────

#[test]
fn is_type_integer() {
    assert!(eval_bool("42 is Integer"));
}

#[test]
fn as_type_filters() {
    let col = vec![
        Value::Integer(1),
        Value::String(Arc::from("a")),
        Value::Integer(2),
    ];
    let result = fhir_fhirpath::evaluate("$this.ofType(Integer)", &col).unwrap();
    assert_eq!(result, vec![Value::Integer(1), Value::Integer(2)]);
}

// ── Union / set operators ─────────────────────────────────────────────────────

#[test]
fn union_merges_deduplicates() {
    let result = evaluate("1 | 2 | 1", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(1), Value::Integer(2)]);
}

#[test]
fn in_operator() {
    assert!(eval_bool("1 in (1 | 2 | 3)"));
}

#[test]
fn contains_operator() {
    assert!(eval_bool("(1 | 2 | 3) contains 2"));
}

// ── iif ──────────────────────────────────────────────────────────────────────

#[test]
fn iif_true_branch() {
    assert_eq!(
        evaluate("iif(true, 1, 2)", &[]).unwrap(),
        vec![Value::Integer(1)]
    );
}

#[test]
fn iif_false_branch() {
    assert_eq!(
        evaluate("iif(false, 1, 2)", &[]).unwrap(),
        vec![Value::Integer(2)]
    );
}

// ── Aggregation ───────────────────────────────────────────────────────────────

#[test]
fn sum_integers() {
    let col = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
    let result = fhir_fhirpath::evaluate("$this.sum()", &col).unwrap();
    assert_eq!(result, vec![Value::Integer(6)]);
}

#[test]
fn min_integers() {
    let col = vec![Value::Integer(3), Value::Integer(1), Value::Integer(2)];
    let result = fhir_fhirpath::evaluate("$this.min()", &col).unwrap();
    assert_eq!(result, vec![Value::Integer(1)]);
}

#[test]
fn max_integers() {
    let col = vec![Value::Integer(3), Value::Integer(1), Value::Integer(2)];
    let result = fhir_fhirpath::evaluate("$this.max()", &col).unwrap();
    assert_eq!(result, vec![Value::Integer(3)]);
}

// ── Extension helper ─────────────────────────────────────────────────────────

#[test]
fn extension_lookup() {
    let mut root: IndexMap<Arc<str>, Collection> = IndexMap::new();
    let mut ext: IndexMap<Arc<str>, Collection> = IndexMap::new();
    ext.insert(
        Arc::from("url"),
        vec![Value::String(Arc::from("http://example.com/ext"))],
    );
    ext.insert(
        Arc::from("valueString"),
        vec![Value::String(Arc::from("test"))],
    );
    root.insert(Arc::from("extension"), vec![Value::Object(Arc::new(ext))]);

    let ctx = vec![Value::Object(Arc::new(root))];
    let result =
        fhir_fhirpath::evaluate("extension('http://example.com/ext').exists()", &ctx).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

// ── Parser round-trip sanity ──────────────────────────────────────────────────

#[test]
fn parse_complex_expression_does_not_panic() {
    fhir_fhirpath::parse(
        "Patient.name.where(use = 'official').family.exists() implies Patient.active = true",
    )
    .unwrap();
}

#[test]
fn parse_error_on_invalid_input() {
    assert!(fhir_fhirpath::parse("1 +").is_err());
}

// ── Join / split ─────────────────────────────────────────────────────────────

#[test]
fn split_and_join() {
    let result = evaluate("'a,b,c'.split(',').join('-')", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("a-b-c"))]);
}

// ── all() / allTrue() ─────────────────────────────────────────────────────────

#[test]
fn all_returns_true() {
    let col = vec![Value::Integer(2), Value::Integer(4), Value::Integer(6)];
    let result = fhir_fhirpath::evaluate("$this.all($this > 1)", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn all_returns_false() {
    let col = vec![Value::Integer(2), Value::Integer(0), Value::Integer(6)];
    let result = fhir_fhirpath::evaluate("$this.all($this > 1)", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

// ── Error paths ───────────────────────────────────────────────────────────────

#[test]
fn division_by_zero_float_returns_error() {
    let result = evaluate("1 / 0", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::DivisionByZero)
    ));
}

#[test]
fn modulo_by_zero_returns_error() {
    let result = evaluate("5 mod 0", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::DivisionByZero)
    ));
}

#[test]
fn div_integer_by_zero_returns_error() {
    let result = evaluate("5 div 0", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::DivisionByZero)
    ));
}

#[test]
fn arity_error_on_empty_with_arg() {
    let result = evaluate("'x'.empty(1)", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::Arity { .. })
    ));
}

#[test]
fn arity_error_on_count_with_arg() {
    let result = evaluate("'x'.count(1)", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::Arity { .. })
    ));
}

#[test]
fn arity_error_on_first_with_arg() {
    let result = evaluate("'x'.first(1)", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::Arity { .. })
    ));
}

#[test]
fn type_error_negate_string() {
    let result = evaluate("-'hello'", &[]);
    assert!(matches!(result, Err(fhir_fhirpath::EvalError::Type(_))));
}

#[test]
fn type_error_length_on_integer() {
    let result = evaluate("42.length()", &[]);
    assert!(matches!(result, Err(fhir_fhirpath::EvalError::Type(_))));
}

#[test]
fn type_error_not_on_integer() {
    let result = evaluate("42.not()", &[]);
    assert!(matches!(result, Err(fhir_fhirpath::EvalError::Type(_))));
}

#[test]
fn single_on_one_element_succeeds() {
    let result = evaluate("(42).single()", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(42)]);
}

#[test]
fn single_on_empty_returns_error() {
    let result = evaluate("{}.single()", &[]);
    assert!(matches!(result, Err(fhir_fhirpath::EvalError::Type(_))));
}

#[test]
fn single_on_two_elements_returns_error() {
    let result = evaluate("(1 | 2).single()", &[]);
    assert!(matches!(result, Err(fhir_fhirpath::EvalError::Type(_))));
}

#[test]
fn undefined_function_returns_error() {
    let result = evaluate("unknownFunction()", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::UndefinedFunction(_))
    ));
}

// ── $index ────────────────────────────────────────────────────────────────────

#[test]
fn dollar_index_returns_undefined_function_error() {
    let result = evaluate("$index", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::UndefinedFunction(_))
    ));
}

// ── Three-valued logic ────────────────────────────────────────────────────────

#[test]
fn null_and_true_yields_empty() {
    let result = evaluate("{} and true", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn null_and_false_yields_false() {
    let result = evaluate("{} and false", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

#[test]
fn null_or_false_yields_empty() {
    let result = evaluate("{} or false", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn null_or_true_yields_true() {
    let result = evaluate("{} or true", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn null_implies_false_yields_empty() {
    let result = evaluate("{} implies false", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn null_implies_true_yields_true() {
    let result = evaluate("{} implies true", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn xor_true_false_yields_true() {
    assert_eq!(
        evaluate("true xor false", &[]).unwrap(),
        vec![Value::Bool(true)]
    );
}

#[test]
fn xor_true_true_yields_false() {
    assert_eq!(
        evaluate("true xor true", &[]).unwrap(),
        vec![Value::Bool(false)]
    );
}

#[test]
fn xor_null_true_yields_empty() {
    let result = evaluate("{} xor true", &[]).unwrap();
    assert!(result.is_empty());
}

// ── String functions ──────────────────────────────────────────────────────────

#[test]
fn index_of_found_returns_char_index() {
    assert_eq!(eval_int("'hello world'.indexOf('world')"), 6);
}

#[test]
fn index_of_not_found_returns_minus_one() {
    assert_eq!(eval_int("'hello'.indexOf('xyz')"), -1);
}

#[test]
fn encode_base64_encodes_string() {
    // "hello" in standard base64 is "aGVsbG8="
    let result = evaluate("'hello'.encode('base64')", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("aGVsbG8="))]);
}

#[test]
fn decode_base64_decodes_string() {
    let result = evaluate("'aGVsbG8='.decode('base64')", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("hello"))]);
}

#[test]
fn encode_urlbase64_encodes_without_padding() {
    // "hello" in url-safe base64 (no padding) is "aGVsbG8"
    let result = evaluate("'hello'.encode('urlbase64')", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("aGVsbG8"))]);
}

#[test]
fn decode_urlbase64_decodes_string() {
    let result = evaluate("'aGVsbG8'.decode('urlbase64')", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("hello"))]);
}

#[test]
fn encode_unknown_format_returns_type_error() {
    let result = evaluate("'hello'.encode('hex')", &[]);
    assert!(matches!(result, Err(fhir_fhirpath::EvalError::Type(_))));
}

#[test]
fn decode_unknown_format_returns_type_error() {
    let result = evaluate("'68656c6c6f'.decode('hex')", &[]);
    assert!(matches!(result, Err(fhir_fhirpath::EvalError::Type(_))));
}

#[test]
fn replace_matches_substitutes_regex_pattern() {
    let result = evaluate("'hello123world'.replaceMatches('[0-9]+', '-')", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("hello-world"))]);
}

#[test]
fn substring_negative_start_yields_empty() {
    let result = evaluate("'hello'.substring(-1)", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn substring_no_length_returns_suffix() {
    let result = evaluate("'hello'.substring(2)", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("llo"))]);
}

#[test]
fn string_replace_substitutes_all_occurrences() {
    assert_eq!(eval_str("'aababc'.replace('ab', 'X')"), "aXXc");
}

// ── Math functions ────────────────────────────────────────────────────────────

#[test]
fn math_exp_integer() {
    match evaluate("1.exp()", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!((d - std::f64::consts::E).abs() < 1e-10),
        other => panic!("expected Decimal(e), got: {other:?}"),
    }
}

#[test]
fn math_ln_of_one_returns_zero() {
    match evaluate("1.ln()", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!(d.abs() < 1e-10),
        other => panic!("expected Decimal(0.0), got: {other:?}"),
    }
}

#[test]
fn math_log_base_ten() {
    match evaluate("100.log(10)", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!((d - 2.0).abs() < 1e-10),
        other => panic!("expected Decimal(2.0), got: {other:?}"),
    }
}

#[test]
fn math_power_two_to_ten() {
    match evaluate("2.power(10)", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!((d - 1024.0).abs() < 1e-10),
        other => panic!("expected Decimal(1024.0), got: {other:?}"),
    }
}

#[test]
fn math_truncate_decimal() {
    assert_eq!(
        evaluate("3.9.truncate()", &[]).unwrap(),
        vec![Value::Integer(3)]
    );
}

#[test]
fn math_truncate_integer_is_identity() {
    assert_eq!(
        evaluate("5.truncate()", &[]).unwrap(),
        vec![Value::Integer(5)]
    );
}

#[test]
fn math_sqrt_on_decimal_input() {
    match evaluate("2.0.sqrt()", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!((d - std::f64::consts::SQRT_2).abs() < 1e-10),
        other => panic!("expected Decimal(sqrt(2)), got: {other:?}"),
    }
}

#[test]
fn math_abs_decimal_value() {
    match evaluate("(-3.5).abs()", &[]).unwrap().as_slice() {
        [Value::Decimal(d)] => assert!((d - 3.5).abs() < 1e-10),
        other => panic!("expected Decimal(3.5), got: {other:?}"),
    }
}

// ── Collection functions ──────────────────────────────────────────────────────

#[test]
fn subset_of_returns_true_when_all_elements_in_other() {
    let result = evaluate("(1 | 2).subsetOf(1 | 2 | 3)", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn subset_of_returns_false_when_element_missing() {
    let result = evaluate("(1 | 4).subsetOf(1 | 2 | 3)", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

#[test]
fn superset_of_returns_true_when_contains_all() {
    let result = evaluate("(1 | 2 | 3).supersetOf(1 | 2)", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn superset_of_returns_false_when_missing_element() {
    let result = evaluate("(1 | 2).supersetOf(1 | 2 | 3)", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

#[test]
fn intersect_returns_common_elements() {
    let result = evaluate("(1 | 2 | 3).intersect(2 | 3 | 4)", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(2), Value::Integer(3)]);
}

#[test]
fn exclude_removes_elements_present_in_other() {
    let result = evaluate("(1 | 2 | 3).exclude(2 | 3)", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(1)]);
}

// ── Type operators ────────────────────────────────────────────────────────────

#[test]
fn is_type_string_returns_true() {
    assert!(eval_bool("'hello' is String"));
}

#[test]
fn is_type_boolean_returns_true() {
    assert!(eval_bool("true is Boolean"));
}

#[test]
fn is_type_decimal_returns_true() {
    assert!(eval_bool("3.14 is Decimal"));
}

#[test]
fn is_type_wrong_type_returns_false() {
    assert!(!eval_bool("42 is String"));
}

#[test]
fn as_operator_keeps_matching_type() {
    let result = evaluate("42 as Integer", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(42)]);
}

#[test]
fn as_operator_filters_non_matching_type() {
    let result = evaluate("'hello' as Integer", &[]).unwrap();
    assert!(result.is_empty());
}

// ── Navigation: children / descendants ───────────────────────────────────────

#[test]
fn children_returns_direct_field_values() {
    // Patient has resourceType, id, name, active, birthDate, telecom
    let result = eval("children()");
    assert!(!result.is_empty());
}

#[test]
fn descendants_returns_more_values_than_children() {
    // descendants goes deeper than direct children
    let children_count = eval("children()").len();
    let descendants_count = eval("descendants()").len();
    assert!(descendants_count >= children_count);
}

// ── Type conversion ───────────────────────────────────────────────────────────

#[test]
fn to_boolean_from_string_true_returns_true() {
    assert_eq!(
        evaluate("'true'.toBoolean()", &[]).unwrap(),
        vec![Value::Bool(true)]
    );
}

#[test]
fn to_boolean_from_string_false_returns_false() {
    assert_eq!(
        evaluate("'false'.toBoolean()", &[]).unwrap(),
        vec![Value::Bool(false)]
    );
}

#[test]
fn to_boolean_from_integer_one_returns_true() {
    assert_eq!(
        evaluate("1.toBoolean()", &[]).unwrap(),
        vec![Value::Bool(true)]
    );
}

#[test]
fn to_boolean_from_invalid_string_yields_empty() {
    let result = evaluate("'maybe'.toBoolean()", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn to_integer_from_bool_true_returns_one() {
    assert_eq!(
        evaluate("true.toInteger()", &[]).unwrap(),
        vec![Value::Integer(1)]
    );
}

#[test]
fn to_integer_from_bool_false_returns_zero() {
    assert_eq!(
        evaluate("false.toInteger()", &[]).unwrap(),
        vec![Value::Integer(0)]
    );
}

#[test]
fn to_integer_from_invalid_string_yields_empty() {
    let result = evaluate("'abc'.toInteger()", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn to_decimal_from_bool_true_returns_one() {
    assert_eq!(
        evaluate("true.toDecimal()", &[]).unwrap(),
        vec![Value::Decimal(1.0)]
    );
}

#[test]
fn to_date_from_date_value_returns_same() {
    let col = vec![Value::Date(Arc::from("2024-01-01"))];
    let result = fhir_fhirpath::evaluate("$this.toDate()", &col).unwrap();
    assert_eq!(result, vec![Value::Date(Arc::from("2024-01-01"))]);
}

#[test]
fn to_date_from_non_date_yields_empty() {
    let result = evaluate("42.toDate()", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn to_datetime_from_datetime_value_returns_same() {
    let col = vec![Value::DateTime(Arc::from("2024-01-01T12:00:00Z"))];
    let result = fhir_fhirpath::evaluate("$this.toDateTime()", &col).unwrap();
    assert_eq!(
        result,
        vec![Value::DateTime(Arc::from("2024-01-01T12:00:00Z"))]
    );
}

#[test]
fn to_time_from_time_value_returns_same() {
    let col = vec![Value::Time(Arc::from("T12:00:00"))];
    let result = fhir_fhirpath::evaluate("$this.toTime()", &col).unwrap();
    assert_eq!(result, vec![Value::Time(Arc::from("T12:00:00"))]);
}

#[test]
fn to_quantity_from_quantity_value_returns_same() {
    let col = vec![Value::Quantity(1.5, Arc::from("kg"))];
    let result = fhir_fhirpath::evaluate("$this.toQuantity()", &col).unwrap();
    assert_eq!(result, vec![Value::Quantity(1.5, Arc::from("kg"))]);
}

#[test]
fn converts_to_boolean_true_for_valid_string() {
    assert!(eval_bool("'true'.convertsToBoolean()"));
}

#[test]
fn converts_to_boolean_false_for_non_boolean_string() {
    assert!(!eval_bool("'maybe'.convertsToBoolean()"));
}

#[test]
fn converts_to_boolean_true_for_zero_or_one_integer() {
    assert!(eval_bool("1.convertsToBoolean()"));
    assert!(eval_bool("0.convertsToBoolean()"));
}

#[test]
fn converts_to_string_true_for_non_empty_focus() {
    assert!(eval_bool("42.convertsToString()"));
}

#[test]
fn converts_to_string_false_for_empty_collection() {
    assert!(!eval_bool("{}.convertsToString()"));
}

#[test]
fn converts_to_date_true_for_date_value() {
    let col = vec![Value::Date(Arc::from("2024-01-01"))];
    let result = fhir_fhirpath::evaluate("$this.convertsToDate()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn converts_to_date_false_for_string_value() {
    assert!(!eval_bool("'2024-01-01'.convertsToDate()"));
}

#[test]
fn converts_to_datetime_true_for_datetime_value() {
    let col = vec![Value::DateTime(Arc::from("2024-01-01T00:00:00Z"))];
    let result = fhir_fhirpath::evaluate("$this.convertsToDateTime()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn converts_to_time_true_for_time_value() {
    let col = vec![Value::Time(Arc::from("T10:00:00"))];
    let result = fhir_fhirpath::evaluate("$this.convertsToTime()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn converts_to_quantity_true_for_quantity_value() {
    let col = vec![Value::Quantity(2.0, Arc::from("mg"))];
    let result = fhir_fhirpath::evaluate("$this.convertsToQuantity()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn converts_to_decimal_true_for_integer() {
    assert!(eval_bool("42.convertsToDecimal()"));
}

// ── Miscellaneous functions ───────────────────────────────────────────────────

#[test]
fn has_value_true_when_focus_is_non_empty() {
    let result = evaluate("42.hasValue()", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn has_value_false_when_focus_is_empty() {
    let result = evaluate("{}.hasValue()", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

#[test]
fn get_value_returns_non_null_items() {
    let result = evaluate("42.getValue()", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(42)]);
}

#[test]
fn html_checks_returns_true() {
    let result = evaluate("htmlChecks()", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn comparable_true_for_numeric_collection() {
    let col = vec![Value::Integer(1), Value::Decimal(2.0)];
    let result = fhir_fhirpath::evaluate("$this.comparable()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn comparable_false_when_collection_contains_object() {
    let fields: IndexMap<Arc<str>, Collection> = IndexMap::new();
    let col = vec![Value::Object(Arc::new(fields))];
    let result = fhir_fhirpath::evaluate("$this.comparable()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

#[test]
fn type_function_returns_type_name_of_integer() {
    let result = evaluate("42.type()", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("Integer"))]);
}

#[test]
fn trace_returns_focus_unchanged() {
    let result = evaluate("42.trace('label')", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(42)]);
}

#[test]
fn iif_no_else_branch_returns_empty_when_condition_false() {
    let result = evaluate("iif(false, 1)", &[]).unwrap();
    assert!(result.is_empty());
}

// ── allTrue / anyTrue / allFalse / anyFalse ───────────────────────────────────

#[test]
fn all_true_returns_true_when_all_booleans_true() {
    let col = vec![Value::Bool(true), Value::Bool(true)];
    let result = fhir_fhirpath::evaluate("$this.allTrue()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn all_true_returns_false_when_any_false() {
    let col = vec![Value::Bool(true), Value::Bool(false)];
    let result = fhir_fhirpath::evaluate("$this.allTrue()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

#[test]
fn any_true_returns_true_when_at_least_one_true() {
    let col = vec![Value::Bool(false), Value::Bool(true)];
    let result = fhir_fhirpath::evaluate("$this.anyTrue()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn all_false_returns_true_when_all_booleans_false() {
    let col = vec![Value::Bool(false), Value::Bool(false)];
    let result = fhir_fhirpath::evaluate("$this.allFalse()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn any_false_returns_true_when_at_least_one_false() {
    let col = vec![Value::Bool(true), Value::Bool(false)];
    let result = fhir_fhirpath::evaluate("$this.anyFalse()", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

// ── select() ─────────────────────────────────────────────────────────────────

#[test]
fn select_maps_each_item_with_expression() {
    let col = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
    let result = fhir_fhirpath::evaluate("$this.select($this * 2)", &col).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], Value::Integer(2));
    assert_eq!(result[1], Value::Integer(4));
    assert_eq!(result[2], Value::Integer(6));
}

// ── exists(condition) ────────────────────────────────────────────────────────

#[test]
fn exists_with_condition_returns_true_when_match_found() {
    let col = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
    let result = fhir_fhirpath::evaluate("$this.exists($this > 2)", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn exists_with_condition_returns_false_when_no_match() {
    let col = vec![Value::Integer(1), Value::Integer(2)];
    let result = fhir_fhirpath::evaluate("$this.exists($this > 5)", &col).unwrap();
    assert_eq!(result, vec![Value::Bool(false)]);
}

// ── all() on empty collection ─────────────────────────────────────────────────

#[test]
fn all_on_empty_collection_returns_true() {
    let result = fhir_fhirpath::evaluate("$this.all($this > 1)", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

// ── aggregate() with initial value ───────────────────────────────────────────

#[test]
fn aggregate_with_initial_value_iterates_focus() {
    let col = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
    // Simplified impl: result equals last item's expression evaluated with $this=item
    let result = fhir_fhirpath::evaluate("$this.aggregate($this, 0)", &col).unwrap();
    assert_eq!(result, vec![Value::Integer(3)]);
}

// ── repeat() ─────────────────────────────────────────────────────────────────

#[test]
fn repeat_traverses_multi_level_hierarchy() {
    let mut grandchild: IndexMap<Arc<str>, Collection> = IndexMap::new();
    grandchild.insert(Arc::from("label"), vec![Value::String(Arc::from("gc"))]);

    let mut child: IndexMap<Arc<str>, Collection> = IndexMap::new();
    child.insert(
        Arc::from("child"),
        vec![Value::Object(Arc::new(grandchild))],
    );
    child.insert(Arc::from("label"), vec![Value::String(Arc::from("c"))]);

    let mut root: IndexMap<Arc<str>, Collection> = IndexMap::new();
    root.insert(Arc::from("child"), vec![Value::Object(Arc::new(child))]);

    let ctx = vec![Value::Object(Arc::new(root))];
    let result = fhir_fhirpath::evaluate("$this.repeat(child)", &ctx).unwrap();
    // root + child_obj + grandchild_obj = 3 items
    assert_eq!(result.len(), 3);
}

// ── Arithmetic edge cases ─────────────────────────────────────────────────────

#[test]
fn arith_with_empty_operand_yields_empty() {
    let result = evaluate("{} + 1", &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn comparison_with_empty_operand_yields_empty() {
    let result = evaluate("{} = 1", &[]).unwrap();
    assert!(result.is_empty());
}

// ── Index operator ────────────────────────────────────────────────────────────

#[test]
fn index_operator_accesses_first_element() {
    let result = evaluate("(1 | 2 | 3)[0]", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(1)]);
}

#[test]
fn index_operator_out_of_bounds_returns_error() {
    let result = evaluate("(1 | 2)[5]", &[]);
    assert!(matches!(
        result,
        Err(fhir_fhirpath::EvalError::IndexOutOfBounds(_))
    ));
}

// ── Parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_syntax_error_returns_parse_error_variant() {
    // Unclosed string literal cannot be parsed
    let result = fhir_fhirpath::parse("'unclosed");
    assert!(matches!(result, Err(fhir_fhirpath::ParseError::Syntax(_))));
}

// ── Quantity in context ─────────────────────────────────────────────────────

#[test]
fn quantity_value_converts_to_quantity() {
    // Verify that a Quantity value round-trips through toQuantity()
    let col = vec![Value::Quantity(70.0, Arc::from("kg"))];
    let result = fhir_fhirpath::evaluate("$this.toQuantity()", &col).unwrap();
    assert_eq!(result, vec![Value::Quantity(70.0, Arc::from("kg"))]);
}

// ── Value::Display ──────────────────────────────────────────────────────────

#[test]
fn display_null() {
    assert_eq!(Value::Null.to_string(), "null");
}

#[test]
fn display_bool_true() {
    assert_eq!(Value::Bool(true).to_string(), "true");
}

#[test]
fn display_bool_false() {
    assert_eq!(Value::Bool(false).to_string(), "false");
}

#[test]
fn display_integer() {
    assert_eq!(Value::Integer(42).to_string(), "42");
}

#[test]
fn display_decimal() {
    assert_eq!(Value::Decimal(1.5).to_string(), "1.5");
}

#[test]
fn display_string() {
    assert_eq!(Value::String(Arc::from("hello")).to_string(), "hello");
}

#[test]
fn display_date() {
    assert_eq!(
        Value::Date(Arc::from("2024-01-01")).to_string(),
        "2024-01-01"
    );
}

#[test]
fn display_datetime() {
    assert_eq!(
        Value::DateTime(Arc::from("2024-01-01T00:00:00Z")).to_string(),
        "2024-01-01T00:00:00Z"
    );
}

#[test]
fn display_time() {
    assert_eq!(Value::Time(Arc::from("10:30:00")).to_string(), "10:30:00");
}

#[test]
fn display_quantity() {
    assert_eq!(
        Value::Quantity(70.0, Arc::from("kg")).to_string(),
        "70 'kg'"
    );
}

#[test]
fn display_object() {
    let map: IndexMap<Arc<str>, Vec<Value>> = IndexMap::new();
    assert_eq!(Value::Object(Arc::new(map)).to_string(), "{…}");
}

// ── Value::type_name ────────────────────────────────────────────────────────

#[test]
fn type_name_null() {
    assert_eq!(Value::Null.type_name(), "null");
}

#[test]
fn type_name_bool() {
    assert_eq!(Value::Bool(true).type_name(), "Boolean");
}

#[test]
fn type_name_decimal() {
    assert_eq!(Value::Decimal(1.0).type_name(), "Decimal");
}

#[test]
fn type_name_string() {
    assert_eq!(Value::String(Arc::from("x")).type_name(), "String");
}

#[test]
fn type_name_date() {
    assert_eq!(Value::Date(Arc::from("2024-01-01")).type_name(), "Date");
}

#[test]
fn type_name_datetime() {
    assert_eq!(
        Value::DateTime(Arc::from("2024-01-01T00:00Z")).type_name(),
        "DateTime"
    );
}

#[test]
fn type_name_time() {
    assert_eq!(Value::Time(Arc::from("10:00:00")).type_name(), "Time");
}

#[test]
fn type_name_quantity() {
    assert_eq!(
        Value::Quantity(1.0, Arc::from("kg")).type_name(),
        "Quantity"
    );
}

#[test]
fn type_name_object() {
    let map: IndexMap<Arc<str>, Vec<Value>> = IndexMap::new();
    assert_eq!(Value::Object(Arc::new(map)).type_name(), "Object");
}

// ── Value helper methods ────────────────────────────────────────────────────

#[test]
fn as_string_returns_date_value() {
    let v = Value::Date(Arc::from("2024-01-01"));
    assert_eq!(v.as_string().as_deref(), Some("2024-01-01"));
}

#[test]
fn as_string_returns_datetime_value() {
    let v = Value::DateTime(Arc::from("2024-01-01T00:00Z"));
    assert_eq!(v.as_string().as_deref(), Some("2024-01-01T00:00Z"));
}

#[test]
fn as_string_returns_time_value() {
    let v = Value::Time(Arc::from("10:30:00"));
    assert_eq!(v.as_string().as_deref(), Some("10:30:00"));
}

#[test]
fn as_string_returns_none_for_non_string() {
    assert!(Value::Integer(1).as_string().is_none());
}

#[test]
fn as_decimal_from_integer() {
    assert_eq!(Value::Integer(3).as_decimal(), Some(3.0));
}

#[test]
fn as_decimal_from_decimal() {
    assert_eq!(Value::Decimal(1.5).as_decimal(), Some(1.5));
}

#[test]
fn as_decimal_returns_none_for_string() {
    assert!(Value::String(Arc::from("x")).as_decimal().is_none());
}

#[test]
fn as_integer_returns_value() {
    assert_eq!(Value::Integer(7).as_integer(), Some(7));
}

#[test]
fn as_integer_returns_none_for_decimal() {
    assert!(Value::Decimal(1.5).as_integer().is_none());
}

#[test]
fn is_truthy_false_for_null() {
    assert!(!Value::Null.is_truthy());
}

#[test]
fn is_truthy_false_for_false_bool() {
    assert!(!Value::Bool(false).is_truthy());
}

// ── from_parser_value ───────────────────────────────────────────────────────

#[test]
fn from_parser_value_null() {
    use fhir_fhirpath::from_parser_value;
    let result = from_parser_value(&fhir_parser::Value::Null);
    assert_eq!(result, vec![Value::Null]);
}

#[test]
fn from_parser_value_bool() {
    use fhir_fhirpath::from_parser_value;
    let result = from_parser_value(&fhir_parser::Value::Bool(true));
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn from_parser_value_integer() {
    use fhir_fhirpath::from_parser_value;
    let result = from_parser_value(&fhir_parser::Value::Integer(42));
    assert_eq!(result, vec![Value::Integer(42)]);
}

#[test]
fn from_parser_value_decimal() {
    use fhir_fhirpath::from_parser_value;
    let result = from_parser_value(&fhir_parser::Value::Decimal(3.0));
    assert_eq!(result, vec![Value::Decimal(3.0)]);
}

#[test]
fn from_parser_value_str() {
    use fhir_fhirpath::from_parser_value;
    let result = from_parser_value(&fhir_parser::Value::Str(Arc::from("hello")));
    assert_eq!(result, vec![Value::String(Arc::from("hello"))]);
}

#[test]
fn from_parser_value_array_flattens_items() {
    use fhir_fhirpath::from_parser_value;
    use fhir_parser::{Node, Span};
    let arr = fhir_parser::Value::Array(vec![
        Node {
            value: fhir_parser::Value::Integer(1),
            span: Span::default(),
        },
        Node {
            value: fhir_parser::Value::Integer(2),
            span: Span::default(),
        },
    ]);
    let result = from_parser_value(&arr);
    assert_eq!(result, vec![Value::Integer(1), Value::Integer(2)]);
}

#[test]
fn from_parser_value_object_becomes_value_object() {
    use fhir_fhirpath::from_parser_value;
    use fhir_parser::{Node, Span};
    let mut fields = IndexMap::new();
    fields.insert(
        Arc::from("name"),
        Node {
            value: fhir_parser::Value::Str(Arc::from("Alice")),
            span: Span::default(),
        },
    );
    let obj = fhir_parser::Value::Object(fields);
    let result = from_parser_value(&obj);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0], Value::Object(_)));
}

// ── resource_to_value ───────────────────────────────────────────────────────

#[test]
fn resource_to_value_maps_resource_type_and_id() {
    use fhir_fhirpath::resource_to_value;
    use fhir_parser::Resource;
    let resource = Resource {
        resource_type: Arc::from("Patient"),
        id: Some(Arc::from("p1")),
        fields: IndexMap::new(),
    };
    let v = resource_to_value(&resource);
    match &v {
        Value::Object(map) => {
            assert!(map.contains_key("resourceType"));
            assert!(map.contains_key("id"));
        }
        _ => panic!("expected Object"),
    }
}

#[test]
fn resource_to_value_without_id() {
    use fhir_fhirpath::resource_to_value;
    use fhir_parser::Resource;
    let resource = Resource {
        resource_type: Arc::from("Observation"),
        id: None,
        fields: IndexMap::new(),
    };
    let v = resource_to_value(&resource);
    match &v {
        Value::Object(map) => {
            assert!(!map.contains_key("id"));
            assert!(map.contains_key("resourceType"));
        }
        _ => panic!("expected Object"),
    }
}

#[test]
fn resource_to_value_includes_fields() {
    use fhir_fhirpath::resource_to_value;
    use fhir_parser::{Node, Resource, Span};
    let mut fields = IndexMap::new();
    fields.insert(
        Arc::from("status"),
        Node {
            value: fhir_parser::Value::Str(Arc::from("active")),
            span: Span::default(),
        },
    );
    let resource = Resource {
        resource_type: Arc::from("Patient"),
        id: None,
        fields,
    };
    let v = resource_to_value(&resource);
    match &v {
        Value::Object(map) => {
            assert!(map.contains_key("status"));
        }
        _ => panic!("expected Object"),
    }
}

// ── Temporal and Quantity literals (parser + evaluator) ──────────────────────

#[test]
fn literal_date_evaluates_to_date_value() {
    let result = evaluate("@2024-01-01", &[]).unwrap();
    assert_eq!(result, vec![Value::Date(Arc::from("2024-01-01"))]);
}

#[test]
fn literal_datetime_evaluates_to_datetime_value() {
    let result = evaluate("@2024-01-01T10:00:00", &[]).unwrap();
    assert_eq!(
        result,
        vec![Value::DateTime(Arc::from("2024-01-01T10:00:00"))]
    );
}

#[test]
fn literal_time_evaluates_to_time_value() {
    let result = evaluate("@T10:30:00", &[]).unwrap();
    assert_eq!(result, vec![Value::Time(Arc::from("10:30:00"))]);
}

#[test]
fn literal_quantity_evaluates_correctly() {
    let result = evaluate("1.5 'kg'", &[]).unwrap();
    assert_eq!(result, vec![Value::Quantity(1.5, Arc::from("kg"))]);
}

// ── Negation ─────────────────────────────────────────────────────────────────

#[test]
fn negate_decimal_value() {
    let result = evaluate("-1.5", &[]).unwrap();
    assert_eq!(result, vec![Value::Decimal(-1.5)]);
}

#[test]
fn negate_type_error_returns_error() {
    let col = vec![Value::String(Arc::from("x"))];
    let err = evaluate("-$this", &col).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::Type(_)));
}

// ── substring edge cases ──────────────────────────────────────────────────────

#[test]
fn substring_negative_start_returns_empty() {
    let result = evaluate("'hello'.substring(-1)", &[]).unwrap();
    assert_eq!(result, vec![]);
}

#[test]
fn substring_negative_length_returns_empty() {
    let result = evaluate("'hello'.substring(0, -1)", &[]).unwrap();
    assert_eq!(result, vec![]);
}

#[test]
fn substring_without_length_to_end_of_string() {
    let result = evaluate("'hello'.substring(2)", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("llo"))]);
}

// ── encode / decode ───────────────────────────────────────────────────────────

#[test]
fn encode_base64url() {
    let result = evaluate("'hello'.encode('urlbase64')", &[]).unwrap();
    // base64url of "hello" = "aGVsbG8"
    assert!(matches!(&result[0], Value::String(s) if s.as_ref() == "aGVsbG8"));
}

#[test]
fn decode_base64url() {
    let result = evaluate("'aGVsbG8'.decode('urlbase64')", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("hello"))]);
}

#[test]
fn decode_unknown_format_returns_error() {
    let err = evaluate("'x'.decode('hex')", &[]).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::Type(_)));
}

// ── Math: integer paths for ceiling / floor ───────────────────────────────────

#[test]
fn ceiling_on_integer_returns_same() {
    let result = evaluate("3.ceiling()", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(3)]);
}

#[test]
fn floor_on_integer_returns_same() {
    let result = evaluate("3.floor()", &[]).unwrap();
    assert_eq!(result, vec![Value::Integer(3)]);
}

#[test]
fn abs_on_decimal() {
    let result = evaluate("(-2.5).abs()", &[]).unwrap();
    assert_eq!(result, vec![Value::Decimal(2.5)]);
}

#[test]
fn ceiling_type_error() {
    let col = vec![Value::String(Arc::from("x"))];
    let err = evaluate("$this.ceiling()", &col).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::Type(_)));
}

#[test]
fn floor_type_error() {
    let col = vec![Value::String(Arc::from("x"))];
    let err = evaluate("$this.floor()", &col).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::Type(_)));
}

// ── length() type error ───────────────────────────────────────────────────────

#[test]
fn length_on_non_string_returns_error() {
    let col = vec![Value::Integer(5)];
    let err = evaluate("$this.length()", &col).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::Type(_)));
}

// ── single() on multi-element collection ─────────────────────────────────────

#[test]
fn single_on_multi_returns_error() {
    let col = vec![Value::Integer(1), Value::Integer(2)];
    let err = evaluate("$this.single()", &col).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::Type(_)));
}

// ── $index returns error ─────────────────────────────────────────────────────

#[test]
fn dollar_index_outside_iteration_returns_error() {
    let err = evaluate("$index", &[]).unwrap_err();
    assert!(matches!(
        err,
        fhir_fhirpath::EvalError::UndefinedFunction(_)
    ));
}

// ── parse error on empty input ───────────────────────────────────────────────

#[test]
fn parse_empty_string_returns_error() {
    let err = fhir_fhirpath::parse("").unwrap_err();
    assert!(matches!(err, fhir_fhirpath::ParseError::Syntax(_)));
}

// ── exists() propagates predicate errors ─────────────────────────────────────

#[test]
fn exists_propagates_eval_error_from_predicate() {
    // $index in a predicate should error rather than silently return false
    let col = vec![Value::Integer(1)];
    let err = evaluate("$this.exists($index > 0)", &col).unwrap_err();
    assert!(matches!(
        err,
        fhir_fhirpath::EvalError::UndefinedFunction(_)
    ));
}

// ── Concat operator ───────────────────────────────────────────────────────────

#[test]
fn concat_operator_joins_strings() {
    let result = evaluate("'foo' & 'bar'", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("foobar"))]);
}

#[test]
fn concat_with_empty_collection_treats_as_empty_string() {
    // Per FHIRPath spec, {} & 'x' = 'x'
    let result = evaluate("{} & 'x'", &[]).unwrap();
    assert_eq!(result, vec![Value::String(Arc::from("x"))]);
}

// ── Division by zero ─────────────────────────────────────────────────────────

#[test]
fn division_by_zero_returns_error() {
    let err = evaluate("1 / 0", &[]).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::DivisionByZero));
}

#[test]
fn mod_by_zero_returns_error() {
    let err = evaluate("5 mod 0", &[]).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::DivisionByZero));
}

#[test]
fn div_by_zero_returns_error() {
    let err = evaluate("5 div 0", &[]).unwrap_err();
    assert!(matches!(err, fhir_fhirpath::EvalError::DivisionByZero));
}
