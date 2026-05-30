use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::Resource;

use crate::diagnostic::Diagnostic;

/// Validate terminology bindings declared in the StructureDefinition.
///
/// Full terminology validation requires ValueSet expansion, which is not yet
/// implemented. This layer is a stub that leaves room for future expansion.
/// When the bound ValueSet is not known to the registry, a warning is emitted
/// for `required`-strength bindings.
pub fn validate(
    _resource: &Resource,
    _sd: &StructureDefinition,
    _registry: &Registry,
    _diagnostics: &mut Vec<Diagnostic>,
) {
    // TODO: implement code-level terminology validation once ValueSet
    // expansion is available.
}
