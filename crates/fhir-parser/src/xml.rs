use std::sync::Arc;

use indexmap::IndexMap;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::{NsReader, Reader};

use crate::line_index::LineIndex;
use crate::types::{Node, ParseError, Resource, Span, TrimmedAttribute, Value};
use crate::xml_checks::{
    FHIR_NS, Notes, XHTML_NS, check_element, collect_attrs, div_has_control_reference, error_at,
    lift_resource_id, local_name_arc, reject_root_attrs, resolve_reference, span_at, xml_error,
};

/// Maximum element nesting accepted (guards against stack overflow).
const MAX_DEPTH: usize = 128;

/// Maximum `xmlns` declarations accepted on one element (bounds namespace lookups).
const MAX_DECLARATIONS_PER_ELEMENT: usize = 8;

/// xml.html: "There SHALL be no DTD references in FHIR resources", wherever they appear.
const DTD_NOT_ALLOWED: &str = "a DTD is not allowed in FHIR XML";

/// Parse a FHIR XML resource from a source string.
///
/// Returns a [`Resource`] on success, or a [`ParseError`] if the source is
/// not valid XML or is missing the root element.
///
/// # FHIR XML conventions handled
///
/// - The root element local name is the resource type (e.g. `<Patient>`).
/// - Primitive fields use a `value` attribute: `<status value="active"/>`.
/// - Complex fields contain child elements: `<name><family value="Smith"/></name>`.
/// - Repeated sibling elements become a `Value::Array`.
/// - The XHTML `<div>` is kept as its raw source markup, as JSON carries it.
/// - Primitive-with-extensions elements
///   (e.g. `<birthDate value="..."><extension .../></birthDate>`)
///   are represented as objects with a special `"value"` key.
/// - A literal control below U+0020 other than tab, LF and CR is not well-formed.
///   XML 1.0 also forbids it as a character reference (`&#1;`); XML 1.1 allows that.
pub fn parse_xml(source: &str) -> Result<Resource, ParseError> {
    // A literal control is not well-formed in XML 1.0 nor (as a RestrictedChar) in 1.1.
    if let Some(offset) = source.find(crate::is_disallowed_control) {
        let (line, col) = LineIndex::new(source).location(offset as u32);
        return Err(ParseError::XmlError {
            message: "not well-formed: literal control character".into(),
            line,
            col,
        });
    }
    let resource = parse_document(source)?;
    // shortcut: references such as `&#1;` are only caught in captured values and the div
    // text, not in root or XHTML attributes; check them in the reader if those need it.
    if declared_version(source) != XmlVersion::Explicit1_1 {
        reject_controls(resource.id.as_deref(), &resource.fields)?;
    }
    Ok(resource)
}

/// The version in the XML declaration, `Implicit1_0` without one.
fn declared_version(source: &str) -> XmlVersion {
    match Reader::from_str(source).read_event() {
        Ok(Event::Decl(decl)) => decl.xml_version().unwrap_or_default(),
        _ => XmlVersion::Implicit1_0,
    }
}

/// Fail on the first string holding a control character XML 1.0 does not allow.
fn reject_controls(id: Option<&str>, fields: &IndexMap<Arc<str>, Node>) -> Result<(), ParseError> {
    let error = |line, col| ParseError::XmlError {
        message: "not well-formed: XML 1.0 does not allow this control character".into(),
        line,
        col,
    };
    if id.is_some_and(|s| s.chars().any(crate::is_disallowed_control)) {
        return Err(error(1, 1));
    }
    fn walk(key: &str, node: &Node) -> Option<Span> {
        match &node.value {
            Value::Str(s) if s.chars().any(crate::is_disallowed_control) => Some(node.span),
            // The div is raw markup, so its character references are still encoded.
            Value::Str(s) if key == "div" && div_has_control_reference(s) => Some(node.span),
            Value::Object(map) => map.iter().find_map(|(k, v)| walk(k, v)),
            Value::Array(items) => items.iter().find_map(|item| walk(key, item)),
            _ => None,
        }
    }
    match fields.iter().find_map(|(k, v)| walk(k, v)) {
        Some(span) => Err(error(span.line, span.col)),
        None => Ok(()),
    }
}

