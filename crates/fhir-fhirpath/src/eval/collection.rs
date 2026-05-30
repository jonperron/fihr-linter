use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{
    check_arity, collect_descendants, compare_values, eval_integer_arg, eval_type_name,
    value_is_type,
};

impl Evaluator {
    /// Handle collection, existence, filtering, aggregation, and navigation functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_collection_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_collection(name, args, focus, ctx).transpose()
    }

    fn try_collection(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            // ── Existence ────────────────────────────────────────────────
            "empty" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(focus.is_empty())]
            }
            "exists" => {
                if args.is_empty() {
                    return Ok(Some(vec![Value::Bool(!focus.is_empty())]));
                }
                check_arity(name, args, 1)?;
                let result = focus.iter().try_fold(false, |acc, item| {
                    let truthy = self
                        .eval(&args[0], std::slice::from_ref(item))?
                        .iter()
                        .any(Value::is_truthy);
                    Ok::<bool, EvalError>(acc || truthy)
                })?;
                vec![Value::Bool(result)]
            }
            "all" => {
                check_arity(name, args, 1)?;
                if focus.is_empty() {
                    return Ok(Some(vec![Value::Bool(true)]));
                }
                let result = focus.iter().try_fold(true, |acc, item| {
                    let truthy = self
                        .eval(&args[0], std::slice::from_ref(item))?
                        .iter()
                        .any(Value::is_truthy);
                    Ok::<bool, EvalError>(acc && truthy)
                })?;
                vec![Value::Bool(result)]
            }
            "allTrue" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(
                    focus.iter().all(|v| matches!(v, Value::Bool(true))),
                )]
            }
            "anyTrue" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(
                    focus.iter().any(|v| matches!(v, Value::Bool(true))),
                )]
            }
            "allFalse" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(
                    focus.iter().all(|v| matches!(v, Value::Bool(false))),
                )]
            }
            "anyFalse" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(
                    focus.iter().any(|v| matches!(v, Value::Bool(false))),
                )]
            }

            // ── Filtering / subsetting ────────────────────────────────────
            "where" => {
                check_arity(name, args, 1)?;
                let mut out = Vec::new();
                for item in focus {
                    let matches = self
                        .eval(&args[0], std::slice::from_ref(item))?
                        .iter()
                        .any(Value::is_truthy);
                    if matches {
                        out.push(item.clone());
                    }
                }
                out
            }
            "select" => {
                check_arity(name, args, 1)?;
                let mut out = Vec::new();
                for item in focus {
                    let col = self.eval(&args[0], std::slice::from_ref(item))?;
                    out.extend(col);
                }
                out
            }
            "repeat" => {
                check_arity(name, args, 1)?;
                let mut seen = focus.to_vec();
                let mut frontier = focus.to_vec();
                loop {
                    let mut next = Vec::new();
                    for item in &frontier {
                        let col = self.eval(&args[0], std::slice::from_ref(item))?;
                        for v in col {
                            if !seen.contains(&v) {
                                seen.push(v.clone());
                                next.push(v);
                            }
                        }
                    }
                    if next.is_empty() {
                        break;
                    }
                    frontier = next;
                }
                seen
            }
            "ofType" => {
                check_arity(name, args, 1)?;
                let type_name = eval_type_name(&args[0], ctx)?;
                focus
                    .iter()
                    .filter(|v| value_is_type(v, &type_name))
                    .cloned()
                    .collect()
            }

            // ── Counting / indexing ───────────────────────────────────────
            "count" => {
                check_arity(name, args, 0)?;
                vec![Value::Integer(focus.len() as i64)]
            }
            "first" => {
                check_arity(name, args, 0)?;
                focus.first().cloned().into_iter().collect()
            }
            "last" => {
                check_arity(name, args, 0)?;
                focus.last().cloned().into_iter().collect()
            }
            "tail" => {
                check_arity(name, args, 0)?;
                focus.iter().skip(1).cloned().collect()
            }
            "skip" => {
                check_arity(name, args, 1)?;
                let n = eval_integer_arg(&self.eval(&args[0], ctx)?, "skip")?;
                focus.iter().skip(n.max(0) as usize).cloned().collect()
            }
            "take" => {
                check_arity(name, args, 1)?;
                let n = eval_integer_arg(&self.eval(&args[0], ctx)?, "take")?;
                focus.iter().take(n.max(0) as usize).cloned().collect()
            }
            "single" => {
                check_arity(name, args, 0)?;
                if focus.len() != 1 {
                    return Err(EvalError::Type(format!(
                        "single() requires exactly 1 item, got {}",
                        focus.len()
                    )));
                }
                focus.to_vec()
            }

            // ── Set operations ────────────────────────────────────────────
            "distinct" => {
                check_arity(name, args, 0)?;
                let mut seen: Collection = Vec::new();
                for v in focus {
                    if !seen.contains(v) {
                        seen.push(v.clone());
                    }
                }
                seen
            }
            "isDistinct" => {
                check_arity(name, args, 0)?;
                let mut seen: Collection = Vec::new();
                let mut distinct = true;
                for v in focus {
                    if seen.contains(v) {
                        distinct = false;
                        break;
                    }
                    seen.push(v.clone());
                }
                vec![Value::Bool(distinct)]
            }
            "subsetOf" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                vec![Value::Bool(focus.iter().all(|v| other.contains(v)))]
            }
            "supersetOf" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                vec![Value::Bool(other.iter().all(|v| focus.contains(v)))]
            }
            "intersect" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                focus
                    .iter()
                    .filter(|v| other.contains(v))
                    .cloned()
                    .collect()
            }
            "exclude" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                focus
                    .iter()
                    .filter(|v| !other.contains(v))
                    .cloned()
                    .collect()
            }
            "combine" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let other = self.eval(&args[0], ctx)?;
                let mut result = focus.to_vec();
                result.extend(other);
                result
            }
            "coalesce" => {
                for arg in args {
                    let result = self.eval(arg, focus)?;
                    if !result.is_empty() {
                        return Ok(Some(result));
                    }
                }
                vec![]
            }

            // ── Aggregation ───────────────────────────────────────────────
            "aggregate" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let mut total = if args.len() == 2 {
                    self.eval(&args[1], ctx)?
                } else {
                    vec![]
                };
                for item in focus {
                    let item_ctx: Collection = vec![item.clone()];
                    total = self.eval(&args[0], &item_ctx)?;
                }
                total
            }
            "sum" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![Value::Integer(0)]));
                }
                let mut sum = 0f64;
                let mut all_integer = true;
                for v in focus {
                    match v {
                        Value::Integer(n) => sum += *n as f64,
                        Value::Decimal(d) => {
                            sum += d;
                            all_integer = false;
                        }
                        other => {
                            return Err(EvalError::Type(format!(
                                "sum() requires numbers, got {}",
                                other.type_name()
                            )));
                        }
                    }
                }
                if all_integer {
                    vec![Value::Integer(sum as i64)]
                } else {
                    vec![Value::Decimal(sum)]
                }
            }
            "min" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let mut min = focus[0].clone();
                for v in &focus[1..] {
                    if compare_values(v, &min)? == std::cmp::Ordering::Less {
                        min = v.clone();
                    }
                }
                vec![min]
            }
            "max" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let mut max = focus[0].clone();
                for v in &focus[1..] {
                    if compare_values(v, &max)? == std::cmp::Ordering::Greater {
                        max = v.clone();
                    }
                }
                vec![max]
            }
            "avg" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let count = focus.len() as f64;
                let mut sum = 0f64;
                for v in focus {
                    match v {
                        Value::Integer(n) => sum += *n as f64,
                        Value::Long(n) => sum += *n as f64,
                        Value::Decimal(d) => sum += d,
                        other => {
                            return Err(EvalError::Type(format!(
                                "avg() requires numbers, got {}",
                                other.type_name()
                            )));
                        }
                    }
                }
                vec![Value::Decimal(sum / count)]
            }
            "sort" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let mut items = focus.to_vec();
                if args.is_empty() {
                    let mut sort_error: Option<EvalError> = None;
                    items.sort_by(|a, b| {
                        compare_values(a, b).unwrap_or_else(|e| {
                            sort_error = Some(e);
                            std::cmp::Ordering::Equal
                        })
                    });
                    if let Some(e) = sort_error {
                        return Err(e);
                    }
                    items
                } else {
                    let mut keys: Vec<Value> = Vec::with_capacity(items.len());
                    for item in &items {
                        let key_col = self.eval(&args[0], std::slice::from_ref(item))?;
                        keys.push(key_col.into_iter().next().unwrap_or(Value::Null));
                    }
                    let mut indices: Vec<usize> = (0..items.len()).collect();
                    let mut sort_error: Option<EvalError> = None;
                    indices.sort_by(|&i, &j| {
                        compare_values(&keys[i], &keys[j]).unwrap_or_else(|e| {
                            sort_error = Some(e);
                            std::cmp::Ordering::Equal
                        })
                    });
                    if let Some(e) = sort_error {
                        return Err(e);
                    }
                    indices.into_iter().map(|i| items[i].clone()).collect()
                }
            }

            // ── Navigation ────────────────────────────────────────────────
            "children" => {
                check_arity(name, args, 0)?;
                let mut out = Vec::new();
                for item in focus {
                    if let Value::Object(fields) = item {
                        for col in fields.values() {
                            out.extend_from_slice(col);
                        }
                    }
                }
                out
            }
            "descendants" => {
                check_arity(name, args, 0)?;
                let mut out = Vec::new();
                for item in focus {
                    collect_descendants(item, &mut out);
                }
                out
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}
