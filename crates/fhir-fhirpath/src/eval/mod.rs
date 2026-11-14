/// FHIRPath expression evaluator (FHIRPath release 2, as used by FHIR R5).
///
/// The evaluator walks an [`Expr`] AST and produces a [`Collection`].
/// Implemented functions are grouped by category; any other function name
/// yields [`EvalError::UndefinedFunction`].
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use crate::ast::{Expr, TemporalKind};
use crate::error::EvalError;
use crate::value::{Collection, Value};

mod base64;
mod collection;
mod helpers;
mod math;
mod misc;
mod regex;
mod string_fns;
mod temporal;
mod type_conv;

use helpers::{
    as_type, bool_col, collections_equal, collections_equivalent, contains_value, is_type,
    numeric_op, singleton, three_valued_and, three_valued_implies, three_valued_or,
};

/// Evaluate a FHIRPath expression against a context value.
///
/// `context` is the *input collection* (the current `$this` node).
/// Returns a collection of results. `is_choice_variant(base, suffix)` tells
/// whether `base` is a choice element with a type for the capitalised `suffix`,
/// so that `value` finds `valueQuantity`.
pub fn eval(
    expr: &Expr,
    context: &[Value],
    is_choice_variant: &dyn Fn(&str, &str) -> bool,
    environment: &Environment<'_>,
) -> Result<Collection, EvalError> {
    Evaluator {
        is_choice_variant,
        environment,
        context,
        total: None,
        index: None,
        string_chars: &Cell::new(0),
        variables: None,
    }
    .eval(expr, context)
}

/// FHIR environment variables (R5 fhirpath.html §2.1.9.1.5), set by the caller.
///
/// Unset, `%resource` is the input when the input is a single resource
/// (an object with a `resourceType`), and `%rootResource` is `%resource`.
/// Otherwise reading them is an error, so a constraint is skipped rather
/// than evaluated against the wrong node.
#[derive(Clone, Copy, Default)]
pub struct Environment<'a> {
    /// `%resource`: the resource that contains the input node.
    pub resource: Option<&'a Value>,
    /// `%rootResource`: the container of `%resource` when it is a contained
    /// resource, else `%resource` itself.
    pub root_resource: Option<&'a Value>,
    /// FHIR type inheritance: `is_subtype(child, ancestor)` tells whether the
    /// FHIR type `child` is `ancestor` or derives from it (build it from
    /// `Registry::type_ancestors`; `code` is a `string`, `url` a `uri`), and
    /// `is_subtype(name, name)` whether `name` is a FHIR type at all. With it,
    /// `is`, `as` and `ofType` answer for values that carry their FHIR type.
    /// Unset, asking whether a String is a FHIR string type (`code`, `uri`...)
    /// is an error, as the String's FHIR type cannot be checked.
    pub is_subtype: Option<&'a IsSubtype<'a>>,
}

impl std::fmt::Debug for Environment<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment")
            .field("resource", &self.resource)
            .field("root_resource", &self.root_resource)
            .field("is_subtype", &self.is_subtype.map(|_| "Fn"))
            .finish()
    }
}

/// FHIR type inheritance, see [`Environment::is_subtype`].
pub type IsSubtype<'a> = dyn Fn(&str, &str) -> bool + 'a;

pub(super) struct Evaluator<'a> {
    is_choice_variant: &'a dyn Fn(&str, &str) -> bool,
    /// Characters of String values built so far, shared by every nested
    /// evaluator (see [`helpers::MAX_EVAL_STRING_CHARS`]).
    string_chars: &'a Cell<usize>,
    environment: &'a Environment<'a>,
    /// `%context`: the input collection the evaluation started from.
    context: &'a [Value],
    /// `$total` while an `aggregate()` expression is evaluated.
    total: Option<&'a [Value]>,
    /// `$index` while an expression parameter is evaluated for one item.
    index: Option<usize>,
    /// Variables of `defineVariable()` in scope, innermost first.
    variables: Scope,
}

