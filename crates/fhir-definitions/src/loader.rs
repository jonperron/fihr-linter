use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use crate::model::{CodeSystem, ElementDefinition, StructureDefinition, ValueSet};
use crate::{Error, Result};

/// Parse a FHIR Bundle JSON file and populate the given maps with any
/// `StructureDefinition`, `ValueSet`, or `CodeSystem` entries found.
pub fn load_bundle(
    path: &Path,
    structure_definitions: &mut HashMap<Arc<str>, StructureDefinition>,
    value_sets: &mut HashMap<Arc<str>, ValueSet>,
    code_systems: &mut HashMap<Arc<str>, CodeSystem>,
) -> Result<()> {
    let file_name = path.to_string_lossy().into_owned();
    let content = std::fs::read_to_string(path).map_err(Error::Io)?;
    let bundle: Value = serde_json::from_str(&content).map_err(|source| Error::Json {
        file: file_name.clone(),
        source,
    })?;

    let entries = bundle["entry"]
        .as_array()
        .ok_or_else(|| Error::MalformedBundle {
            file: file_name.clone(),
            message: "missing 'entry' array".to_string(),
        })?;

    for entry in entries {
        let resource = &entry["resource"];
        let resource_type = resource["resourceType"].as_str().unwrap_or("");

        match resource_type {
            "StructureDefinition" => {
                if let Some(sd) = parse_structure_definition(resource) {
                    structure_definitions.insert(Arc::clone(&sd.url), sd);
                }
            }
            "ValueSet" => {
                if let Some(vs) = parse_value_set(resource) {
                    value_sets.insert(Arc::clone(&vs.url), vs);
                }
            }
            "CodeSystem" => {
                if let Some(cs) = parse_code_system(resource) {
                    code_systems.insert(Arc::clone(&cs.url), cs);
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_structure_definition(resource: &Value) -> Option<StructureDefinition> {
    let url: Arc<str> = resource["url"].as_str()?.into();
    let name: Arc<str> = resource["name"].as_str()?.into();
    let kind: Arc<str> = resource["kind"].as_str()?.into();
    let is_abstract = resource["abstract"].as_bool().unwrap_or(false);
    let base_definition = resource["baseDefinition"].as_str().map(Arc::from);

    let snapshot = resource["snapshot"]["element"]
        .as_array()
        .map(|elems| elems.iter().filter_map(parse_element_definition).collect())
        .unwrap_or_default();

    Some(StructureDefinition {
        url,
        name,
        kind,
        is_abstract,
        base_definition,
        snapshot,
    })
}

fn parse_element_definition(elem: &Value) -> Option<ElementDefinition> {
    let path: Arc<str> = elem["path"].as_str()?.into();
    let min = u32::try_from(elem["min"].as_u64().unwrap_or(0)).unwrap_or(0);
    let max: Arc<str> = elem["max"].as_str().unwrap_or("*").into();
    let types = elem["type"]
        .as_array()
        .map(|type_list| {
            type_list
                .iter()
                .filter_map(|t| t["code"].as_str().map(Arc::from))
                .collect()
        })
        .unwrap_or_default();

    Some(ElementDefinition {
        path,
        min,
        max,
        types,
    })
}

fn parse_value_set(resource: &Value) -> Option<ValueSet> {
    let url: Arc<str> = resource["url"].as_str()?.into();
    let name: Arc<str> = resource["name"].as_str()?.into();
    Some(ValueSet { url, name })
}

fn parse_code_system(resource: &Value) -> Option<CodeSystem> {
    let url: Arc<str> = resource["url"].as_str()?.into();
    let name: Arc<str> = resource["name"].as_str()?.into();
    Some(CodeSystem { url, name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn write_temp(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_bundle_missing_file_returns_io_error() {
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        let result = load_bundle(
            std::path::Path::new("/nonexistent/path/file.json"),
            &mut sds,
            &mut vss,
            &mut css,
        );
        assert!(matches!(result, Err(crate::Error::Io(_))));
    }

    #[test]
    fn load_bundle_invalid_json_returns_json_error() {
        let path = write_temp("fhir_def_test_invalid_json.json", "not valid json {{{{");
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        let result = load_bundle(&path, &mut sds, &mut vss, &mut css);
        assert!(matches!(result, Err(crate::Error::Json { .. })));
    }

    #[test]
    fn load_bundle_missing_entry_array_returns_malformed_bundle_error() {
        let path = write_temp(
            "fhir_def_test_no_entry.json",
            r#"{"resourceType": "Bundle"}"#,
        );
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        let result = load_bundle(&path, &mut sds, &mut vss, &mut css);
        assert!(matches!(result, Err(crate::Error::MalformedBundle { .. })));
    }

    #[test]
    fn load_bundle_skips_unknown_resource_types() {
        let content = r#"{"entry": [{"resource": {"resourceType": "Observation", "url": "http://example.com"}}]}"#;
        let path = write_temp("fhir_def_test_unknown_type.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(sds.is_empty());
        assert!(vss.is_empty());
        assert!(css.is_empty());
    }

    #[test]
    fn load_bundle_skips_structure_definition_with_missing_url() {
        let content = r#"{"entry": [{"resource": {"resourceType": "StructureDefinition", "name": "Foo", "kind": "resource"}}]}"#;
        let path = write_temp("fhir_def_test_sd_no_url.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(sds.is_empty());
    }

    #[test]
    fn load_bundle_parses_value_set_and_code_system() {
        let content = r#"{
            "entry": [
                {"resource": {"resourceType": "ValueSet", "url": "http://example.com/vs", "name": "TestVS"}},
                {"resource": {"resourceType": "CodeSystem", "url": "http://example.com/cs", "name": "TestCS"}}
            ]
        }"#;
        let path = write_temp("fhir_def_test_vs_cs.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(vss.contains_key("http://example.com/vs"));
        assert_eq!(vss["http://example.com/vs"].name.as_ref(), "TestVS");
        assert!(css.contains_key("http://example.com/cs"));
        assert_eq!(css["http://example.com/cs"].name.as_ref(), "TestCS");
    }

    #[test]
    fn load_bundle_parses_structure_definition_with_snapshot() {
        let content = r#"{
            "entry": [{
                "resource": {
                    "resourceType": "StructureDefinition",
                    "url": "http://example.com/sd",
                    "name": "TestSD",
                    "kind": "resource",
                    "abstract": true,
                    "snapshot": {
                        "element": [
                            {"path": "TestSD.id", "min": 0, "max": "1", "type": [{"code": "id"}]}
                        ]
                    }
                }
            }]
        }"#;
        let path = write_temp("fhir_def_test_sd_snapshot.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        let sd = sds
            .get("http://example.com/sd")
            .expect("SD should be parsed");
        assert_eq!(sd.name.as_ref(), "TestSD");
        assert!(sd.is_abstract);
        assert_eq!(sd.snapshot.len(), 1);
        assert_eq!(sd.snapshot[0].path.as_ref(), "TestSD.id");
        assert_eq!(sd.snapshot[0].min, 0);
        assert_eq!(sd.snapshot[0].max.as_ref(), "1");
        assert_eq!(sd.snapshot[0].types[0].as_ref(), "id");
    }

    #[test]
    fn load_bundle_element_definition_without_path_is_skipped() {
        let content = r#"{
            "entry": [{
                "resource": {
                    "resourceType": "StructureDefinition",
                    "url": "http://example.com/sd2",
                    "name": "TestSD2",
                    "kind": "resource",
                    "snapshot": {
                        "element": [
                            {"min": 0, "max": "1"},
                            {"path": "TestSD2.id", "min": 0, "max": "1"}
                        ]
                    }
                }
            }]
        }"#;
        let path = write_temp("fhir_def_test_sd_elem_no_path.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        let sd = sds
            .get("http://example.com/sd2")
            .expect("SD should be parsed");
        assert_eq!(
            sd.snapshot.len(),
            1,
            "element without path should be skipped"
        );
        assert_eq!(sd.snapshot[0].path.as_ref(), "TestSD2.id");
    }

    #[test]
    fn load_bundle_skips_value_set_with_missing_name() {
        let content = r#"{"entry": [{"resource": {"resourceType": "ValueSet", "url": "http://example.com/vs-no-name"}}]}"#;
        let path = write_temp("fhir_def_test_vs_no_name.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(vss.is_empty(), "ValueSet without name should be skipped");
    }

    #[test]
    fn load_bundle_skips_code_system_with_missing_name() {
        let content = r#"{"entry": [{"resource": {"resourceType": "CodeSystem", "url": "http://example.com/cs-no-name"}}]}"#;
        let path = write_temp("fhir_def_test_cs_no_name.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(css.is_empty(), "CodeSystem without name should be skipped");
    }
}
