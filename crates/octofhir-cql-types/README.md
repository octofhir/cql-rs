# octofhir-cql-types

CQL values and types for [cql-rs](https://github.com/octofhir/cql-rs).

- `CqlValue`: `Null`, `Boolean`, `Integer`, `Long`, `Decimal` (arbitrary precision), `String`,
  `Date`, `DateTime` and `Time` with partial precision and timezone offsets, `Quantity` (UCUM),
  `Ratio`, `Code`, `Concept`, `List`, `Interval`, `Tuple`
- `CqlType`: the CQL type of a value, including `List<T>`, `Interval<T>`, tuple and choice types
- type-system helpers: subtype checks and implicit conversion rules (`TypeCoercer`), plus symbol
  tables and type inference over the AST (`semantic`, `inference`) for tooling. The translator does not
  run these yet.

```rust
use octofhir_cql_types::{CqlDate, CqlDateTime, CqlQuantity, CqlValue};

let date = CqlDate::parse("2024-03").unwrap(); // month precision
assert_eq!(date.day, None);
assert_eq!(date.to_string(), "2024-03");

let instant = CqlDateTime::parse("2024-03-15T10:30:00.250+02:00").unwrap();
assert_eq!(instant.millisecond, Some(250));
assert_eq!(instant.timezone_offset, Some(120));

let hba1c = CqlValue::Quantity(CqlQuantity::new("7.2".parse().unwrap(), "%"));
assert_eq!(hba1c.to_string(), "7.2 '%'");
```

## License

MIT OR Apache-2.0
