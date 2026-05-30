use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::check_arity;

impl Evaluator {
    /// Handle type conversion and type-checking functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_type_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        _ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_type_conv(name, args, focus).transpose()
    }

    #[allow(clippy::too_many_lines)]
    fn try_type_conv(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            // ── Integer ───────────────────────────────────────────────────
            "toInteger" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Bool(b)) => vec![Value::Integer(if *b { 1 } else { 0 })],
                    Some(Value::String(s)) => match s.parse::<i64>() {
                        Ok(n) => vec![Value::Integer(n)],
                        Err(_) => vec![],
                    },
                    None => vec![],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "toInteger() cannot convert {}",
                            v.type_name()
                        )));
                    }
                }
            }
            "convertsToInteger" => {
                check_arity(name, args, 0)?;
                let result = match focus.first() {
                    Some(Value::Integer(_)) | Some(Value::Bool(_)) => true,
                    Some(Value::String(s)) => s.parse::<i64>().is_ok(),
                    _ => false,
                };
                vec![Value::Bool(result)]
            }

            // ── Long ──────────────────────────────────────────────────────
            "toLong" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Long(n)) => vec![Value::Long(*n)],
                    Some(Value::Integer(n)) => vec![Value::Long(*n)],
                    Some(Value::Bool(b)) => vec![Value::Long(if *b { 1 } else { 0 })],
                    Some(Value::String(s)) => match s.parse::<i64>() {
                        Ok(n) => vec![Value::Long(n)],
                        Err(_) => vec![],
                    },
                    None => vec![],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "toLong() cannot convert {}",
                            v.type_name()
                        )));
                    }
                }
            }
            "convertsToLong" => {
                check_arity(name, args, 0)?;
                let result = match focus.first() {
                    Some(Value::Long(_) | Value::Integer(_) | Value::Bool(_)) => true,
                    Some(Value::String(s)) => s.parse::<i64>().is_ok(),
                    _ => false,
                };
                vec![Value::Bool(result)]
            }

            // ── Decimal ───────────────────────────────────────────────────
            "toDecimal" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Decimal(d)) => vec![Value::Decimal(*d)],
                    Some(Value::Integer(n)) => vec![Value::Decimal(*n as f64)],
                    Some(Value::Bool(b)) => vec![Value::Decimal(if *b { 1.0 } else { 0.0 })],
                    Some(Value::String(s)) => match s.parse::<f64>() {
                        Ok(d) => vec![Value::Decimal(d)],
                        Err(_) => vec![],
                    },
                    None => vec![],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "toDecimal() cannot convert {}",
                            v.type_name()
                        )));
                    }
                }
            }
            "convertsToDecimal" => {
                check_arity(name, args, 0)?;
                let result = match focus.first() {
                    Some(Value::Decimal(_) | Value::Integer(_) | Value::Bool(_)) => true,
                    Some(Value::String(s)) => s.parse::<f64>().is_ok(),
                    _ => false,
                };
                vec![Value::Bool(result)]
            }

            // ── Boolean ───────────────────────────────────────────────────
            "toBoolean" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Bool(b)) => vec![Value::Bool(*b)],
                    Some(Value::Integer(1)) => vec![Value::Bool(true)],
                    Some(Value::Integer(0)) => vec![Value::Bool(false)],
                    Some(Value::String(s)) => match s.as_ref() {
                        "true" | "yes" | "1" => vec![Value::Bool(true)],
                        "false" | "no" | "0" => vec![Value::Bool(false)],
                        _ => vec![],
                    },
                    None => vec![],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "toBoolean() cannot convert {}",
                            v.type_name()
                        )));
                    }
                }
            }
            "convertsToBoolean" => {
                check_arity(name, args, 0)?;
                let result = match focus.first() {
                    Some(Value::Bool(_)) => true,
                    Some(Value::Integer(n)) => *n == 0 || *n == 1,
                    Some(Value::String(s)) => {
                        matches!(s.as_ref(), "true" | "false" | "yes" | "no" | "1" | "0")
                    }
                    _ => false,
                };
                vec![Value::Bool(result)]
            }

            // ── String ────────────────────────────────────────────────────
            "convertsToString" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(!focus.is_empty())]
            }

            // ── Date / DateTime / Time ─────────────────────────────────────
            "toDate" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::Date(_) => Some(v.clone()),
                        _ => None,
                    })
                    .collect()
            }
            "convertsToDate" => {
                check_arity(name, args, 0)?;
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::Date(_)))
                    .unwrap_or(false);
                vec![Value::Bool(result)]
            }
            "toDateTime" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::DateTime(_) => Some(v.clone()),
                        _ => None,
                    })
                    .collect()
            }
            "convertsToDateTime" => {
                check_arity(name, args, 0)?;
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::DateTime(_)))
                    .unwrap_or(false);
                vec![Value::Bool(result)]
            }
            "toTime" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::Time(_) => Some(v.clone()),
                        _ => None,
                    })
                    .collect()
            }
            "convertsToTime" => {
                check_arity(name, args, 0)?;
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::Time(_)))
                    .unwrap_or(false);
                vec![Value::Bool(result)]
            }

            // ── Quantity ──────────────────────────────────────────────────
            "toQuantity" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::Quantity(_, _) => Some(v.clone()),
                        _ => None,
                    })
                    .collect()
            }
            "convertsToQuantity" => {
                check_arity(name, args, 0)?;
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::Quantity(_, _)))
                    .unwrap_or(false);
                vec![Value::Bool(result)]
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}
