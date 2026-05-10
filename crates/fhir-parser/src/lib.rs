#![forbid(unsafe_code)]
//! FHIR resource parser.
//!
//! Converts JSON or XML input into a typed `Resource` AST, preserving source
//! spans for precise diagnostic messages.

mod json;
mod line_index;
mod types;
mod xml;

pub use json::parse_json;
pub use types::{Node, ParseError, Resource, Span, Value};
pub use xml::parse_xml;
