//! Reshape a resource parsed from XML into the JSON form the checks expect.
//!
//! In XML every primitive is text and a primitive carrying an `id` or extension is an
//! object holding `value`. The reshape types each primitive from its Registry element type
//! and moves the non-value part into `_name`, so the checks see one shape for both formats.
//! Text that does not fit its type is left as a string so `TYPE_001` still reports it.
//! Arrays stay as the XML parser grouped them (repeated siblings only).
//!
//! The reshape also checks the XML form of each element: elements whose `representation`
//! is `xmlAttr` (a primitive's `value`, `Element.id`, `Extension.url`) must be attributes,
//! every other element a child element. The parser gives an attribute its element's span
//! and a child element its own, which is how the two are told apart. Child elements must
//! follow snapshot order (xml.html); their spans give the written order back after the
//! parser grouped repeated siblings, and `STRUCTURE_004` reports each one out of place.
//!
//! The parser trims attribute values (xml.html: readers SHOULD trim); each trimmed value of
//! a known element gets a `TYPE_007` Warning here, where its instance path is known
//! (user decision 11.7: trim and warn rather than report the whitespace as an error).

use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Span, TrimmedAttribute, Value};
use indexmap::IndexMap;
use std::collections::HashSet;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::embedded::{concrete_resource, holds_resource, typed_element};
use crate::resolve::{complex_type, is_primitive, resolve};

type Fields = IndexMap<Arc<str>, Node>;

/// The JSON-shaped resource and the XML representation findings.
pub(crate) fn to_json_shape(
    resource: &Resource,
    sd: &StructureDefinition,
    registry: &Registry,
) -> (Resource, Vec<Diagnostic>) {
    let name = resource.resource_type.as_ref();
    let mut shaper = Shaper {
        registry,
        // A set: hostile input can pad every attribute, so no linear scan per lookup.
        trimmed: resource
            .trimmed_attributes
            .iter()
            .filter_map(|trimmed| match trimmed {
                TrimmedAttribute::Element(offset, name) => Some((*offset, name.as_ref())),
                TrimmedAttribute::ResourceId(_) => None,
            })
            .collect(),
        // xml.html 2.1.6.3.4: content SHALL NOT "even contain the schema instance namespace".
        diagnostics: resource
            .schema_instance
            .iter()
            .map(|span| {
                Diagnostic::error(
                    "STRUCTURE_003",
                    "The XML schema instance namespace is not allowed in FHIR resources",
                    name.to_owned(),
                )
                .with_location(*span)
            })
            .collect(),
    };
    for trimmed in &resource.trimmed_attributes {
        if let TrimmedAttribute::ResourceId(span) = trimmed {
            shaper.trimmed_warning(&format!("{name}.id"), *span);
        }
    }
    // The parser rejects attributes on the root, so its fields have no parent offset.
    let fields = shaper.fields(&resource.fields, sd, name, name, None);
    let reshaped = Resource {
        fields,
        resource_type: Arc::clone(&resource.resource_type),
        id: resource.id.clone(),
        from_xml: false,
        trimmed_attributes: Vec::new(),
        schema_instance: Vec::new(),
    };
    (reshaped, shaper.diagnostics)
}

struct Shaper<'a> {
    registry: &'a Registry,
    trimmed: HashSet<(u32, &'a str)>,
    diagnostics: Vec<Diagnostic>,
}

/// The items of a node: itself, or each element of an array with its indexed path.
fn items<'n>(node: &'n Node, path: &str) -> Vec<(&'n Node, String)> {
    match &node.value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| (item, format!("{path}[{i}]")))
            .collect(),
        _ => vec![(node, path.to_owned())],
    }
}

