//! Content defined or held elsewhere: `contentReference` targets (PLAN 11.6.3-V6) and the
//! resources embedded in an element typed `Resource`, which the validator checks against
//! their own type.

use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
use fhir_fhirpath::Value as FhirPathValue;
use fhir_parser::{Node, Resource, Value};
use std::sync::Arc;

use crate::diagnostic::Diagnostic;

/// The ids of a resource's `contained` resources, shared by every contained resource
/// (one allocation, not one copy per contained resource).
pub(crate) type ContainedIds = Arc<[Arc<str>]>;

/// A resource inside another one (`contained`, `Bundle.entry.resource`,
/// `Parameters.parameter.resource`), to validate against its own type.
pub(crate) struct Embedded<'a> {
    pub resource: Resource,
    /// Its FHIRPath value, as converted with its container.
    pub typed: FhirPathValue,
    pub sd: &'a StructureDefinition,
    pub path: String,
    /// For a contained resource, the `contained` ids of its container: `#id` resolves there
    /// and `#` names the container (references.html, ref-1). `None` for a resource that is
    /// its own root (a Bundle entry), which resolves `#id` against its own `contained`.
    pub container_ids: Option<ContainedIds>,
    /// For a contained resource, its container's FHIRPath value: `%rootResource`.
    pub container: Option<FhirPathValue>,
}

/// The concrete resource StructureDefinition of `resource_type`, or the `STRUCTURE_000`
/// message: only a definition with `kind = resource` and `abstract = false` has instances.
pub(crate) fn concrete_resource<'a>(
    registry: &'a Registry,
    resource_type: &str,
) -> Result<&'a StructureDefinition, String> {
    let url = format!("http://hl7.org/fhir/StructureDefinition/{resource_type}");
    let definition = registry.structure_definition(&url);
    definition
        .filter(|sd| sd.kind.as_ref() == "resource" && !sd.is_abstract)
        .ok_or_else(|| match definition {
            Some(_) => format!("'{resource_type}' is not a concrete resource type"),
            None => format!("Unknown resource type '{resource_type}'"),
        })
}

/// The element whose content rules apply to a `contentReference` element of `sd`: a local
/// `Questionnaire.item`, or `http://.../StructureDefinition/X#X.path` in another definition.
pub(crate) fn content_target<'a>(
    registry: &'a Registry,
    sd: &'a StructureDefinition,
    elem: &ElementDefinition,
) -> Option<(&'a StructureDefinition, &'a ElementDefinition)> {
    let reference = elem.content_reference.as_deref()?;
    let (target_sd, path) = match reference.split_once('#') {
        Some((url, path)) => (registry.structure_definition(url)?, path),
        None => (sd, reference),
    };
    Some((target_sd, target_sd.element(path)?))
}

/// The element, definition and type that give the content rules of `elem` (of `sd`, with
/// type `ty` from `resolve::resolve`): itself, or for a typeless contentReference element
/// every rule of its target (V6). `None` only for an unresolved reference, which no R5
/// definition has: all 153 resolve to a typed element.
pub(crate) fn typed_element<'a>(
    registry: &'a Registry,
    sd: &'a StructureDefinition,
    elem: &'a ElementDefinition,
    ty: Option<Arc<str>>,
) -> Option<(&'a ElementDefinition, &'a StructureDefinition, Arc<str>)> {
    match ty {
        Some(ty) => Some((elem, sd, ty)),
        None => {
            let (sd, target) = content_target(registry, sd, elem)?;
            Some((target, sd, Arc::clone(target.types.first()?)))
        }
    }
}

/// `Registry::child_element`, following `contentReference` elements to their target, so
/// FHIRPath values below `Questionnaire.item.item` are typed too.
pub(crate) fn child_element(
    registry: &Registry,
    parent: &str,
    name: &str,
) -> Option<(Arc<str>, Arc<str>)> {
    registry.child_element(parent, name).or_else(|| {
        let type_name = parent.split('.').next()?;
        let sd = registry.structure_definition(&format!(
            "http://hl7.org/fhir/StructureDefinition/{type_name}"
        ))?;
        let (_, target) = content_target(registry, sd, sd.element(&format!("{parent}.{name}"))?)?;
        let ty = target.types.first()?;
        let definition = match ty.as_ref() {
            "BackboneElement" | "Element" => Arc::clone(&target.path),
            _ => Arc::clone(ty),
        };
        Some((definition, Arc::clone(ty)))
    })
}

/// Whether an element of type `ty` holds a whole resource (`Resource` in `contained`,
/// `Bundle.entry.resource`, `Parameters.parameter.resource`), typed by its own
/// `resourceType` rather than by the element.
pub(crate) fn holds_resource(ty: &str, registry: &Registry) -> bool {
    registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{ty}"))
        .is_some_and(|t| t.kind.as_ref() == "resource")
}

