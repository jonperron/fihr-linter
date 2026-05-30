use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::base64::{base64_decode, base64_decode_url, base64_encode, base64_encode_url};
use super::helpers::{
    check_arity, eval_integer_arg, eval_string_arg, json_unescape, string_transform,
};
use super::regex::{regex_matches, regex_matches_full, regex_replace_all};

impl Evaluator {
    /// Handle string manipulation functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_string_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_string(name, args, focus, ctx).transpose()
    }

    #[allow(clippy::too_many_lines)]
    fn try_string(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            "toString" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .map(|v| Value::String(Arc::from(v.to_string().as_str())))
                    .collect()
            }
            "length" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::String(s)) => vec![Value::Integer(s.chars().count() as i64)],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "length() requires String, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "startsWith" => {
                check_arity(name, args, 1)?;
                let prefix = eval_string_arg(&self.eval(&args[0], ctx)?, "startsWith")?;
                vec![Value::Bool(
                    focus
                        .first()
                        .and_then(|v| v.as_string())
                        .map(|s| s.starts_with(prefix.as_ref()))
                        .unwrap_or(false),
                )]
            }
            "endsWith" => {
                check_arity(name, args, 1)?;
                let suffix = eval_string_arg(&self.eval(&args[0], ctx)?, "endsWith")?;
                vec![Value::Bool(
                    focus
                        .first()
                        .and_then(|v| v.as_string())
                        .map(|s| s.ends_with(suffix.as_ref()))
                        .unwrap_or(false),
                )]
            }
            "contains" => {
                check_arity(name, args, 1)?;
                let sub = eval_string_arg(&self.eval(&args[0], ctx)?, "contains")?;
                vec![Value::Bool(
                    focus
                        .first()
                        .and_then(|v| v.as_string())
                        .map(|s| s.contains(sub.as_ref()))
                        .unwrap_or(false),
                )]
            }
            "upper" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, |s| s.to_uppercase()).map(Some);
            }
            "lower" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, |s| s.to_lowercase()).map(Some);
            }
            "trim" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, |s| s.trim().to_owned()).map(Some);
            }
            "escape" => {
                check_arity(name, args, 1)?;
                let target = eval_string_arg(&self.eval(&args[0], ctx)?, "escape")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let result = match target.as_ref() {
                            "html" => s
                                .replace('&', "&amp;")
                                .replace('<', "&lt;")
                                .replace('>', "&gt;")
                                .replace('"', "&quot;")
                                .replace('\'', "&#39;"),
                            "json" => s
                                .replace('\\', "\\\\")
                                .replace('"', "\\\"")
                                .replace('\n', "\\n")
                                .replace('\r', "\\r")
                                .replace('\t', "\\t"),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "escape(): unknown target '{other}'; expected 'html' or 'json'"
                                )));
                            }
                        };
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "unescape" => {
                check_arity(name, args, 1)?;
                let target = eval_string_arg(&self.eval(&args[0], ctx)?, "unescape")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let result = match target.as_ref() {
                            "html" => s
                                .replace("&lt;", "<")
                                .replace("&gt;", ">")
                                .replace("&quot;", "\"")
                                .replace("&#39;", "'")
                                .replace("&amp;", "&"),
                            "json" => json_unescape(&s),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "unescape(): unknown target '{other}'; expected 'html' or 'json'"
                                )));
                            }
                        };
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "substring" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let s = match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => s,
                    None => return Ok(Some(vec![])),
                };
                let chars: Vec<char> = s.chars().collect();
                let start_i = eval_integer_arg(&self.eval(&args[0], ctx)?, "substring start")?;
                if start_i < 0 {
                    return Ok(Some(vec![]));
                }
                let start = start_i as usize;
                let end = if args.len() == 2 {
                    let len_i = eval_integer_arg(&self.eval(&args[1], ctx)?, "substring length")?;
                    if len_i <= 0 {
                        return Ok(Some(vec![Value::String(Arc::from(""))]));
                    }
                    (start + len_i as usize).min(chars.len())
                } else {
                    chars.len()
                };
                let sub: String = chars[start.min(chars.len())..end].iter().collect();
                vec![Value::String(Arc::from(sub.as_str()))]
            }
            "replace" => {
                check_arity(name, args, 2)?;
                let pattern = eval_string_arg(&self.eval(&args[0], ctx)?, "replace pattern")?;
                let replacement =
                    eval_string_arg(&self.eval(&args[1], ctx)?, "replace replacement")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let result = s.replace(pattern.as_ref(), replacement.as_ref());
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "matches" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let pattern = eval_string_arg(&self.eval(&args[0], ctx)?, "matches pattern")?;
                let flags = if args.len() == 2 {
                    eval_string_arg(&self.eval(&args[1], ctx)?, "matches flags")?
                } else {
                    Arc::from("")
                };
                let text = focus
                    .first()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| Arc::from(""));
                let matched = regex_matches(&text, &pattern, &flags)?;
                vec![Value::Bool(matched)]
            }
            "replaceMatches" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let pattern =
                    eval_string_arg(&self.eval(&args[0], ctx)?, "replaceMatches pattern")?;
                let replacement =
                    eval_string_arg(&self.eval(&args[1], ctx)?, "replaceMatches replacement")?;
                let flags = if args.len() == 3 {
                    eval_string_arg(&self.eval(&args[2], ctx)?, "replaceMatches flags")?
                } else {
                    Arc::from("")
                };
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let result = regex_replace_all(&s, &pattern, &replacement, &flags)?;
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "matchesFull" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let pattern = eval_string_arg(&self.eval(&args[0], ctx)?, "matchesFull pattern")?;
                let flags = if args.len() == 2 {
                    eval_string_arg(&self.eval(&args[1], ctx)?, "matchesFull flags")?
                } else {
                    Arc::from("")
                };
                let text = focus
                    .first()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| Arc::from(""));
                let matched = regex_matches_full(&text, &pattern, &flags)?;
                vec![Value::Bool(matched)]
            }
            "indexOf" => {
                check_arity(name, args, 1)?;
                let sub = eval_string_arg(&self.eval(&args[0], ctx)?, "indexOf")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let idx = s
                            .char_indices()
                            .enumerate()
                            .find(|(_, (byte_pos, _))| s[*byte_pos..].starts_with(sub.as_ref()))
                            .map(|(char_idx, _)| char_idx as i64)
                            .unwrap_or(-1);
                        vec![Value::Integer(idx)]
                    }
                    None => vec![Value::Integer(-1)],
                }
            }
            "lastIndexOf" => {
                check_arity(name, args, 1)?;
                let sub = eval_string_arg(&self.eval(&args[0], ctx)?, "lastIndexOf")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        let sub_chars: Vec<char> = sub.chars().collect();
                        let mut last: i64 = -1;
                        if !sub_chars.is_empty() && sub_chars.len() <= chars.len() {
                            for i in 0..=(chars.len() - sub_chars.len()) {
                                if chars[i..i + sub_chars.len()] == sub_chars[..] {
                                    last = i as i64;
                                }
                            }
                        } else if sub_chars.is_empty() {
                            last = chars.len() as i64;
                        }
                        vec![Value::Integer(last)]
                    }
                    None => vec![Value::Integer(-1)],
                }
            }
            "split" => {
                check_arity(name, args, 1)?;
                let sep = eval_string_arg(&self.eval(&args[0], ctx)?, "split")?;
                match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => s
                        .split(sep.as_ref())
                        .map(|part| Value::String(Arc::from(part)))
                        .collect(),
                    None => vec![],
                }
            }
            "join" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
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
                vec![Value::String(Arc::from(joined.join(sep.as_ref()).as_str()))]
            }
            "encode" => {
                check_arity(name, args, 1)?;
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
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "decode" => {
                check_arity(name, args, 1)?;
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
                        vec![Value::String(Arc::from(decoded.as_str()))]
                    }
                    None => vec![],
                }
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}
