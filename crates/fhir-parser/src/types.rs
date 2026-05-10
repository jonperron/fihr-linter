use std::sync::Arc;

use indexmap::IndexMap;

/// Byte offset and 1-indexed line/column of a source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub col: u32,
    pub offset: u32,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            line: 1,
            col: 1,
            offset: 0,
        }
    }
}

/// A FHIR JSON value variant.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Decimal(f64),
    Str(Arc<str>),
    Array(Vec<Node>),
    Object(IndexMap<Arc<str>, Node>),
}

/// A FHIR value together with its source location.
#[derive(Debug, Clone)]
pub struct Node {
    pub value: Value,
    pub span: Span,
}

/// A parsed FHIR resource.
#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: Arc<str>,
    pub id: Option<Arc<str>>,
    pub fields: IndexMap<Arc<str>, Node>,
}

/// Errors produced by the parser.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("missing required field 'resourceType'")]
    MissingResourceType,
    #[error("JSON parse error at line {line}, column {col}: {message}")]
    JsonError {
        message: String,
        line: u32,
        col: u32,
    },
    #[error("XML parse error at line {line}, column {col}: {message}")]
    XmlError {
        message: String,
        line: u32,
        col: u32,
    },
}
