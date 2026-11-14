//! XML conformance rules from the FHIR R5 `xml.html` page (audit 11.6.4).
use fhir_parser::{ParseError, TrimmedAttribute, Value, parse_xml};

const FHIR_NS: &str = "xmlns=\"http://hl7.org/fhir\"";

/// The `XmlError` message for `source`; panics when it parses.
fn xml_error(source: &str) -> String {
    match parse_xml(source) {
        Err(ParseError::XmlError { message, .. }) => message,
        other => panic!("expected an XmlError for {source:?}, got {other:?}"),
    }
}

#[test]
fn dtd_is_rejected() {
    let source = format!("<!DOCTYPE Patient><Patient {FHIR_NS}><active value=\"true\"/></Patient>");
    assert!(xml_error(&source).contains("DTD"));
}

#[test]
fn dtd_inside_or_after_the_root_is_rejected() {
    let cases = [
        format!("<Patient {FHIR_NS}><!DOCTYPE x><active value=\"true\"/></Patient>"),
        format!("<Patient {FHIR_NS}><active value=\"true\"/></Patient><!DOCTYPE x>"),
        format!(
            "<Patient {FHIR_NS}><text><status value=\"generated\"/>{}</text></Patient>",
            "<div xmlns=\"http://www.w3.org/1999/xhtml\"><!DOCTYPE x>x</div>"
        ),
    ];
    for source in cases {
        assert!(parse_xml(&source).is_err(), "{source}");
    }
}

#[test]
fn malformed_attributes_inside_the_div_are_rejected() {
    let div = "<div xmlns=\"http://www.w3.org/1999/xhtml\"><p class=\"a\" class=\"b\">x</p></div>";
    let source =
        format!("<Patient {FHIR_NS}><text><status value=\"generated\"/>{div}</text></Patient>");
    assert!(parse_xml(&source).is_err());
}

#[test]
fn non_utf8_encoding_declaration_is_rejected() {
    for encoding in ["UTF-16", "ISO-8859-1"] {
        let source = format!("<?xml version=\"1.0\" encoding=\"{encoding}\"?><Patient {FHIR_NS}/>");
        assert!(xml_error(&source).contains("UTF-8"), "{encoding}");
    }
}

#[test]
fn text_inside_fhir_elements_is_rejected() {
    let cases = [
        "<name>Smith<family value=\"Smith\"/></name>",
        "<name><family value=\"Smith\"/>&amp;</name>",
        "<name><![CDATA[Smith]]><family value=\"Smith\"/></name>",
        "<name><![CDATA[ ]]><family value=\"Smith\"/></name>",
    ];
    for body in cases {
        let source = format!("<Patient {FHIR_NS}>{body}</Patient>");
        assert!(xml_error(&source).contains("text"), "{body}");
    }
}

#[test]
fn whitespace_comments_and_processing_instructions_between_elements_are_ignored() {
    let source = format!(
        "<Patient {FHIR_NS}>\n  <!-- c -->\n  <?pi x?>\n  <name>\n\t{}\r\n  </name>\n</Patient>",
        "<family value=\"S\"/>"
    );
    assert!(parse_xml(&source).is_ok());
}

#[test]
fn root_attributes_other_than_namespace_declarations_are_rejected() {
    for attrs in ["id=\"p\"", "foo=\"bar\""] {
        let open = format!("<Patient {FHIR_NS} {attrs}");
        for source in [
            format!("{open}/>"),
            format!("{open}><active value=\"true\"/></Patient>"),
        ] {
            assert!(xml_error(&source).contains("root"), "{source}");
        }
    }
}

#[test]
fn attribute_and_child_element_with_the_same_name_are_rejected() {
    let cases = [
        "<gender id=\"g1\" value=\"male\"><id value=\"zz\"/></gender>",
        "<active value=\"true\"><value value=\"false\"/></active>",
    ];
    for body in cases {
        let source = format!("<Patient {FHIR_NS}>{body}</Patient>");
        assert!(xml_error(&source).contains("both"), "{body}");
    }
}

/// The `text.div` value of a Patient whose narrative div is `div`.
fn div_value(div: &str) -> Value {
    let source =
        format!("<Patient {FHIR_NS}><text><status value=\"generated\"/>{div}</text></Patient>");
    let resource = parse_xml(&source).expect("parses");
    let Value::Object(text) = &resource.fields["text"].value else {
        panic!("text should be an object");
    };
    text["div"].value.clone()
}

#[test]
fn xhtml_div_is_kept_as_its_raw_source() {
    let cases = [
        "<div xmlns=\"http://www.w3.org/1999/xhtml\"><img src=\"a.png\"/></div>",
        "<div xmlns=\"http://www.w3.org/1999/xhtml\"><p>a &amp; b</p><div>x</div></div>",
        "<div xmlns=\"http://www.w3.org/1999/xhtml\"/>",
    ];
    for div in cases {
        assert!(
            matches!(div_value(div), Value::Str(s) if s.as_ref() == div),
            "{div}"
        );
    }
}

