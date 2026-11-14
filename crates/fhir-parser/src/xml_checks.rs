//! Attribute, namespace, error and root id helpers for the XML parser.
use std::sync::Arc;

use indexmap::IndexMap;
use quick_xml::XmlVersion;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, BytesStart};
use quick_xml::name::{NamespaceResolver, ResolveResult};

use crate::line_index::LineIndex;
use crate::types::{Node, ParseError, Span, Value};

/// xml.html: "FHIR elements are always in the namespace http://hl7.org/fhir".
pub(crate) const FHIR_NS: &[u8] = b"http://hl7.org/fhir";
/// xml.html: the narrative div is "in the xhtml namespace".
pub(crate) const XHTML_NS: &[u8] = b"http://www.w3.org/1999/xhtml";
/// xml.html: content SHALL NOT "even contain the schema instance namespace".
pub(crate) const XSI_NS: &[u8] = b"http://www.w3.org/2001/XMLSchema-instance";

/// Resolve a character reference or one of the five predefined entities.
pub(crate) fn resolve_reference(reference: &BytesRef<'_>) -> Option<String> {
    if let Ok(Some(c)) = reference.resolve_char_ref() {
        return Some(c.to_string());
    }
    let name = reference.decode().ok()?;
    resolve_predefined_entity(&name).map(str::to_owned)
}

/// Convert a local name byte slice to `Arc<str>`, returning an error when the
/// bytes are not valid UTF-8 (which should not happen for well-formed XML but
/// is propagated rather than silently substituted).
pub(crate) fn local_name_arc(
    bytes: &[u8],
    line_index: &LineIndex,
    pos: u32,
) -> Result<Arc<str>, ParseError> {
    std::str::from_utf8(bytes).map(Arc::from).map_err(|_| {
        let (line, col) = line_index.location(pos);
        ParseError::XmlError {
            message: "element name is not valid UTF-8".into(),
            line,
            col,
        }
    })
}

/// Collect all non-xmlns attributes of an element into an ordered map of
/// `local_name -> Node`. Namespace declarations (`xmlns`, `xmlns:prefix`) are
/// skipped; all data-carrying attributes (including `value` and `url`) are
/// captured as `Value::Str` nodes at the element's span. Values are trimmed of leading
/// and trailing XML whitespace (xml.html: readers SHOULD trim), and each trimmed
/// attribute is recorded in `trimmed` so the validator can warn about it.
pub(crate) fn collect_attrs(
    resolver: &NamespaceResolver,
    e: &BytesStart<'_>,
    line_index: &LineIndex,
    pos: u32,
    trimmed: &mut Vec<(u32, Arc<str>)>,
) -> Result<IndexMap<Arc<str>, Node>, ParseError> {
    let span = span_at(line_index, pos);
    let mut attrs: IndexMap<Arc<str>, Node> = IndexMap::new();

    for attr_result in e.attributes() {
        let attr = attr_result.map_err(|e| {
            let (line, col) = line_index.location(pos);
            ParseError::XmlError {
                message: format!("malformed attribute: {e}"),
                line,
                col,
            }
        })?;

        // Skip namespace declarations.
        let raw_key = attr.key.as_ref();
        if raw_key == b"xmlns" || raw_key.starts_with(b"xmlns:") {
            continue;
        }

        // xml.html names no namespaced attribute on FHIR elements (`fhir:value` included).
        // Schema instance attributes are recorded by `check_element` and carry no data.
        match resolver.resolve_attribute(attr.key).0 {
            ResolveResult::Unbound => {}
            ResolveResult::Bound(ns) if ns.as_ref() == XSI_NS => continue,
            _ => {
                let key = String::from_utf8_lossy(raw_key);
                let message = format!("attribute '{key}' must not be in a namespace");
                return Err(error_at(line_index, pos, message));
            }
        }
        let local = local_name_arc(attr.key.local_name().as_ref(), line_index, pos)?;
        let val = attr
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|e| {
                let (line, col) = line_index.location(pos);
                ParseError::XmlError {
                    message: format!("invalid attribute value: {e}"),
                    line,
                    col,
                }
            })?;

        // A whitespace-only value is kept: trimming it would read as an empty value, while
        // the validator judges it like the same JSON string (a Warning for `string`).
        let trimmed_value = val.trim_matches([' ', '\t', '\r', '\n']);
        let value = if trimmed_value.is_empty() {
            val.as_ref()
        } else {
            trimmed_value
        };
        if value.len() != val.len() {
            trimmed.push((pos, Arc::clone(&local)));
        }
        attrs.insert(
            local,
            Node {
                value: Value::Str(Arc::from(value)),
                span,
            },
        );
    }

    Ok(attrs)
}

/// xml.html: `id` is an attribute on elements "but not on resources"; a resource root
/// carries namespace declarations only.
// shortcut: resources nested in `contained`, `Bundle.entry.resource` or `Parameters` are
// read as ordinary elements, so their root attributes are not rejected; upgrade with the
// embedded-resource unwrapping (11.6.4-P1).
pub(crate) fn reject_root_attrs(
    resolver: &NamespaceResolver,
    e: &BytesStart<'_>,
    line_index: &LineIndex,
    pos: u32,
) -> Result<(), ParseError> {
    let attrs = collect_attrs(resolver, e, line_index, pos, &mut Vec::new())?;
    if attrs.is_empty() {
        return Ok(());
    }
    let names: Vec<&str> = attrs.keys().map(AsRef::as_ref).collect();
    let message = format!(
        "attributes are not allowed on the resource root element: {}",
        names.join(", ")
    );
    Err(error_at(line_index, pos, message))
}

