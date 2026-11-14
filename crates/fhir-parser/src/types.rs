use std::sync::Arc;

use indexmap::IndexMap;

/// A source location, counted in UTF-8 bytes of the input as given (a leading
/// BOM included), not in characters: after `"ééé"` the column is 3 more than the
/// character count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// 1-indexed line.
    pub line: u32,
    /// 1-indexed UTF-8 byte column within the line.
    pub col: u32,
    /// 0-indexed UTF-8 byte offset from the start of the input.
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
    /// True when parsed from XML, where every primitive is text and a primitive with
    /// an extension is an object holding `value`; the validator reshapes it to the JSON form.
    pub from_xml: bool,
    pub resource_type: Arc<str>,
    pub id: Option<Arc<str>>,
    pub fields: IndexMap<Arc<str>, Node>,
    /// XML attribute values the reader trimmed (xml.html: readers SHOULD trim leading and
    /// trailing whitespace). Empty for JSON.
    pub trimmed_attributes: Vec<TrimmedAttribute>,
    /// XML elements that declare or use the schema instance namespace, which xml.html
    /// 2.1.6.3.4 forbids; the validator reports each one. Empty for JSON.
    pub schema_instance: Vec<Span>,
}

/// An XML attribute value the reader trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrimmedAttribute {
    /// Attribute `name` of the element at this byte offset.
    Element(u32, Arc<str>),
    /// The `value` of the root `<id>` element, lifted into [`Resource::id`]: the span of
    /// that `<id>`, which no node in `fields` keeps.
    ResourceId(Span),
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
