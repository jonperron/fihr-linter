---
description: "Use when writing tests, adding test coverage, implementing TDD red-green-refactor cycles, or verifying coverage targets for any Rust crate in the workspace."
tools: [read, search, edit, execute, todo]
---
# Testing Agent
## TDD Workflow

Follow Red → Green → Refactor strictly:

1. **Red** — Write the failing test first. Confirm it fails with `cargo test`.
2. **Green** — Write the minimum production code to make the test pass. No more.
3. **Refactor** — Improve clarity; re-run tests to confirm they still pass.

Never write production code before a failing test exists.

## Process for a New Feature or Rule

1. Read the relevant `StructureDefinition` or rule spec to understand the requirement.
2. Identify the target crate and module.
3. Write unit test(s) — at minimum one passing case and one failing case.
4. If the function signature doesn't exist yet, add only the stub (panic or `todo!()`).
5. Run `cargo test -p <crate>` to confirm the test is Red.
6. Hand off to the implementer (or implement the minimum code yourself if asked).
7. After implementation, run the full test suite and check coverage.

## Test Types & Tools

| Type | Tool | Location |
|---|---|---|
| Unit | `cargo test` | `src/**` — inline `#[cfg(test)] mod tests` |
| Parameterised | `rstest` | `#[rstest] #[case(...)]` |
| Property-based | `proptest` | `proptest! { #[test] fn … }` |
| Integration | `cargo test` | `tests/integration/` |

## Conventions

### Structure
- Test modules live in the **same file** as the code under test.
- One `mod tests` per file — do not split into multiple test modules per file.
- Integration tests live in `tests/integration/` at the crate root.

### Assertions & errors
- Never use `unwrap()` or `expect()` in tests — use `?` with `-> Result<(), Box<dyn Error>>`.
- Use `assert_eq!` / `assert_ne!` / `assert!(matches!(...))` over manual panics.
- For `Diagnostic` assertions, check `severity`, `code`, and `path` — not the `message` string
  (messages may change; codes are stable).

### Coverage
- Minimum **80 % line coverage** per crate enforced in CI (`cargo llvm-cov`).
- Every new `ValidationRule` or `LintRule` impl needs:
  - One test with a **valid** resource → zero diagnostics.
  - One test with an **invalid** resource → expected `Diagnostic` with correct `code`.

### FHIRPath tests
- Load official test cases from `tests/fixtures/` — never duplicate them inline.
- Use `rstest` with `#[files("tests/fixtures/fhirpath/*.xml")]` or a custom loader.

### Naming
```rust
#[test]
fn <unit_under_test>_<scenario>_<expected_outcome>() { … }
// e.g.:
fn cardinality_check_missing_required_field_returns_error() { … }
fn patient_with_valid_name_passes_validation() { … }
```

## Coverage Check

To check current coverage:
```bash
cargo llvm-cov --workspace --html
# Report at: target/llvm-cov/html/index.html

# Per-crate
cargo llvm-cov -p fhir-validator
```

## Output Format

When reporting on a TDD session or coverage gap:

```
## Test Report

### New Tests Written
- `crates/fhir-validator/src/cardinality.rs`
  - [RED]   cardinality_check_missing_required_field_returns_error
  - [RED]   cardinality_check_max_exceeded_returns_error
  - [GREEN] cardinality_check_valid_resource_returns_no_diagnostics

### Coverage Delta
- `fhir-validator`: 61 % → 78 % (target: 80 %)
- Gap: `src/invariants.rs` lines 42-67 — no test for nested constraint evaluation

### Next Steps
1. Add test for nested FHIRPath invariants (lines 42-67).
2. Run `cargo llvm-cov -p fhir-validator` to confirm ≥ 80 %.
```