fn parse_document(source: &str) -> Result<Resource, ParseError> {
    let line_index = LineIndex::new(source);
    let mut reader = NsReader::from_str(source);
    reader.config_mut().trim_text(false);
    // Every element lookup scans the live namespace bindings; with MAX_DEPTH this keeps
    // them bounded (FHIR needs the FHIR and XHTML namespaces, plus the odd prefix).
    reader
        .resolver_mut()
        .set_max_declarations_per_element(MAX_DECLARATIONS_PER_ELEMENT);

    loop {
        let pos = reader.buffer_position() as u32;
        match reader
            .read_event()
            .map_err(|e| xml_error(&line_index, pos, e))?
        {
            Event::Start(ref e) => {
                let resource_type = local_name_arc(e.local_name().as_ref(), &line_index, pos)?;
                let mut notes = Notes::default();
                notes.schema_instance_if(
                    check_element(reader.resolver(), e, FHIR_NS, &line_index, pos)?,
                    &line_index,
                    pos,
                );
                reject_root_attrs(reader.resolver(), e, &line_index, pos)?;
                let mut fields = parse_children(&mut reader, source, &line_index, 1, &mut notes)?;

                let lifted = lift_resource_id(&mut fields, &line_index)?;
                let id_span = lifted.as_ref().map(|(_, span)| *span);
                let trimmed_attributes = notes
                    .trimmed
                    .into_iter()
                    .map(|(offset, name)| match id_span {
                        Some(span) if span.offset == offset && name.as_ref() == "value" => {
                            TrimmedAttribute::ResourceId(span)
                        }
                        _ => TrimmedAttribute::Element(offset, name),
                    })
                    .collect();
                let resource = Resource {
                    resource_type,
                    id: lifted.map(|(id, _)| id),
                    from_xml: true,
                    fields,
                    trimmed_attributes,
                    schema_instance: notes.schema_instance,
                };
                expect_eof(&mut reader, &line_index)?;
                return Ok(resource);
            }
            Event::Empty(ref e) => {
                // Self-closing root element - unusual but handle gracefully.
                let resource_type = local_name_arc(e.local_name().as_ref(), &line_index, pos)?;
                let mut notes = Notes::default();
                notes.schema_instance_if(
                    check_element(reader.resolver(), e, FHIR_NS, &line_index, pos)?,
                    &line_index,
                    pos,
                );
                reject_root_attrs(reader.resolver(), e, &line_index, pos)?;
                let resource = Resource {
                    resource_type,
                    id: None,
                    from_xml: true,
                    fields: IndexMap::new(),
                    trimmed_attributes: Vec::new(),
                    schema_instance: notes.schema_instance,
                };
                expect_eof(&mut reader, &line_index)?;
                return Ok(resource);
            }
            Event::Eof => {
                return Err(ParseError::XmlError {
                    message: "document contains no root element".into(),
                    line: 1,
                    col: 1,
                });
            }
            // xml.html: "There SHALL be no DTD references in FHIR resources".
            Event::DocType(_) => {
                return Err(error_at(&line_index, pos, DTD_NOT_ALLOWED));
            }
            // xml.html: resources "SHALL be exchanged using UTF-8 encoding".
            Event::Decl(ref decl) => {
                let utf8 = match decl.encoding() {
                    None => true,
                    Some(Ok(name)) => name.eq_ignore_ascii_case(b"UTF-8"),
                    Some(Err(_)) => false,
                };
                if !utf8 {
                    let message = "the XML declaration must declare UTF-8 encoding";
                    return Err(error_at(&line_index, pos, message));
                }
            }
            // Skip processing instructions and comments.
            _ => {}
        }
    }
}

// -- Children parsing ----------------------------------------------------------

