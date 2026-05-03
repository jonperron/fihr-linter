//! Python bindings for the FHIR linter and validator.
//!
//! Built with PyO3. Phase 8 will expose `validate()` and `lint()` functions.
use pyo3::prelude::*;

#[pymodule]
fn fhir_python(_py: Python<'_>, _m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