/// Reject an element outside the `expected` namespace or with a malformed attribute.
/// Returns whether the element declares or uses the schema instance namespace, which
/// xml.html 2.1.6.3.4 forbids; the validator reports it, so the document still parses.
pub(crate) fn check_element(
    resolver: &NamespaceResolver,
    e: &BytesStart<'_>,
    expected: &[u8],
    line_index: &LineIndex,
    pos: u32,
) -> Result<bool, ParseError> {
    let in_expected = matches!(
        resolver.resolve_element(e.name()).0,
        ResolveResult::Bound(ns) if ns.as_ref() == expected
    );
    if !in_expected {
        let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
        let expected = String::from_utf8_lossy(expected);
        let message = format!("element '{name}' must be in the namespace {expected}");
        return Err(error_at(line_index, pos, message));
    }
    // Every element passes here, XHTML ones included, so malformed attributes stop here too.
    let mut uses_xsi = false;
    for attr in e.attributes() {
        let attr =
            attr.map_err(|err| error_at(line_index, pos, format!("malformed attribute: {err}")))?;
        let key = attr.key.as_ref();
        // Compare the normalized value, so a character reference cannot hide the namespace.
        let declares_xsi = (key == b"xmlns" || key.starts_with(b"xmlns:"))
            && attr
                .normalized_value(XmlVersion::Implicit1_0)
                .is_ok_and(|v| v.as_bytes() == XSI_NS);
        let in_xsi = matches!(
            resolver.resolve_attribute(attr.key).0,
            ResolveResult::Bound(ns) if ns.as_ref() == XSI_NS
        );
        uses_xsi |= declares_xsi || in_xsi;
    }
    Ok(uses_xsi)
}

pub(crate) fn span_at(line_index: &LineIndex, pos: u32) -> Span {
    let (line, col) = line_index.location(pos);
    Span {
        line,
        col,
        offset: pos,
    }
}

pub(crate) fn error_at(line_index: &LineIndex, pos: u32, message: impl Into<String>) -> ParseError {
    let (line, col) = line_index.location(pos);
    ParseError::XmlError {
        message: message.into(),
        line,
        col,
    }
}

pub(crate) fn xml_error(line_index: &LineIndex, pos: u32, e: quick_xml::Error) -> ParseError {
    let (line, col) = line_index.location(pos);
    ParseError::XmlError {
        message: e.to_string(),
        line,
        col,
    }
}

/// What the reader records for the validator while parsing.
#[derive(Default)]
pub(crate) struct Notes {
    /// Trimmed attribute values, as (element offset, attribute name).
    pub(crate) trimmed: Vec<(u32, Arc<str>)>,
    /// Elements that declare or use the schema instance namespace.
    pub(crate) schema_instance: Vec<Span>,
}

impl Notes {
    pub(crate) fn schema_instance_if(&mut self, uses_xsi: bool, line_index: &LineIndex, pos: u32) {
        if uses_xsi {
            self.schema_instance.push(span_at(line_index, pos));
        }
    }
}

/// A character reference in the div markup that resolves to a disallowed control.
pub(crate) fn div_has_control_reference(div: &str) -> bool {
    let mut reader = quick_xml::Reader::from_str(div);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::GeneralRef(r)) => {
                if let Ok(Some(c)) = r.resolve_char_ref() {
                    if crate::is_disallowed_control(c) {
                        return true;
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => return false,
            Ok(_) => {}
        }
    }
}

/// Take the root `id` element's `value` attribute out of `fields` as the resource id.
///
/// With an extension or element id (`<id value=".."><extension/></id>`) the rest stays as
/// an `id` primitive object without `value`, which the validator reshapes into `_id` like
/// any other primitive. The attribute shares its element's offset; a `<value>` child
/// element does not, so it stays in the object for the validator to report. Returns the
/// id with the `<id>` element's span.
pub(crate) fn lift_resource_id(
    fields: &mut IndexMap<Arc<str>, Node>,
    line_index: &LineIndex,
) -> Result<Option<(Arc<str>, Span)>, ParseError> {
    let Some((index, key, node)) = fields.shift_remove_full("id") else {
        return Ok(None);
    };
    let mut parts = match node.value {
        Value::Str(id) => return Ok(Some((id, node.span))),
        Value::Object(parts) => parts,
        _ => IndexMap::new(),
    };
    let id = match parts.get("value") {
        Some(Node {
            value: Value::Str(id),
            span,
        }) if span.offset == node.span.offset => Some(Arc::clone(id)),
        _ => None,
    };
    if id.is_some() {
        parts.shift_remove("value");
    }
    // An empty `<id/>` or a repeated `<id>`: no single resource id to take.
    if parts.is_empty() && id.is_none() {
        let (line, col) = line_index.location(node.span.offset);
        return Err(ParseError::XmlError {
            message: "'id' element must be a primitive string value".into(),
            line,
            col,
        });
    }
    if !parts.is_empty() {
        let rest = Node {
            value: Value::Object(parts),
            span: node.span,
        };
        fields.shift_insert(index, key, rest);
    }
    Ok(id.map(|id| (id, node.span)))
}
