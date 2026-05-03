//! FHIR R5 resource validator.
//!
//! Applies validation layers in order: structural, cardinality, type,
//! terminology, invariants, references, and profile checks.
//! Emits `Diagnostic` values for every detected issue.
