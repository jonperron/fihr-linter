---
description: "Use when writing Rust code. Covers naming conventions, error handling, and other Rust-specific guidelines."
applyTo: "**/*.rs"
---
# Coding style

- Prefer block indent over visual indent
- In comma-separated lists of any kind, use a trailing comma when followed by a newline
- Separate items and statements by either zero or one blank lines (i.e., one or two newlines)
- Do not include trailing whitespace on the end of any line. This includes blank lines, comment lines, code lines, and string literals.

## Indentation and line width

- Use spaces, not tabs.
- Each level of indentation must be 4 spaces (that is, all indentation outside of string literals and comments must be a multiple of 4).
- The maximum width for a line is 100 characters.

# Code organization

- Use `mod` to organize code into modules and submodules.
- Each module should be in its own file, and the file name should match the module name.
- Use `use` statements to import items from other modules, and group them at the top of the file.
- Many small files over few large files
- High cohesion, low coupling
- 200-400 lines typical, 800 max per file
- No circular import

# Code style

- No emojis in code, comments, or documentation
- Immutability always — never mutate objects or arrays
- Proper error handling with try/catch
- Use descriptive variable and function names that convey their purpose and intent
- Avoid abbreviations and acronyms unless they are widely known and unambiguous
- Use snake_case for variable and function names, and PascalCase for type names (structs, enums, traits)

# Rust-specific guidelines

- Use `thiserror` for error types; never use `anyhow` in library crates.
- Prefer `Arc<str>` over `String` for interned FHIR URLs and type names.
- `Diagnostic` is the single output type across all crates — do not introduce alternatives.
- `serde_json::Value`-based generic tree in `fhir-definitions`; no per-resource Serde structs.
- Use `rstest` for parameterised tests, `proptest` for property-based tests.
- Use `?` to propagate `None` for required fields rather than `unwrap_or("")`/`unwrap_or_default()`. Defaulting to an empty value hides malformed inputs and can produce model instances that violate invariants.