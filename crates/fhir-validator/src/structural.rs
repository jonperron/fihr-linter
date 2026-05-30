use std::collections::HashSet;

use fhir_definitions::StructureDefinition;
use fhir_parser::Resource;

use crate::diagnostic::Diagnostic;

/// Known FHIR type suffixes used for polymorphic (`value[x]`) fields.
const TYPE_SUFFIXES: &[&str] = &[
    "Boolean",
    "Integer",
    "String",
    "Decimal",
    "Uri",
    "Url",
    "Canonical",
    "Base64Binary",
    "Instant",
    "Date",
    "DateTime",
    "Time",
    "Code",
    "Oid",
    "Id",
    "Markdown",
    "UnsignedInt",
    "PositiveInt",
    "Uuid",
    "CodeableConcept",
    "Coding",
    "Reference",
    "Identifier",
    "Quantity",
    "Range",
    "Ratio",
    "SampledData",
    "Signature",
    "HumanName",
    "Address",
    "ContactPoint",
    "Timing",
    "Meta",
    "Period",
    "Attachment",
    "Annotation",
    "Age",
    "Count",
    "Distance",
    "Duration",
    "Money",
    "Expression",
    "RelatedArtifact",
    "UsageContext",
    "DataRequirement",
    "ParameterDefinition",
    "TriggerDefinition",
    "ContactDetail",
    "ExtendedContactDetail",
    "Availability",
    "VirtualServiceDetail",
    "MonetaryComponent",
    "CodeableReference",
    "RatioRange",
];

/// Check that all resource fields correspond to known paths in the snapshot.
///
/// Emits `STRUCTURE_001` for every element whose name cannot be matched to
/// a depth-1 path in the StructureDefinition.
pub fn validate(resource: &Resource, sd: &StructureDefinition, diagnostics: &mut Vec<Diagnostic>) {
    let resource_type = resource.resource_type.as_ref();
    let prefix = format!("{resource_type}.");

    let mut known: HashSet<&str> = HashSet::new();
    let mut polymorphic_bases: HashSet<&str> = HashSet::new();

    for elem in &sd.snapshot {
        let path = elem.path.as_ref();
        // Only direct children: one segment beyond the resource type prefix.
        if let Some(rest) = path.strip_prefix(&prefix) {
            if !rest.contains('.') {
                if let Some(base) = rest.strip_suffix("[x]") {
                    polymorphic_bases.insert(base);
                } else {
                    known.insert(rest);
                }
            }
        }
    }

    for (field_name, node) in &resource.fields {
        let name = field_name.as_ref();

        // Primitive extensions (_fieldName) and explicit extensions are always allowed.
        if name.starts_with('_') || name == "extension" || name == "modifierExtension" {
            continue;
        }

        if known.contains(name) {
            continue;
        }

        if is_polymorphic_match(name, &polymorphic_bases) {
            continue;
        }

        diagnostics.push(
            Diagnostic::error(
                "STRUCTURE_001",
                format!("Unknown element '{resource_type}.{name}'"),
                format!("{resource_type}.{name}"),
            )
            .with_location(node.span),
        );
    }
}

/// Return `true` if `name` matches a polymorphic base with a known type suffix appended.
fn is_polymorphic_match(name: &str, bases: &HashSet<&str>) -> bool {
    for suffix in TYPE_SUFFIXES {
        if let Some(base) = name.strip_suffix(suffix) {
            if !base.is_empty() && bases.contains(base) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use fhir_definitions::{ElementDefinition, StructureDefinition};
    use fhir_parser::{Node, Span, Value};
    use indexmap::IndexMap;

    fn make_sd(resource_type: &str, field_names: &[&str]) -> StructureDefinition {
        let mut snapshot = vec![ElementDefinition {
            path: Arc::from(resource_type),
            min: 0,
            max: Arc::from("1"),
            types: vec![],
            constraints: vec![],
            binding: None,
        }];
        for name in field_names {
            snapshot.push(ElementDefinition {
                path: Arc::from(format!("{resource_type}.{name}").as_str()),
                min: 0,
                max: Arc::from("*"),
                types: vec![],
                constraints: vec![],
                binding: None,
            });
        }
        StructureDefinition {
            url: Arc::from(
                format!("http://hl7.org/fhir/StructureDefinition/{resource_type}").as_str(),
            ),
            name: Arc::from(resource_type),
            kind: Arc::from("resource"),
            is_abstract: false,
            base_definition: None,
            snapshot,
        }
    }

    fn make_resource(resource_type: &str, fields: &[(&str, Value)]) -> fhir_parser::Resource {
        let mut map: IndexMap<Arc<str>, Node> = IndexMap::new();
        let span = Span::default();
        for (name, val) in fields {
            map.insert(
                Arc::from(*name),
                Node {
                    value: val.clone(),
                    span,
                },
            );
        }
        fhir_parser::Resource {
            resource_type: Arc::from(resource_type),
            id: None,
            fields: map,
        }
    }

    #[test]
    fn known_field_produces_no_error() {
        let sd = make_sd("Patient", &["active"]);
        let resource = make_resource("Patient", &[("active", Value::Bool(true))]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn unknown_field_produces_structure_001() {
        let sd = make_sd("Patient", &["active"]);
        let resource = make_resource("Patient", &[("unknownField", Value::Bool(true))]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_ref(), "STRUCTURE_001");
        assert!(diagnostics[0].path.contains("unknownField"));
    }

    #[test]
    fn extension_field_is_always_allowed() {
        let sd = make_sd("Patient", &[]);
        let resource = make_resource(
            "Patient",
            &[
                ("extension", Value::Array(vec![])),
                ("modifierExtension", Value::Array(vec![])),
            ],
        );
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn primitive_extension_prefix_is_allowed() {
        let sd = make_sd("Patient", &["birthDate"]);
        // _birthDate is the primitive extension for birthDate
        let resource = make_resource("Patient", &[("_birthDate", Value::Object(IndexMap::new()))]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn polymorphic_field_with_known_base_is_allowed() {
        // snapshot has "value[x]"
        let mut sd = make_sd("Observation", &["value[x]"]);
        // Replace the auto-created entry with a proper [x] path
        sd.snapshot[1].path = Arc::from("Observation.value[x]");
        let resource = make_resource(
            "Observation",
            &[("valueString", Value::Str(Arc::from("hello")))],
        );
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn multiple_unknown_fields_each_produce_an_error() {
        let sd = make_sd("Patient", &[]);
        let resource = make_resource(
            "Patient",
            &[("foo", Value::Bool(true)), ("bar", Value::Bool(false))],
        );
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.code.as_ref() == "STRUCTURE_001")
        );
    }
}
