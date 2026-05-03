#![forbid(unsafe_code)]
//! Configurable FHIR lint rules.
//!
//! Implements the `Rule` trait and a `RuleRegistry` for running user-configured
//! checks on top of the core validator output.
