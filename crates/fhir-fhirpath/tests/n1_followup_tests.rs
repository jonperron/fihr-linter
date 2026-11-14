/// FHIRPath N1 follow-ups (PLAN §11.3): skip/take arguments, the string
/// function singleton rule, conversions, date literals, `$total` and `$index`.
///
/// Expressions come from `tests-fhir-r5.xml` where a case exists.
use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn integers(values: &[i64]) -> Vec<Value> {
    values.iter().map(|n| Value::Integer(*n)).collect()
}

// ── skip / take ──────────────────────────────────────────────────────────────

// N1 §4.4.1: a single-input function passed an empty argument gives empty.
#[test]
fn skip_and_take_with_an_empty_count_yield_empty() {
    for expr in [
        "(0 | 1 | 2).skip({})", // testSkip7
        "{}.skip({})",          // testSkip8
        "(0 | 1 | 2).take({})", // testTake8
        "{}.take({})",          // testTake9
    ] {
        assert_eq!(eval(expr), vec![], "{expr}");
    }
    assert_eq!(eval("(0 | 1 | 2).skip(1)"), integers(&[1, 2]));
    assert_eq!(eval("(0 | 1 | 2).take(2)"), integers(&[0, 1]));
}

// N1 §5: an argument with several items is an error, not its first item.
#[test]
fn skip_and_take_with_several_counts_are_errors() {
    for expr in [
        "(0 | 1 | 2).skip(1 | 2)",
        "(0 | 1 | 2).take(1 | 2)",
        "(0 | 1 | 2).skip('1')",
    ] {
        assert!(evaluate(expr, &[]).is_err(), "{expr}");
    }
}

// ── String functions: singleton input ────────────────────────────────────────

/// Every N1 §5.6 string function, called with a valid argument list.
const STRING_CALLS: [&str; 20] = [
    "indexOf('a')",
    "lastIndexOf('a')",
    "substring(0)",
    "startsWith('a')",
    "endsWith('a')",
    "contains('a')",
    "upper()",
    "lower()",
    "replace('a', 'b')",
    "matches('a')",
    "matchesFull('a')",
    "replaceMatches('a', 'b')",
    "length()",
    "trim()",
    "split(',')",
    "encode('base64')",
    "decode('base64')",
    "escape('html')",
    "unescape('html')",
    "substring(0, 1)",
];

// N1 §5.6: "If there is more than one item, or an item that is not a
// String, the evaluation of the expression will end and signal an error".
#[test]
fn string_functions_on_several_items_are_errors() {
    for call in STRING_CALLS {
        let expr = format!("('YQ==' | 'Yg==').{call}");
        assert!(evaluate(&expr, &[]).is_err(), "{expr}");
    }
}

#[test]
fn string_functions_on_a_non_string_item_are_errors() {
    for call in STRING_CALLS {
        for input in ["1", "true", "@2014-01-01"] {
            let expr = format!("{input}.{call}");
            assert!(evaluate(&expr, &[]).is_err(), "{expr}");
        }
    }
}

#[test]
fn string_functions_on_one_string_still_work() {
    for call in STRING_CALLS {
        let expr = format!("'YQ=='.{call}");
        assert!(evaluate(&expr, &[]).is_ok(), "{expr}");
    }
    assert_eq!(eval("'abc'.upper()"), [Value::String("ABC".into())]);
    assert_eq!(eval("'a,b'.split(',').count()"), integers(&[2]));
}

// ── Date literals ────────────────────────────────────────────────────────────

// N1 grammar: the DATE token is not validated by the lexer, "that task is
// for the parser or interpreter"; N1 §4.1.5: dates run @0001-01-01 to
// @9999-12-31.
#[test]
fn date_literals_that_are_not_calendar_dates_are_syntax_errors() {
    for expr in [
        "@2023-02-29",
        "@2023-04-31",
        "@2023-13",
        "@2023-00-10",
        "@2023-01-00",
        "@0000",
        "@2023-02-29T10:00",
        "@2023-13T",
    ] {
        assert!(
            matches!(evaluate(expr, &[]), Err(fhir_fhirpath::EvalError::Parse(_))),
            "{expr}"
        );
    }
    for expr in [
        "@2024-02-29",
        "@2000-02-29",
        "@2023-12-31T23:59",
        "@2023-12",
        "@0001",
    ] {
        assert_eq!(eval(expr).len(), 1, "{expr}");
    }
}

// ── Conversions ──────────────────────────────────────────────────────────────