impl Shaper<'_> {
    /// Reshape the children of one element; `parent` is its offset, shared by its attributes.
    fn fields(
        &mut self,
        fields: &Fields,
        sd: &StructureDefinition,
        def_path: &str,
        path: &str,
        parent: Option<u32>,
    ) -> Fields {
        let prefix = format!("{def_path}.");
        let mut out = Fields::with_capacity(fields.len());
        // Child elements as (span, snapshot position, path); attributes have no order.
        let mut siblings = Vec::new();
        for (name, node) in fields {
            let resolved = resolve(sd, &format!("{prefix}{name}"), name);
            // A choice variant resolves to its `[x]` element, so it takes that element's place.
            let position = resolved
                .as_ref()
                .and_then(|(elem, _)| sd.snapshot.iter().position(|e| std::ptr::eq(e, *elem)));
            let found = resolved.and_then(|(elem, ty)| typed_element(self.registry, sd, elem, ty));
            let Some((elem, sd, ty)) = found else {
                out.insert(Arc::clone(name), node.clone());
                continue;
            };
            let child_path = format!("{path}.{name}");
            for (item, item_path) in items(node, &child_path) {
                self.check_form(item, name, elem.xml_attr, parent, &item_path);
                if let Some(position) = position.filter(|_| parent != Some(item.span.offset)) {
                    siblings.push((item.span, position, item_path));
                }
            }
            if holds_resource(&ty, self.registry) {
                out.insert(Arc::clone(name), self.embedded(node, &child_path));
                continue;
            }
            if is_primitive(&ty) {
                let extension_path = format!("{path}._{name}");
                let (values, extensions): (Vec<Node>, Vec<Node>) = items(node, &child_path)
                    .into_iter()
                    .zip(items(node, &extension_path))
                    .map(|((item, value_path), (_, ext_path))| {
                        self.split_primitive(item, &ty, &value_path, &ext_path, parent)
                    })
                    .unzip();
                let wrap = |nodes: Vec<Node>| match &node.value {
                    Value::Array(_) => Node {
                        value: Value::Array(nodes),
                        span: node.span,
                    },
                    _ => nodes.into_iter().next().unwrap_or_else(|| node.clone()),
                };
                if values.iter().any(|n| !matches!(n.value, Value::Null)) {
                    out.insert(Arc::clone(name), wrap(values));
                }
                if extensions.iter().any(|n| !matches!(n.value, Value::Null)) {
                    out.insert(Arc::from(format!("_{name}")), wrap(extensions));
                }
                continue;
            }
            let (type_sd, type_path) =
                if ty.as_ref() == "BackboneElement" || ty.as_ref() == "Element" {
                    (Some(sd), elem.path.to_string())
                } else {
                    let type_sd = complex_type(&ty, self.registry);
                    (
                        type_sd,
                        type_sd.map(|t| t.name.to_string()).unwrap_or_default(),
                    )
                };
            let reshaped = match type_sd {
                Some(type_sd) => self.complex(node, type_sd, &type_path, &child_path),
                None => node.clone(),
            };
            out.insert(Arc::clone(name), reshaped);
        }
        self.check_order(siblings);
        out
    }

    /// `STRUCTURE_004` for each child element written after a sibling that the snapshot
    /// lists later (xml.html: "Elements must always appear in the order documented").
    fn check_order(&mut self, mut siblings: Vec<(Span, usize, String)>) {
        siblings.sort_by_key(|(span, ..)| span.offset);
        let mut latest: Option<(usize, &str)> = None;
        for (span, position, path) in &siblings {
            match latest {
                Some((latest_position, latest_path)) if *position < latest_position => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "STRUCTURE_004",
                            format!(
                                "Element '{path}' is out of order: it must come before \
                                 '{latest_path}'"
                            ),
                            path.clone(),
                        )
                        .with_location(*span),
                    );
                }
                _ => latest = Some((*position, path)),
            }
        }
    }

    /// Unwrap the resources of an element typed `Resource` (references.html: XML
    /// `<contained><Practitioner>...</Practitioner></contained>`) into the JSON form: an
    /// object whose `resourceType` names the wrapping element, its content reshaped against
    /// that resource's definition. Anything else is kept for the walk to report.
    fn embedded(&mut self, node: &Node, path: &str) -> Node {
        let value = match &node.value {
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| self.embedded(item, &format!("{path}[{i}]")))
                    .collect(),
            ),
            Value::Object(wrapper) => match wrapper.iter().collect::<Vec<_>>().as_slice() {
                [(resource_type, inner)] => match (
                    &inner.value,
                    concrete_resource(self.registry, resource_type),
                ) {
                    (Value::Object(fields), Ok(sd)) => {
                        let reshaped =
                            self.fields(fields, sd, resource_type, path, Some(inner.span.offset));
                        let type_node = Node {
                            value: Value::Str(Arc::clone(resource_type)),
                            span: inner.span,
                        };
                        Value::Object(
                            std::iter::once((Arc::from("resourceType"), type_node))
                                .chain(reshaped)
                                .collect(),
                        )
                    }
                    _ => node.value.clone(),
                },
                _ => node.value.clone(),
            },
            other => other.clone(),
        };
        Node {
            value,
            span: node.span,
        }
    }

    fn complex(
        &mut self,
        node: &Node,
        sd: &StructureDefinition,
        def_path: &str,
        path: &str,
    ) -> Node {
        let value = match &node.value {
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| self.complex(item, sd, def_path, &format!("{path}[{i}]")))
                    .collect(),
            ),
            Value::Object(fields) => {
                Value::Object(self.fields(fields, sd, def_path, path, Some(node.span.offset)))
            }
            other => other.clone(),
        };
        Node {
            value,
            span: node.span,
        }
    }

    /// `STRUCTURE_001` when an element is an attribute but must be a child element, or
    /// the reverse (xml.html: `value`, `id` and `url` are attributes, other elements not).
    fn check_form(
        &mut self,
        node: &Node,
        name: &str,
        xml_attr: bool,
        parent: Option<u32>,
        path: &str,
    ) {
        let is_attribute = parent == Some(node.span.offset);
        if is_attribute != xml_attr {
            self.unknown_form(node, path, xml_attr);
        } else if is_attribute {
            self.warn_if_trimmed(node, name, path);
        }
    }

    /// `TYPE_007` Warning when the parser trimmed attribute `name` of the element at
    /// `node`'s offset.
    fn warn_if_trimmed(&mut self, node: &Node, name: &str, path: &str) {
        if self.trimmed.contains(&(node.span.offset, name)) {
            self.trimmed_warning(path, node.span);
        }
    }

    fn trimmed_warning(&mut self, path: &str, span: Span) {
        self.diagnostics.push(
            Diagnostic::warning(
                "TYPE_007",
                format!("Leading or trailing whitespace trimmed from the XML value at '{path}'"),
                path.to_owned(),
            )
            .with_location(span),
        );
    }

    fn unknown_form(&mut self, node: &Node, path: &str, xml_attr: bool) {
        let (expected, found) = if xml_attr {
            ("an XML attribute", "a child element")
        } else {
            ("a child element", "an XML attribute")
        };
        self.diagnostics.push(
            Diagnostic::error(
                "STRUCTURE_001",
                format!("Unknown element '{path}': it must be {expected}, not {found}"),
                path.to_owned(),
            )
            .with_location(node.span),
        );
    }

    /// Split one XML primitive into its typed value and its `_name` part (`Null` when
    /// absent). The `_name` part is reshaped as an `Element`, so its extension values are
    /// typed too. An element with neither value nor `_name` part is kept as is, so the type
    /// check reports it.
    fn split_primitive(
        &mut self,
        node: &Node,
        ty: &str,
        path: &str,
        extension_path: &str,
        parent: Option<u32>,
    ) -> (Node, Node) {
        let null = Node {
            value: Value::Null,
            span: node.span,
        };
        match &node.value {
            Value::Str(text) => {
                // A plain string is an element whose only attribute is `value`, or an
                // attribute itself (`id`, `url`), which `check_form` already handled.
                if parent != Some(node.span.offset) {
                    self.warn_if_trimmed(node, "value", path);
                }
                (
                    Node {
                        value: typed(text, ty),
                        span: node.span,
                    },
                    null,
                )
            }
            Value::Object(fields) => {
                let rest: Fields = fields
                    .iter()
                    .filter(|(k, _)| k.as_ref() != "value")
                    .map(|(k, v)| (Arc::clone(k), v.clone()))
                    .collect();
                let value = match fields.get("value") {
                    None => null.clone(),
                    Some(v) => {
                        // Every primitive's `value` is an attribute (`representation` xmlAttr).
                        if v.span.offset != node.span.offset {
                            self.unknown_form(v, &format!("{path}.value"), true);
                        } else {
                            self.warn_if_trimmed(v, "value", path);
                        }
                        match &v.value {
                            Value::Str(text) => Node {
                                value: typed(text, ty),
                                span: v.span,
                            },
                            _ => v.clone(),
                        }
                    }
                };
                if rest.is_empty() && matches!(value.value, Value::Null) {
                    return (node.clone(), null);
                }
                let extension = if rest.is_empty() {
                    null
                } else {
                    let rest = match complex_type("Element", self.registry) {
                        Some(element_sd) => self.fields(
                            &rest,
                            element_sd,
                            "Element",
                            extension_path,
                            Some(node.span.offset),
                        ),
                        None => rest,
                    };
                    Node {
                        value: Value::Object(rest),
                        span: node.span,
                    }
                };
                (value, extension)
            }
            _ => (node.clone(), null),
        }
    }
}

