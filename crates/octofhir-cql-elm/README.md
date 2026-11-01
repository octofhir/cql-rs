# octofhir-cql-elm

The [HL7 ELM](https://cql.hl7.org/elm.html) (Expression Logical Model) for Rust, plus the CQL-to-ELM
translator of [cql-rs](https://github.com/octofhir/cql-rs).

- `model`: ELM libraries, definitions and every expression the evaluator runs, serde
  (de)serializable
- `AstToElmConverter`: translates a parsed CQL library. Identifiers become `AliasRef`, `QueryLetRef`,
  `OperandRef`, `ExpressionRef`, `ParameterRef`, `ValueSetRef`, `CodeSystemRef`, `CodeRef` or
  `ConceptRef`; library-qualified names and calls keep their include alias; `context Patient` adds the
  implicit `Patient` definition; function definitions carry their operands; FHIR retrieves get the
  qualified type name and the resource's primary code path
- `diagnostics()`: unresolved identifiers, unknown include aliases, unknown members of included
  libraries, duplicate definitions
- ELM JSON in the standard `{ "library": ... }` envelope (`Library::to_json`, `Library::from_json`)

## Translate a library

```rust
use octofhir_cql_elm::{AstToElmConverter, Library};

let common = octofhir_cql_parser::parse(r#"library Common define "Answer": 42"#)?;
let main = octofhir_cql_parser::parse(
    r#"library Main
       include Common called C
       define "From Common": C."Answer"
       define "Broken": "Missing" + 1"#,
)?;

let mut converter = AstToElmConverter::new();
// Optional: with the included library's declarations known, qualified names are
// classified precisely and unknown ones are reported.
converter.add_included_library("Common", &common);
let elm = converter.convert_library(&main);

let diagnostics = converter.diagnostics();
assert_eq!(diagnostics.len(), 1);
assert!(diagnostics[0].to_string().contains("Missing"));

let json: serde_json::Value = serde_json::from_str(&elm.to_json()?)?;
let reference = &json["library"]["statements"]["def"][0]["expression"];
assert_eq!(reference["type"], "ExpressionRef");
assert_eq!(reference["libraryName"], "C");

let round_trip = Library::from_json(&elm.to_json()?)?;
assert_eq!(round_trip.identifier.id, "Main");
# Ok::<(), Box<dyn std::error::Error>>(())
```

`with_source(text)` gives diagnostics line and column locations (of the enclosing definition).

## What the translator does not do

There is no static type checking: ELM output carries no `resultTypeSpecifier`s on expressions, no
implicit conversions are inserted, and overloads are not resolved statically. The evaluator in
[`octofhir-cql-eval`](https://crates.io/crates/octofhir-cql-eval) resolves function overloads at run
time. ELM XML is not supported (JSON only).

## License

MIT OR Apache-2.0
