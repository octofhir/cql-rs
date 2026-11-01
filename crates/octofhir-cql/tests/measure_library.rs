//! End-to-end test of a realistic measure-style library: parse, translate to
//! ELM (with an included library) and evaluate per patient against a FHIR
//! bundle with value sets.

use octofhir_cql::eval::fhir::InMemoryFhirData;
use octofhir_cql::eval::{DataProvider, TerminologyProvider};
use octofhir_cql::types::CqlValue;
use octofhir_cql::{CompiledLibrary, DirectoryLibrarySource, parse};
use std::path::PathBuf;
use std::sync::Arc;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn fixture(name: &str) -> String {
    let path = fixtures().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn compile() -> CompiledLibrary {
    CompiledLibrary::compile_with(
        &fixture("DiabetesScreening.cql"),
        &DirectoryLibrarySource::new([fixtures()]),
    )
    .expect("library compiles")
}

fn data() -> Arc<InMemoryFhirData> {
    let bundle: serde_json::Value = serde_json::from_str(&fixture("diabetes-bundle.json")).unwrap();
    let mut data = InMemoryFhirData::new();
    data.add_json(&bundle);
    Arc::new(data)
}

/// Evaluate every definition for the patient with `patient_id`.
fn evaluate_for(patient_id: &str) -> indexmap::IndexMap<String, CqlValue> {
    let library = compile();
    let data = data();
    let patient = data
        .resources("Patient")
        .iter()
        .find(|p| matches!(p, CqlValue::Tuple(t) if t.get("id") == Some(&CqlValue::string(patient_id))))
        .cloned()
        .expect("patient in bundle");
    let mut ctx = library
        .context()
        .with_context("Patient", patient)
        .with_data_provider(data.clone() as Arc<dyn DataProvider>)
        .with_terminology_provider(data as Arc<dyn TerminologyProvider>);
    library.register(&mut ctx);
    library.evaluate(&mut ctx).expect("evaluation succeeds")
}

fn list_len(value: &CqlValue) -> usize {
    match value {
        CqlValue::List(list) => list.len(),
        other => panic!("expected a list, got {other:?}"),
    }
}

#[test]
fn parses_measure_library() {
    for name in ["DiabetesScreening.cql", "CommonLogic.cql"] {
        let source = fixture(name);
        if let Err(e) = parse(&source) {
            panic!("{name}: {e}");
        }
    }
}

#[test]
fn translates_with_resolved_references() {
    let library = compile();
    assert!(
        library.diagnostics().is_empty(),
        "unexpected diagnostics: {:?}",
        library.diagnostics()
    );
    assert!(library.uses_builtin_fhir_helpers());
    assert_eq!(
        library.included().map(|(name, _)| name).collect::<Vec<_>>(),
        ["CommonLogic"]
    );

    let elm = serde_json::to_value(library.elm()).unwrap();
    let defs = elm["statements"]["def"].as_array().unwrap();
    let def = |name: &str| {
        defs.iter()
            .find(|d| d["name"] == name)
            .unwrap_or_else(|| panic!("definition {name}"))
    };

    // Implicit context definition
    assert_eq!(def("Patient")["expression"]["type"], "SingletonFrom");
    // Functions carry their operands
    let function = def("Is Poorly Controlled");
    assert_eq!(function["type"], "FunctionDef");
    assert_eq!(function["operand"][0]["name"], "value");
    // Retrieve with a value set filter and the default FHIR code path
    let retrieve = &def("Diabetes Diagnosis")["expression"]["source"][0]["expression"];
    assert_eq!(retrieve["type"], "Retrieve");
    assert_eq!(retrieve["dataType"], "{http://hl7.org/fhir}Condition");
    assert_eq!(retrieve["codeProperty"], "code");
    assert_eq!(retrieve["codes"]["type"], "ValueSetRef");
    // Library-qualified reference into the include
    let where_clause = &def("Diabetes Diagnosis")["expression"]["where"];
    assert_eq!(where_clause["operand"][1]["type"], "ConceptRef");
    assert_eq!(where_clause["operand"][1]["libraryName"], "Common");
    // Parameters keep their defaults
    let parameters = elm["parameters"]["def"].as_array().unwrap();
    assert!(parameters.iter().all(|p| p.get("default").is_some()));
}

#[test]
fn evaluates_numerator_patient() {
    let results = evaluate_for("adult-diabetic");

    assert_eq!(results["Initial Population"], CqlValue::Boolean(true));
    assert_eq!(results["Denominator"], CqlValue::Boolean(true));
    assert_eq!(list_len(&results["Qualifying Encounters"]), 1);
    assert_eq!(list_len(&results["Diabetes Diagnosis"]), 1);
    assert_eq!(list_len(&results["HbA1c Tests"]), 2);
    match &results["Most Recent HbA1c Value"] {
        CqlValue::Quantity(q) => {
            assert_eq!(q.value.to_string(), "9.5");
            assert_eq!(q.unit.as_deref(), Some("%"));
        }
        other => panic!("expected a Quantity, got {other:?}"),
    }
    assert_eq!(results["Numerator"], CqlValue::Boolean(true));
    assert_eq!(results["Poorly Controlled"], CqlValue::Boolean(true));
    assert_eq!(results["Has Type 2 Code"], CqlValue::Boolean(true));
    assert_eq!(results["HbA1c Concept Matches"], CqlValue::Boolean(true));
    assert_eq!(list_len(&results["Encounters With Diabetes"]), 1);
    assert_eq!(list_len(&results["Encounters Without HbA1c"]), 0);
    assert_eq!(list_len(&results["HbA1c Test Periods"]), 2);
    assert_eq!(list_len(&results["Active Diabetes Codes"]), 1);
}

#[test]
fn evaluates_patient_outside_initial_population() {
    let results = evaluate_for("child");

    assert_eq!(results["Initial Population"], CqlValue::Boolean(false));
    assert_eq!(results["Denominator"], CqlValue::Boolean(false));
    assert_eq!(list_len(&results["Qualifying Encounters"]), 1);
    assert_eq!(list_len(&results["HbA1c Tests"]), 0);
    assert_eq!(results["Most Recent HbA1c Value"], CqlValue::Null);
    assert_eq!(results["Numerator"], CqlValue::Null);
    assert_eq!(results["Has Type 2 Code"], CqlValue::Boolean(false));
}

#[test]
fn supplied_parameter_overrides_default() {
    let library = compile();
    let data = data();
    let patient = data.resources("Patient")[0].clone();
    let mut ctx = library
        .context()
        .with_context("Patient", patient)
        .with_data_provider(data.clone() as Arc<dyn DataProvider>)
        .with_terminology_provider(data as Arc<dyn TerminologyProvider>);
    library.register(&mut ctx);
    ctx.set_parameter(
        "HbA1c Threshold",
        CqlValue::Decimal("10.0".parse().unwrap()),
    );
    let numerator = library.evaluate_definition("Numerator", &mut ctx).unwrap();
    assert_eq!(numerator, CqlValue::Boolean(false));
}

#[test]
fn type_tests_understand_fhir_type_names() {
    let library = CompiledLibrary::compile(
        r#"
        library Types
        using FHIR version '4.0.1'
        context Patient

        define "Period Effective": [Observation] O where O.effective is FHIR.Period
        define "DateTime Effective": [Observation] O where O.effective is DateTime
        define "Observations": [Observation] O where O is Observation
        define "Not Conditions": [Observation] O where O is Condition
        define "Values": [Observation] O return O.value as FHIR.Quantity
        "#,
    )
    .unwrap();
    let data = data();
    let patient = data
        .resources("Patient")
        .iter()
        .find(|p| matches!(p, CqlValue::Tuple(t) if t.get("id") == Some(&CqlValue::string("adult-diabetic"))))
        .cloned()
        .unwrap();
    let mut ctx = library
        .context()
        .with_context("Patient", patient)
        .with_data_provider(data as Arc<dyn DataProvider>);
    let results = library.evaluate(&mut ctx).unwrap();
    assert_eq!(list_len(&results["Period Effective"]), 1);
    assert_eq!(list_len(&results["DateTime Effective"]), 1);
    assert_eq!(list_len(&results["Observations"]), 2);
    assert_eq!(list_len(&results["Not Conditions"]), 0);
    let CqlValue::List(values) = &results["Values"] else {
        panic!("expected list");
    };
    assert!(values.iter().all(|v| matches!(v, CqlValue::Quantity(_))));
}
