use std::sync::Arc;

/// A lightweight StructureDefinition — only the fields the validator needs.
#[derive(Debug, Clone)]
pub struct StructureDefinition {
    pub url: Arc<str>,
    pub name: Arc<str>,
    pub kind: Arc<str>,
    pub is_abstract: bool,
    pub base_definition: Option<Arc<str>>,
    pub snapshot: Vec<ElementDefinition>,
}

/// A single element in a StructureDefinition snapshot.
#[derive(Debug, Clone)]
pub struct ElementDefinition {
    pub path: Arc<str>,
    pub min: u32,
    /// Either `"*"` or a non-negative integer represented as a string.
    pub max: Arc<str>,
    pub types: Vec<Arc<str>>,
}

/// A lightweight ValueSet (url + name only; compose details are looked up on demand).
#[derive(Debug, Clone)]
pub struct ValueSet {
    pub url: Arc<str>,
    pub name: Arc<str>,
}

/// A lightweight CodeSystem.
#[derive(Debug, Clone)]
pub struct CodeSystem {
    pub url: Arc<str>,
    pub name: Arc<str>,
}
