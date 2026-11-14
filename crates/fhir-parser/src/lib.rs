#![forbid(unsafe_code)]
//! FHIR resource parser.
//!
//! Converts JSON or XML input into a typed `Resource` AST, preserving source
//! spans for precise diagnostic messages.

mod json;
mod line_index;
mod types;
mod xml;
mod xml_checks;

pub use json::parse_json;
pub use types::{Node, ParseError, Resource, Span, TrimmedAttribute, Value};
pub use xml::parse_xml;

/// A control character below U+0020 other than tab, LF and CR: R5 strings SHOULD NOT
/// contain one (datatypes.html), and XML 1.0 does not allow one at all.
pub fn is_disallowed_control(c: char) -> bool {
    c < ' ' && !matches!(c, '\t' | '\n' | '\r')
}