/// Queue for validation each resource that `node` (an element typed `Resource`, instance
/// `path`) holds. `typed` is its FHIRPath value as converted with its parent, when known;
/// `container` is set for `contained`: the ids and the value of the container (see
/// [`Embedded::container_ids`] and [`Embedded::container`]).
pub(crate) fn queue<'a>(
    node: &Node,
    typed: Option<&[FhirPathValue]>,
    container: Option<(&ContainedIds, &FhirPathValue)>,
    registry: &'a Registry,
    path: &str,
    queue: &mut Vec<Embedded<'a>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let items: Vec<(&Node, String)> = match &node.value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| (item, format!("{path}[{i}]")))
            .collect(),
        _ => vec![(node, path.to_owned())],
    };
    // An object converts to exactly one value, so objects align item by item.
    let aligned = typed.filter(|typed| {
        typed.len() == items.len()
            && items
                .iter()
                .all(|(n, _)| matches!(n.value, Value::Object(_)))
    });
    for (i, (item, item_path)) in items.into_iter().enumerate() {
        let error = |code, message: String| {
            Diagnostic::error(code, message, item_path.clone()).with_location(item.span)
        };
        let fields = match &item.value {
            Value::Object(fields) => fields,
            Value::Null => continue,
            _ => {
                let message = format!("Expected Resource object at '{item_path}'");
                diagnostics.push(error("TYPE_001", message));
                continue;
            }
        };
        let sd = match fields.get("resourceType").map(|n| &n.value) {
            Some(Value::Str(resource_type)) => {
                concrete_resource(registry, resource_type).map(|sd| (sd, resource_type))
            }
            _ => Err(format!(
                "Resource at '{item_path}' has no string resourceType"
            )),
        };
        let (sd, resource_type) = match sd {
            Ok(found) => found,
            Err(message) => {
                diagnostics.push(error("STRUCTURE_000", message));
                continue;
            }
        };
        // The same split as the JSON parser makes for the root: `resourceType` and a string
        // `id` leave `fields` (any other `id` stays, for the type check to report).
        let mut fields = fields.clone();
        fields.shift_remove("resourceType");
        let id = match fields.get("id").map(|n| &n.value) {
            Some(Value::Str(id)) => Some(Arc::clone(id)),
            _ => None,
        };
        if id.is_some() {
            fields.shift_remove("id");
        }
        let typed = match aligned {
            Some(typed) => typed[i].clone(),
            None => fhir_fhirpath::node_to_typed_value(
                Some(&item.value),
                None,
                Some(&(Arc::clone(resource_type), Arc::clone(resource_type))),
                &|parent, name| child_element(registry, parent, name),
            )
            .pop()
            .unwrap_or_else(|| FhirPathValue::Object(Arc::default())),
        };
        queue.push(Embedded {
            resource: Resource {
                from_xml: false,
                resource_type: Arc::clone(resource_type),
                id,
                fields,
                trimmed_attributes: Vec::new(),
                schema_instance: Vec::new(),
            },
            typed,
            sd,
            path: item_path,
            // A resource contained in a contained one (dom-2) still resolves against the root.
            container_ids: container.map(|(ids, _)| Arc::clone(ids)),
            container: container.map(|(_, value)| value.clone()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn registry() -> Result<Registry, Box<dyn std::error::Error>> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Ok(Registry::from_definitions_dir(dir)?)
    }

    #[test]
    fn content_target_resolves_local_and_absolute_references() -> TestResult {
        let registry = registry()?;
        let sd = |url: &str| {
            registry
                .structure_definition(url)
                .ok_or("definition not loaded")
        };
        let questionnaire = sd("http://hl7.org/fhir/StructureDefinition/Questionnaire")?;
        let local = questionnaire
            .element("Questionnaire.item.item")
            .ok_or("no element")?;
        let (target_sd, target) =
            content_target(&registry, questionnaire, local).ok_or("local unresolved")?;
        assert_eq!(target_sd.url, questionnaire.url);
        assert_eq!(target.path.as_ref(), "Questionnaire.item");

        let vitalsigns = sd("http://hl7.org/fhir/StructureDefinition/vitalsigns")?;
        let absolute = vitalsigns
            .element("Observation.component.referenceRange")
            .ok_or("no element")?;
        let (target_sd, target) =
            content_target(&registry, vitalsigns, absolute).ok_or("absolute unresolved")?;
        assert_eq!(
            target_sd.url.as_ref(),
            "http://hl7.org/fhir/StructureDefinition/Observation"
        );
        assert_eq!(target.path.as_ref(), "Observation.referenceRange");
        assert_eq!(
            target.types.first().map(|t| t.as_ref()),
            Some("BackboneElement")
        );
        Ok(())
    }

    #[test]
    fn contained_resources_share_one_id_list() -> TestResult {
        let registry = registry()?;
        let resource = fhir_parser::parse_json(
            r#"{"resourceType":"Patient","contained":[{"resourceType":"Patient","id":"a"},
            {"resourceType":"Patient","id":"b"}]}"#,
        )?;
        let node = resource.fields.get("contained").ok_or("no contained")?;
        let ids: ContainedIds = crate::references::contained_ids(&resource).into();
        let container = fhir_fhirpath::resource_to_value(&resource);
        let (mut queued, mut found) = (Vec::new(), Vec::new());
        queue(
            node,
            None,
            Some((&ids, &container)),
            &registry,
            "Patient.contained",
            &mut queued,
            &mut found,
        );
        assert_eq!(queued.len(), 2, "{found:?}");
        for embedded in &queued {
            let shared = embedded.container_ids.as_ref().ok_or("not contained")?;
            assert!(Arc::ptr_eq(shared, &ids));
        }
        Ok(())
    }

    #[test]
    fn child_element_follows_content_references() -> TestResult {
        let registry = registry()?;
        assert_eq!(
            child_element(&registry, "Questionnaire.item", "item"),
            Some((
                Arc::from("Questionnaire.item"),
                Arc::from("BackboneElement")
            )),
        );
        assert_eq!(
            child_element(&registry, "Questionnaire.item", "required"),
            Some((Arc::from("boolean"), Arc::from("boolean"))),
        );
        Ok(())
    }
}
