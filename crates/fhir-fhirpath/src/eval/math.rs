use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{check_arity, eval_integer_arg};

impl Evaluator {
    /// Handle mathematical functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_math_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_math(name, args, focus, ctx).transpose()
    }

    fn try_math(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            "abs" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Integer(n.abs())],
                    Some(Value::Decimal(d)) => vec![Value::Decimal(d.abs())],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "abs() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "ceiling" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => vec![Value::Integer(d.ceil() as i64)],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "ceiling() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "floor" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => vec![Value::Integer(d.floor() as i64)],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "floor() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "round" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let precision = if args.is_empty() {
                    0i64
                } else {
                    eval_integer_arg(&self.eval(&args[0], ctx)?, "round precision")?
                };
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => {
                        let factor = 10f64.powi(precision as i32);
                        vec![Value::Decimal((d * factor).round() / factor)]
                    }
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "round() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "sqrt" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Decimal((*n as f64).sqrt())],
                    Some(Value::Decimal(d)) => vec![Value::Decimal(d.sqrt())],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "sqrt() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "exp" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Decimal((*n as f64).exp())],
                    Some(Value::Decimal(d)) => vec![Value::Decimal(d.exp())],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "exp() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "ln" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Decimal((*n as f64).ln())],
                    Some(Value::Decimal(d)) => vec![Value::Decimal(d.ln())],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "ln() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "log" => {
                check_arity(name, args, 1)?;
                let base = match self.eval(&args[0], ctx)?.first() {
                    Some(v) => v
                        .as_decimal()
                        .ok_or_else(|| EvalError::Type("log() base must be a number".into()))?,
                    None => return Ok(Some(vec![])),
                };
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Decimal((*n as f64).log(base))],
                    Some(Value::Decimal(d)) => vec![Value::Decimal(d.log(base))],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "log() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "power" => {
                check_arity(name, args, 1)?;
                let exp = match self.eval(&args[0], ctx)?.first() {
                    Some(v) => v.as_decimal().ok_or_else(|| {
                        EvalError::Type("power() exponent must be a number".into())
                    })?,
                    None => return Ok(Some(vec![])),
                };
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Decimal((*n as f64).powf(exp))],
                    Some(Value::Decimal(d)) => vec![Value::Decimal(d.powf(exp))],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "power() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "truncate" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => vec![Value::Integer(d.trunc() as i64)],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "truncate() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}