fn quantity(value: f64, unit: &str) -> Vec<Value> {
    vec![Value::Quantity(value, unit.into())]
}

fn boolean(b: bool) -> Vec<Value> {
    vec![Value::Bool(b)]
}

fn string(s: &str) -> Vec<Value> {
    vec![Value::String(s.into())]
}

// N1 §5.5.7 toQuantity: Integer, Decimal, Quantity, Boolean, and Strings in
// the form `(\+|-)?\d+(\.\d+)?\s*('unit'|calendar keyword)?`.
#[test]
fn to_quantity_converts_numbers_booleans_and_quantity_strings() {
    assert_eq!(eval("1.toQuantity()"), quantity(1.0, "1"));
    assert_eq!(eval("1.5.toQuantity()"), quantity(1.5, "1"));
    assert_eq!(eval("true.toQuantity()"), quantity(1.0, "1"));
    assert_eq!(eval("false.toQuantity()"), quantity(0.0, "1"));
    assert_eq!(eval("'1'.toQuantity()"), quantity(1.0, "1")); // testStringIntegerLiteralToQuantity
    assert_eq!(eval("'-1.5'.toQuantity()"), quantity(-1.5, "1"));
    assert_eq!(eval("'4 days'.toQuantity()"), quantity(4.0, "day"));
    assert_eq!(
        eval(r"'10 \'mg[Hg]\''.toQuantity()"),
        quantity(10.0, "mg[Hg]")
    );
    assert_eq!(eval("4 'mg'.toQuantity()"), quantity(4.0, "mg"));
    // testStringDecimalLiteralToQuantity
    assert_eq!(eval("'1.0'.toQuantity() ~ 1 '1'"), boolean(true));
    for expr in [
        "'abc'.toQuantity()",
        "'1 foo'.toQuantity()",
        "'1.'.toQuantity()",
        "@2014.toQuantity()",
        "{}.toQuantity()",
    ] {
        assert_eq!(eval(expr), vec![], "{expr}");
    }
    assert!(evaluate("(1 | 2).toQuantity()", &[]).is_err());
}

// N1 §5.5.7: without UCUM conversion, a different unit argument gives empty
// (toQuantity) or false (convertsToQuantity).
#[test]
fn to_quantity_with_a_unit_keeps_only_the_same_unit() {
    assert_eq!(eval("4 'mg'.toQuantity('mg')"), quantity(4.0, "mg"));
    assert_eq!(eval("'4 days'.toQuantity('day')"), quantity(4.0, "day"));
    assert_eq!(eval("'4 days'.toQuantity('days')"), quantity(4.0, "day"));
    assert_eq!(eval("4 'mg'.toQuantity('g')"), vec![]);
    assert_eq!(eval("4 'mg'.convertsToQuantity('mg')"), boolean(true));
    assert_eq!(eval("4 'mg'.convertsToQuantity('g')"), boolean(false));
}

#[test]
fn converts_to_quantity_follows_to_quantity() {
    for expr in [
        "1.convertsToQuantity()",           // testIntegerLiteralConvertsToQuantity
        "1.0.convertsToQuantity()",         // testDecimalLiteralConvertsToQuantity
        "'1'.convertsToQuantity()",         // testStringIntegerLiteralConvertsToQuantity
        "'1 day'.convertsToQuantity()",     // testStringQuantityLiteralConvertsToQuantity
        r"'1 \'wk\''.convertsToQuantity()", // testStringQuantityWeekConvertsToQuantity
        "true.convertsToQuantity()",        // testBooleanLiteralConvertsToQuantity
    ] {
        assert_eq!(eval(expr), boolean(true), "{expr}");
    }
    assert_eq!(eval("'abc'.convertsToQuantity()"), boolean(false));
    assert_eq!(eval("{}.convertsToQuantity()"), vec![]);
    assert!(evaluate("(1 | 2).convertsToQuantity()", &[]).is_err());
}

// N1 grammar: `unit : dateTimePrecision | pluralDateTimePrecision | STRING`;
// the calendar unit is represented as its singular keyword.
#[test]
fn calendar_duration_quantity_literals_parse() {
    assert_eq!(eval("4 days"), quantity(4.0, "day"));
    assert_eq!(eval("1 year"), quantity(1.0, "year"));
    assert_eq!(eval("2.5 milliseconds"), quantity(2.5, "millisecond"));
    // testStringQuantityLiteralToQuantity
    assert_eq!(eval("'1 day'.toQuantity() = 1 day"), boolean(true));
    assert_eq!(eval("'1 month'.toQuantity() ~ 1 month"), boolean(true));
    assert!(evaluate("1 fortnight", &[]).is_err());
    assert!(evaluate("1 dayz", &[]).is_err());
}

