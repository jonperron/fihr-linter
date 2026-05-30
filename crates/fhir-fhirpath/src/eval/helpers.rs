use std::sync::Arc;

use super::ArithOp;
use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

/// Check that a function received the expected number of arguments.
pub(super) fn check_arity(name: &str, args: &[Expr], expected: usize) -> Result<(), EvalError> {
    if args.len() != expected {
        return Err(EvalError::Arity {
            name: name.to_owned(),
            expected,
            got: args.len(),
        });
    }
    Ok(())
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

pub(super) fn numeric_op(lv: &Value, rv: &Value, op: ArithOp) -> Result<Collection, EvalError> {
    let a = lv
        .as_decimal()
        .ok_or_else(|| EvalError::Type("arithmetic requires numbers".into()))?;
    let b = rv
        .as_decimal()
        .ok_or_else(|| EvalError::Type("arithmetic requires numbers".into()))?;
    let result = match op {
        ArithOp::Add => a + b,
        ArithOp::Sub => a - b,
        ArithOp::Mul => a * b,
        ArithOp::Div => {
            if b == 0.0 {
                return Err(EvalError::DivisionByZero);
            }
            a / b
        }
        ArithOp::Mod => {
            if b == 0.0 {
                return Err(EvalError::DivisionByZero);
            }
            a % b
        }
        ArithOp::DivInt => unreachable!(),
    };
    // Preserve Long type when both operands are Long and result is exact.
    if matches!(lv, Value::Long(_))
        && matches!(rv, Value::Long(_))
        && result.fract() == 0.0
        && matches!(
            op,
            ArithOp::Add | ArithOp::Sub | ArithOp::Mul | ArithOp::Mod
        )
    {
        return Ok(vec![Value::Long(result as i64)]);
    }
    // Preserve integer type when both operands are integers and result is exact.
    if matches!(lv, Value::Integer(_))
        && matches!(rv, Value::Integer(_))
        && result.fract() == 0.0
        && matches!(
            op,
            ArithOp::Add | ArithOp::Sub | ArithOp::Mul | ArithOp::Mod
        )
    {
        return Ok(vec![Value::Integer(result as i64)]);
    }
    Ok(vec![Value::Decimal(result)])
}

// ── Equality / comparison ─────────────────────────────────────────────────────

pub(super) fn values_equal(a: &Value, b: &Value) -> Option<bool> {
    match (a, b) {
        (Value::Null, Value::Null) => None,
        (Value::Null, _) | (_, Value::Null) => None,
        (Value::Bool(x), Value::Bool(y)) => Some(x == y),
        (Value::Integer(x), Value::Integer(y)) => Some(x == y),
        (Value::Long(x), Value::Long(y)) => Some(x == y),
        (Value::Long(x), Value::Integer(y)) | (Value::Integer(y), Value::Long(x)) => Some(x == y),
        (Value::Decimal(x), Value::Decimal(y)) => Some(x == y),
        (Value::Integer(x), Value::Decimal(y)) => Some(*x as f64 == *y),
        (Value::Decimal(x), Value::Integer(y)) => Some(*x == *y as f64),
        (Value::Long(x), Value::Decimal(y)) => Some(*x as f64 == *y),
        (Value::Decimal(x), Value::Long(y)) => Some(*x == *y as f64),
        (Value::String(x), Value::String(y)) => Some(x == y),
        (Value::Date(x), Value::Date(y))
        | (Value::DateTime(x), Value::DateTime(y))
        | (Value::Time(x), Value::Time(y)) => Some(x == y),
        (Value::Quantity(xv, xu), Value::Quantity(yv, yu)) => Some(xv == yv && xu == yu),
        _ => None,
    }
}

pub(super) fn values_equivalent(a: &Value, b: &Value) -> bool {
    // For ~: null ~ null is true, case-insensitive string comparison
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::String(x), Value::String(y)) => x.to_lowercase() == y.to_lowercase(),
        _ => values_equal(a, b).unwrap_or(false),
    }
}

