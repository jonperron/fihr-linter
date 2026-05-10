use std::sync::Arc;

use indexmap::IndexMap;
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::line_index::LineIndex;
use crate::types::{Node, ParseError, Resource, Span, Value};

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
/// - XHTML `<div>` content is captured as plain text.
/// - Primitive-with-extensions elements (e.g. `<birthDate value="..."><extension .../></birthDate>`)
///   are represented as objects with a special `"value"` key.
pub fn parse_xml(source: &str) -> Result<Resource, ParseError> {
    let line_index = LineIndex::new(source);
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(false);

    loop {
        let pos = reader.buffer_position() as u32;
        match reader
            .read_event()
            .map_err(|e| xml_error(&line_index, pos, e))?
        {
            Event::Start(ref e) => {
                let resource_type = local_name_arc(e.local_name().as_ref(), &line_index, pos)?;
                let mut fields = parse_children(&mut reader, &line_index)?;

                let id = match fields.shift_remove("id") {
                    None => None,
                    Some(n) => match n.value {
                        Value::Str(s) => Some(s),
                        _ => {
                            let (line, col) = line_index.location(n.span.offset);
                            return Err(ParseError::XmlError {
                                message: "'id' element must be a primitive string value".into(),
                                line,
                                col,
                            });
                        }
                    },
                };

                return Ok(Resource {
                    resource_type,
                    id,
                    fields,
                });
            }
            Event::Empty(ref e) => {
                // Self-closing root element - unusual but handle gracefully.
                let resource_type = local_name_arc(e.local_name().as_ref(), &line_index, pos)?;
                return Ok(Resource {
                    resource_type,
                    id: None,
                    fields: IndexMap::new(),
                });
            }
            Event::Eof => {
                return Err(ParseError::XmlError {
                    message: "document contains no root element".into(),
                    line: 1,
                    col: 1,
                });
            }
            // Skip XML declaration, processing instructions, comments.
            _ => {}
        }
    }
}

// -- Children parsing ----------------------------------------------------------

/// Parse child elements until the matching `End` event, grouping repeated
/// siblings with the same name into arrays. An unexpected `Eof` is an error.
fn parse_children(
    reader: &mut Reader<&[u8]>,
    line_index: &LineIndex,
) -> Result<IndexMap<Arc<str>, Node>, ParseError> {
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
                    // XHTML narrative: collect text content and skip child elements.
                    let text = read_xhtml_text(reader, line_index, pos)?;
                    Node {
                        value: Value::Str(Arc::from(text.as_str())),
                        span,
                    }
                } else {
                    let attr_val = value_attr(e, line_index, pos)?;
                    let children = parse_children(reader, line_index)?;
                    build_complex_node(attr_val, children, span)
                };
                groups.entry(name).or_default().push(node);
            }
            Event::Empty(ref e) => {
                let name = local_name_arc(e.local_name().as_ref(), line_index, pos)?;
                let span = span_at(line_index, pos);
                let attr_val = value_attr(e, line_index, pos)?;
                let node = Node {
                    value: attr_val
                        .map(Value::Str)
                        .unwrap_or(Value::Object(IndexMap::new())),
                    span,
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
            // Text nodes, comments, PIs between elements are skipped.
            _ => {}
        }
    }

    collapse_groups(groups)
}

/// A complex element can carry both a `value` attribute (primitive value) and
/// child elements (typically extensions). Represent it as an `Object` with a
/// `"value"` key prepended.
fn build_complex_node(
    attr_val: Option<Arc<str>>,
    children: IndexMap<Arc<str>, Node>,
    span: Span,
) -> Node {
    if let Some(val) = attr_val {
        if children.is_empty() {
            Node {
                value: Value::Str(val),
                span,
            }
        } else {
            let mut obj: IndexMap<Arc<str>, Node> = IndexMap::new();
            obj.insert(
                Arc::from("value"),
                Node {
                    value: Value::Str(val),
                    span,
                },
            );
            obj.extend(children);
            Node {
                value: Value::Object(obj),
                span,
            }
        }
    } else {
        Node {
            value: Value::Object(children),
            span,
        }
    }
}

