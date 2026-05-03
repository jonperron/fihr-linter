//! TypeScript/WASM bindings for the FHIR linter and validator.
//!
//! Built with wasm-bindgen. Phase 9 will expose validate() and lint() APIs.
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
