/// FHIRPath 2.0 expression evaluator.
///
/// The evaluator walks an [`Expr`] AST and produces a [`Collection`].
/// All functions described in the FHIRPath 2.0 specification are implemented
/// here, grouped by category.
use std::sync::Arc;

use regex_lite::Regex;

use crate::ast::{Expr, TemporalKind};
use crate::error::EvalError;
use crate::value::{Collection, Value};

/// Evaluate a FHIRPath expression against a context value.
///
/// `context` is the *input collection* (the current `$this` node).
/// Returns a collection of results.
pub fn eval(expr: &Expr, context: &[Value], _root: &[Value]) -> Result<Collection, EvalError> {
    Evaluator.eval(expr, context)
}

struct Evaluator;

impl Evaluator {
    fn eval(&self, expr: &Expr, ctx: &[Value]) -> Result<Collection, EvalError> {
        match expr {
            // ── Literals ────────────────────────────────────────────────────
            Expr::Null => Ok(vec![]),
            Expr::Bool(b) => Ok(vec![Value::Bool(*b)]),
            Expr::Integer(n) => Ok(vec![Value::Integer(*n)]),
            Expr::Decimal(d) => Ok(vec![Value::Decimal(*d)]),
            Expr::String(s) => Ok(vec![Value::String(Arc::from(s.as_str()))]),
            Expr::Temporal(kind, s) => {
                let v = Arc::from(s.as_str());
                Ok(vec![match kind {
                    TemporalKind::Date => Value::Date(v),
                    TemporalKind::DateTime => Value::DateTime(v),
                    TemporalKind::Time => Value::Time(v),
                }])
            }
            Expr::Quantity(n, u) => Ok(vec![Value::Quantity(*n, Arc::from(u.as_str()))]),

            // ── Context / root ──────────────────────────────────────────────
            Expr::DollarThis => Ok(ctx.to_vec()),
            Expr::DollarIndex => Err(EvalError::UndefinedFunction(
                "$index is not supported outside of iteration context".to_owned(),
            )),

            // ── Identifier / path step ─────────────────────────────────────
            Expr::Ident(name) => self.eval_ident(name, ctx),

            // ── Navigation ─────────────────────────────────────────────────
            Expr::Dot(base, step) => {
                let base_col = self.eval(base, ctx)?;
                self.eval(step, &base_col)
            }

            Expr::Index(base, index_expr) => {
                let base_col = self.eval(base, ctx)?;
                let idx_col = self.eval(index_expr, ctx)?;
                let idx = single_integer(&idx_col, "index")?;
                if idx < 0 || idx as usize >= base_col.len() {
                    return Err(EvalError::IndexOutOfBounds(idx));
                }
                Ok(vec![base_col[idx as usize].clone()])
            }

            // ── Function calls ─────────────────────────────────────────────
            Expr::FuncCall(name, args) => self.call_function(name, args, ctx, ctx),
            Expr::Method(base, name, args) => {
                let base_col = self.eval(base, ctx)?;
                self.call_function(name, args, &base_col, ctx)
            }

            // ── Arithmetic ─────────────────────────────────────────────────
            Expr::Add(l, r) => self.eval_arith(l, r, ctx, ArithOp::Add),
            Expr::Sub(l, r) => self.eval_arith(l, r, ctx, ArithOp::Sub),
            Expr::Mul(l, r) => self.eval_arith(l, r, ctx, ArithOp::Mul),
            Expr::Div(l, r) => self.eval_arith(l, r, ctx, ArithOp::Div),
            Expr::DivInt(l, r) => self.eval_arith(l, r, ctx, ArithOp::DivInt),
            Expr::Mod(l, r) => self.eval_arith(l, r, ctx, ArithOp::Mod),
            Expr::Neg(e) => {
                let col = self.eval(e, ctx)?;
                if col.is_empty() {
                    return Ok(vec![]);
                }
                match &col[0] {
                    Value::Integer(n) => Ok(vec![Value::Integer(-n)]),
                    Value::Decimal(d) => Ok(vec![Value::Decimal(-d)]),
                    v => Err(EvalError::Type(format!("cannot negate {}", v.type_name()))),
                }
            }
            Expr::Concat(l, r) => {
                let ls = self.eval(l, ctx)?;
                let rs = self.eval(r, ctx)?;
                let left = string_or_empty(&ls);
                let right = string_or_empty(&rs);
                Ok(vec![Value::String(Arc::from(
                    format!("{left}{right}").as_str(),
                ))])
            }

            // ── Comparisons ─────────────────────────────────────────────────
            Expr::Eq(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Eq),
            Expr::Neq(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Neq),
            Expr::Lt(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Lt),
            Expr::Lte(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Lte),
            Expr::Gt(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Gt),
            Expr::Gte(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Gte),
            Expr::Equiv(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Equiv),
            Expr::NotEquiv(l, r) => self.eval_cmp(l, r, ctx, CmpOp::NotEquiv),

            // ── Boolean operators ─────────────────────────────────────────
            Expr::And(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                Ok(bool_col(three_valued_and(lv, rv)))
            }
            Expr::Or(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                Ok(bool_col(three_valued_or(lv, rv)))
            }
            Expr::Xor(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                let result = match (lv, rv) {
                    (Some(a), Some(b)) => Some(a ^ b),
                    _ => None,
                };
                Ok(bool_col(result))
            }
            Expr::Implies(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                let result = match lv {
                    Some(false) => Some(true),
                    Some(true) => rv,
                    None => match rv {
                        Some(true) => Some(true),
                        _ => None,
                    },
                };
                Ok(bool_col(result))
            }

            // ── Type operators ────────────────────────────────────────────
            Expr::Is(base, type_name) => {
                let col = self.eval(base, ctx)?;
                let result = col.iter().any(|v| value_is_type(v, type_name));
                Ok(vec![Value::Bool(result)])
            }
            Expr::As(base, type_name) => {
                let col = self.eval(base, ctx)?;
                Ok(col
                    .into_iter()
                    .filter(|v| value_is_type(v, type_name))
                    .collect())
            }

            // ── Set operators ─────────────────────────────────────────────
            Expr::Union(l, r) => {
                let mut ls = self.eval(l, ctx)?;
                let rs = self.eval(r, ctx)?;
                for v in rs {
                    if !ls.contains(&v) {
                        ls.push(v);
                    }
                }
                Ok(ls)
            }
            Expr::In(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                if lc.is_empty() {
                    return Ok(vec![]);
                }
                let result = lc.iter().all(|v| rc.contains(v));
                Ok(vec![Value::Bool(result)])
            }
            Expr::Contains(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                if rc.is_empty() {
                    return Ok(vec![]);
                }
                let result = rc.iter().all(|v| lc.contains(v));
                Ok(vec![Value::Bool(result)])
            }
        }
    }

