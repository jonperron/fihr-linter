use std::sync::Arc;

use indexmap::IndexMap;

use crate::line_index::LineIndex;
use crate::types::{Node, ParseError, Resource, Span, Value};

/// Parse a FHIR JSON resource from a source string.
///
/// Returns a [`Resource`] on success, or a [`ParseError`] if the source is
/// not valid JSON or is missing the `"resourceType"` field.
pub fn parse_json(source: &str) -> Result<Resource, ParseError> {
    let line_index = LineIndex::new(source);
    let mut scanner = Scanner::new(source, &line_index);

    let root = scanner.scan_node()?;

    scanner.skip_whitespace();
    if scanner.pos < scanner.source.len() {
        return Err(scanner.error_at(scanner.pos, "unexpected trailing content after JSON value"));
    }

    let mut fields = match root.value {
        Value::Object(obj) => obj,
        _ => {
            return Err(ParseError::JsonError {
                message: "expected a JSON object at root".into(),
                line: root.span.line,
                col: root.span.col,
            });
        }
    };

    let resource_type = match fields.shift_remove("resourceType") {
        None => return Err(ParseError::MissingResourceType),
        Some(n) => match n.value {
            Value::Str(s) => s,
            _ => {
                return Err(ParseError::JsonError {
                    message: "'resourceType' must be a string".into(),
                    line: n.span.line,
                    col: n.span.col,
                });
            }
        },
    };

    let id = match fields.shift_remove("id") {
        None => None,
        Some(n) => match n.value {
            Value::Str(s) => Some(s),
            _ => {
                return Err(ParseError::JsonError {
                    message: "'id' must be a string".into(),
                    line: n.span.line,
                    col: n.span.col,
                });
            }
        },
    };

    Ok(Resource {
        resource_type,
        id,
        fields,
    })
}

// ── Internal scanner ──────────────────────────────────────────────────────────

struct Scanner<'src, 'idx> {
    source: &'src [u8],
    pos: usize,
    line_index: &'idx LineIndex,
}

