# octofhir-cql-parser

The CQL 1.5 parser of [cql-rs](https://github.com/octofhir/cql-rs), written with
[winnow](https://crates.io/crates/winnow). It parses the CQL used by the official
[cqframework/cql-tests](https://github.com/cqframework/cql-tests) (all 1693 pass end to end) and by
measure libraries:

- library structure in any order: `library`, `using ... called`, `include ... version ... called`,
  `codesystem`, `valueset` (with `codesystems { }`), `code ... from ... display`, `concept`,
  `parameter` with type and default, `context`, `define [public|private]`,
  `define [fluent] function ... returns ...: body | external`
- queries over retrieves, lists, parenthesised expressions and (qualified) identifiers, with `let`,
  `with`/`without ... such that`, `where`, `return [distinct]`, `aggregate`, `sort by <expression>`
- retrieves with terminology filters: `[Condition: "Diabetes"]`, `[Observation: code in "VS"]`,
  `[FHIR.Encounter]`
- timing phrases: `starts before end of`, `3 days or less on or after`, `within 30 days of`,
  `ends during`, `same day as`, ...
- Code and Concept selectors, `Choice<...>`/`Tuple { ... }` type specifiers, all literals

Comments are blanked without shifting offsets, so every top-level item's span and every error location
refers to the original text.

## Parse

```rust
use octofhir_cql_parser::{ParseMode, parse, parse_expression, parse_with_mode};

let library = parse(
    "library Example version '1.0.0'
     using FHIR version '4.0.1'
     valueset \"Diabetes\": 'http://example.org/vs/diabetes'
     context Patient
     define \"Has Diabetes\": exists [Condition: \"Diabetes\"]",
)?;
assert_eq!(library.valuesets.len(), 1);
assert_eq!(library.statements.len(), 1);

let expression = parse_expression("Interval[1, 10] contains 5")?;
# let _ = expression;

// Errors carry the location of the offending token.
let error = parse("library L\ndefine A: 1 +\n").unwrap_err();
assert_eq!(error.location().map(|l| l.line), Some(2));

// Analysis mode recovers at the next top-level item and reports every error.
let result = parse_with_mode(
    "library L\ndefine A: (1 +\ndefine B: 2\ndefine C: )\n",
    ParseMode::Analysis,
);
assert_eq!(result.errors.len(), 2);
assert_eq!(result.library.map(|l| l.statements.len()), Some(1));
# Ok::<(), Box<dyn std::error::Error>>(())
```

The result is the syntax tree of [`octofhir-cql-ast`](https://crates.io/crates/octofhir-cql-ast).
Translate it to ELM with [`octofhir-cql-elm`](https://crates.io/crates/octofhir-cql-elm).

## Limitations

Spans are recorded for top-level items (definitions, declarations, type specifiers of parameters and
operands); expression nodes carry placeholder spans.

## License

MIT OR Apache-2.0
