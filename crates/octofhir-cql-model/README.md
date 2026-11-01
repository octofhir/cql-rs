# octofhir-cql-model

Data-model support for [cql-rs](https://github.com/octofhir/cql-rs):

- **FHIR JSON ⇄ CQL values** (`fhir::json`): turns FHIR R4 JSON into the CQL values that FHIR-based CQL
  works with, applying the `FHIRHelpers` conversions at load time, and renders CQL values back to JSON
- **ModelInfo**: parser for HL7 ModelInfo (XML and JSON) and a `ModelRegistry` for type and property
  lookup
- **`DataRetriever`**: an async retrieve trait for data sources (bridged to the evaluator by
  `octofhir_cql_eval::DataRetrieverAdapter`)

## FHIR JSON to CQL values

```rust
use octofhir_cql_model::fhir::json::{cql_to_json, fhir_to_cql};
use octofhir_cql_types::CqlValue;
use serde_json::json;

let observation = fhir_to_cql(&json!({
    "resourceType": "Observation",
    "status": "final",
    "code": { "coding": [{ "system": "http://loinc.org", "code": "4548-4" }] },
    "effectiveDateTime": "2024-03-10T09:15:00Z",
    "valueQuantity": { "value": 9.5, "unit": "%", "system": "http://unitsofmeasure.org", "code": "%" }
}));

let CqlValue::Tuple(o) = &observation else { unreachable!() };
assert!(matches!(o.get("code"), Some(CqlValue::Concept(_))));     // CodeableConcept -> Concept
assert!(matches!(o.get("effective"), Some(CqlValue::DateTime(_)))); // effectiveDateTime -> effective
assert!(matches!(o.get("value"), Some(CqlValue::Quantity(_))));     // valueQuantity -> value

assert_eq!(cql_to_json(o.get("value").unwrap()), json!({ "value": 9.5, "unit": "%" }));
```

| FHIR | CQL |
|------|-----|
| `boolean`, `integer`/`positiveInt`/`unsignedInt`, `decimal` | `Boolean`, `Integer` (`Long` if large), `Decimal` |
| `string`, `code`, `uri`, `id`, `markdown`, ... | `String` |
| `date`, `dateTime`/`instant`, `time` | `Date`, `DateTime`, `Time` |
| `Coding`, `CodeableConcept` | `Code`, `Concept` (`text` becomes the display) |
| `Quantity`, `Age`, `Duration`, `Count`, `Distance`, `SimpleQuantity` | `Quantity` (UCUM `code`, else `unit`) |
| `Period` | `Interval<DateTime>` (`Interval(null, end]` without a start) |
| `Range`, `Ratio` | `Interval<Quantity>`, `Ratio` |
| choice elements `<name><Type>` | `<name>` typed by the suffix |
| resources and other complex types | `Tuple` |

The mapping does not use StructureDefinitions. Data types are recognised by element names
(`coding`, `start`/`end`, `low`/`high`, `numerator`/`denominator`), date elements by name
(`birthDate`, `recordedDate`, `authoredOn`, ...), and any string with a full `dateTime` lexical form
(with a time part) is read as a `DateTime`.

## ModelInfo

`ModelRegistry::from_file` loads a full ModelInfo, for example `FHIR-ModelInfo-4.0.1.xml` from a FHIR
package. The embedded `fhir::fhir_r4_registry()` / `fhir_r5_registry()` hold a small hand-curated
subset: FHIR primitives, common data types and six resources (Patient, Observation, Condition,
Procedure, MedicationRequest, Encounter). The R5 file differs from the R4 one only in its version.
The translator and evaluator do not consult ModelInfo today.

## License

MIT OR Apache-2.0