// N1 §6.1.1 / §6.1.2: calendar durations equal definite durations only for
// seconds and below (above, they are not comparable: empty, official
// testEquality36); they are equivalent to all of them.
#[test]
fn calendar_durations_against_ucum_units() {
    assert_eq!(eval("1 second = 1 's'"), boolean(true));
    assert_eq!(eval("1 millisecond = 1 'ms'"), boolean(true));
    assert_eq!(eval("1 year = 1 'a'"), vec![]);
    assert_eq!(eval("1 year ~ 1 'a'"), boolean(true));
    assert_eq!(eval("1 week ~ 1 'wk'"), boolean(true));
    assert_eq!(eval("1 week ~ 1 'd'"), boolean(false));
    assert_eq!(eval("1 'g' ~ 1 'mg'"), boolean(false));
}

// toLong is not in N1 (FHIRPath 3.0 STU); it follows the N1 §5.5 conversion
// pattern of toInteger: singleton input, empty when not convertible.
#[test]
fn to_long_follows_the_conversion_pattern() {
    assert_eq!(eval("'12'.toLong()"), vec![Value::Long(12)]);
    assert_eq!(eval("1.toLong()"), vec![Value::Long(1)]);
    assert_eq!(eval("true.toLong()"), vec![Value::Long(1)]);
    assert_eq!(eval("1.5.toLong()"), vec![]);
    assert_eq!(eval("'1.5'.toLong()"), vec![]);
    assert_eq!(eval("1.5.convertsToLong()"), boolean(false));
    assert_eq!(eval("'12'.convertsToLong()"), boolean(true));
    assert_eq!(eval("{}.convertsToLong()"), vec![]);
    assert!(evaluate("(1 | 2).toLong()", &[]).is_err());
    assert!(evaluate("(1 | 2).convertsToLong()", &[]).is_err());
}

// N1 §5.5.8 toString: Decimal renders as `(\+|-)?\d+(.\d+)?`, kept distinct
// from Integer; Quantity as `value 'unit'`; several items are an error.
#[test]
fn to_string_renders_per_the_n1_table() {
    assert_eq!(eval("1.0.toString()"), string("1.0")); // testDecimalLiteralToString
    assert_eq!(eval("0.0.toString()"), string("0.0")); // testToString4
    assert_eq!(eval("1.5.toString()"), string("1.5"));
    assert_eq!(eval("(-2.0).toString()"), string("-2.0"));
    assert_eq!(eval("1.toString()"), string("1"));
    assert_eq!(eval("1 'wk'.toString()"), string("1 'wk'")); // testQuantityLiteralWkToString
    assert_eq!(eval("4 days.toString()"), string("4 'day'"));
    assert_eq!(eval("@2014-12-14.toString()"), string("2014-12-14"));
    assert_eq!(eval("{}.toString()"), vec![]);
    assert!(evaluate("(1 | 2).toString()", &[]).is_err());
    assert!(evaluate("(1 | 2).convertsToString()", &[]).is_err());
}

// ── aggregate / $total ───────────────────────────────────────────────────────

// N1 §7.1: "$total is set to init, or empty ({ }) if no init value is
// supplied, and is set to the result of the aggregator expression after
// every iteration."
#[test]
fn aggregate_carries_total_through_each_iteration() {
    for expr in [
        "(1|2|3|4|5|6|7|8|9).aggregate($this+$total, 0) = 45", // testAggregate1
        "(1|2|3|4|5|6|7|8|9).aggregate($this+$total, 2) = 47", // testAggregate2
        // testAggregate3, testAggregate4
        "(1|2|3|4|5|6|7|8|9).aggregate(iif($total.empty(), $this, \
         iif($this < $total, $this, $total))) = 1",
        "(1|2|3|4|5|6|7|8|9).aggregate(iif($total.empty(), $this, \
         iif($this > $total, $this, $total))) = 9",
    ] {
        assert_eq!(eval(expr), boolean(true), "{expr}");
    }
    assert_eq!(eval("{}.aggregate($this + $total, 7)"), integers(&[7]));
    assert_eq!(eval("(1 | 2).aggregate($total + $this)"), vec![]);
    // An inner aggregate has its own $total.
    assert_eq!(
        eval("(1 | 2).aggregate($total + (10 | 20).aggregate($total + $this, 0), 0)"),
        integers(&[60])
    );
}

// ── $index ───────────────────────────────────────────────────────────────────

