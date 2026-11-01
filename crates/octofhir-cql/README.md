# octofhir-cql

Clinical Quality Language (CQL) 1.5 for Rust: parse, translate to ELM and evaluate, as a library and
as the `cql` command-line tool.

This is the entry crate of [cql-rs](https://github.com/octofhir/cql-rs). It re-exports the parser,
ELM translator, evaluator, value types and FHIR support, and adds `CompiledLibrary`: one call that
parses a library, translates it and every library it includes, and gives you an evaluation context.

- 1693 of 1693 [cqframework/cql-tests](https://github.com/cqframework/cql-tests) pass
- includes resolved from directories or any `LibrarySource`; `FHIRHelpers` built in
- FHIR R4 JSON data (resources, Bundles, NDJSON) through `eval::fhir::InMemoryFhirData`
- ELM JSON output with name resolution and located diagnostics

## Install

```toml
[dependencies]
octofhir-cql = "0.1"
```

The `cli` feature (on by default) builds the `cql` binary and pulls in its dependencies. Library users
can turn it off: `octofhir-cql = { version = "0.1", default-features = false }`.

```bash
cargo install octofhir-cql   # the `cql` binary
```

## Evaluate a library

```rust
use octofhir_cql::CompiledLibrary;
use octofhir_cql::types::CqlValue;

let library = CompiledLibrary::compile(
    r#"
    library Example version '1.0.0'
    parameter "Limit" Integer default 3

    define "Small": { 1, 2, 3, 4, 5 } N where N <= "Limit"
    define "Count Small": Count("Small")
    "#,
)?;

let mut ctx = library.context();
assert_eq!(library.evaluate_definition("Count Small", &mut ctx)?, CqlValue::Integer(3));

// Supplied parameter values override the defaults.
let mut ctx = library.context();
ctx.set_parameter("Limit", CqlValue::Integer(4));
assert_eq!(library.evaluate_definition("Count Small", &mut ctx)?, CqlValue::Integer(4));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Libraries with includes

`CompiledLibrary::compile_with` takes a `LibrarySource` (`DirectoryLibrarySource`, or any
`Fn(&str, Option<&str>) -> Option<String>`); `compile_file` searches the file's directory and extra
library paths. Name-resolution problems are reported by `diagnostics()` rather than failing compilation.

```rust
use octofhir_cql::CompiledLibrary;
use octofhir_cql::types::CqlValue;

let common = r#"
    library Common version '1.0.0'
    define fluent function "squared"(x Integer): x * x
    define "Answer": 42
"#;
let main = r#"
    library Main version '1.0.0'
    include Common version '1.0.0' called C

    define "Nine": 3.squared()
    define "Answer": C."Answer"
"#;

let library = CompiledLibrary::compile_with(main, &|name: &str, _version: Option<&str>| {
    (name == "Common").then(|| common.to_string())
})?;
assert!(library.diagnostics().is_empty());

let results = library.evaluate(&mut library.context())?;
assert_eq!(results["Nine"], CqlValue::Integer(9));
assert_eq!(results["Answer"], CqlValue::Integer(42));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Lower-level pipeline

```rust
use octofhir_cql::elm::AstToElmConverter;
use octofhir_cql::eval::{CqlEngine, EvaluationContext};
use octofhir_cql::types::CqlValue;

let ast = octofhir_cql::parse("library L define X: 'a' + 'b'")?;
let elm = AstToElmConverter::new().convert_library(&ast);
println!("{}", elm.to_json_pretty()?); // ELM JSON

let value = CqlEngine::new().evaluate_expression(&elm, "X", &mut EvaluationContext::new())?;
assert_eq!(value, CqlValue::string("ab"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## The `cql` tool

| Command | What it does |
|---------|--------------|
| `cql eval <file> [-d data.json] [-p NAME=EXPR] [-e DEF] [--patient ID] [--now TS]` | Evaluate definitions; per patient for `context Patient` libraries; JSON output |
| `cql translate <file> [-o out.json]` | ELM JSON |
| `cql validate <files...>` | Syntax and name-resolution errors with line/column; exit code 1 on errors |
| `cql parse <file> [--debug]` | Syntax tree as JSON (or Rust debug output) |
| `cql repl [-d data.json --patient ID]` | Interactive evaluation; `define`s persist in the session |

All commands accept `-L <dir>` (where to find included libraries) where includes matter, and
`--color auto|always|never`. See the [workspace README](https://github.com/octofhir/cql-rs#readme)
for a full walkthrough.

## License

MIT OR Apache-2.0
