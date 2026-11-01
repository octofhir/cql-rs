# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
