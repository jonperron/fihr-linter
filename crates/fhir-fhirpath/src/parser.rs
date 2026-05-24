/// FHIRPath expression parser.
///
/// Converts raw text into an [`Expr`] AST via the Pest grammar.
use pest::Parser as _;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{Expr, TemporalKind};
use crate::error::ParseError;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct FhirPathParser;

/// Parse a FHIRPath expression string into an [`Expr`] AST.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let pairs = FhirPathParser::parse(Rule::expression, input)
        .map_err(|e| ParseError::Syntax(e.to_string()))?;

    let top = pairs
        .into_iter()
        .next()
        .ok_or_else(|| ParseError::Syntax("empty input".into()))?;

    build_expr(
        top.into_inner()
            .next()
            .ok_or_else(|| ParseError::Syntax("empty expression".into()))?,
    )
}

fn build_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::implies_expr => build_implies(pair),
        Rule::or_expr => build_or(pair),
        Rule::and_expr => build_and(pair),
        Rule::membership_expr => build_membership(pair),
        Rule::ineq_expr => build_ineq(pair),
        Rule::eq_expr => build_eq(pair),
        Rule::type_expr => build_type(pair),
        Rule::union_expr => build_union(pair),
        Rule::add_expr => build_add(pair),
        Rule::mul_expr => build_mul(pair),
        Rule::unary_expr => build_unary(pair),
        Rule::postfix_expr => build_postfix(pair),
        Rule::expression_inner => build_expr(only_child(pair)?),
        Rule::paren_expr => build_expr(only_child(pair)?),
        // Primary atoms
        Rule::null_lit => Ok(Expr::Null),
        Rule::bool_lit => Ok(Expr::Bool(pair.as_str() == "true")),
        Rule::integer_lit => parse_integer(pair),
        Rule::decimal_lit => parse_decimal(pair),
        Rule::string_lit => Ok(Expr::String(unescape_string(pair.as_str()))),
        Rule::date_lit => Ok(Expr::Temporal(TemporalKind::Date, trim_at(pair.as_str()))),
        Rule::datetime_lit => Ok(Expr::Temporal(
            TemporalKind::DateTime,
            trim_at(pair.as_str()),
        )),
        Rule::time_lit => Ok(Expr::Temporal(TemporalKind::Time, trim_at_t(pair.as_str()))),
        Rule::quantity_lit => parse_quantity(pair),
        Rule::ident => Ok(Expr::Ident(pair.as_str().to_owned())),
        Rule::func_call => build_func_call(pair),
        Rule::dollar_this => Ok(Expr::DollarThis),
        Rule::dollar_index => Ok(Expr::DollarIndex),
        other => Err(ParseError::Syntax(format!("unexpected rule: {other:?}"))),
    }
}

// ── Binary operator helpers ────────────────────────────────────────────────

fn build_implies(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // Skip implies_kw if present, then parse the rhs
    match iter.next() {
        None => Ok(lhs),
        Some(_kw) => {
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(Expr::Implies(Box::new(lhs), Box::new(rhs)))
        }
    }
}

fn build_or(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // alternating: (or_kw | xor_kw, and_expr)*
    while let Some(op_pair) = iter.next() {
        let is_xor = op_pair.as_rule() == Rule::xor_kw;
        let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
        lhs = if is_xor {
            Expr::Xor(Box::new(lhs), Box::new(rhs))
        } else {
            Expr::Or(Box::new(lhs), Box::new(rhs))
        };
    }
    Ok(lhs)
}

fn build_and(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // alternating: (and_kw, membership_expr)*
    while let Some(_kw) = iter.next() {
        let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
        lhs = Expr::And(Box::new(lhs), Box::new(rhs));
    }
    Ok(lhs)
}

fn build_membership(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let is_contains = op_pair.as_rule() == Rule::contains_kw;
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            if is_contains {
                Ok(Expr::Contains(Box::new(lhs), Box::new(rhs)))
            } else {
                Ok(Expr::In(Box::new(lhs), Box::new(rhs)))
            }
        }
    }
}

fn build_ineq(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let op = op_pair.as_rule();
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(match op {
                Rule::lt => Expr::Lt(Box::new(lhs), Box::new(rhs)),
                Rule::lte => Expr::Lte(Box::new(lhs), Box::new(rhs)),
                Rule::gt => Expr::Gt(Box::new(lhs), Box::new(rhs)),
                Rule::gte => Expr::Gte(Box::new(lhs), Box::new(rhs)),
                other => return Err(ParseError::Syntax(format!("unexpected ineq op: {other:?}"))),
            })
        }
    }
}

fn build_eq(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let op = op_pair.as_rule();
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(match op {
                Rule::eq => Expr::Eq(Box::new(lhs), Box::new(rhs)),
                Rule::neq => Expr::Neq(Box::new(lhs), Box::new(rhs)),
                Rule::tilde_eq => Expr::Equiv(Box::new(lhs), Box::new(rhs)),
                Rule::tilde_neq => Expr::NotEquiv(Box::new(lhs), Box::new(rhs)),
                other => return Err(ParseError::Syntax(format!("unexpected eq op: {other:?}"))),
            })
        }
    }
}

fn build_type(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let is_as = op_pair.as_rule() == Rule::as_kw;
            let type_spec = iter
                .next()
                .ok_or_else(missing)?
                .into_inner()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(".");
            if is_as {
                Ok(Expr::As(Box::new(lhs), type_spec))
            } else {
                Ok(Expr::Is(Box::new(lhs), type_spec))
            }
        }
    }
}

fn build_union(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // alternating: (pipe, add_expr)*
    while let Some(_pipe) = iter.next() {
        let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
        lhs = Expr::Union(Box::new(lhs), Box::new(rhs));
    }
    Ok(lhs)
}

