# octofhir-cql-diagnostics

Errors, error codes and source locations shared by the [cql-rs](https://github.com/octofhir/cql-rs)
crates.

- `CqlError`: parse, semantic, evaluation, model and system errors, each with an `ErrorCode` and,
  where known, a `SourceLocation` (line, column, byte offset, length)
- `ErrorCode`: numbered codes in ranges: `CQL0001`–`CQL0099` syntax, `CQL0100`–`CQL0199` semantic
  (resolution), `CQL0200`–`CQL0299` evaluation, `CQL0300`–`CQL0399` model, `CQL0400`–`CQL0499` system
- `Span`, `Spanned<T>`, `offset_to_line_col`

```rust
use octofhir_cql_diagnostics::{CQL0012, CqlError, SourceLocation, Span};

let source = "library L\ndefine A: 1 +\n";
let location = SourceLocation::from_span(Span::new(20, 21), source);
assert_eq!((location.line, location.column), (2, 11));

let error = CqlError::parse_at(CQL0012, "expected an expression", "define A: 1 +", location);
assert_eq!(error.code().to_string(), "CQL0012");
assert_eq!(error.to_string(), "CQL0012: expected an expression (line 2, column 11)");
assert_eq!(CQL0012.info().description, "Expected expression");
```

## License

MIT OR Apache-2.0