pub(super) fn compare_values(a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Ok(x.cmp(y)),
        (Value::Long(x), Value::Long(y)) => Ok(x.cmp(y)),
        (Value::Long(x), Value::Integer(y)) => Ok(x.cmp(y)),
        (Value::Integer(x), Value::Long(y)) => Ok(x.cmp(y)),
        (Value::Decimal(x), Value::Decimal(y)) => {
            Ok(x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
        }
        (Value::Integer(x), Value::Decimal(y)) => Ok((*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Decimal(x), Value::Integer(y)) => Ok(x
            .partial_cmp(&(*y as f64))
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Long(x), Value::Decimal(y)) => Ok((*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Decimal(x), Value::Long(y)) => Ok(x
            .partial_cmp(&(*y as f64))
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
        (Value::Date(x), Value::Date(y))
        | (Value::DateTime(x), Value::DateTime(y))
        | (Value::Time(x), Value::Time(y)) => Ok(x.cmp(y)),
        (Value::Quantity(xv, xu), Value::Quantity(yv, yu)) => {
            if xu != yu {
                return Err(EvalError::Type(format!(
                    "cannot compare quantities with different units: '{xu}' vs '{yu}'"
                )));
            }
            Ok(xv.partial_cmp(yv).unwrap_or(std::cmp::Ordering::Equal))
        }
        (a, b) => Err(EvalError::Type(format!(
            "cannot compare {} and {}",
            a.type_name(),
            b.type_name()
        ))),
    }
}

// ── Boolean helpers ───────────────────────────────────────────────────────────

pub(super) fn collection_to_bool(col: &Collection) -> Option<bool> {
    match col.as_slice() {
        [] => None,
        [Value::Bool(b)] => Some(*b),
        _ => None,
    }
}

pub(super) fn bool_col(v: Option<bool>) -> Collection {
    match v {
        Some(b) => vec![Value::Bool(b)],
        None => vec![],
    }
}

pub(super) fn three_valued_and(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

pub(super) fn three_valued_or(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

// ── Type helpers ──────────────────────────────────────────────────────────────

pub(super) fn value_is_type(v: &Value, type_name: &str) -> bool {
    match type_name {
        "Boolean" | "boolean" | "bool" => matches!(v, Value::Bool(_)),
        "Integer" | "integer" => matches!(v, Value::Integer(_)),
        "Long" | "long" => matches!(v, Value::Long(_)),
        "Decimal" | "decimal" => matches!(v, Value::Decimal(_)),
        "String" | "string" => matches!(v, Value::String(_)),
        "Date" | "date" => matches!(v, Value::Date(_)),
        "DateTime" | "dateTime" => matches!(v, Value::DateTime(_)),
        "Time" | "time" => matches!(v, Value::Time(_)),
        "Quantity" | "quantity" => matches!(v, Value::Quantity(_, _)),
        _ => false,
    }
}

// ── String helpers ────────────────────────────────────────────────────────────

pub(super) fn string_or_empty(col: &Collection) -> Arc<str> {
    col.first()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| Arc::from(""))
}

pub(super) fn string_transform(
    focus: &[Value],
    f: impl Fn(&str) -> String,
) -> Result<Collection, EvalError> {
    match focus.first().and_then(|v| v.as_string()) {
        Some(s) => Ok(vec![Value::String(Arc::from(f(&s).as_str()))]),
        None => Ok(vec![]),
    }
}

/// Single-pass JSON string unescape (FHIRPath 3.0 `unescape('json')`).
pub(super) fn json_unescape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => result.push('\\'),
            Some('"') => result.push('"'),
            Some('/') => result.push('/'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

// ── Argument extraction ───────────────────────────────────────────────────────

pub(super) fn single_integer(col: &Collection, context: &str) -> Result<i64, EvalError> {
    match col.first() {
        Some(Value::Integer(n)) => Ok(*n),
        Some(v) => Err(EvalError::Type(format!(
            "{context}: expected Integer, got {}",
            v.type_name()
        ))),
        None => Err(EvalError::Type(format!(
            "{context}: expected Integer, got empty"
        ))),
    }
}

pub(super) fn eval_integer_arg(col: &Collection, context: &str) -> Result<i64, EvalError> {
    single_integer(col, context)
}

pub(super) fn eval_string_arg(col: &Collection, context: &str) -> Result<Arc<str>, EvalError> {
    match col.first().and_then(|v| v.as_string()) {
        Some(s) => Ok(s),
        None => Err(EvalError::Type(format!("{context}: expected String"))),
    }
}

pub(super) fn eval_type_name(expr: &Expr, _ctx: &[Value]) -> Result<String, EvalError> {
    // For ofType(TypeName), the argument is an identifier, not evaluated
    match expr {
        Expr::Ident(name) => Ok(name.clone()),
        _ => Err(EvalError::Type(
            "ofType() argument must be a type name identifier".into(),
        )),
    }
}

// ── Tree traversal ────────────────────────────────────────────────────────────

pub(super) fn collect_descendants(v: &Value, out: &mut Collection) {
    if let Value::Object(fields) = v {
        for children in fields.values() {
            for child in children {
                out.push(child.clone());
                collect_descendants(child, out);
            }
        }
    }
}
