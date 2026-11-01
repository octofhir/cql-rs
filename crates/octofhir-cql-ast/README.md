# octofhir-cql-ast

The CQL 1.5 abstract syntax tree used by [cql-rs](https://github.com/octofhir/cql-rs).

- `Library`: definition, usings, includes, parameters, code systems, value sets, codes, concepts,
  contexts and statements (expression and function definitions, each recording the `context` it was
  declared in)
- `Expression`: literals, references, operators, timing and interval operators, queries
  (`let`, `with`/`without`, `where`, `return`, `aggregate`, `sort`), retrieves with terminology
  filters, selectors (Tuple, Instance, Code, Concept, List, Interval), conditionals and more
- `TypeSpecifier`: named, `List<T>`, `Interval<T>`, `Tuple { ... }`, `Choice<...>`

Every node implements `serde::Serialize`, so a parsed library can be dumped as JSON (this is what
`cql parse` prints). Nodes are wrapped in `Spanned<T>` with byte offsets into the source.

```rust
use octofhir_cql_ast::{Identifier, QualifiedIdentifier, TypeSpecifier};

let name = Identifier::quoted("Measurement Period");
assert!(name.quoted);

let reference = QualifiedIdentifier::qualified("Common", "Active Conditions");
assert_eq!(reference.qualifier.as_deref(), Some("Common"));

let ty = TypeSpecifier::named("Integer");
assert!(matches!(ty, TypeSpecifier::Named(_)));
```

Parse CQL into this tree with [`octofhir-cql-parser`](https://crates.io/crates/octofhir-cql-parser).

## License

MIT OR Apache-2.0
