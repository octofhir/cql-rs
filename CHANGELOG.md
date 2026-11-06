# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.5] - 2026-11-06

### Added
- Debug trace: `EvaluationContext::record_trace` records `enter`, `step` and `exit` events for
  definitions, function calls and `Message()`, in the format of the FHIR Mapping Language IG.
- `ExpressionDef.locator`: where a definition is in the CQL text. Not serialized.

### Fixed
- `Message()` failed with `UndefinedFunction`.

## [0.2.4] - 2026-11-06

### Fixed
- `included in` (`during`) and `includes` accept a point: for a point they mean `in` and
  `contains`, with precision honoured. Measure logic filtering by date failed on this.

## [0.2.3] - 2026-11-04

### Changed
- Use UCUM 0.9.3 platform math for quantity comparison without enabling unused
  fuzzy/regex unit search or FHIR/serialization integration. The numeric backend
  remains the same platform float implementation as before.

## [0.2.2] - 2026-11-04

### Changed

- Remove unused dependencies from the AST, types, ELM, model and evaluator crates.
- Stop enabling the FHIR model HTTP client in the evaluator and model crates; use
  the lightweight model 0.1.19 abstractions instead.
- Limit the evaluator Tokio dependency to runtime support; test macros and example
  error handling stay in development dependencies.
- Keep the existing CLI defaults and the `colored` compatibility feature.
- Update the private Winnow parser to 1.0.4, the CLI line editor to Rustyline 19,
  and test fixtures to rstest 0.28.
- Document the library-only dependency configuration and refresh compatible lockfile
  dependencies.

## [0.2.0] - 2026-11-01

### Added

- Libraries: `include ... called`, `codesystem`, `valueset`, `code` and `concept` declarations,
  typed parameters with defaults, function definitions (fluent and `external` declarations),
  `define private`, statements in any order, per-definition contexts.
- Queries: `let`, `with` / `without ... such that`, `sort by` expressions, identifiers and
  qualified identifiers as sources; retrieves with code filters and value sets.
- Evaluation of user-defined functions and of included libraries; parameter defaults; real
  `Code` / `Concept` values; built-in FHIRHelpers conversions; `AgeIn*`, `AgeIn*At`,
  `CalculateAgeIn*`.
- `InMemoryFhirData`: a FHIR resource, Bundle, JSON array or NDJSON as data and terminology
  source; FHIR JSON to CQL value mapping.
- `CompiledLibrary` and `DirectoryLibrarySource` to compile a library with its includes.
- The `cql` CLI: `parse`, `translate`, `validate`, `eval` / `run`, `repl`.
- Prebuilt `cql` binaries on GitHub releases (Linux x64/arm64, macOS x64/arm64, Windows x64).

### Changed

- Identifiers translate to their ELM reference kinds (`ExpressionRef`, `ParameterRef`,
  `AliasRef`, `ValueSetRef`, ...) instead of a bare identifier reference.
- Quantities compare through UCUM 0.9 conversions, temperature offsets included.
- `is` / `as` handle list, interval, choice and tuple types; `as` returns null on a mismatch.
- Parse errors carry line and column; analysis mode collects every error.
- The CLI dependencies are behind the `cli` feature (default on).

### Removed

- The XML ELM serializer (JSON only) and the `octofhir-cql-cli` stub crate.

### Fixed

- The official cqframework/cql-tests pass 1693/1693 and gate CI.
- Async adapters no longer panic inside a Tokio runtime.

## [0.2.1] - 2026-11-01

### Security

- quick-xml 0.42 (RUSTSEC-2026-0194, RUSTSEC-2026-0195) and patched releases of the
  other dependencies; `cargo audit` is clean.