/// A `defineVariable()` variable and the scope it was defined in.
pub(super) struct Variable {
    name: Arc<str>,
    value: Collection,
    outer: Scope,
}

pub(super) type Scope = Option<Rc<Variable>>;

/// Names of the system variables `defineVariable()` may not redefine.
fn is_system_variable(name: &str) -> bool {
    matches!(
        name,
        "context" | "resource" | "rootResource" | "ucum" | "sct" | "loinc"
    ) || name.starts_with("vs-")
        || name.starts_with("ext-")
}

impl Evaluator<'_> {
    /// Evaluate an expression parameter for the input item at `index`, which
    /// becomes `$this` and `$index` (N1 §5).
    pub(super) fn eval_item(
        &self,
        expr: &Expr,
        item: &Value,
        index: usize,
    ) -> Result<Collection, EvalError> {
        Evaluator {
            index: Some(index),
            variables: self.variables.clone(),
            ..*self
        }
        .eval(expr, std::slice::from_ref(item))
    }

    /// Resolve `%name`: the constants of FHIRPath N1 (`%ucum`) and R5
    /// fhirpath.html §2.1.9.1.5 (`%sct`, `%loinc`, `%vs-*`, `%ext-*`),
    /// `%context`, and `%resource` / `%rootResource` (see [`Environment`]).
    fn environment_variable(&self, name: &str) -> Result<Collection, EvalError> {
        if let Some(variable) = self.variable(name) {
            return Ok(variable.value.clone());
        }
        let input_resource = match self.context {
            [value @ Value::Object(map)] if map.contains_key("resourceType") => Some(value),
            _ => None,
        };
        let resource = self.environment.resource.or(input_resource);
        let constant = match name {
            "ucum" => Some("http://unitsofmeasure.org".to_owned()),
            "sct" => Some("http://snomed.info/sct".to_owned()),
            "loinc" => Some("http://loinc.org".to_owned()),
            _ => name
                .strip_prefix("vs-")
                .filter(|vs| !vs.is_empty())
                .map(|vs| format!("http://hl7.org/fhir/ValueSet/{vs}"))
                .or_else(|| {
                    name.strip_prefix("ext-")
                        .filter(|ext| !ext.is_empty())
                        .map(|ext| format!("http://hl7.org/fhir/StructureDefinition/{ext}"))
                }),
        };
        if let Some(url) = constant {
            return Ok(vec![Value::String(Arc::from(url))]);
        }
        let value = match name {
            "context" => return Ok(self.context.to_vec()),
            "resource" => resource,
            "rootResource" => self.environment.root_resource.or(resource),
            _ => {
                return Err(EvalError::Type(format!(
                    "unknown environment variable %{name}"
                )));
            }
        };
        value
            .map(|v| vec![v.clone()])
            .ok_or_else(|| EvalError::Type(format!("%{name} is not set for this input")))
    }

    /// Evaluate `expr`; every intermediate collection is checked against
    /// [`helpers::MAX_COLLECTION_LEN`]. Producers that iterate their focus
    /// also check per item, so a step overshoots by at most one item's output
    /// (or the other side of a combine or union).
    pub(super) fn eval(&self, expr: &Expr, ctx: &[Value]) -> Result<Collection, EvalError> {
        let col = self.eval_expr(expr, ctx)?;
        helpers::check_collection_len(col.len())?;
        Ok(col)
    }

    fn eval_expr(&self, expr: &Expr, ctx: &[Value]) -> Result<Collection, EvalError> {
        match expr {
            // ── Literals ────────────────────────────────────────────────────
            Expr::Null => Ok(vec![]),
            Expr::Bool(b) => Ok(vec![Value::Bool(*b)]),
            Expr::Integer(n) => Ok(vec![Value::Integer(*n)]),
            Expr::Long(n) => Ok(vec![Value::Long(*n)]),
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
            Expr::DollarIndex => self
                .index
                .map(|i| vec![Value::Integer(i as i64)])
                .ok_or_else(|| EvalError::Type("$index outside an iteration".to_owned())),
            Expr::DollarTotal => self
                .total
                .map(<[Value]>::to_vec)
                .ok_or_else(|| EvalError::Type("$total outside aggregate()".to_owned())),
            Expr::External(name) => self.environment_variable(name),

            // ── Identifier / path step ─────────────────────────────────────
            Expr::Ident(name) => self.eval_ident(name, ctx, true),

            // ── Navigation and function calls ──────────────────────────────
            Expr::Dot(..) | Expr::FuncCall(..) | Expr::Method(..) | Expr::Index(..) => {
                Ok(self.eval_chain(expr, ctx)?.0)
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
                let Some(v) = singleton(&col, "negation")? else {
                    return Ok(vec![]);
                };
                match v {
                    Value::Integer(n) => n
                        .checked_neg()
                        .map(|r| vec![Value::Integer(r)])
                        .ok_or_else(|| EvalError::Type("integer overflow in negation".into())),
                    Value::Long(n) => n
                        .checked_neg()
                        .map(|r| vec![Value::Long(r)])
                        .ok_or_else(|| EvalError::Type("integer overflow in negation".into())),
                    Value::Decimal(d) => Ok(vec![Value::Decimal(-d)]),
                    v => Err(EvalError::Type(format!("cannot negate {}", v.type_name()))),
                }
            }
            Expr::Concat(l, r) => {
                let ls = self.eval(l, ctx)?;
                let rs = self.eval(r, ctx)?;
                singleton(&ls, "&")?;
                singleton(&rs, "&")?;
                let left = helpers::string_or_empty(&ls);
                let right = helpers::string_or_empty(&rs);
                helpers::check_string_len(left.chars().count() + right.chars().count())?;
                Ok(vec![self.string_value(&format!("{left}{right}"))?])
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
            Expr::And(l, r) => self.eval_logical(l, r, ctx, [false, false], three_valued_and),
            Expr::Or(l, r) => self.eval_logical(l, r, ctx, [true, true], three_valued_or),
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
                self.eval_logical(l, r, ctx, [false, true], three_valued_implies)
            }

            // ── Type operators ────────────────────────────────────────────
            Expr::Is(base, type_name) => is_type(
                &self.eval(base, ctx)?,
                type_name,
                self.environment.is_subtype,
            ),
            Expr::As(base, type_name) => as_type(
                &self.eval(base, ctx)?,
                type_name,
                self.environment.is_subtype,
            ),

            // ── Set operators ─────────────────────────────────────────────
            Expr::Union(l, r) => {
                let ls = self.eval(l, ctx)?;
                let rs = self.eval(r, ctx)?;
                Ok(helpers::distinct(ls.into_iter().chain(rs)))
            }
            Expr::In(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                Ok(singleton(&lc, "in")?
                    .map(|v| Value::Bool(contains_value(&rc, v)))
                    .into_iter()
                    .collect())
            }
            Expr::Contains(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                Ok(singleton(&rc, "contains")?
                    .map(|v| Value::Bool(contains_value(&lc, v)))
                    .into_iter()
                    .collect())
            }
        }
    }

    // ── Path navigation ─────────────────────────────────────────────────────

    fn eval_ident(
        &self,
        name: &str,
        ctx: &[Value],
        allow_type_name: bool,
    ) -> Result<Collection, EvalError> {
        let mut result = Vec::new();
        for item in ctx {
            if let Some(fields) = item.fields() {
                if let Some(children) = fields.get(name) {
                    result.extend_from_slice(children);
                } else if let Some(variants) = self.choice_variants(fields, name) {
                    // R5 §2.1.9.1.1: a choice element is named without its `[x]` suffix.
                    result.extend(variants);
                } else if allow_type_name
                    && matches!(
                        fields.get("resourceType").map(Vec::as_slice),
                        Some([Value::String(resource_type)]) if &**resource_type == name
                    )
                {
                    // FHIRPath: an expression may start with the context's type name.
                    result.push(item.clone());
                }
            }
            // Per item, so many copies of one node cannot expand past the cap.
            helpers::check_collection_len(result.len())?;
        }
        Ok(result)
    }

    /// Values of the choice variants of `name` (`valueQuantity` for `value`), or `None`.
    fn choice_variants(
        &self,
        fields: &indexmap::IndexMap<Arc<str>, Collection>,
        name: &str,
    ) -> Option<Collection> {
        let variants: Collection = fields
            .iter()
            .filter(|(key, _)| {
                key.strip_prefix(name).is_some_and(|suffix| {
                    suffix.starts_with(|c: char| c.is_ascii_uppercase())
                        && (self.is_choice_variant)(name, suffix)
                })
            })
            .flat_map(|(_, values)| values.iter().cloned())
            .collect();
        (!variants.is_empty()).then_some(variants)
    }

    // ── Expression chains and defineVariable ────────────────────────────────

    /// The innermost `defineVariable()` variable called `name`, if in scope.
    fn variable(&self, name: &str) -> Option<&Variable> {
        std::iter::successors(self.variables.as_deref(), |v| v.outer.as_deref())
            .find(|v| &*v.name == name)
    }

    /// Evaluate a path or call chain, and return the variables it leaves in
    /// scope: a `defineVariable()` is visible to the rest of its chain only.
    fn eval_chain(&self, expr: &Expr, ctx: &[Value]) -> Result<(Collection, Scope), EvalError> {
        let (col, scope) = match expr {
            Expr::Dot(base, step) => {
                let (base_col, scope) = self.eval_chain(base, ctx)?;
                let next = Evaluator {
                    variables: scope,
                    ..*self
                };
                match &**step {
                    // A type name may only lead the expression, not follow a dot.
                    Expr::Ident(name) => (next.eval_ident(name, &base_col, false)?, next.variables),
                    _ => next.eval_chain(step, &base_col)?,
                }
            }
            Expr::Method(base, name, args) => {
                let (base_col, scope) = self.eval_chain(base, ctx)?;
                let next = Evaluator {
                    variables: scope,
                    ..*self
                };
                next.call_chain(name, args, &base_col, ctx)?
            }
            Expr::FuncCall(name, args) => self.call_chain(name, args, ctx, ctx)?,
            Expr::Index(base, index_expr) => {
                let (base_col, scope) = self.eval_chain(base, ctx)?;
                let next = Evaluator {
                    variables: scope,
                    ..*self
                };
                let idx_col = next.eval(index_expr, ctx)?;
                let idx = helpers::single_integer(&idx_col, "index")?;
                // FHIRPath §5.5: an index past the end (or negative) yields empty.
                let item = usize::try_from(idx).ok().and_then(|i| base_col.get(i));
                (item.cloned().into_iter().collect(), next.variables)
            }
            _ => return Ok((self.eval(expr, ctx)?, self.variables.clone())),
        };
        helpers::check_collection_len(col.len())?;
        Ok((col, scope))
    }

    /// Call a function in a chain; `defineVariable(name [, expr])` returns
    /// its input and adds the variable (`expr` on the input, or the input).
    fn call_chain(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<(Collection, Scope), EvalError> {
        if name != "defineVariable" {
            let col = self.call_function(name, args, focus, ctx)?;
            return Ok((col, self.variables.clone()));
        }
        helpers::check_arity_range(name, args, 1, 2)?;
        let name_col = self.eval(&args[0], focus)?;
        let Some(Value::String(var_name)) = singleton(&name_col, name)?.map(Value::primitive_value)
        else {
            return Err(EvalError::Type(
                "defineVariable() name must be a String".to_owned(),
            ));
        };
        if is_system_variable(var_name) || self.variable(var_name).is_some() {
            return Err(EvalError::Type(format!(
                "variable %{var_name} is already defined"
            )));
        }
        let value = match args.get(1) {
            Some(expr) => self.eval(expr, focus)?,
            None => focus.to_vec(),
        };
        let variable = Variable {
            name: Arc::clone(var_name),
            value,
            outer: self.variables.clone(),
        };
        Ok((focus.to_vec(), Some(Rc::new(variable))))
    }

    // ── Function dispatch ───────────────────────────────────────────────────

    fn call_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        if let Some(r) = self.call_collection_function(name, args, focus, ctx) {
            return r;
        }
        // String, math and conversion functions see each primitive's value; a
        // primitive without one is not an input item for them.
        let values = helpers::primitive_values(focus);
        if let Some(r) = self.call_string_function(name, args, &values, ctx) {
            return r;
        }
        if let Some(r) = self.call_math_function(name, args, &values, ctx) {
            return r;
        }
        if let Some(r) = self.call_type_function(name, args, &values, ctx) {
            return r;
        }
        if let Some(r) = self.call_misc_function(name, args, focus, ctx) {
            return r;
        }
        Err(EvalError::UndefinedFunction(name.to_owned()))
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
        let (Some(lv), Some(rv)) = (singleton(&lc, "arithmetic")?, singleton(&rc, "arithmetic")?)
        else {
            return Ok(vec![]);
        };
        if let (Value::String(a), Value::String(b), ArithOp::Add) = (lv, rv, &op) {
            helpers::check_string_len(a.chars().count() + b.chars().count())?;
            return Ok(vec![self.string_value(&format!("{a}{b}"))?]);
        }
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
                    return Ok(vec![]);
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
        // A lone item without a value (a primitive with only extensions) is
        // empty, as in singleton evaluation.
        let without_lone_null = |col: Collection| match col.as_slice() {
            [v] if matches!(v.primitive_value(), Value::Null) => vec![],
            _ => col,
        };
        let lc = without_lone_null(self.eval(l, ctx)?);
        let rc = without_lone_null(self.eval(r, ctx)?);
        let either_empty = lc.is_empty() || rc.is_empty();
        let result = match op {
            // N1 §6.1.3: `~` is never empty, even on empty operands.
            CmpOp::Equiv => Some(collections_equivalent(&lc, &rc)),
            CmpOp::NotEquiv => Some(!collections_equivalent(&lc, &rc)),
            CmpOp::Eq if !either_empty => collections_equal(&lc, &rc),
            CmpOp::Neq if !either_empty => collections_equal(&lc, &rc).map(|b| !b),
            CmpOp::Eq | CmpOp::Neq => None,
            // Ordering checks both operands are singletons before an empty side wins.
            _ => {
                let (Some(a), Some(b)) =
                    (singleton(&lc, "comparison")?, singleton(&rc, "comparison")?)
                else {
                    return Ok(vec![]);
                };
                let Some(ord) = helpers::operator_order(a, b)? else {
                    return Ok(vec![]);
                };
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

    /// `and`, `or` and `implies`: when one side has the value in `deciding`
    /// (left, right), the truth table fixes the result, so an error on the
    /// other side is ignored (N1 §6.5). The right side is skipped when the
    /// left decides.
    fn eval_logical(
        &self,
        l: &Expr,
        r: &Expr,
        ctx: &[Value],
        deciding: [bool; 2],
        combine: fn(Option<bool>, Option<bool>) -> Option<bool>,
    ) -> Result<Collection, EvalError> {
        let left = self.eval_bool(l, ctx);
        if let Ok(Some(b)) = left
            && b == deciding[0]
        {
            return Ok(bool_col(combine(Some(b), None)));
        }
        let right = self.eval_bool(r, ctx);
        if let Ok(Some(b)) = right
            && b == deciding[1]
        {
            return Ok(bool_col(combine(left.unwrap_or(None), Some(b))));
        }
        Ok(bool_col(combine(left?, right?)))
    }

    /// Evaluate an expression as a three-valued boolean (`None` = empty collection).
    fn eval_bool(&self, expr: &Expr, ctx: &[Value]) -> Result<Option<bool>, EvalError> {
        helpers::singleton_bool(&self.eval(expr, ctx)?, "boolean operator")
    }
}

// ── Helper types ─────────────────────────────────────────────────────────────

pub(super) enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    DivInt,
    Mod,
}

pub(super) enum CmpOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Equiv,
    NotEquiv,
}
