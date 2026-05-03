# FHIR Linter — Agent Guidelines

## Project Overview

Rust workspace implementing a FHIR R5 linter and validator, with Python (PyO3) and
TypeScript (WASM) bindings.

## Specifications

Feature specifications live in openspec/specs/. Each subfolder contains a spec.md describing a functional area (e.g., cardinality rules, FHIRPath functions) and a rules.

Always read the relevant spec before implementing or modifying a feature.

## Tech stack

- Rust for core logic and validation rules
- PyO3 for Python bindings
- wasm-bindgen for TypeScript bindings
- clap for CLI argument parsing

### FHIR definitions

- Official artifacts are fetched by `scripts/download-definitions.sh` (SHA-256 verified).
- Never hard-code FHIR R5 resource shapes; always derive from the `Registry`.
- The `Registry` is read-only at runtime; build it once and share via `Arc`.

## Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` (root) | Workspace definition, shared dependencies |
| `definitions/r5/` | Bundled FHIR R5 JSON artifacts |
| `tests/fixtures/` | Official FHIR test cases (submodule) |
| `scripts/download-definitions.sh` | Fetches & checksums FHIR R5 definitions |
| `fhir-linter.toml` | User-facing linter rule configuration |

## Architecture

```
crates/
  fhir-definitions/   # FHIR R5 StructureDefinitions / ValueSets registry
  fhir-parser/        # JSON + XML → typed AST with source spans
  fhir-fhirpath/      # FHIRPath 2.0 evaluator (required for invariants)
  fhir-validator/     # Structural, cardinality, type, terminology, invariant checks
  fhir-linter/        # Configurable lint rules (fhir-linter.toml)
  fhir-cli/           # CLI binary (clap), outputs text / JSON / SARIF
bindings/
  python/             # maturin + PyO3
  typescript/         # wasm-pack + wasm-bindgen
definitions/r5/       # Bundled FHIR R5 JSON artifacts (do not edit manually)
tests/fixtures/       # git submodule → github.com/FHIR/fhir-test-cases
```

## Coding Rules

Coding rules are defined in .github/instructions/ and loaded automatically by file pattern or on-demand:

- [Coding style](coding-style.instructions.md): Guidelines for code formatting, organization, and idiomatic patterns.
- [Testing](testing.instructions.md): Best practices for writing tests, TDD workflow, coverage targets, and test types.
- [Commit messages](commit-messages.instructions.md): Conventions for writing clear, consistent commit messages that enhance project history and collaboration.

## Subagents

When completing a technical task (new feature, bug fix, refactor), delegate to subagents for quality assurance:

- **Build Agent**: Use the build agent (build.agent.md) to ensure your code compiles and builds correctly.
- **Linting Agent**: Use the linting agent (linter.agent.md) to check for code style, formatting, clippy warnings, and adherence to project conventions.
- **Testing Agent**: Use the testing agent (test.agent.md) to write tests, implement TDD cycles, and verify coverage targets.
- **Security Agent**: Use the security agent (security.agent.md) to review code for security vulnerabilities, especially when handling user input or external data.