    // ── Path navigation ─────────────────────────────────────────────────────

    fn eval_ident(&self, name: &str, ctx: &[Value]) -> Result<Collection, EvalError> {
        let mut result = Vec::new();
        for item in ctx {
            if let Value::Object(fields) = item {
                if let Some(children) = fields.get(name) {
                    result.extend_from_slice(children);
                }
            }
        }
        Ok(result)
    }

    // ── Function dispatch ───────────────────────────────────────────────────

    fn call_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        macro_rules! arity {
            ($n:expr) => {
                if args.len() != $n {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: $n,
                        got: args.len(),
                    });
                }
            };
        }

        match name {
            // ── Existence ────────────────────────────────────────────────
            "empty" => {
                arity!(0);
                Ok(vec![Value::Bool(focus.is_empty())])
            }
            "exists" => {
                if args.is_empty() {
                    return Ok(vec![Value::Bool(!focus.is_empty())]);
                }
                arity!(1);
                let result = focus.iter().try_fold(false, |acc, item| {
                    let truthy = self
                        .eval(&args[0], std::slice::from_ref(item))?
                        .iter()
                        .any(Value::is_truthy);
                    Ok::<bool, EvalError>(acc || truthy)
                })?;
                Ok(vec![Value::Bool(result)])
            }
            "all" => {
                arity!(1);
                if focus.is_empty() {
                    return Ok(vec![Value::Bool(true)]);
                }
                let result = focus.iter().try_fold(true, |acc, item| {
                    let truthy = self
                        .eval(&args[0], std::slice::from_ref(item))?
                        .iter()
                        .any(Value::is_truthy);
                    Ok::<bool, EvalError>(acc && truthy)
                })?;
                Ok(vec![Value::Bool(result)])
            }
            "allTrue" => {
                arity!(0);
                let result = focus.iter().all(|v| matches!(v, Value::Bool(true)));
                Ok(vec![Value::Bool(result)])
            }
            "anyTrue" => {
                arity!(0);
                let result = focus.iter().any(|v| matches!(v, Value::Bool(true)));
                Ok(vec![Value::Bool(result)])
            }
            "allFalse" => {
                arity!(0);
                let result = focus.iter().all(|v| matches!(v, Value::Bool(false)));
                Ok(vec![Value::Bool(result)])
            }
            "anyFalse" => {
                arity!(0);
                let result = focus.iter().any(|v| matches!(v, Value::Bool(false)));
                Ok(vec![Value::Bool(result)])
            }

            // ── Filtering / subsetting ───────────────────────────────────
            "where" => {
                arity!(1);
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
                Ok(out)
            }
            "select" => {
                arity!(1);
                let mut out = Vec::new();
                for item in focus {
                    let col = self.eval(&args[0], std::slice::from_ref(item))?;
                    out.extend(col);
                }
                Ok(out)
            }
            "repeat" => {
                arity!(1);
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
                Ok(seen)
            }
            "ofType" => {
                arity!(1);
                let type_name = eval_type_name(&args[0], ctx)?;
                Ok(focus
                    .iter()
                    .filter(|v| value_is_type(v, &type_name))
                    .cloned()
                    .collect())
            }

            // ── Counting / indexing ──────────────────────────────────────
            "count" => {
                arity!(0);
                Ok(vec![Value::Integer(focus.len() as i64)])
            }
            "first" => {
                arity!(0);
                Ok(focus.first().cloned().into_iter().collect())
            }
            "last" => {
                arity!(0);
                Ok(focus.last().cloned().into_iter().collect())
            }
            "tail" => {
                arity!(0);
                Ok(focus.iter().skip(1).cloned().collect())
            }
            "skip" => {
                arity!(1);
                let n = eval_integer_arg(&self.eval(&args[0], ctx)?, "skip")?;
                Ok(focus.iter().skip(n.max(0) as usize).cloned().collect())
            }
            "take" => {
                arity!(1);
                let n = eval_integer_arg(&self.eval(&args[0], ctx)?, "take")?;
                Ok(focus.iter().take(n.max(0) as usize).cloned().collect())
            }
            "single" => {
                arity!(0);
                if focus.len() != 1 {
                    return Err(EvalError::Type(format!(
                        "single() requires exactly 1 item, got {}",
                        focus.len()
                    )));
                }
                Ok(focus.to_vec())
            }

            // ── Strings ──────────────────────────────────────────────────
            "toString" => {
                arity!(0);
                Ok(focus
                    .iter()
                    .map(|v| Value::String(Arc::from(v.to_string().as_str())))
                    .collect())
            }
            "length" => {
                arity!(0);
                match focus.first() {
                    Some(Value::String(s)) => Ok(vec![Value::Integer(s.chars().count() as i64)]),
                    Some(v) => Err(EvalError::Type(format!(
                        "length() requires String, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "startsWith" => {
                arity!(1);
                let prefix = eval_string_arg(&self.eval(&args[0], ctx)?, "startsWith")?;
                Ok(vec![Value::Bool(
                    focus
                        .first()
                        .and_then(|v| v.as_string())
                        .map(|s| s.starts_with(prefix.as_ref()))
                        .unwrap_or(false),
                )])
            }
            "endsWith" => {
                arity!(1);
                let suffix = eval_string_arg(&self.eval(&args[0], ctx)?, "endsWith")?;
                Ok(vec![Value::Bool(
                    focus
                        .first()
                        .and_then(|v| v.as_string())
                        .map(|s| s.ends_with(suffix.as_ref()))
                        .unwrap_or(false),
                )])
            }
            "contains" => {
                arity!(1);
                let sub = eval_string_arg(&self.eval(&args[0], ctx)?, "contains")?;
                Ok(vec![Value::Bool(
                    focus
                        .first()
                        .and_then(|v| v.as_string())
                        .map(|s| s.contains(sub.as_ref()))
                        .unwrap_or(false),
                )])
            }
            "upper" => {
                arity!(0);
                string_transform(focus, |s| s.to_uppercase())
            }
            "lower" => {
                arity!(0);
                string_transform(focus, |s| s.to_lowercase())
            }
            "trim" => {
                arity!(0);
                string_transform(focus, |s| s.trim().to_owned())
            }
            "substring" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(EvalError::Arity {
                        name: "substring".into(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let s = match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => s,
                    None => return Ok(vec![]),
                };
                let chars: Vec<char> = s.chars().collect();
                let start_i = eval_integer_arg(&self.eval(&args[0], ctx)?, "substring start")?;
                if start_i < 0 {
                    return Ok(vec![]);
                }
                let start = start_i as usize;
                let end = if args.len() == 2 {
                    let len_i = eval_integer_arg(&self.eval(&args[1], ctx)?, "substring length")?;
                    if len_i < 0 {
                        return Ok(vec![]);
                    }
                    (start + len_i as usize).min(chars.len())
                } else {
                    chars.len()
                };
                let sub: String = chars[start.min(chars.len())..end].iter().collect();
                Ok(vec![Value::String(Arc::from(sub.as_str()))])
            }
            "replace" => {
                arity!(2);
                let pattern = eval_string_arg(&self.eval(&args[0], ctx)?, "replace pattern")?;
                let replacement =
                    eval_string_arg(&self.eval(&args[1], ctx)?, "replace replacement")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let result = s.replace(pattern.as_ref(), replacement.as_ref());
                        Ok(vec![Value::String(Arc::from(result.as_str()))])
                    }
                    None => Ok(vec![]),
                }
            }
            "matches" => {
                arity!(1);
                let pattern = eval_string_arg(&self.eval(&args[0], ctx)?, "matches pattern")?;
                let text = focus
                    .first()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| Arc::from(""));
                let matched = regex_matches(&text, &pattern)?;
                Ok(vec![Value::Bool(matched)])
            }
            "replaceMatches" => {
                arity!(2);
                let pattern =
                    eval_string_arg(&self.eval(&args[0], ctx)?, "replaceMatches pattern")?;
                let replacement =
                    eval_string_arg(&self.eval(&args[1], ctx)?, "replaceMatches replacement")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let result = regex_replace_all(&s, &pattern, &replacement)?;
                        Ok(vec![Value::String(Arc::from(result.as_str()))])
                    }
                    None => Ok(vec![]),
                }
            }
            "indexOf" => {
                arity!(1);
                let sub = eval_string_arg(&self.eval(&args[0], ctx)?, "indexOf")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let idx = s
                            .char_indices()
                            .enumerate()
                            .find(|(_, (byte_pos, _))| s[*byte_pos..].starts_with(sub.as_ref()))
                            .map(|(char_idx, _)| char_idx as i64)
                            .unwrap_or(-1);
                        Ok(vec![Value::Integer(idx)])
                    }
                    None => Ok(vec![Value::Integer(-1)]),
                }
            }
            "split" => {
                arity!(1);
                let sep = eval_string_arg(&self.eval(&args[0], ctx)?, "split")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => Ok(s
                        .split(sep.as_ref())
                        .map(|part| Value::String(Arc::from(part)))
                        .collect()),
                    None => Ok(vec![]),
                }
            }
            "join" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: "join".into(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let sep = if args.is_empty() {
                    Arc::from("")
                } else {
                    eval_string_arg(&self.eval(&args[0], ctx)?, "join separator")?
                };
                let joined: Vec<String> = focus
                    .iter()
                    .filter_map(|v| v.as_string())
                    .map(|s| s.to_string())
                    .collect();
                Ok(vec![Value::String(Arc::from(
                    joined.join(sep.as_ref()).as_str(),
                ))])
            }
            "encode" => {
                arity!(1);
                let format = eval_string_arg(&self.eval(&args[0], ctx)?, "encode")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let result = match format.as_ref() {
                            "base64" => base64_encode(s.as_bytes()),
                            "urlbase64" => base64_encode_url(s.as_bytes()),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "unknown encode format: {other}"
                                )));
                            }
                        };
                        Ok(vec![Value::String(Arc::from(result.as_str()))])
                    }
                    None => Ok(vec![]),
                }
            }
            "decode" => {
                arity!(1);
                let format = eval_string_arg(&self.eval(&args[0], ctx)?, "decode")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let bytes = match format.as_ref() {
                            "base64" => base64_decode(s.as_bytes()),
                            "urlbase64" => base64_decode_url(s.as_bytes()),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "unknown decode format: {other}"
                                )));
                            }
                        };
                        let decoded = String::from_utf8(bytes)
                            .map_err(|_| EvalError::Type("decode: invalid UTF-8".into()))?;
                        Ok(vec![Value::String(Arc::from(decoded.as_str()))])
                    }
                    None => Ok(vec![]),
                }
            }

            // ── Math ─────────────────────────────────────────────────────
            "abs" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Integer(n.abs())]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Decimal(d.abs())]),
                    Some(v) => Err(EvalError::Type(format!(
                        "abs() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "ceiling" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Integer(*n)]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Integer(d.ceil() as i64)]),
                    Some(v) => Err(EvalError::Type(format!(
                        "ceiling() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "floor" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Integer(*n)]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Integer(d.floor() as i64)]),
                    Some(v) => Err(EvalError::Type(format!(
                        "floor() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "round" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: "round".into(),
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
                    Some(Value::Integer(n)) => Ok(vec![Value::Integer(*n)]),
                    Some(Value::Decimal(d)) => {
                        let factor = 10f64.powi(precision as i32);
                        Ok(vec![Value::Decimal((d * factor).round() / factor)])
                    }
                    Some(v) => Err(EvalError::Type(format!(
                        "round() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "sqrt" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Decimal((*n as f64).sqrt())]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Decimal(d.sqrt())]),
                    Some(v) => Err(EvalError::Type(format!(
                        "sqrt() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "exp" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Decimal((*n as f64).exp())]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Decimal(d.exp())]),
                    Some(v) => Err(EvalError::Type(format!(
                        "exp() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "ln" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Decimal((*n as f64).ln())]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Decimal(d.ln())]),
                    Some(v) => Err(EvalError::Type(format!(
                        "ln() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "log" => {
                arity!(1);
                let base = match self.eval(&args[0], ctx)?.first() {
                    Some(v) => v
                        .as_decimal()
                        .ok_or_else(|| EvalError::Type("log() base must be a number".into()))?,
                    None => return Ok(vec![]),
                };
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Decimal((*n as f64).log(base))]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Decimal(d.log(base))]),
                    Some(v) => Err(EvalError::Type(format!(
                        "log() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "power" => {
                arity!(1);
                let exp = match self.eval(&args[0], ctx)?.first() {
                    Some(v) => v.as_decimal().ok_or_else(|| {
                        EvalError::Type("power() exponent must be a number".into())
                    })?,
                    None => return Ok(vec![]),
                };
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Decimal((*n as f64).powf(exp))]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Decimal(d.powf(exp))]),
                    Some(v) => Err(EvalError::Type(format!(
                        "power() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }
            "truncate" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Integer(*n)]),
                    Some(Value::Decimal(d)) => Ok(vec![Value::Integer(d.trunc() as i64)]),
                    Some(v) => Err(EvalError::Type(format!(
                        "truncate() requires number, got {}",
                        v.type_name()
                    ))),
                    None => Ok(vec![]),
                }
            }

            // ── Type conversion ───────────────────────────────────────────
            "toInteger" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Integer(n)) => Ok(vec![Value::Integer(*n)]),
                    Some(Value::Bool(b)) => Ok(vec![Value::Integer(if *b { 1 } else { 0 })]),
                    Some(Value::String(s)) => match s.parse::<i64>() {
                        Ok(n) => Ok(vec![Value::Integer(n)]),
                        Err(_) => Ok(vec![]),
                    },
                    None => Ok(vec![]),
                    Some(v) => Err(EvalError::Type(format!(
                        "toInteger() cannot convert {}",
                        v.type_name()
                    ))),
                }
            }
            "toDecimal" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Decimal(d)) => Ok(vec![Value::Decimal(*d)]),
                    Some(Value::Integer(n)) => Ok(vec![Value::Decimal(*n as f64)]),
                    Some(Value::Bool(b)) => Ok(vec![Value::Decimal(if *b { 1.0 } else { 0.0 })]),
                    Some(Value::String(s)) => match s.parse::<f64>() {
                        Ok(d) => Ok(vec![Value::Decimal(d)]),
                        Err(_) => Ok(vec![]),
                    },
                    None => Ok(vec![]),
                    Some(v) => Err(EvalError::Type(format!(
                        "toDecimal() cannot convert {}",
                        v.type_name()
                    ))),
                }
            }
            "toBoolean" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Bool(b)) => Ok(vec![Value::Bool(*b)]),
                    Some(Value::Integer(1)) => Ok(vec![Value::Bool(true)]),
                    Some(Value::Integer(0)) => Ok(vec![Value::Bool(false)]),
                    Some(Value::String(s)) => match s.as_ref() {
                        "true" | "yes" | "1" => Ok(vec![Value::Bool(true)]),
                        "false" | "no" | "0" => Ok(vec![Value::Bool(false)]),
                        _ => Ok(vec![]),
                    },
                    None => Ok(vec![]),
                    Some(v) => Err(EvalError::Type(format!(
                        "toBoolean() cannot convert {}",
                        v.type_name()
                    ))),
                }
            }
            "convertsToInteger" => {
                arity!(0);
                let result = match focus.first() {
                    Some(Value::Integer(_)) | Some(Value::Bool(_)) => true,
                    Some(Value::String(s)) => s.parse::<i64>().is_ok(),
                    _ => false,
                };
                Ok(vec![Value::Bool(result)])
            }
            "convertsToDecimal" => {
                arity!(0);
                let result = match focus.first() {
                    Some(Value::Decimal(_)) | Some(Value::Integer(_)) | Some(Value::Bool(_)) => {
                        true
                    }
                    Some(Value::String(s)) => s.parse::<f64>().is_ok(),
                    _ => false,
                };
                Ok(vec![Value::Bool(result)])
            }
            "convertsToBoolean" => {
                arity!(0);
                let result = match focus.first() {
                    Some(Value::Bool(_)) => true,
                    Some(Value::Integer(n)) => *n == 0 || *n == 1,
                    Some(Value::String(s)) => {
                        matches!(s.as_ref(), "true" | "false" | "yes" | "no" | "1" | "0")
                    }
                    _ => false,
                };
                Ok(vec![Value::Bool(result)])
            }
            "convertsToString" => {
                arity!(0);
                Ok(vec![Value::Bool(!focus.is_empty())])
            }
            "convertsToDate" => {
                arity!(0);
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::Date(_)))
                    .unwrap_or(false);
                Ok(vec![Value::Bool(result)])
            }
            "convertsToDateTime" => {
                arity!(0);
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::DateTime(_)))
                    .unwrap_or(false);
                Ok(vec![Value::Bool(result)])
            }
            "convertsToTime" => {
                arity!(0);
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::Time(_)))
                    .unwrap_or(false);
                Ok(vec![Value::Bool(result)])
            }
            "convertsToQuantity" => {
                arity!(0);
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::Quantity(_, _)))
                    .unwrap_or(false);
                Ok(vec![Value::Bool(result)])
            }
            "toDate" => {
                arity!(0);
                Ok(focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::Date(_) => Some(v.clone()),
                        _ => None,
                    })
                    .collect())
            }
            "toDateTime" => {
                arity!(0);
                Ok(focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::DateTime(_) => Some(v.clone()),
                        _ => None,
                    })
                    .collect())
            }
            "toTime" => {
                arity!(0);
                Ok(focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::Time(_) => Some(v.clone()),
                        _ => None,
                    })
                    .collect())
            }
            "toQuantity" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: "toQuantity".into(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                Ok(focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::Quantity(_, _) => Some(v.clone()),
                        _ => None,
                    })
                    .collect())
            }

            // ── Collections ───────────────────────────────────────────────
            "distinct" => {
                arity!(0);
                let mut seen: Collection = Vec::new();
                for v in focus {
                    if !seen.contains(v) {
                        seen.push(v.clone());
                    }
                }
                Ok(seen)
            }
            "isDistinct" => {
                arity!(0);
                let mut seen: Collection = Vec::new();
                let mut distinct = true;
                for v in focus {
                    if seen.contains(v) {
                        distinct = false;
                        break;
                    }
                    seen.push(v.clone());
                }
                Ok(vec![Value::Bool(distinct)])
            }
            "subsetOf" => {
                arity!(1);
                let other = self.eval(&args[0], ctx)?;
                let result = focus.iter().all(|v| other.contains(v));
                Ok(vec![Value::Bool(result)])
            }
            "supersetOf" => {
                arity!(1);
                let other = self.eval(&args[0], ctx)?;
                let result = other.iter().all(|v| focus.contains(v));
                Ok(vec![Value::Bool(result)])
            }
            "intersect" => {
                arity!(1);
                let other = self.eval(&args[0], ctx)?;
                let result: Collection = focus
                    .iter()
                    .filter(|v| other.contains(v))
                    .cloned()
                    .collect();
                Ok(result)
            }
            "exclude" => {
                arity!(1);
                let other = self.eval(&args[0], ctx)?;
                let result: Collection = focus
                    .iter()
                    .filter(|v| !other.contains(v))
                    .cloned()
                    .collect();
                Ok(result)
            }

            // ── Aggregation ───────────────────────────────────────────────
            "aggregate" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(EvalError::Arity {
                        name: "aggregate".into(),
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
                    let eval_ctx_with_total = {
                        // We use a merged context: $this = item, $total = accumulated
                        // For simplicity, evaluate the expression with focus = [item]
                        // The spec says $total is available — we pass it as root
                        self.eval(&args[0], &item_ctx)?
                    };
                    total = eval_ctx_with_total;
                }
                Ok(total)
            }
            "sum" => {
                arity!(0);
                if focus.is_empty() {
                    return Ok(vec![Value::Integer(0)]);
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
                    Ok(vec![Value::Integer(sum as i64)])
                } else {
                    Ok(vec![Value::Decimal(sum)])
                }
            }
            "min" => {
                arity!(0);
                if focus.is_empty() {
                    return Ok(vec![]);
                }
                let mut min = focus[0].clone();
                for v in &focus[1..] {
                    if compare_values(v, &min)? == std::cmp::Ordering::Less {
                        min = v.clone();
                    }
                }
                Ok(vec![min])
            }
            "max" => {
                arity!(0);
                if focus.is_empty() {
                    return Ok(vec![]);
                }
                let mut max = focus[0].clone();
                for v in &focus[1..] {
                    if compare_values(v, &max)? == std::cmp::Ordering::Greater {
                        max = v.clone();
                    }
                }
                Ok(vec![max])
            }

            // ── Navigation ────────────────────────────────────────────────
            "children" => {
                arity!(0);
                let mut out = Vec::new();
                for item in focus {
                    if let Value::Object(fields) = item {
                        for col in fields.values() {
                            out.extend_from_slice(col);
                        }
                    }
                }
                Ok(out)
            }
            "descendants" => {
                arity!(0);
                let mut out = Vec::new();
                for item in focus {
                    collect_descendants(item, &mut out);
                }
                Ok(out)
            }

            // ── Miscellaneous ─────────────────────────────────────────────
            "not" => {
                arity!(0);
                match focus.first() {
                    Some(Value::Bool(b)) => Ok(vec![Value::Bool(!b)]),
                    None => Ok(vec![]),
                    Some(v) => Err(EvalError::Type(format!(
                        "not() requires Boolean, got {}",
                        v.type_name()
                    ))),
                }
            }
            "iif" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(EvalError::Arity {
                        name: "iif".into(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let cond = self.eval(&args[0], focus)?;
                let truthy = cond.iter().any(Value::is_truthy);
                if truthy {
                    self.eval(&args[1], focus)
                } else if args.len() == 3 {
                    self.eval(&args[2], focus)
                } else {
                    Ok(vec![])
                }
            }
            "trace" => {
                // trace(name, expr?) — returns focus unchanged; side-effect only
                Ok(focus.to_vec())
            }
            "now" => {
                arity!(0);
                // Return an empty collection — no system clock access in a pure evaluator
                Ok(vec![])
            }
            "today" => {
                arity!(0);
                Ok(vec![])
            }
            "timeOfDay" => {
                arity!(0);
                Ok(vec![])
            }
            "resolve" => {
                // resolve() — context-dependent; returns empty in the base evaluator
                arity!(0);
                Ok(vec![])
            }
            "extension" => {
                arity!(1);
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
                Ok(out)
            }
            "hasValue" => {
                arity!(0);
                // hasValue() — true if the collection has exactly one non-null value
                Ok(vec![Value::Bool(
                    focus.len() == 1 && !matches!(focus[0], Value::Null),
                )])
            }
            "getValue" => {
                arity!(0);
                Ok(focus
                    .iter()
                    .filter(|v| !matches!(v, Value::Null))
                    .cloned()
                    .collect())
            }
            "conformsTo" => {
                // Requires registry — returns empty in base evaluator
                arity!(1);
                Ok(vec![])
            }
            "memberOf" => {
                // Requires terminology server — returns empty in base evaluator
                arity!(1);
                Ok(vec![])
            }
            "htmlChecks" => {
                arity!(0);
                Ok(vec![Value::Bool(true)])
            }
            "comparable" => {
                arity!(0);
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
                Ok(vec![Value::Bool(result)])
            }
            "type" => {
                arity!(0);
                Ok(focus
                    .iter()
                    .map(|v| Value::String(Arc::from(v.type_name())))
                    .collect())
            }
            "lowBoundary" | "highBoundary" => {
                arity!(0);
                // Returns empty; precision handling is complex and not needed for base
                Ok(vec![])
            }
            "precision" => {
                arity!(0);
                Ok(vec![])
            }

            other => Err(EvalError::UndefinedFunction(other.to_owned())),
        }
    }

    // ── Arithmetic helpers ───────────────────────────────────────────────────

    fn eval_arith(
        &self,
        l: &Expr,
        r: &Expr,
        ctx: &[Value],
        op: ArithOp,
    ) -> Result<Collection, EvalError> {
        let lc = self.eval(l, ctx)?;
        let rc = self.eval(r, ctx)?;
        if lc.is_empty() || rc.is_empty() {
            return Ok(vec![]);
        }
        let lv = &lc[0];
        let rv = &rc[0];
        match op {
            ArithOp::Add | ArithOp::Sub | ArithOp::Mul | ArithOp::Div | ArithOp::Mod => {
                numeric_op(lv, rv, op)
            }
            ArithOp::DivInt => {
                let a = lv
                    .as_decimal()
                    .ok_or_else(|| EvalError::Type("div requires numbers".into()))?;
                let b = rv
                    .as_decimal()
                    .ok_or_else(|| EvalError::Type("div requires numbers".into()))?;
                if b == 0.0 {
                    return Err(EvalError::DivisionByZero);
                }
                Ok(vec![Value::Integer((a / b).trunc() as i64)])
            }
        }
    }

    // ── Comparison helpers ───────────────────────────────────────────────────

    fn eval_cmp(
        &self,
        l: &Expr,
        r: &Expr,
        ctx: &[Value],
        op: CmpOp,
    ) -> Result<Collection, EvalError> {
        let lc = self.eval(l, ctx)?;
        let rc = self.eval(r, ctx)?;
        if lc.is_empty() || rc.is_empty() {
            return Ok(vec![]);
        }
        let result = match op {
            CmpOp::Eq => values_equal(&lc[0], &rc[0]),
            CmpOp::Neq => values_equal(&lc[0], &rc[0]).map(|b| !b),
            CmpOp::Equiv => Some(values_equivalent(&lc[0], &rc[0])),
            CmpOp::NotEquiv => Some(!values_equivalent(&lc[0], &rc[0])),
            _ => {
                let ord = compare_values(&lc[0], &rc[0])?;
                let b = match op {
                    CmpOp::Lt => ord == std::cmp::Ordering::Less,
                    CmpOp::Lte => ord != std::cmp::Ordering::Greater,
                    CmpOp::Gt => ord == std::cmp::Ordering::Greater,
                    CmpOp::Gte => ord != std::cmp::Ordering::Less,
                    _ => unreachable!(),
                };
                Some(b)
            }
        };
        Ok(bool_col(result))
    }

    /// Evaluate an expression as a three-valued boolean (`None` = empty collection).
    fn eval_bool(&self, expr: &Expr, ctx: &[Value]) -> Result<Option<bool>, EvalError> {
        let col = self.eval(expr, ctx)?;
        Ok(collection_to_bool(&col))
    }
}