const XHTML_DIV: &str = "<div xmlns=\"http://www.w3.org/1999/xhtml\">x</div>";

#[test]
fn elements_outside_the_fhir_namespace_are_rejected() {
    let cases = [
        "<Patient xmlns=\"urn:not-fhir\"/>".to_owned(),
        "<Patient><active value=\"true\"/></Patient>".to_owned(),
        format!("<Patient {FHIR_NS}><x:active xmlns:x=\"urn:other\" value=\"true\"/></Patient>"),
        format!("<Patient {FHIR_NS}><name><family xmlns=\"\" value=\"S\"/></name></Patient>"),
    ];
    for source in cases {
        assert!(xml_error(&source).contains("namespace"), "{source}");
    }
}

#[test]
fn narrative_div_outside_the_xhtml_namespace_is_rejected() {
    let cases = [
        "<div>x</div>",
        "<div xmlns=\"urn:other\">x</div>",
        "<div xmlns=\"http://www.w3.org/1999/xhtml\"><p xmlns=\"urn:other\">x</p></div>",
    ];
    for div in cases {
        let source =
            format!("<Patient {FHIR_NS}><text><status value=\"generated\"/>{div}</text></Patient>");
        assert!(xml_error(&source).contains("namespace"), "{div}");
    }
}

#[test]
fn prefixed_attributes_are_rejected() {
    let source = format!(
        "<Patient {FHIR_NS} xmlns:fhir=\"{}\"><gender fhir:value=\"male\"/></Patient>",
        "http://hl7.org/fhir"
    );
    assert!(xml_error(&source).contains("namespace"));
}

const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