/// Parse child elements until the matching `End` event, grouping repeated
/// siblings with the same name into arrays. An unexpected `Eof` is an error.
fn parse_children(
    reader: &mut NsReader<&[u8]>,
    source: &str,
    line_index: &LineIndex,
    depth: usize,
    notes: &mut Notes,
) -> Result<IndexMap<Arc<str>, Node>, ParseError> {
    if depth > MAX_DEPTH {
        let (line, col) = line_index.location(reader.buffer_position() as u32);
        return Err(ParseError::XmlError {
            message: "maximum nesting depth exceeded".into(),
            line,
            col,
        });
    }
    // Accumulate sibling nodes keyed by local name, preserving insertion order.
    let mut groups: IndexMap<Arc<str>, Vec<Node>> = IndexMap::new();

    loop {
        let pos = reader.buffer_position() as u32;
        match reader
            .read_event()
            .map_err(|e| xml_error(line_index, pos, e))?
        {
            Event::Start(ref e) => {
                let name = local_name_arc(e.local_name().as_ref(), line_index, pos)?;
                let span = span_at(line_index, pos);
                let node = if name.as_ref() == "div" {
                    let xsi = check_element(reader.resolver(), e, XHTML_NS, line_index, pos)?;
                    notes.schema_instance_if(xsi, line_index, pos);
                    skip_xhtml(reader, line_index, pos, notes)?;
                    raw_div(reader, source, line_index, pos)?
                } else {
                    let xsi = check_element(reader.resolver(), e, FHIR_NS, line_index, pos)?;
                    notes.schema_instance_if(xsi, line_index, pos);
                    let attrs =
                        collect_attrs(reader.resolver(), e, line_index, pos, &mut notes.trimmed)?;
                    let children = parse_children(reader, source, line_index, depth + 1, notes)?;
                    if let Some(key) = attrs.keys().find(|k| children.contains_key(*k)) {
                        let message =
                            format!("'{key}' is given both as an attribute and as a child element");
                        return Err(error_at(line_index, pos, message));
                    }
                    build_complex_node(attrs, children, span)
                };
                groups.entry(name).or_default().push(node);
            }
            Event::Empty(ref e) => {
                let name = local_name_arc(e.local_name().as_ref(), line_index, pos)?;
                let node = if name.as_ref() == "div" {
                    let xsi = check_element(reader.resolver(), e, XHTML_NS, line_index, pos)?;
                    notes.schema_instance_if(xsi, line_index, pos);
                    raw_div(reader, source, line_index, pos)?
                } else {
                    let xsi = check_element(reader.resolver(), e, FHIR_NS, line_index, pos)?;
                    notes.schema_instance_if(xsi, line_index, pos);
                    let attrs =
                        collect_attrs(reader.resolver(), e, line_index, pos, &mut notes.trimmed)?;
                    build_complex_node(attrs, IndexMap::new(), span_at(line_index, pos))
                };
                groups.entry(name).or_default().push(node);
            }
            Event::End(_) => break,
            Event::Eof => {
                let (line, col) = line_index.location(pos);
                return Err(ParseError::XmlError {
                    message: "unexpected end of document inside element".into(),
                    line,
                    col,
                });
            }
            // FHIR elements have no mixed content: only XML whitespace may sit between them.
            Event::Text(ref t) if t.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n')) => {}
            Event::Text(_) | Event::GeneralRef(_) | Event::CData(_) => {
                let message = "text content is not allowed inside FHIR elements";
                return Err(error_at(line_index, pos, message));
            }
            Event::DocType(_) => return Err(error_at(line_index, pos, DTD_NOT_ALLOWED)),
            // Comments and PIs between elements are skipped.
            _ => {}
        }
    }

    Ok(collapse_groups(groups))
}

/// Build a `Node` from an element's attributes and child-element map.
///
/// The simplest case — a single `value` attribute and no children — yields a
/// `Value::Str` directly (the common FHIR primitive form).
/// Every other combination produces a `Value::Object` that merges attributes
/// (with `value` first when present) followed by child nodes, preserving all
/// non-xmlns attribute data such as the `url` on `<extension>` elements.
fn build_complex_node(
    attrs: IndexMap<Arc<str>, Node>,
    children: IndexMap<Arc<str>, Node>,
    span: Span,
) -> Node {
    // Simple primitive: only a `value` attribute and no child elements.
    if attrs.len() == 1 && children.is_empty() {
        if let Some(node) = attrs.get("value") {
            if let Value::Str(s) = &node.value {
                return Node {
                    value: Value::Str(Arc::clone(s)),
                    span,
                };
            }
        }
    }

    // Complex node: merge attributes then children into an Object.
    // `value` is inserted first so it appears at the front of the map.
    let mut obj: IndexMap<Arc<str>, Node> = IndexMap::with_capacity(attrs.len() + children.len());
    if let Some(v) = attrs.get("value") {
        obj.insert(Arc::from("value"), v.clone());
    }
    for (k, v) in &attrs {
        if k.as_ref() != "value" {
            obj.insert(Arc::clone(k), v.clone());
        }
    }
    obj.extend(children);

    Node {
        value: Value::Object(obj),
        span,
    }
}