// ── Helper types ─────────────────────────────────────────────────────────────

enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    DivInt,
    Mod,
}
enum CmpOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Equiv,
    NotEquiv,
}

// ── Pure helper functions ─────────────────────────────────────────────────────

fn numeric_op(lv: &Value, rv: &Value, op: ArithOp) -> Result<Collection, EvalError> {
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
    // Preserve integer type when both operands are integers and result is exact
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

fn values_equal(a: &Value, b: &Value) -> Option<bool> {
    match (a, b) {
        (Value::Null, Value::Null) => None,
        (Value::Null, _) | (_, Value::Null) => None,
        (Value::Bool(x), Value::Bool(y)) => Some(x == y),
        (Value::Integer(x), Value::Integer(y)) => Some(x == y),
        (Value::Decimal(x), Value::Decimal(y)) => Some(x == y),
        (Value::Integer(x), Value::Decimal(y)) => Some(*x as f64 == *y),
        (Value::Decimal(x), Value::Integer(y)) => Some(*x == *y as f64),
        (Value::String(x), Value::String(y)) => Some(x == y),
        (Value::Date(x), Value::Date(y))
        | (Value::DateTime(x), Value::DateTime(y))
        | (Value::Time(x), Value::Time(y)) => Some(x == y),
        (Value::Quantity(xv, xu), Value::Quantity(yv, yu)) => Some(xv == yv && xu == yu),
        _ => None,
    }
}

fn values_equivalent(a: &Value, b: &Value) -> bool {
    // For ~: null ~ null is true, case-insensitive string comparison
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::String(x), Value::String(y)) => x.to_lowercase() == y.to_lowercase(),
        _ => values_equal(a, b).unwrap_or(false),
    }
}