/// The JSON value for XML text of primitive type `ty`; the text itself when it does not fit.
fn typed(text: &Arc<str>, ty: &str) -> Value {
    let keep = || Value::Str(Arc::clone(text));
    match ty {
        "boolean" => match text.as_ref() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => keep(),
        },
        // Official integer regex `[0]|[-+]?[1-9][0-9]*`.
        "integer" | "unsignedInt" | "positiveInt" => {
            let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
            let canonical = text.as_ref() == "0"
                || (digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0'));
            match text.parse::<i64>() {
                Ok(n) if canonical => Value::Integer(n),
                _ => keep(),
            }
        }
        "decimal" if is_decimal(text) => match text.parse::<f64>() {
            Ok(n) if n.is_finite() => Value::Decimal(n),
            _ => keep(),
        },
        _ => keep(),
    }
}

/// Official decimal regex `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
fn is_decimal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (mantissa, exponent) = s
        .split_once(['e', 'E'])
        .map_or((s, None), |(m, e)| (m, Some(e)));
    let (int, frac) = mantissa
        .split_once('.')
        .map_or((mantissa, None), |(i, f)| (i, Some(f)));
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    (int == "0" || (digits(int) && !int.starts_with('0')))
        && frac.is_none_or(digits)
        && exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_converts_only_well_formed_text() {
        let v = |t: &str, ty| typed(&Arc::from(t), ty);
        assert!(matches!(v("2", "integer"), Value::Integer(2)));
        assert!(matches!(v("007", "integer"), Value::Str(_)));
        assert!(matches!(v("1.50", "decimal"), Value::Decimal(_)));
        assert!(matches!(v("1e999", "decimal"), Value::Str(_)));
        assert!(matches!(v("NaN", "decimal"), Value::Str(_)));
        assert!(matches!(v("false", "boolean"), Value::Bool(false)));
        assert!(matches!(v("yes", "boolean"), Value::Str(_)));
    }
}
