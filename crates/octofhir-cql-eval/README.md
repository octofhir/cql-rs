# octofhir-cql-eval

The CQL 1.5 evaluation engine of [cql-rs](https://github.com/octofhir/cql-rs): evaluates ELM
libraries against an `EvaluationContext` that supplies parameters, included libraries, the evaluation
context (e.g. a Patient), and data and terminology providers.

- the system operators exercised by the 1693 [cqframework/cql-tests](https://github.com/cqframework/cql-tests)
  (all passing): arithmetic with UCUM-aware quantities, comparison, three-valued logic, strings,
  date/time with precision, intervals, lists, aggregates, type operators and conversions, queries
- library semantics: expression and function definitions (fluent, overloaded), parameters with
  defaults, code systems, value sets, codes, concepts, `include`d libraries, `context Patient`
- retrieves with terminology filters, `InValueSet`/`InCodeSystem`, `AgeInYearsAt` and friends
- an in-memory FHIR data and terminology provider (`fhir::InMemoryFhirData`)

Most applications use it through [`octofhir-cql`](https://crates.io/crates/octofhir-cql)
(`CompiledLibrary`), which also translates CQL and resolves includes.

## Plugging in data

Implement `DataProvider` to answer retrieves (`[Encounter]`, `[Condition: "Diabetes"]`). The
engine passes the data type (`{http://hl7.org/fhir}Encounter`), the current context, and any
terminology filter: a Code, Concept or list of Codes, or a value set as `{ name, id, version }`.
When a `TerminologyProvider` can expand that value set, the engine passes the expansion instead.

```rust
use octofhir_cql_eval::{CqlEngine, DataProvider, EvaluationContext};
use octofhir_cql_types::{CqlTuple, CqlValue};
use std::sync::Arc;

struct Encounters;

impl DataProvider for Encounters {
    fn retrieve(
        &self,
        data_type: &str,
        _context_type: Option<&str>,
        _context_value: Option<&CqlValue>,
        _template_id: Option<&str>,
        _code_property: Option<&str>,
        _codes: Option<&CqlValue>,
        _date_property: Option<&str>,
        _date_range: Option<&CqlValue>,
    ) -> Vec<CqlValue> {
        if !data_type.ends_with("Encounter") {
            return Vec::new();
        }
        ["finished", "cancelled", "finished"]
            .into_iter()
            .map(|status| {
                CqlValue::Tuple(CqlTuple::from_elements([("status", CqlValue::string(status))]))
            })
            .collect()
    }

    fn get_property(&self, _resource: &CqlValue, _path: &str) -> Option<CqlValue> {
        None // resources are tuples; the engine reads their elements directly
    }
}

let ast = octofhir_cql_parser::parse(
    "library Visits using FHIR version '4.0.1'
     define Finished: Count([Encounter] E where E.status = 'finished')",
)?;
let elm = octofhir_cql_elm::AstToElmConverter::new().convert_library(&ast);

let mut ctx = EvaluationContext::new().with_data_provider(Arc::new(Encounters));
let finished = CqlEngine::new().evaluate_expression(&elm, "Finished", &mut ctx)?;
assert_eq!(finished, CqlValue::Integer(2));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`TerminologyProvider` answers `in_value_set`, `in_code_system`, `expand_value_set` and
`lookup_display` with value set and code system ids (canonical URLs) taken from the library
declarations. Adapters for async sources: `DataRetrieverAdapter` (for
`octofhir_cql_model::DataRetriever`) and `TerminologyAdapter` (for `octofhir_fhir_model`
terminology providers). Both work inside and outside a Tokio runtime.

## FHIR data in memory

```rust
use octofhir_cql_eval::fhir::InMemoryFhirData;
use octofhir_cql_eval::{DataProvider, TerminologyProvider};
use octofhir_cql_types::{CqlCode, CqlValue};
use serde_json::json;

let mut data = InMemoryFhirData::new();
data.add_json(&json!({
    "resourceType": "Bundle",
    "entry": [
        { "resource": { "resourceType": "Patient", "id": "p1" } },
        { "resource": { "resourceType": "Condition", "id": "c1",
            "subject": { "reference": "Patient/p1" },
            "code": { "coding": [{ "system": "http://snomed.info/sct", "code": "44054006" }] } } },
        { "resource": { "resourceType": "ValueSet", "url": "http://example.org/vs/diabetes",
            "compose": { "include": [{ "system": "http://snomed.info/sct",
                "concept": [{ "code": "44054006" }] }] } } }
    ]
}));

let patient = data.resources("Patient")[0].clone();
let conditions = data.retrieve(
    "{http://hl7.org/fhir}Condition",
    Some("Patient"),
    Some(&patient),
    None,
    Some("code"),
    Some(&CqlValue::List(octofhir_cql_types::CqlList::from_elements(
        data.expand_value_set("http://example.org/vs/diabetes").unwrap(),
    ))),
    None,
    None,
);
assert_eq!(conditions.len(), 1);

let code = CqlValue::Code(CqlCode::new("44054006", "http://snomed.info/sct", None::<String>, None::<String>));
assert_eq!(data.in_value_set(&code, "http://example.org/vs/diabetes"), Some(true));
```

Loading accepts a resource, a `Bundle` (with `urn:uuid:` references), a JSON array, or NDJSON
(`add_ndjson`). In `Patient` context, resources whose `subject`/`patient`/`beneficiary` reference
another patient are filtered out. Value sets come from `expansion.contains` or enumerated
`compose.include.concept`; `filter` definitions are not evaluated.

## Semantics worth knowing

- Unqualified references resolve against the library whose definition is being evaluated;
  `Alias."Name"` goes through that library's `include` declarations. Register included libraries with
  `EvaluationContext::add_library(name, elm)`.
- Definitions are cached per context. Function bodies and referenced definitions cannot see the
  caller's query aliases.
- Parameters take the supplied value, else their default, else `null`.
- `FHIRHelpers.ToX(...)` calls on CQL system values are built in, so `FHIRHelpers` does not have to be
  loaded. A loaded `FHIRHelpers` library is used for arguments that are still raw FHIR structures.
- Type tests and casts accept FHIR type names (`is FHIR.Period`, `as FHIR.Quantity`,
  `is Observation`) for data presented as system values.
- `DataProvider::retrieve` returns a `Vec` and has no error channel. Report provider failures out of
  band.

## License

MIT OR Apache-2.0