fn compare_values(a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Ok(x.cmp(y)),
        (Value::Decimal(x), Value::Decimal(y)) => {
            Ok(x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
        }
        (Value::Integer(x), Value::Decimal(y)) => Ok((*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Decimal(x), Value::Integer(y)) => Ok(x
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

fn collection_to_bool(col: &Collection) -> Option<bool> {
    match col.as_slice() {
        [] => None,
        [Value::Bool(b)] => Some(*b),
        _ => None,
    }
}

fn bool_col(v: Option<bool>) -> Collection {
    match v {
        Some(b) => vec![Value::Bool(b)],
        None => vec![],
    }
}

fn three_valued_and(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

fn three_valued_or(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

fn value_is_type(v: &Value, type_name: &str) -> bool {
    match type_name {
        "Boolean" | "boolean" | "bool" => matches!(v, Value::Bool(_)),
        "Integer" | "integer" => matches!(v, Value::Integer(_)),
        "Decimal" | "decimal" => matches!(v, Value::Decimal(_)),
        "String" | "string" => matches!(v, Value::String(_)),
        "Date" | "date" => matches!(v, Value::Date(_)),
        "DateTime" | "dateTime" => matches!(v, Value::DateTime(_)),
        "Time" | "time" => matches!(v, Value::Time(_)),
        "Quantity" | "quantity" => matches!(v, Value::Quantity(_, _)),
        _ => false,
    }
}

fn string_or_empty(col: &Collection) -> Arc<str> {
    col.first()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| Arc::from(""))
}

fn string_transform(focus: &[Value], f: impl Fn(&str) -> String) -> Result<Collection, EvalError> {
    match focus.first().and_then(|v| v.as_string()) {
        Some(s) => Ok(vec![Value::String(Arc::from(f(&s).as_str()))]),
        None => Ok(vec![]),
    }
}

fn single_integer(col: &Collection, context: &str) -> Result<i64, EvalError> {
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

fn eval_integer_arg(col: &Collection, context: &str) -> Result<i64, EvalError> {
    single_integer(col, context)
}

fn eval_string_arg(col: &Collection, context: &str) -> Result<Arc<str>, EvalError> {
    match col.first().and_then(|v| v.as_string()) {
        Some(s) => Ok(s),
        None => Err(EvalError::Type(format!("{context}: expected String"))),
    }
}

fn eval_type_name(expr: &Expr, _ctx: &[Value]) -> Result<String, EvalError> {
    // For ofType(TypeName), the argument is an identifier, not evaluated
    match expr {
        Expr::Ident(name) => Ok(name.clone()),
        _ => Err(EvalError::Type(
            "ofType() argument must be a type name identifier".into(),
        )),
    }
}

fn collect_descendants(v: &Value, out: &mut Collection) {
    if let Value::Object(fields) = v {
        for children in fields.values() {
            for child in children {
                out.push(child.clone());
                collect_descendants(child, out);
            }
        }
    }
}

// ── Regex helpers (powered by regex-lite) ────────────────────────────────────

fn regex_matches(text: &str, pattern: &str) -> Result<bool, EvalError> {
    Regex::new(pattern)
        .map(|re| re.is_match(text))
        .map_err(|e| EvalError::Type(format!("matches(): invalid pattern: {e}")))
}

fn regex_replace_all(text: &str, pattern: &str, replacement: &str) -> Result<String, EvalError> {
    Regex::new(pattern)
        .map(|re| re.replace_all(text, replacement).into_owned())
        .map_err(|e| EvalError::Type(format!("replaceMatches(): invalid pattern: {e}")))
}

// ── Minimal base64 (no external dep) ─────────────────────────────────────────

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64URL_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64_encode_with(input: &[u8], alphabet: &[u8]) -> String {
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };
        out.push(alphabet[(b0 >> 2) & 0x3f] as char);
        out.push(alphabet[((b0 << 4) | (b1 >> 4)) & 0x3f] as char);
        if chunk.len() > 1 {
            out.push(alphabet[((b1 << 2) | (b2 >> 6)) & 0x3f] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(alphabet[b2 & 0x3f] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_encode(input: &[u8]) -> String {
    base64_encode_with(input, BASE64_CHARS)
}

fn base64_encode_url(input: &[u8]) -> String {
    base64_encode_with(input, BASE64URL_CHARS).replace('=', "")
}

fn base64_decode_with(input: &[u8], alphabet: &[u8]) -> Vec<u8> {
    let lookup: Vec<i8> = {
        let mut table = vec![-1i8; 256];
        for (i, &c) in alphabet.iter().enumerate() {
            table[c as usize] = i as i8;
        }
        table
    };
    let filtered: Vec<u8> = input
        .iter()
        .filter(|&&c| c != b'=' && lookup[c as usize] >= 0)
        .copied()
        .collect();
    let mut out = Vec::new();
    for chunk in filtered.chunks(4) {
        let get = |i: usize| {
            if i < chunk.len() {
                lookup[chunk[i] as usize].max(0) as u8
            } else {
                0
            }
        };
        let b0 = get(0);
        let b1 = get(1);
        let b2 = get(2);
        let b3 = get(3);
        out.push((b0 << 2) | (b1 >> 4));
        if chunk.len() > 2 {
            out.push((b1 << 4) | (b2 >> 2));
        }
        if chunk.len() > 3 {
            out.push((b2 << 6) | b3);
        }
    }
    out
}

fn base64_decode(input: &[u8]) -> Vec<u8> {
    base64_decode_with(input, BASE64_CHARS)
}

fn base64_decode_url(input: &[u8]) -> Vec<u8> {
    base64_decode_with(input, BASE64URL_CHARS)
}

#[allow(unused_imports)]
use crate::value::from_parser_value;
