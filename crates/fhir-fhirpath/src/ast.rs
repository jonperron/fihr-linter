/// Abstract Syntax Tree for FHIRPath 2.0 expressions.
///
/// The AST is produced by `parser::parse` and consumed by `eval::Evaluator`.
/// A FHIRPath expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // ── Literals ────────────────────────────────────────────────────────────
    Null,
    Bool(bool),
    Integer(i64),
    Decimal(f64),
    /// String literal (single-quoted in FHIRPath).
    String(String),
    /// Temporal literals: stored as raw ISO string + kind.
    Temporal(TemporalKind, String),
    /// Quantity literal: value + unit.
    Quantity(f64, String),

    // ── Navigation ──────────────────────────────────────────────────────────
    /// `$this`
    DollarThis,
    /// `$index`
    DollarIndex,
    /// Identifier (path step, e.g. `name`, `Patient`).
    Ident(String),
    /// `.field` navigation applied to a base expression.
    Dot(Box<Expr>, Box<Expr>),
    /// `expr[index]` index step.
    Index(Box<Expr>, Box<Expr>),

    // ── Function calls ──────────────────────────────────────────────────────
    /// `name(args…)` function call on `$this` context.
    FuncCall(String, Vec<Expr>),
    /// `expr.name(args…)` method-style call.
    Method(Box<Expr>, String, Vec<Expr>),

    // ── Arithmetic ──────────────────────────────────────────────────────────
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    DivInt(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    /// String concatenation (`&`).
    Concat(Box<Expr>, Box<Expr>),
    /// Unary negation.
    Neg(Box<Expr>),

    // ── Comparisons ─────────────────────────────────────────────────────────
    Eq(Box<Expr>, Box<Expr>),
    Neq(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Lte(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Gte(Box<Expr>, Box<Expr>),
    /// Equivalent (`~`).
    Equiv(Box<Expr>, Box<Expr>),
    /// Not-equivalent (`!~`).
    NotEquiv(Box<Expr>, Box<Expr>),

    // ── Boolean operators ────────────────────────────────────────────────────
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Xor(Box<Expr>, Box<Expr>),
    Implies(Box<Expr>, Box<Expr>),

    // ── Type operators ───────────────────────────────────────────────────────
    Is(Box<Expr>, String),
    As(Box<Expr>, String),

    // ── Set operators ────────────────────────────────────────────────────────
    Union(Box<Expr>, Box<Expr>),
    In(Box<Expr>, Box<Expr>),
    Contains(Box<Expr>, Box<Expr>),
}

/// Kind of a temporal literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalKind {
    Date,
    DateTime,
    Time,
}