// xml.html 2.1.6.3.4 forbids the schema instance namespace, but the document still parses:
// the parser records each element that declares or uses it, and the validator reports it.
#[test]
fn schema_instance_namespace_is_recorded_not_rejected() {
    let cases = [
        (format!("<Patient {FHIR_NS} xmlns:xsi=\"{XSI}\"/>"), 1),
        (
            format!(
                "<Patient {FHIR_NS} xmlns:xsi=\"{XSI}\" xsi:schemaLocation=\"{}\">{}</Patient>",
                "http://hl7.org/fhir patient.xsd", "<active value=\"true\"/>"
            ),
            1,
        ),
        (
            format!(
                "<Patient {FHIR_NS}><name xmlns:s=\"{XSI}\"><family value=\"S\"/></name></Patient>"
            ),
            1,
        ),
        (
            format!(
                "<Patient {FHIR_NS} xmlns:xsi=\"{XSI}\">{}</Patient>",
                "<active xsi:type=\"x\" value=\"true\"/>"
            ),
            2,
        ),
    ];
    for (source, count) in cases {
        let resource = parse_xml(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(resource.schema_instance.len(), count, "{source}");
    }
}

#[test]
fn prefixed_fhir_and_xhtml_elements_are_accepted() {
    let source = concat!(
        "<f:Patient xmlns:f=\"http://hl7.org/fhir\"><f:text><f:status value=\"generated\"/>",
        "<h:div xmlns:h=\"http://www.w3.org/1999/xhtml\"><h:p xml:lang=\"en\">x</h:p></h:div>",
        "</f:text><f:active value=\"true\"/></f:Patient>",
    );
    assert!(parse_xml(source).is_ok());
    let source = format!(
        "<Patient {FHIR_NS}><text><status value=\"generated\"/>{XHTML_DIV}</text></Patient>"
    );
    assert!(parse_xml(&source).is_ok());
}

#[test]
fn deep_nesting_inside_the_div_is_rejected() {
    let n = 10_000;
    let div = format!(
        "<div xmlns=\"http://www.w3.org/1999/xhtml\">{}{}</div>",
        "<p>".repeat(n),
        "</p>".repeat(n)
    );
    let source =
        format!("<Patient {FHIR_NS}><text><status value=\"generated\"/>{div}</text></Patient>");
    assert!(xml_error(&source).contains("depth"));
}

#[test]
fn many_namespace_declarations_on_one_element_are_rejected() {
    let declarations: String = (0..64)
        .map(|i| format!(" xmlns:p{i}=\"urn:{i}\""))
        .collect();
    let source = format!("<Patient {FHIR_NS}><name{declarations}/></Patient>");
    assert!(xml_error(&source).contains("namespace bindings"));
}

#[test]
fn schema_instance_namespace_written_with_a_character_reference_is_recorded() {
    let source =
        format!("<Patient {FHIR_NS} xmlns:s=\"http://www.w3.org/2001/XMLSchema&#45;instance\"/>");
    assert_eq!(parse_xml(&source).expect("parses").schema_instance.len(), 1);
}

#[test]
fn utf8_encoding_declaration_is_accepted_in_any_case() {
    for encoding in ["UTF-8", "utf-8"] {
        let source = format!("<?xml version=\"1.0\" encoding=\"{encoding}\"?><Patient {FHIR_NS}/>");
        assert!(parse_xml(&source).is_ok(), "{encoding}");
    }
}

#[test]
fn attribute_values_are_trimmed_on_read() {
    let source = format!("<Patient {FHIR_NS}><active value=\" true\t\"/></Patient>");
    let resource = parse_xml(&source).expect("parses");
    assert!(matches!(&resource.fields["active"].value, Value::Str(s) if s.as_ref() == "true"));
    assert_eq!(resource.trimmed_attributes.len(), 1);
}

#[test]
fn trimmed_resource_id_is_recorded_with_its_element_span() {
    let source =
        format!("<Patient {FHIR_NS}>\n<id value=\" p1 \"/><active value=\"true\"/></Patient>");
    let resource = parse_xml(&source).expect("parses");
    assert_eq!(resource.id.as_deref(), Some("p1"));
    match resource.trimmed_attributes.as_slice() {
        [TrimmedAttribute::ResourceId(span)] => assert_eq!(span.line, 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn whitespace_only_attribute_values_are_kept_as_read() {
    let source = format!("<Patient {FHIR_NS}><gender value=\" \t \"/></Patient>");
    let resource = parse_xml(&source).expect("parses");
    // Attribute normalization turns the tab into a space; no trim, so no empty value.
    assert!(matches!(&resource.fields["gender"].value, Value::Str(s) if s.as_ref() == "   "));
    assert!(resource.trimmed_attributes.is_empty());
}

#[test]
fn documents_without_the_schema_instance_namespace_record_nothing() {
    let source = format!(
        "<Patient {FHIR_NS} xmlns:s=\"{}\"><active value=\"true\"/></Patient>",
        "http://www.w3.org/2001/XMLSchema"
    );
    assert!(
        parse_xml(&source)
            .expect("parses")
            .schema_instance
            .is_empty()
    );
}

/// A `<Patient>` whose `<id>` has `id_attrs` and one extension holding `extension_value`.
fn patient_with_id(id_attrs: &str, extension_value: &str) -> String {
    format!(
        "<Patient {FHIR_NS}><id {id_attrs}><extension url=\"http://x\">{extension_value}{}",
        "</extension></id></Patient>"
    )
}

/// A resource `id` with an extension keeps its value as `Resource::id` and the rest
/// as an `id` primitive object without `value`, which the validator reshapes into `_id`.
#[test]
fn resource_id_with_an_extension_parses() {
    let source = patient_with_id("id=\"e1\" value=\"p1\"", "<valueString value=\"v\"/>");
    let resource = parse_xml(&source).expect("parses");
    assert_eq!(resource.id.as_deref(), Some("p1"));
    let kept = &resource.fields["id"];
    let Value::Object(parts) = &kept.value else {
        panic!("expected the id element to be kept as an object, got {kept:?}");
    };
    assert!(
        !parts.contains_key("value"),
        "the value is lifted to Resource::id"
    );
    assert!(matches!(&parts["id"].value, Value::Str(s) if s.as_ref() == "e1"));
    let Value::Object(extension) = &parts["extension"].value else {
        panic!("expected the extension to be reachable");
    };
    assert!(matches!(&extension["url"].value, Value::Str(s) if s.as_ref() == "http://x"));
    // The kept element carries the id element's span, at its `<id` tag.
    assert_eq!(&source[kept.span.offset as usize..][..3], "<id");
}

#[test]
fn plain_resource_id_is_lifted_out_of_the_fields() {
    let resource =
        parse_xml(&format!("<Patient {FHIR_NS}><id value=\"p1\"/></Patient>")).expect("parses");
    assert_eq!(resource.id.as_deref(), Some("p1"));
    assert!(resource.fields.is_empty());
}

#[test]
fn repeated_resource_id_is_rejected() {
    let source = format!("<Patient {FHIR_NS}><id value=\"a\"/><id value=\"b\"/></Patient>");
    assert!(xml_error(&source).contains("'id'"));
}

#[test]
fn resource_id_value_child_element_is_not_lifted() {
    let source = format!("<Patient {FHIR_NS}><id><value value=\"p1\"/></id></Patient>");
    let resource = parse_xml(&source).expect("parses");
    assert_eq!(resource.id, None);
    let Value::Object(parts) = &resource.fields["id"].value else {
        panic!("expected the id element to be kept");
    };
    assert!(
        parts.contains_key("value"),
        "left for the validator to report"
    );
}

#[test]
fn empty_resource_id_is_rejected() {
    let source = format!("<Patient {FHIR_NS}><id/></Patient>");
    assert!(xml_error(&source).contains("'id'"));
}

#[test]
fn resource_id_with_only_an_element_id_is_kept_for_the_validator() {
    let source = format!("<Patient {FHIR_NS}><id id=\"e1\"/></Patient>");
    let resource = parse_xml(&source).expect("parses");
    assert_eq!(resource.id, None);
    assert!(
        matches!(&resource.fields["id"].value, Value::Object(parts) if parts.contains_key("id"))
    );
}

#[test]
fn control_character_in_a_resource_id_extension_is_rejected() {
    let source = patient_with_id("value=\"p1\"", "<valueString value=\"a&#1;b\"/>");
    assert!(xml_error(&source).contains("control"));
}