impl<'src, 'idx> Scanner<'src, 'idx> {
    fn new(source: &'src str, line_index: &'idx LineIndex) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line_index,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.source.len()
            && matches!(self.source[self.pos], b' ' | b'\t' | b'\r' | b'\n')
        {
            self.pos += 1;
        }
    }

    fn span_at(&self, offset: usize) -> Span {
        let (line, col) = self.line_index.location(offset as u32);
        Span {
            line,
            col,
            offset: offset as u32,
        }
    }

    fn error_at(&self, offset: usize, msg: impl Into<String>) -> ParseError {
        let (line, col) = self.line_index.location(offset as u32);
        ParseError::JsonError {
            message: msg.into(),
            line,
            col,
        }
    }

    fn current_byte(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    // ── Top-level value dispatch ──────────────────────────────────────────

    fn scan_node(&mut self) -> Result<Node, ParseError> {
        self.skip_whitespace();
        let start = self.pos;
        let span = self.span_at(start);

        match self.current_byte() {
            None => Err(self.error_at(start, "unexpected end of input")),
            Some(b'"') => {
                let s = self.scan_string()?;
                Ok(Node {
                    value: Value::Str(s),
                    span,
                })
            }
            Some(b'{') => {
                let obj = self.scan_object()?;
                Ok(Node {
                    value: Value::Object(obj),
                    span,
                })
            }
            Some(b'[') => {
                let arr = self.scan_array()?;
                Ok(Node {
                    value: Value::Array(arr),
                    span,
                })
            }
            Some(b't') => {
                self.expect_literal(b"true")?;
                Ok(Node {
                    value: Value::Bool(true),
                    span,
                })
            }
            Some(b'f') => {
                self.expect_literal(b"false")?;
                Ok(Node {
                    value: Value::Bool(false),
                    span,
                })
            }
            Some(b'n') => {
                self.expect_literal(b"null")?;
                Ok(Node {
                    value: Value::Null,
                    span,
                })
            }
            Some(b) if b == b'-' || b.is_ascii_digit() => {
                let num = self.scan_number()?;
                Ok(Node { value: num, span })
            }
            Some(b) => Err(self.error_at(start, format!("unexpected byte '{}'", b as char))),
        }
    }

    // ── Literal tokens ────────────────────────────────────────────────────

    fn expect_literal(&mut self, literal: &[u8]) -> Result<(), ParseError> {
        let end = self.pos + literal.len();
        if end > self.source.len() {
            return Err(self.error_at(self.pos, "unexpected end of input"));
        }
        if &self.source[self.pos..end] != literal {
            let got = std::str::from_utf8(&self.source[self.pos..end]).unwrap_or("?");
            return Err(self.error_at(
                self.pos,
                format!(
                    "expected '{}', got '{}'",
                    std::str::from_utf8(literal).unwrap_or("?"),
                    got,
                ),
            ));
        }
        self.pos = end;
        Ok(())
    }

    // ── String scanning ───────────────────────────────────────────────────

    fn scan_string(&mut self) -> Result<Arc<str>, ParseError> {
        debug_assert_eq!(self.source[self.pos], b'"');
        let start = self.pos;
        self.pos += 1;

        let mut result = String::new();

        loop {
            let slice = &self.source[self.pos..];
            // Find the next byte requiring special handling.
            let next_special = slice
                .iter()
                .position(|&b| b == b'"' || b == b'\\' || b < 0x20);

            match next_special {
                None => return Err(self.error_at(start, "unterminated string")),
                Some(n) => {
                    if n > 0 {
                        let prefix = std::str::from_utf8(&slice[..n])
                            .map_err(|_| self.error_at(self.pos, "invalid UTF-8 in string"))?;
                        result.push_str(prefix);
                        self.pos += n;
                    }

                    match self.source[self.pos] {
                        b'"' => {
                            self.pos += 1;
                            return Ok(Arc::from(result.as_str()));
                        }
                        b'\\' => {
                            self.pos += 1;
                            if self.pos >= self.source.len() {
                                return Err(self.error_at(start, "unterminated escape sequence"));
                            }
                            match self.source[self.pos] {
                                b'"' => {
                                    result.push('"');
                                    self.pos += 1;
                                }
                                b'\\' => {
                                    result.push('\\');
                                    self.pos += 1;
                                }
                                b'/' => {
                                    result.push('/');
                                    self.pos += 1;
                                }
                                b'b' => {
                                    result.push('\x08');
                                    self.pos += 1;
                                }
                                b'f' => {
                                    result.push('\x0c');
                                    self.pos += 1;
                                }
                                b'n' => {
                                    result.push('\n');
                                    self.pos += 1;
                                }
                                b'r' => {
                                    result.push('\r');
                                    self.pos += 1;
                                }
                                b't' => {
                                    result.push('\t');
                                    self.pos += 1;
                                }
                                b'u' => {
                                    // self.pos currently points at 'u'; backslash is one behind.
                                    let escape_offset = self.pos - 1;
                                    self.pos += 1;
                                    let c = self.scan_unicode_escape(escape_offset)?;
                                    result.push(c);
                                }
                                other => {
                                    return Err(self.error_at(
                                        self.pos - 1,
                                        format!("invalid escape '\\{}'", other as char),
                                    ));
                                }
                            }
                        }
                        b => {
                            return Err(self.error_at(
                                self.pos,
                                format!("control character in string: {:#04x}", b),
                            ));
                        }
                    }
                }
            }
        }
    }

    fn scan_unicode_escape(&mut self, err_offset: usize) -> Result<char, ParseError> {
        if self.pos + 4 > self.source.len() {
            return Err(self.error_at(err_offset, "incomplete unicode escape"));
        }
        let hex = std::str::from_utf8(&self.source[self.pos..self.pos + 4])
            .map_err(|_| self.error_at(err_offset, "invalid unicode escape sequence"))?;
        let code_point = u32::from_str_radix(hex, 16)
            .map_err(|_| self.error_at(err_offset, format!("invalid unicode hex: {}", hex)))?;

        self.pos += 4;

        // Handle surrogate pairs: high surrogate must be followed by \uXXXX low surrogate.
        if (0xD800..=0xDBFF).contains(&code_point) {
            return self.scan_surrogate_pair(err_offset, code_point);
        }

        char::from_u32(code_point).ok_or_else(|| {
            self.error_at(
                err_offset,
                format!("invalid Unicode code point: U+{:04X}", code_point),
            )
        })
    }

    fn scan_surrogate_pair(&mut self, err_offset: usize, high: u32) -> Result<char, ParseError> {
        if self.pos + 6 > self.source.len() || &self.source[self.pos..self.pos + 2] != b"\\u" {
            return Err(self.error_at(err_offset, "unpaired high surrogate"));
        }
        self.pos += 2;
        let hex = std::str::from_utf8(&self.source[self.pos..self.pos + 4])
            .map_err(|_| self.error_at(err_offset, "invalid low surrogate hex"))?;
        let low = u32::from_str_radix(hex, 16)
            .map_err(|_| self.error_at(err_offset, "invalid low surrogate hex"))?;
        if !(0xDC00..=0xDFFF).contains(&low) {
            return Err(self.error_at(err_offset, "invalid low surrogate value"));
        }
        self.pos += 4;
        let code_point = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
        char::from_u32(code_point)
            .ok_or_else(|| self.error_at(err_offset, "invalid surrogate pair"))
    }

    // ── Number scanning ───────────────────────────────────────────────────

    fn scan_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        let mut is_float = false;

        if self.pos < self.source.len() && self.source[self.pos] == b'-' {
            self.pos += 1;
        }

        let digit_start = self.pos;
        while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == digit_start {
            return Err(self.error_at(start, "invalid number: expected digit"));
        }

        // JSON forbids leading zeros in integer parts: 01, -00, etc.
        let integer_digits = &self.source[digit_start..self.pos];
        if integer_digits.len() > 1 && integer_digits[0] == b'0' {
            return Err(self.error_at(digit_start, "invalid number: leading zeros are not allowed"));
        }

        if self.pos < self.source.len() && self.source[self.pos] == b'.' {
            is_float = true;
            self.pos += 1;
            let frac_start = self.pos;
            while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == frac_start {
                return Err(self.error_at(self.pos, "invalid number: expected digit after '.'"));
            }
        }

        if self.pos < self.source.len() && matches!(self.source[self.pos], b'e' | b'E') {
            is_float = true;
            self.pos += 1;
            if self.pos < self.source.len() && matches!(self.source[self.pos], b'+' | b'-') {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == exp_start {
                return Err(self.error_at(self.pos, "invalid number: expected digit in exponent"));
            }
        }

        let num_str = std::str::from_utf8(&self.source[start..self.pos])
            .map_err(|_| self.error_at(start, "invalid number encoding"))?;

        if is_float {
            let f: f64 = num_str
                .parse()
                .map_err(|_| self.error_at(start, format!("invalid float: {}", num_str)))?;
            Ok(Value::Decimal(f))
        } else {
            let i: i64 = num_str
                .parse()
                .map_err(|_| self.error_at(start, format!("invalid integer: {}", num_str)))?;
            Ok(Value::Integer(i))
        }
    }

    // ── Object scanning ───────────────────────────────────────────────────

    fn scan_object(&mut self) -> Result<IndexMap<Arc<str>, Node>, ParseError> {
        debug_assert_eq!(self.source[self.pos], b'{');
        self.pos += 1;

        let mut map: IndexMap<Arc<str>, Node> = IndexMap::new();
        self.skip_whitespace();

        if self.current_byte() == Some(b'}') {
            self.pos += 1;
            return Ok(map);
        }

        loop {
            self.skip_whitespace();
            if self.current_byte() != Some(b'"') {
                return Err(self.error_at(self.pos, "expected string key in object"));
            }
            let key = self.scan_string()?;

            self.skip_whitespace();
            if self.current_byte() != Some(b':') {
                return Err(self.error_at(self.pos, "expected ':' after object key"));
            }
            self.pos += 1;

            let node = self.scan_node()?;
            map.insert(key, node);

            self.skip_whitespace();
            match self.current_byte() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.error_at(self.pos, "expected ',' or '}' in object")),
            }
        }

        Ok(map)
    }

    // ── Array scanning ────────────────────────────────────────────────────

    fn scan_array(&mut self) -> Result<Vec<Node>, ParseError> {
        debug_assert_eq!(self.source[self.pos], b'[');
        self.pos += 1;

        let mut arr = Vec::new();
        self.skip_whitespace();

        if self.current_byte() == Some(b']') {
            self.pos += 1;
            return Ok(arr);
        }

        loop {
            let node = self.scan_node()?;
            arr.push(node);

            self.skip_whitespace();
            match self.current_byte() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.error_at(self.pos, "expected ',' or ']' in array")),
            }
        }

        Ok(arr)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ParseError, Value};

    const MINIMAL_PATIENT: &str = r#"{
  "resourceType": "Patient",
  "id": "example"
}"#;

    #[test]
    fn parse_patient_example_json() {
        let resource = parse_json(MINIMAL_PATIENT).unwrap();
        assert_eq!(&*resource.resource_type, "Patient");
        assert_eq!(resource.id.as_deref(), Some("example"));
    }

    #[test]
    fn parse_returns_all_top_level_fields() {
        let source = r#"{
  "resourceType": "Observation",
  "id": "obs1",
  "status": "final",
  "code": {}
}"#;
        let resource = parse_json(source).unwrap();
        assert_eq!(&*resource.resource_type, "Observation");
        assert!(resource.fields.contains_key("status"));
        assert!(resource.fields.contains_key("code"));
    }

    #[test]
    fn span_points_to_correct_line_for_field() {
        let source = "{\n  \"resourceType\": \"Patient\",\n  \"status\": \"active\"\n}";
        let resource = parse_json(source).unwrap();
        let status_node = resource.fields.get("status").unwrap();
        // "status" key starts on line 3
        assert_eq!(status_node.span.line, 3);
    }

    #[test]
    fn malformed_json_returns_parse_error() {
        let result = parse_json("{not valid json}");
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }

    #[test]
    fn missing_resource_type_returns_error() {
        let result = parse_json(r#"{"id": "x"}"#);
        assert!(matches!(result, Err(ParseError::MissingResourceType)));
    }

    #[test]
    fn parse_nested_object_preserves_span() {
        let source = "{\n  \"resourceType\": \"Patient\",\n  \"name\": [\n    {\"family\": \"Smith\"}\n  ]\n}";
        let resource = parse_json(source).unwrap();
        let name_node = resource.fields.get("name").unwrap();
        // "name" value (the array) starts on line 3
        assert_eq!(name_node.span.line, 3);
    }

    #[test]
    fn parse_unicode_escape_in_string() {
        let source = r#"{"resourceType": "Patient", "id": "caf\u00e9"}"#;
        let resource = parse_json(source).unwrap();
        assert_eq!(resource.id.as_deref(), Some("café"));
    }

    #[test]
    fn parse_empty_object() {
        let source = r#"{"resourceType": "Basic"}"#;
        let resource = parse_json(source).unwrap();
        assert_eq!(&*resource.resource_type, "Basic");
        assert!(resource.id.is_none());
        assert!(resource.fields.is_empty());
    }

    #[test]
    fn parse_number_values() {
        let source = r#"{"resourceType": "Basic", "count": 42, "ratio": 3.14}"#;
        let resource = parse_json(source).unwrap();
        assert!(matches!(
            resource.fields.get("count").unwrap().value,
            Value::Integer(42)
        ));
        assert!(matches!(
            resource.fields.get("ratio").unwrap().value,
            Value::Decimal(_)
        ));
    }

    #[test]
    fn parse_bool_and_null_values() {
        let source = r#"{"resourceType": "Basic", "active": true, "deceased": null}"#;
        let resource = parse_json(source).unwrap();
        assert!(matches!(
            resource.fields.get("active").unwrap().value,
            Value::Bool(true)
        ));
        assert!(matches!(
            resource.fields.get("deceased").unwrap().value,
            Value::Null
        ));
    }

    #[test]
    fn parse_empty_array() {
        let source = r#"{"resourceType": "Basic", "tags": []}"#;
        let resource = parse_json(source).unwrap();
        assert!(matches!(
            &resource.fields.get("tags").unwrap().value,
            Value::Array(arr) if arr.is_empty()
        ));
    }

    #[test]
    fn trailing_content_returns_error() {
        let result = parse_json(r#"{"resourceType": "Basic"} GARBAGE"#);
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }

    #[test]
    fn resource_type_wrong_type_returns_json_error() {
        let result = parse_json(r#"{"resourceType": 42}"#);
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }

    #[test]
    fn id_wrong_type_returns_json_error() {
        let result = parse_json(r#"{"resourceType": "Patient", "id": 123}"#);
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }

    #[test]
    fn leading_zero_integer_rejected() {
        let result = parse_json(r#"{"resourceType": "Basic", "count": 01}"#);
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }

    #[test]
    fn leading_zero_negative_rejected() {
        let result = parse_json(r#"{"resourceType": "Basic", "count": -00}"#);
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }

    #[test]
    fn zero_alone_is_valid() {
        let source = r#"{"resourceType": "Basic", "count": 0}"#;
        let resource = parse_json(source).unwrap();
        assert!(matches!(
            resource.fields.get("count").unwrap().value,
            Value::Integer(0)
        ));
    }

    #[test]
    fn zero_point_fraction_is_valid() {
        let source = r#"{"resourceType": "Basic", "ratio": 0.5}"#;
        let resource = parse_json(source).unwrap();
        assert!(matches!(
            resource.fields.get("ratio").unwrap().value,
            Value::Decimal(_)
        ));
    }

    #[test]
    fn root_non_object_returns_error() {
        let result = parse_json(r#"[1, 2, 3]"#);
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }

    #[test]
    fn surrogate_pair_decoded_correctly() {
        // \uD83D\uDE00 = U+1F600 = emoji face
        let source = r#"{"resourceType": "Basic", "note": "\uD83D\uDE00"}"#;
        let resource = parse_json(source).unwrap();
        assert!(matches!(
            &resource.fields.get("note").unwrap().value,
            Value::Str(s) if s.as_ref() == "\u{1F600}"
        ));
    }

    #[test]
    fn unpaired_high_surrogate_returns_error() {
        let source = "{\"resourceType\": \"Basic\", \"note\": \"\\uD800\"}";
        let result = parse_json(source);
        assert!(matches!(result, Err(ParseError::JsonError { .. })));
    }
}