fn build_add(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    let children: Vec<Pair<Rule>> = iter.collect();
    let mut i = 0;
    while i < children.len() {
        let op = children[i].as_rule();
        i += 1;
        let rhs = build_expr(children[i].clone())?;
        i += 1;
        lhs = match op {
            Rule::plus => Expr::Add(Box::new(lhs), Box::new(rhs)),
            Rule::minus => Expr::Sub(Box::new(lhs), Box::new(rhs)),
            Rule::amp => Expr::Concat(Box::new(lhs), Box::new(rhs)),
            other => return Err(ParseError::Syntax(format!("unexpected add op: {other:?}"))),
        };
    }
    Ok(lhs)
}

fn build_mul(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    let children: Vec<Pair<Rule>> = iter.collect();
    let mut i = 0;
    while i < children.len() {
        let op = children[i].as_rule();
        i += 1;
        let rhs = build_expr(children[i].clone())?;
        i += 1;
        lhs = match op {
            Rule::star => Expr::Mul(Box::new(lhs), Box::new(rhs)),
            Rule::slash => Expr::Div(Box::new(lhs), Box::new(rhs)),
            Rule::div_kw => Expr::DivInt(Box::new(lhs), Box::new(rhs)),
            Rule::mod_kw => Expr::Mod(Box::new(lhs), Box::new(rhs)),
            other => return Err(ParseError::Syntax(format!("unexpected mul op: {other:?}"))),
        };
    }
    Ok(lhs)
}

fn build_unary(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let first = iter.next().ok_or_else(missing)?;
    match first.as_rule() {
        Rule::postfix_expr => build_postfix(first),
        Rule::unary_minus => {
            let operand = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(Expr::Neg(Box::new(operand)))
        }
        Rule::unary_plus => {
            // unary + is a no-op
            build_expr(iter.next().ok_or_else(missing)?)
        }
        other => Err(ParseError::Syntax(format!(
            "unexpected unary child: {other:?}"
        ))),
    }
}

fn build_postfix(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut base = build_expr(iter.next().ok_or_else(missing)?)?;
    for step in iter {
        match step.as_rule() {
            Rule::dot_step => {
                let inner = step.into_inner().next().ok_or_else(missing)?;
                match inner.as_rule() {
                    Rule::func_call => {
                        let (name, args) = parse_func_call_inner(inner)?;
                        base = Expr::Method(Box::new(base), name, args);
                    }
                    Rule::ident => {
                        base = Expr::Dot(
                            Box::new(base),
                            Box::new(Expr::Ident(inner.as_str().to_owned())),
                        );
                    }
                    other => {
                        return Err(ParseError::Syntax(format!(
                            "unexpected dot step: {other:?}"
                        )));
                    }
                }
            }
            Rule::index_step => {
                let inner = step.into_inner().next().ok_or_else(missing)?;
                let index_expr = build_expr(inner)?;
                base = Expr::Index(Box::new(base), Box::new(index_expr));
            }
            other => {
                return Err(ParseError::Syntax(format!(
                    "unexpected postfix step: {other:?}"
                )));
            }
        }
    }
    Ok(base)
}

fn build_func_call(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let (name, args) = parse_func_call_inner(pair)?;
    Ok(Expr::FuncCall(name, args))
}

fn parse_func_call_inner(pair: Pair<Rule>) -> Result<(String, Vec<Expr>), ParseError> {
    let mut iter = pair.into_inner();
    let name = iter.next().ok_or_else(missing)?.as_str().to_owned();
    let args = iter.map(build_expr).collect::<Result<Vec<_>, _>>()?;
    Ok((name, args))
}

// ── Literal parsers ────────────────────────────────────────────────────────

fn parse_integer(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let s = pair.into_inner().next().ok_or_else(missing)?.as_str();
    s.parse::<i64>()
        .map(Expr::Integer)
        .map_err(|_| ParseError::Syntax(format!("invalid integer: {s}")))
}

fn parse_decimal(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let s = pair.into_inner().next().ok_or_else(missing)?.as_str();
    s.parse::<f64>()
        .map(Expr::Decimal)
        .map_err(|_| ParseError::Syntax(format!("invalid decimal: {s}")))
}

fn parse_quantity(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let text = pair.as_str();
    // Format: <number> '<unit>'
    // Split on first whitespace
    let (num_str, rest) = text
        .split_once(' ')
        .ok_or_else(|| ParseError::Syntax(format!("invalid quantity: {text}")))?;
    let value = num_str
        .parse::<f64>()
        .map_err(|_| ParseError::Syntax(format!("invalid quantity number: {num_str}")))?;
    let unit = rest.trim().trim_matches('\'').to_owned();
    Ok(Expr::Quantity(value, unit))
}

/// Remove the leading `@` from a date/dateTime literal.
fn trim_at(s: &str) -> String {
    s.trim_start_matches('@').to_owned()
}

/// Remove the leading `@T` from a time literal.
fn trim_at_t(s: &str) -> String {
    s.trim_start_matches("@T").to_owned()
}

/// Unescape a FHIRPath string literal (strips surrounding `'`).
fn unescape_string(raw: &str) -> String {
    let inner = raw.trim_matches('\'');
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\'') => result.push('\''),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(n) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(n) {
                            result.push(c);
                        }
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

// ── Utilities ──────────────────────────────────────────────────────────────

fn only_child(pair: Pair<Rule>) -> Result<Pair<Rule>, ParseError> {
    pair.into_inner()
        .next()
        .ok_or_else(|| ParseError::Syntax("expected child pair".into()))
}

fn missing() -> ParseError {
    ParseError::Syntax("missing expected pair".into())
}
