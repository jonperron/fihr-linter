#![forbid(unsafe_code)]
//! FHIRPath 2.0 evaluator.
//!
//! Implements the full FHIRPath specification required to evaluate
//! `constraint.expression` invariants from FHIR StructureDefinitions.

mod ast;
mod error;
mod eval;
mod parser;
mod value;

pub use error::{EvalError, ParseError};
pub use value::{Collection, Value, from_parser_value, resource_to_value};

/// Parse and evaluate a FHIRPath expression against a context collection.
///
/// # Errors
///
/// Returns [`ParseError`] if the expression is syntactically invalid, or
/// [`EvalError`] if evaluation fails (type errors, undefined functions, etc.).
pub fn evaluate(expression: &str, context: &[Value]) -> Result<Collection, EvalError> {
    let expr = parser::parse(expression)?;
    let root = context.to_vec();
    eval::eval(&expr, &root, &root)
}

/// Parse a FHIRPath expression into an AST without evaluating it.
pub fn parse(expression: &str) -> Result<ast::Expr, ParseError> {
    parser::parse(expression)
}