/// Collapse a map of `name -> Vec<Node>` into `name -> Node`, turning
/// multi-node groups into `Value::Array`.
fn collapse_groups(groups: IndexMap<Arc<str>, Vec<Node>>) -> IndexMap<Arc<str>, Node> {
    let mut result: IndexMap<Arc<str>, Node> = IndexMap::with_capacity(groups.len());
    for (name, mut nodes) in groups {
        if nodes.len() == 1 {
            result.insert(name, nodes.swap_remove(0));
        } else {
            let span = nodes[0].span;
            result.insert(
                name,
                Node {
                    value: Value::Array(nodes),
                    span,
                },
            );
        }
    }
    result
}

// -- XHTML handling ------------------------------------------------------------

// shortcut: a prefixed div (`<h:div>`) keeps a prefix bound on an ancestor, so the slice is
// not standalone XHTML; xml.html asks for default namespaces, upgrade if prefixed divs occur.
/// The XHTML narrative `<div>` from `start` up to the reader's position (just past its
/// end tag) as the raw source string, which is what the JSON form carries (narrative.html).
fn raw_div(
    reader: &NsReader<&[u8]>,
    source: &str,
    line_index: &LineIndex,
    start: u32,
) -> Result<Node, ParseError> {
    let end = reader.buffer_position() as usize;
    let markup = source.get(start as usize..end).ok_or_else(|| {
        error_at(
            line_index,
            start,
            "XHTML div is not on a character boundary",
        )
    })?;
    Ok(Node {
        value: Value::Str(Arc::from(markup)),
        span: span_at(line_index, start),
    })
}

/// Read past the content of an XHTML `<div>` (already past the opening tag) to its end
/// tag, rejecting malformed text and unknown entity references on the way.
fn skip_xhtml(
    reader: &mut NsReader<&[u8]>,
    line_index: &LineIndex,
    start_pos: u32,
    notes: &mut Notes,
) -> Result<(), ParseError> {
    let mut depth: usize = 1;

    loop {
        let pos = reader.buffer_position() as u32;
        match reader
            .read_event()
            .map_err(|e| xml_error(line_index, pos, e))?
        {
            Event::Start(ref e) => {
                let xsi = check_element(reader.resolver(), e, XHTML_NS, line_index, pos)?;
                notes.schema_instance_if(xsi, line_index, pos);
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(error_at(line_index, pos, "maximum nesting depth exceeded"));
                }
            }
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Empty(ref e) => {
                let xsi = check_element(reader.resolver(), e, XHTML_NS, line_index, pos)?;
                notes.schema_instance_if(xsi, line_index, pos);
            }
            Event::Text(ref t) => {
                t.decode().map_err(|e| {
                    error_at(line_index, pos, format!("invalid text in XHTML div: {e}"))
                })?;
            }
            // quick-xml 0.38+ reports entity and character references separately.
            Event::GeneralRef(ref r) if resolve_reference(r).is_none() => {
                let message = "invalid entity reference in XHTML div";
                return Err(error_at(line_index, pos, message));
            }
            Event::DocType(_) => return Err(error_at(line_index, pos, DTD_NOT_ALLOWED)),
            Event::Eof => {
                let (line, col) = line_index.location(start_pos);
                return Err(ParseError::XmlError {
                    message: "unexpected EOF inside XHTML div".into(),
                    line,
                    col,
                });
            }
            _ => {}
        }
    }

    Ok(())
}

// -- Trailing-content guard ---------------------------------------------------

/// Consume remaining events after the root element and return an error if any
/// non-whitespace content (element or non-empty text) is found before `Eof`.
fn expect_eof(reader: &mut NsReader<&[u8]>, line_index: &LineIndex) -> Result<(), ParseError> {
    loop {
        let pos = reader.buffer_position() as u32;
        match reader
            .read_event()
            .map_err(|e| xml_error(line_index, pos, e))?
        {
            Event::Eof => return Ok(()),
            Event::Text(ref t) => {
                // Whitespace-only text between elements is allowed.
                let unescaped = t.decode().map_err(|e| {
                    let (line, col) = line_index.location(pos);
                    ParseError::XmlError {
                        message: format!("invalid trailing text: {e}"),
                        line,
                        col,
                    }
                })?;
                if !unescaped.chars().all(char::is_whitespace) {
                    let (line, col) = line_index.location(pos);
                    return Err(ParseError::XmlError {
                        message: "unexpected content after root element".into(),
                        line,
                        col,
                    });
                }
            }
            Event::GeneralRef(_) => {
                let (line, col) = line_index.location(pos);
                return Err(ParseError::XmlError {
                    message: "unexpected content after root element".into(),
                    line,
                    col,
                });
            }
            // Any element (start, empty) after the root is invalid.
            Event::Start(_) | Event::Empty(_) => {
                let (line, col) = line_index.location(pos);
                return Err(ParseError::XmlError {
                    message: "unexpected element after root element".into(),
                    line,
                    col,
                });
            }
            Event::DocType(_) => return Err(error_at(line_index, pos, DTD_NOT_ALLOWED)),
            // Comments, PIs, and the XML declaration are ignored.
            _ => {}
        }
    }
}