/// Collapse a map of `name -> Vec<Node>` into `name -> Node`, turning
/// multi-node groups into `Value::Array`.
fn collapse_groups(
    groups: IndexMap<Arc<str>, Vec<Node>>,
) -> Result<IndexMap<Arc<str>, Node>, ParseError> {
    let mut result: IndexMap<Arc<str>, Node> = IndexMap::with_capacity(groups.len());
    for (name, mut nodes) in groups {
        if nodes.len() == 1 {
            result.insert(name, nodes.remove(0));
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
    Ok(result)
}

// -- XHTML handling ------------------------------------------------------------

/// Read all content inside an XHTML `<div>` (already past the opening tag),
/// returning the concatenated text content while skipping markup.
fn read_xhtml_text(
    reader: &mut Reader<&[u8]>,
    line_index: &LineIndex,
    start_pos: u32,
) -> Result<String, ParseError> {
    let mut text = String::new();
    let mut depth: usize = 1;

    loop {
        let pos = reader.buffer_position() as u32;
        match reader
            .read_event()
            .map_err(|e| xml_error(line_index, pos, e))?
        {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Empty(_) => {}
            Event::Text(ref t) => {
                let s = t.unescape().map_err(|e| {
                    let (line, col) = line_index.location(pos);
                    ParseError::XmlError {
                        message: format!("invalid text in XHTML div: {e}"),
                        line,
                        col,
                    }
                })?;
                text.push_str(&s);
            }
            Event::CData(ref c) => {
                // CDATA content is already literal text - no escaping needed.
                let s = std::str::from_utf8(c.as_ref()).map_err(|_| {
                    let (line, col) = line_index.location(pos);
                    ParseError::XmlError {
                        message: "invalid UTF-8 in CDATA section".into(),
                        line,
                        col,
                    }
                })?;
                text.push_str(s);
            }
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

    Ok(text)
}

// -- Helpers -------------------------------------------------------------------

/// Convert a local name byte slice to `Arc<str>`, returning an error when the
/// bytes are not valid UTF-8 (which should not happen for well-formed XML but
/// is propagated rather than silently substituted).
fn local_name_arc(bytes: &[u8], line_index: &LineIndex, pos: u32) -> Result<Arc<str>, ParseError> {
    std::str::from_utf8(bytes).map(Arc::from).map_err(|_| {
        let (line, col) = line_index.location(pos);
        ParseError::XmlError {
            message: "element name is not valid UTF-8".into(),
            line,
            col,
        }
    })
}

/// Extract the `value` attribute from a FHIR element, returning `None` when
/// the attribute is absent.
fn value_attr<'a>(
    e: &quick_xml::events::BytesStart<'a>,
    line_index: &LineIndex,
    pos: u32,
) -> Result<Option<Arc<str>>, ParseError> {
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(|e| {
            let (line, col) = line_index.location(pos);
            ParseError::XmlError {
                message: format!("malformed attribute: {e}"),
                line,
                col,
            }
        })?;
        if attr.key.local_name().as_ref() == b"value" {
            let val = attr.unescape_value().map_err(|e| {
                let (line, col) = line_index.location(pos);
                ParseError::XmlError {
                    message: format!("invalid attribute value: {e}"),
                    line,
                    col,
                }
            })?;
            return Ok(Some(Arc::from(val.as_ref())));
        }
    }
    Ok(None)
}

fn span_at(line_index: &LineIndex, pos: u32) -> Span {
    let (line, col) = line_index.location(pos);
    Span {
        line,
        col,
        offset: pos,
    }
}

fn xml_error(line_index: &LineIndex, pos: u32, e: quick_xml::Error) -> ParseError {
    let (line, col) = line_index.location(pos);
    ParseError::XmlError {
        message: e.to_string(),
        line,
        col,
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
