#![forbid(unsafe_code)]
//! FHIR R5 definitions registry.
//!
//! Loads and indexes StructureDefinitions, ValueSets, and CodeSystems from
//! the bundled FHIR R5 artifact files.

mod loader;
mod model;
mod registry;

pub use model::{CodeSystem, ElementDefinition, StructureDefinition, ValueSet};
pub use registry::Registry;

use thiserror::Error;

/// Errors that can occur while loading FHIR definitions.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error in {file}: {source}")]
    Json {
        file: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("malformed FHIR bundle in {file}: {message}")]
    MalformedBundle { file: String, message: String },
}

pub type Result<T> = std::result::Result<T, Error>;
