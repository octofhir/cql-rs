//! Tests of the `cql` binary.
#![cfg(feature = "cli")]

use octofhir_cql::cli::repl::{ReplOptions, Session};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn cql(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cql"))
        .args(["--color", "never"])
        .args(args)
        .current_dir(fixtures())
        .output()
        .expect("run cql")
}

fn stdout_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "cql failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

fn write_temp(name: &str, content: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

#[test]
fn eval_per_patient_with_data_and_parameters() {
    let output = cql(&[
        "eval",
        "DiabetesScreening.cql",
        "--data",
        "diabetes-bundle.json",
        "-e",
        "Initial Population",
        "-e",
        "Numerator",
        "-e",
        "Most Recent HbA1c Value",
    ]);
    let json = stdout_json(&output);
    assert_eq!(json["adult-diabetic"]["Initial Population"], true);
    assert_eq!(json["adult-diabetic"]["Numerator"], true);
    assert_eq!(
        json["adult-diabetic"]["Most Recent HbA1c Value"],
        serde_json::json!({ "value": 9.5, "unit": "%" })
    );
    assert_eq!(json["child"]["Initial Population"], false);
    assert_eq!(json["child"]["Numerator"], Value::Null);

    let output = cql(&[
        "eval",
        "DiabetesScreening.cql",
        "-d",
        "diabetes-bundle.json",
        "--patient",
        "adult-diabetic",
        "-p",
        "\"HbA1c Threshold\"=10.0",
        "-e",
        "Numerator",
    ]);
    let json = stdout_json(&output);
    assert_eq!(
        json,
        serde_json::json!({ "adult-diabetic": { "Numerator": false } })
    );
}

#[test]
fn eval_ndjson_and_now() {
    let bundle: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("diabetes-bundle.json")).unwrap(),
    )
    .unwrap();
    let ndjson: String = bundle["entry"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| format!("{}\n", e["resource"]))
        .collect();
    let (_dir, data) = write_temp("data.ndjson", &ndjson);
    let (_lib_dir, library) = write_temp(
        "Ages.cql",
        "library Ages\nusing FHIR version '4.0.1'\ncontext Patient\ndefine Age: AgeInYears()\n",
    );
    let output = cql(&[
        "eval",
        library.to_str().unwrap(),
        "-d",
        data.to_str().unwrap(),
        "--now",
        "2024-06-30",
    ]);
    let json = stdout_json(&output);
    assert_eq!(json["adult-diabetic"]["Age"], 54);
    assert_eq!(json["child"]["Age"], 8);
}

#[test]
fn eval_without_context() {
    let (_dir, library) = write_temp(
        "Logic.cql",
        "library Logic\nparameter Limit Integer default 3\ndefine Doubled: { 1, 2, 3 } X where X < Limit return X * 2\n",
    );
    let json = stdout_json(&cql(&["eval", library.to_str().unwrap(), "--compact"]));
    assert_eq!(json, serde_json::json!({ "Doubled": [2, 4] }));
}

#[test]
fn eval_reports_evaluation_errors() {
    let (_dir, library) = write_temp(
        "Err.cql",
        "library Err\ndefine X: singleton from { 1, 2 }\n",
    );
    let output = cql(&["eval", library.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).is_empty());
}

#[test]
fn translate_prints_elm_json() {
    let json = stdout_json(&cql(&["translate", "DiabetesScreening.cql", "--compact"]));
    let library = &json["library"];
    assert_eq!(library["identifier"]["id"], "DiabetesScreening");
    assert_eq!(library["includes"]["def"][1]["localIdentifier"], "Common");
    assert!(library["statements"]["def"].as_array().unwrap().len() > 10);
}

#[test]
fn parse_prints_ast_json() {
    let json = stdout_json(&cql(&["parse", "CommonLogic.cql"]));
    assert_eq!(json["definition"]["name"]["name"]["name"], "CommonLogic");
    assert_eq!(json["statements"].as_array().unwrap().len(), 2);
}

#[test]
fn validate_succeeds_and_fails_with_exit_code() {
    let output = cql(&["validate", "DiabetesScreening.cql", "CommonLogic.cql"]);
    assert!(output.status.success());

    let (_dir, broken) = write_temp(
        "Broken.cql",
        "library Broken\ndefine A: 1 +\ndefine B: Missing + 1\n",
    );
    let output = cql(&["validate", broken.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Broken.cql:2:"), "{stderr}");
    assert!(stderr.contains("error[CQL0012]"), "{stderr}");

    let (_dir, unresolved) = write_temp("Unresolved.cql", "library U\ndefine B: Missing + 1\n");
    let output = cql(&["validate", unresolved.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("`Missing`"));
}

#[test]
fn missing_include_is_an_error() {
    let (_dir, library) = write_temp(
        "Inc.cql",
        "library Inc\ninclude Nowhere version '1' called N\ndefine X: N.Y\n",
    );
    let output = cql(&["validate", library.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("`Nowhere`"));
}

#[test]
fn repl_session_keeps_definitions() {
    let mut session = Session::new(&ReplOptions {
        data: vec![fixtures().join("diabetes-bundle.json")],
        patient: Some("adult-diabetic".to_string()),
        library_paths: vec![],
    })
    .unwrap();
    assert_eq!(session.eval_line("1 + 1").unwrap(), "2");
    assert_eq!(session.eval_line("define Twice: 21 * 2").unwrap(), "ok");
    assert_eq!(session.eval_line("Twice").unwrap(), "42");
    assert_eq!(session.eval_line("Count([Observation])").unwrap(), "2");
    assert!(session.eval_line("define Bad: Missing").is_err());
    assert!(session.eval_line("Twice + ").is_err());
}