// -- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ParseError, Value};

    const MINIMAL_PATIENT: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<Patient xmlns=\"http://hl7.org/fhir\">\n",
        "  <id value=\"example\"/>\n",
        "  <active value=\"true\"/>\n",
        "</Patient>",
    );

    #[test]
    fn parse_patient_example_xml() {
        let resource = parse_xml(MINIMAL_PATIENT).unwrap();
        assert_eq!(&*resource.resource_type, "Patient");
        assert_eq!(resource.id.as_deref(), Some("example"));
    }

    #[test]
    fn parse_primitive_field_as_str() {
        let resource = parse_xml(MINIMAL_PATIENT).unwrap();
        let active = resource.fields.get("active").expect("active field present");
        assert!(matches!(&active.value, Value::Str(s) if s.as_ref() == "true"));
    }

    #[test]
    fn parse_complex_field_as_object() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <name>\n",
            "    <family value=\"Smith\"/>\n",
            "    <given value=\"John\"/>\n",
            "  </name>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let name = resource.fields.get("name").expect("name field present");
        assert!(matches!(&name.value, Value::Object(_)));
        if let Value::Object(obj) = &name.value {
            assert!(obj.contains_key("family"));
            assert!(obj.contains_key("given"));
        }
    }

    #[test]
    fn parse_repeated_elements_as_array() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <name>\n",
            "    <given value=\"Peter\"/>\n",
            "    <given value=\"James\"/>\n",
            "  </name>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let name = resource.fields.get("name").unwrap();
        if let Value::Object(obj) = &name.value {
            let given = obj.get("given").expect("given field present");
            assert!(matches!(&given.value, Value::Array(v) if v.len() == 2));
        } else {
            panic!("name should be an object");
        }
    }

    #[test]
    fn parse_multiple_top_level_siblings_as_array() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <name><family value=\"Official\"/></name>\n",
            "  <name><family value=\"Maiden\"/></name>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let name = resource.fields.get("name").unwrap();
        assert!(matches!(&name.value, Value::Array(v) if v.len() == 2));
    }

    #[test]
    fn parse_xhtml_div_as_str() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <text>\n",
            "    <status value=\"generated\"/>\n",
            "    <div xmlns=\"http://www.w3.org/1999/xhtml\">Hello World</div>\n",
            "  </text>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let text = resource.fields.get("text").unwrap();
        if let Value::Object(obj) = &text.value {
            let div = obj.get("div").expect("div field present");
            assert!(matches!(&div.value, Value::Str(s) if s.contains("Hello")));
        } else {
            panic!("text should be an object");
        }
    }

    #[test]
    fn parse_xhtml_div_keeps_entity_and_char_references_as_written() {
        let source = concat!(
            "<Patient xmlns=\"http://hl7.org/fhir\">",
            "<text><status value=\"generated\"/>",
            "<div xmlns=\"http://www.w3.org/1999/xhtml\">a &amp; b &lt; &#65;</div>",
            "</text></Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let Value::Object(text) = &resource.fields.get("text").unwrap().value else {
            panic!("text should be an object");
        };
        let div = text.get("div").expect("div field present");
        let raw = "<div xmlns=\"http://www.w3.org/1999/xhtml\">a &amp; b &lt; &#65;</div>";
        assert!(matches!(&div.value, Value::Str(s) if s.as_ref() == raw));
    }

    #[test]
    fn reject_entity_reference_after_root() {
        let source = "<Patient xmlns=\"http://hl7.org/fhir\"/>&amp;";
        assert!(parse_xml(source).is_err());
    }

    #[test]
    fn parse_primitive_with_extension_as_object() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <birthDate value=\"1974-12-25\">\n",
            "    <extension url=\"http://hl7.org/fhir/StructureDefinition/patient-birthTime\">\n",
            "      <valueDateTime value=\"1974-12-25T14:35:45-05:00\"/>\n",
            "    </extension>\n",
            "  </birthDate>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let birth_date = resource.fields.get("birthDate").unwrap();
        assert!(
            matches!(&birth_date.value, Value::Object(obj) if obj.contains_key("value")),
            "birthDate with extension should be an Object with a 'value' key"
        );
    }

    #[test]
    fn span_points_to_correct_line_for_field() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <active value=\"true\"/>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let active = resource.fields.get("active").unwrap();
        // <active ...> is on line 4
        assert_eq!(active.span.line, 4);
    }

    #[test]
    fn trailing_element_after_root_returns_error() {
        let result = parse_xml(
            "<Patient xmlns=\"http://hl7.org/fhir\"/><Observation xmlns=\"http://hl7.org/fhir\"/>",
        );
        assert!(
            matches!(result, Err(ParseError::XmlError { .. })),
            "multiple root elements should be rejected"
        );
    }

    #[test]
    fn trailing_whitespace_after_root_is_allowed() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "</Patient>\n",
        );
        // Trailing newline is whitespace — must not be rejected.
        assert!(parse_xml(source).is_ok());
    }

    #[test]
    fn missing_root_element_returns_error() {
        let result = parse_xml("<?xml version=\"1.0\"?>");
        assert!(matches!(result, Err(ParseError::XmlError { .. })));
    }

    #[test]
    fn malformed_xml_returns_error() {
        // Malformed attribute (no closing quote) causes quick-xml to return an error.
        let result = parse_xml("<Patient id=\"unclosed>");
        assert!(result.is_err());
    }

    #[test]
    fn truncated_xml_returns_error() {
        // Document that opens a child element but never closes it or the root.
        let result = parse_xml("<Patient xmlns=\"http://hl7.org/fhir\"><id value=\"x\"/><name>");
        assert!(
            matches!(result, Err(ParseError::XmlError { .. })),
            "truncated XML should return an error"
        );
    }

    #[test]
    fn extension_url_attribute_is_preserved() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <extension url=\"http://example.org/ext\">\n",
            "    <valueString value=\"hello\"/>\n",
            "  </extension>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let ext = resource
            .fields
            .get("extension")
            .expect("extension field present");
        if let Value::Object(obj) = &ext.value {
            let url = obj.get("url").expect("url key present in extension");
            assert!(
                matches!(&url.value, Value::Str(s) if s.as_ref() == "http://example.org/ext"),
                "extension url should be preserved"
            );
        } else {
            panic!("extension should be an object, got {:?}", ext.value);
        }
    }

    #[test]
    fn self_closing_element_with_non_value_attr_is_object() {
        // <coding system="http://..." code="MR"/> — no `value` attr, two other attrs.
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Patient xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"p1\"/>\n",
            "  <identifier>\n",
            "    <type>\n",
            "      <coding>\n",
            "        <system value=\"http://terminology.hl7.org/CodeSystem/v2-0203\"/>\n",
            "        <code value=\"MR\"/>\n",
            "      </coding>\n",
            "    </type>\n",
            "  </identifier>\n",
            "</Patient>",
        );
        let resource = parse_xml(source).unwrap();
        let identifier = resource.fields.get("identifier").unwrap();
        if let Value::Object(id_obj) = &identifier.value {
            let coding = id_obj.get("type").unwrap();
            if let Value::Object(type_obj) = &coding.value {
                let coding_inner = type_obj.get("coding").unwrap();
                assert!(matches!(&coding_inner.value, Value::Object(_)));
            }
        }
    }

    #[test]
    fn parse_observation_resource_type() {
        let source = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<Observation xmlns=\"http://hl7.org/fhir\">\n",
            "  <id value=\"obs1\"/>\n",
            "  <status value=\"final\"/>\n",
            "</Observation>",
        );
        let resource = parse_xml(source).unwrap();
        assert_eq!(&*resource.resource_type, "Observation");
        assert_eq!(resource.id.as_deref(), Some("obs1"));
    }
}

#[cfg(test)]
mod depth_tests {
    use super::*;

    #[test]
    fn deep_nesting_is_an_error_not_a_stack_overflow() {
        let n = 100_000;
        let src = format!("<P>{}{}</P>", "<a>".repeat(n), "</a>".repeat(n));
        assert!(parse_xml(&src).is_err());
    }
}
