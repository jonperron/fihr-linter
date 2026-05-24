/// Errors produced by the FHIRPath engine.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("syntax error: {0}")]
    Syntax(String),
}

/// Errors produced during evaluation.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum EvalError {
    #[error("type error: {0}")]
    Type(String),
    #[error("undefined function: {0}")]
    UndefinedFunction(String),
    #[error("arity error: function '{name}' expects {expected} argument(s), got {got}")]
    Arity {
        name: String,
        expected: usize,
        got: usize,
    },
    #[error("division by zero")]
    DivisionByZero,
    #[error("index out of bounds: {0}")]
    IndexOutOfBounds(i64),
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
}
