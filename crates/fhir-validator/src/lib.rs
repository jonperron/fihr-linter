#![forbid(unsafe_code)]
//! FHIR R5 resource validator.
//!
//! Applies validation layers in order: structural, cardinality, type,
//! terminology, reference and element invariant checks (one recursive walk),
//! then the root's invariants and profile checks. Resources embedded in it
//! (contained, Bundle entries) go through the same layers at their own path.
//! Emits `Diagnostic` values for every detected issue.

mod cardinality;
mod diagnostic;
mod embedded;
mod fixed_pattern;
mod invariants;
mod nested;
mod profile;
mod references;
mod resolve;
mod slicing;
mod terminology;
mod type_check;
mod xml_shape;

pub use diagnostic::{Diagnostic, Severity};

use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::Resource;

/// Validate a parsed FHIR resource against the definitions in the registry.
///
/// Applies validation layers in order and returns all findings.
///
/// Returns a single `STRUCTURE_000` error when the resource type is not a
/// concrete resource known to the registry (abstract resources such as
/// `DomainResource` and datatypes are rejected).
pub fn validate(resource: &Resource, registry: &Registry) -> Vec<Diagnostic> {
    let resource_type = resource.resource_type.as_ref();
    let sd = match embedded::concrete_resource(registry, resource_type) {
        Ok(sd) => sd,
        Err(message) => {
            return vec![Diagnostic::error(
                "STRUCTURE_000",
                message,
                resource_type.to_owned(),
            )];
        }
    };

    // XML input is reshaped once, so every layer sees the JSON form (typed primitives, `_name`).
    // The reshape keeps XML grouping, so only JSON input has a meaningful array shape.
    let json = !resource.from_xml;
    let reshaped;
    let mut diagnostics = Vec::new();
    let resource = if resource.from_xml {
        let (shaped, xml_diagnostics) = xml_shape::to_json_shape(resource, sd, registry);
        reshaped = shaped;
        diagnostics = xml_diagnostics;
        &reshaped
    } else {
        resource
    };

    // The FHIRPath value of the resource, converted once for the walk and the root invariants.
    let root = fhir_fhirpath::resource_to_typed_value(resource, &|parent, name| {
        embedded::child_element(registry, parent, name)
    });
    let mut pending = validate_one(
        resource,
        &root,
        None,
        sd,
        registry,
        json,
        resource_type,
        None,
        &mut diagnostics,
    );
    // Embedded resources (contained, Bundle entries) in document order, each after its
    // container; their own embedded resources go next, so nesting needs no recursion.
    pending.reverse();
    while let Some(embedded) = pending.pop() {
        let mut inner = validate_one(
            &embedded.resource,
            &embedded.typed,
            embedded.container.as_ref(),
            embedded.sd,
            registry,
            json,
            &embedded.path,
            embedded.container_ids,
            &mut diagnostics,
        );
        inner.reverse();
        pending.append(&mut inner);
    }

    diagnostics
}

/// Every layer for one resource at instance `path`; returns the resources it embeds.
/// `container` is the FHIRPath value of a contained resource's container.
#[allow(clippy::too_many_arguments)] // the resource, its definition and its place
fn validate_one<'a>(
    resource: &Resource,
    root: &fhir_fhirpath::Value,
    container: Option<&fhir_fhirpath::Value>,
    sd: &StructureDefinition,
    registry: &'a Registry,
    json: bool,
    path: &str,
    container_ids: Option<embedded::ContainedIds>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<embedded::Embedded<'a>> {
    let is_subtype =
        |child: &str, ancestor: &str| registry.type_ancestors(child).any(|code| code == ancestor);
    // fhirpath.html: %rootResource is the container only for a contained resource.
    let environment = fhir_fhirpath::Environment {
        resource: Some(root),
        root_resource: Some(container.unwrap_or(root)),
        is_subtype: Some(&is_subtype),
    };
    let embedded = nested::validate(
        resource,
        root,
        &environment,
        sd,
        registry,
        json,
        path,
        container_ids,
        diagnostics,
    );
    invariants::validate(
        resource,
        sd,
        &|base, suffix| registry.is_choice_variant(base, suffix),
        root,
        &environment,
        path,
        diagnostics,
    );
    profile::validate(resource, sd, registry, &environment, path, diagnostics);
    embedded
}