fn strings(values: &[&str]) -> Vec<Value> {
    values.iter().map(|s| Value::String((*s).into())).collect()
}

// N1 §5: expression parameters "may refer to the special $this and $index
// elements, which represent the item from the input collection currently
// under evaluation, and its index in the collection".
#[test]
fn index_is_the_position_of_the_item_under_evaluation() {
    assert_eq!(
        eval("('a' | 'b' | 'c').select($index)"),
        integers(&[0, 1, 2])
    );
    assert_eq!(
        eval("('a' | 'b' | 'c').where($index > 0)"),
        strings(&["b", "c"])
    );
    assert_eq!(eval("('a' | 'b').all($index < 2)"), boolean(true));
    assert_eq!(eval("('a' | 'b').all($index < 1)"), boolean(false));
    assert_eq!(eval("('a' | 'b').exists($index = 1)"), boolean(true));
    assert_eq!(eval("('a' | 'b').exists($index = 2)"), boolean(false));
    assert_eq!(
        eval("(5 | 6 | 7).aggregate($total + $index, 0)"),
        integers(&[3])
    );
    // repeat: each round's items are indexed within that round.
    assert_eq!(
        eval("(1).repeat(iif($index = 0 and $this < 3, $this + 1, {}))"),
        integers(&[2, 3])
    );
    // testIndex shape: iif inside select sees the select's $index.
    assert_eq!(
        eval("('x' | 'y' | 'z').select(iif($this = 'z', $index, {}))"),
        integers(&[2])
    );
}

#[test]
fn index_is_scoped_to_the_innermost_iteration() {
    assert_eq!(
        eval("(1 | 2).select((10 | 20 | 30).where($index = 2))"),
        integers(&[30, 30])
    );
    assert_eq!(
        eval("(1 | 2).select((10 | 20).count() + $index)"),
        integers(&[2, 3])
    );
}

#[test]
fn index_outside_an_iteration_is_an_error() {
    assert!(evaluate("$index", &[]).is_err());
    assert!(evaluate("(1 | 2).first() + $index", &[]).is_err());
}

#[test]
fn total_outside_aggregate_is_an_error() {
    assert!(evaluate("$total", &[]).is_err());
    assert!(evaluate("(1 | 2).select($total)", &[]).is_err());
}

// A digit string beyond f64 is not a Decimal (N1 §4.1.4 Decimal is finite).
#[test]
fn digit_strings_beyond_f64_do_not_convert() {
    let huge = format!("'1{}'", "0".repeat(400));
    for function in ["toQuantity()", "toDecimal()"] {
        let expr = format!("{huge}.{function}");
        assert_eq!(eval(&expr), vec![], "{expr}");
    }
    assert_eq!(
        eval(&format!("{huge}.convertsToQuantity()")),
        boolean(false)
    );
}

// Gaps found in review: malformed units and arguments of the conversions.
#[test]
fn calendar_unit_with_trailing_garbage_is_a_parse_error() {
    for expr in ["5 daysx", "5 days days"] {
        assert!(
            matches!(evaluate(expr, &[]), Err(fhir_fhirpath::EvalError::Parse(_))),
            "{expr}"
        );
    }
}

#[test]
fn to_quantity_with_a_bad_unit_argument() {
    assert_eq!(eval("4 'mg'.toQuantity({})"), vec![]);
    assert!(evaluate("4 'mg'.toQuantity(1)", &[]).is_err());
    assert!(evaluate("4 'mg'.convertsToQuantity(1)", &[]).is_err());
}

#[test]
fn index_inside_iif_outside_an_iteration_is_an_error() {
    assert!(evaluate("iif(true, $index, 0)", &[]).is_err());
}

// N1 §5: sort's key is an expression parameter, so `$index` is the position
// of the item being sorted, not an enclosing iteration's.
#[test]
fn sort_key_sees_its_own_index() {
    assert_eq!(
        eval("(30 | 10 | 20).sort(-$index)"),
        integers(&[20, 10, 30])
    );
    assert_eq!(
        eval("(1 | 2).select((30 | 10 | 20).sort(-$index))"),
        integers(&[20, 10, 30, 20, 10, 30])
    );
}

// A decimal literal beyond f64 is infinite; it has no Quantity or String form.
#[test]
fn non_finite_decimals_do_not_convert() {
    let huge = format!("1{}.0", "0".repeat(400));
    for function in ["toQuantity()", "toString()"] {
        let expr = format!("{huge}.{function}");
        assert_eq!(eval(&expr), vec![], "{expr}");
    }
}
