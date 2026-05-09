use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::Result;
use crate::loader;
use crate::model::{CodeSystem, StructureDefinition, ValueSet};

/// Indexed registry of FHIR R5 definitions, built once and shared via `Arc`.
///
/// Keyed by canonical URL for all entry types.
pub struct Registry {
    structure_definitions: HashMap<Arc<str>, StructureDefinition>,
    value_sets: HashMap<Arc<str>, ValueSet>,
    code_systems: HashMap<Arc<str>, CodeSystem>,
}

impl Registry {
    /// Load all FHIR R5 definitions from a directory that contains the standard
    /// FHIR definition JSON files:
    ///
    /// - `profiles-resources.json`
    /// - `profiles-types.json`
    /// - `valuesets.json`
    pub fn from_definitions_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();

        let mut structure_definitions = HashMap::new();
        let mut value_sets = HashMap::new();
        let mut code_systems = HashMap::new();

        for file in &["profiles-resources.json", "profiles-types.json"] {
            let path = dir.join(file);
            loader::load_bundle(
                &path,
                &mut structure_definitions,
                &mut value_sets,
                &mut code_systems,
            )?;
        }

        let valuesets_path = dir.join("valuesets.json");
        loader::load_bundle(
            &valuesets_path,
            &mut structure_definitions,
            &mut value_sets,
            &mut code_systems,
        )?;

        Ok(Self {
            structure_definitions,
            value_sets,
            code_systems,
        })
    }

    /// Look up a `StructureDefinition` by its canonical URL.
    pub fn structure_definition(&self, url: &str) -> Option<&StructureDefinition> {
        self.structure_definitions.get(url)
    }

    /// Look up a `ValueSet` by its canonical URL.
    pub fn value_set(&self, url: &str) -> Option<&ValueSet> {
        self.value_sets.get(url)
    }

    /// Look up a `CodeSystem` by its canonical URL.
    pub fn code_system(&self, url: &str) -> Option<&CodeSystem> {
        self.code_systems.get(url)
    }

    /// Total number of indexed `StructureDefinition` entries.
    pub fn structure_definition_count(&self) -> usize {
        self.structure_definitions.len()
    }

    /// Total number of indexed `ValueSet` entries.
    pub fn value_set_count(&self) -> usize {
        self.value_sets.len()
    }

    /// Total number of indexed `CodeSystem` entries.
    pub fn code_system_count(&self) -> usize {
        self.code_systems.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn definitions_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
    }

    #[test]
    fn registry_structure_definition_lookup_is_case_sensitive() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        assert!(
            registry
                .structure_definition("http://hl7.org/fhir/StructureDefinition/patient")
                .is_none()
        );
    }

    #[test]
    fn registry_loads_patient_structure_definition() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .expect("Patient StructureDefinition should be present");
        assert_eq!(sd.name.as_ref(), "Patient");
        assert_eq!(sd.kind.as_ref(), "resource");
        assert!(!sd.snapshot.is_empty(), "snapshot should have elements");
    }

    #[test]
    fn registry_resolves_valueset_by_url() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let vs = registry
            .value_set("http://hl7.org/fhir/ValueSet/administrative-gender")
            .expect("administrative-gender ValueSet should be present");
        assert_eq!(
            vs.url.as_ref(),
            "http://hl7.org/fhir/ValueSet/administrative-gender"
        );
    }

    #[test]
    fn registry_loads_multiple_structure_definitions() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        assert!(
            registry.structure_definition_count() > 100,
            "should load more than 100 StructureDefinitions, got {}",
            registry.structure_definition_count()
        );
    }

    #[test]
    fn registry_loads_code_systems() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        assert!(
            registry.code_system_count() > 0,
            "should load at least one CodeSystem"
        );
    }

    #[test]
    fn patient_snapshot_contains_identifier_element() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        let identifier = sd
            .snapshot
            .iter()
            .find(|e| e.path.as_ref() == "Patient.identifier");
        assert!(
            identifier.is_some(),
            "snapshot should contain Patient.identifier"
        );
        let identifier = identifier.unwrap();
        assert_eq!(identifier.min, 0);
        assert_eq!(identifier.max.as_ref(), "*");
    }

    #[test]
    fn registry_returns_none_for_unknown_url() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        assert!(
            registry
                .structure_definition("http://example.com/unknown")
                .is_none()
        );
        assert!(registry.value_set("http://example.com/unknown").is_none());
        assert!(registry.code_system("http://example.com/unknown").is_none());
    }

    #[test]
    fn structure_definition_has_base_definition() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        assert_eq!(
            sd.base_definition.as_deref(),
            Some("http://hl7.org/fhir/StructureDefinition/DomainResource")
        );
    }

    #[test]
    fn registry_value_set_count_is_positive() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        assert!(
            registry.value_set_count() > 0,
            "should load at least one ValueSet, got {}",
            registry.value_set_count()
        );
    }

    #[test]
    fn registry_code_system_has_url_and_name() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let cs = registry
            .code_system("http://hl7.org/fhir/administrative-gender")
            .expect("administrative-gender CodeSystem should be present");
        assert_eq!(cs.url.as_ref(), "http://hl7.org/fhir/administrative-gender");
        assert!(!cs.name.is_empty(), "CodeSystem name should not be empty");
    }

    #[test]
    fn registry_value_set_has_name() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let vs = registry
            .value_set("http://hl7.org/fhir/ValueSet/administrative-gender")
            .expect("administrative-gender ValueSet should be present");
        assert!(!vs.name.is_empty(), "ValueSet name should not be empty");
    }

    #[test]
    fn element_definition_types_are_populated() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        let id_elem = sd
            .snapshot
            .iter()
            .find(|e| e.path.as_ref() == "Patient.identifier")
            .expect("Patient.identifier element should exist");
        assert!(
            !id_elem.types.is_empty(),
            "Patient.identifier should have at least one type"
        );
        assert!(
            id_elem.types.iter().any(|t| t.as_ref() == "Identifier"),
            "Patient.identifier should have type 'Identifier', got {:?}",
            id_elem.types
        );
    }

    #[test]
    fn structure_definition_is_abstract_for_resource_type() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Resource")
            .expect("Resource StructureDefinition should be present");
        assert!(
            sd.is_abstract,
            "Resource StructureDefinition should be abstract"
        );
    }

    #[test]
    fn structure_definition_is_not_abstract_for_patient() {
        let registry = Registry::from_definitions_dir(definitions_dir()).unwrap();
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        assert!(
            !sd.is_abstract,
            "Patient StructureDefinition should not be abstract"
        );
    }

    #[test]
    fn registry_from_nonexistent_dir_returns_error() {
        let result = Registry::from_definitions_dir("/nonexistent/path/that/does/not/exist");
        assert!(
            result.is_err(),
            "loading from a nonexistent dir should fail"
        );
    }
}
