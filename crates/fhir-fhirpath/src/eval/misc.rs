use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{check_arity, eval_string_arg};

impl Evaluator {
    /// Handle miscellaneous functions (logic, system, FHIR-specific stubs).
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_misc_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_misc(name, args, focus, ctx).transpose()
    }

    fn try_misc(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            // ── Logic ─────────────────────────────────────────────────────
            "not" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Bool(b)) => vec![Value::Bool(!b)],
                    None => vec![],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "not() requires Boolean, got {}",
                            v.type_name()
                        )));
                    }
                }
            }
            "iif" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let cond = self.eval(&args[0], focus)?;
                let truthy = cond.iter().any(Value::is_truthy);
                let result = if truthy {
                    self.eval(&args[1], focus)?
                } else if args.len() == 3 {
                    self.eval(&args[2], focus)?
                } else {
                    vec![]
                };
                return Ok(Some(result));
            }

            // ── System / environment ───────────────────────────────────────
            "trace" => {
                // trace(name, expr?) — returns focus unchanged; side-effect only
                focus.to_vec()
            }
            "now" => {
                check_arity(name, args, 0)?;
                // No system clock access in a pure evaluator
                vec![]
            }
            "today" => {
                check_arity(name, args, 0)?;
                vec![]
            }
            "timeOfDay" => {
                check_arity(name, args, 0)?;
                vec![]
            }
            "resolve" => {
                // Context-dependent; returns empty in the base evaluator
                check_arity(name, args, 0)?;
                vec![]
            }

            // ── FHIR-specific helpers ──────────────────────────────────────
            "extension" => {
                check_arity(name, args, 1)?;
                let url = eval_string_arg(&self.eval(&args[0], ctx)?, "extension url")?;
                let mut out = Vec::new();
                for item in focus {
                    if let Value::Object(fields) = item {
                        if let Some(exts) = fields.get("extension") {
                            for ext in exts {
                                if let Value::Object(ext_fields) = ext {
                                    if let Some(urls) = ext_fields.get("url") {
                                        if urls.iter().any(|u| {
                                            u.as_string()
                                                .map(|s| s.as_ref() == url.as_ref())
                                                .unwrap_or(false)
                                        }) {
                                            out.push(ext.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                out
            }
            "hasValue" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(
                    focus.len() == 1 && !matches!(focus[0], Value::Null),
                )]
            }
            "getValue" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .filter(|v| !matches!(v, Value::Null))
                    .cloned()
                    .collect()
            }
            "conformsTo" => {
                // Requires registry — returns empty in base evaluator
                check_arity(name, args, 1)?;
                vec![]
            }
            "memberOf" => {
                // Requires terminology server — returns empty in base evaluator
                check_arity(name, args, 1)?;
                vec![]
            }
            "htmlChecks" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(true)]
            }

            // ── Introspection ─────────────────────────────────────────────
            "comparable" => {
                check_arity(name, args, 0)?;
                let result = focus.iter().all(|v| {
                    matches!(
                        v,
                        Value::Integer(_)
                            | Value::Decimal(_)
                            | Value::Date(_)
                            | Value::DateTime(_)
                            | Value::Time(_)
                            | Value::String(_)
                            | Value::Quantity(_, _)
                    )
                });
                vec![Value::Bool(result)]
            }
            "type" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .map(|v| Value::String(Arc::from(v.type_name())))
                    .collect()
            }

            // ── Precision / boundaries (stubs) ────────────────────────────
            "lowBoundary" | "highBoundary" => {
                check_arity(name, args, 0)?;
                vec![]
            }
            "precision" => {
                check_arity(name, args, 0)?;
                vec![]
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}
