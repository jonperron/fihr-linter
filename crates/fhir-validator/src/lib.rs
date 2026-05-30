#![forbid(unsafe_code)]
//! FHIR R5 resource validator.
//!
//! Applies validation layers in order: structural, cardinality, type,
//! terminology, invariants, references, and profile checks.
//! Emits `Diagnostic` values for every detected issue.

mod cardinality;
mod diagnostic;
mod invariants;
mod structural;
mod terminology;
mod type_check;

pub use diagnostic::{Diagnostic, Severity};

use fhir_definitions::Registry;
use fhir_parser::Resource;

/// Validate a parsed FHIR resource against the definitions in the registry.
///
/// Applies validation layers in order and returns all findings.
///
/// Returns a single `STRUCTURE_000` error when the resource type is not known
/// to the registry.
pub fn validate(resource: &Resource, registry: &Registry) -> Vec<Diagnostic> {
    let resource_type = resource.resource_type.as_ref();
    let url = format!("http://hl7.org/fhir/StructureDefinition/{resource_type}");

    let Some(sd) = registry.structure_definition(&url) else {
        return vec![Diagnostic::error(
            "STRUCTURE_000",
            format!("Unknown resource type '{resource_type}'"),
            resource_type.to_owned(),
        )];
    };

    let mut diagnostics = Vec::new();

    structural::validate(resource, sd, &mut diagnostics);
    cardinality::validate(resource, sd, &mut diagnostics);
    type_check::validate(resource, sd, &mut diagnostics);
    terminology::validate(resource, sd, registry, &mut diagnostics);
    invariants::validate(resource, sd, &mut diagnostics);

    diagnostics
}
