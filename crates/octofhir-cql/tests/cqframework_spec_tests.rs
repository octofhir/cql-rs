//! CQFramework Specification Tests
//!
//! This test module runs the official CQL specification tests from the
//! cqframework/cql-tests repository.
//!
//! Test files are located in tests/spec_tests/cqframework/

mod spec_tests;

use spec_tests::{parse_test_file, SpecTestRunner, generate_report};
use std::path::PathBuf;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("spec_tests")
        .join("cqframework")
}

/// Parse all test suites and verify they can be parsed
#[test]
fn test_parse_all_suites() {
    let dir = test_data_dir();
    let entries = std::fs::read_dir(&dir).expect("Failed to read test directory");

    let mut parsed_count = 0;
    let mut total_tests = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "xml").unwrap_or(false) {
            match parse_test_file(&path) {
                Ok(suite) => {
                    parsed_count += 1;
                    let test_count: usize = suite.groups.iter()
                        .map(|g| g.tests.len())
                        .sum();
                    total_tests += test_count;
                    println!("Parsed {}: {} groups, {} tests",
                        suite.name,
                        suite.groups.len(),
                        test_count
                    );
                }
                Err(e) => {
                    panic!("Failed to parse {:?}: {}", path, e);
                }
            }
        }
    }

    println!("\nTotal: {} suites, {} tests", parsed_count, total_tests);
    assert!(parsed_count > 0, "No test suites were parsed");
}

/// Run arithmetic function tests
#[test]
fn test_arithmetic_functions() {
    let path = test_data_dir().join("CqlArithmeticFunctionsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse arithmetic tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));

    // Verify tests run and some pass (many fail due to parser syntax differences)
    assert!(result.total > 0);
    assert!(result.passed > 0, "At least some arithmetic tests should pass");
}

/// Run comparison operator tests
#[test]
fn test_comparison_operators() {
    let path = test_data_dir().join("CqlComparisonOperatorsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse comparison tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    assert!(result.total > 0);
    assert!(result.passed > 60, "At least 60 comparison tests should pass");
}

/// Run logical operator tests - 100% pass rate expected
#[test]
fn test_logical_operators() {
    let path = test_data_dir().join("CqlLogicalOperatorsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse logical tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    // Logical operators should have 100% pass rate
    assert_eq!(result.passed, result.total, "All logical operator tests should pass");
}

/// Run string operator tests
#[test]
fn test_string_operators() {
    let path = test_data_dir().join("CqlStringOperatorsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse string tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    assert!(result.total > 0);
    assert!(result.passed > 40, "At least 40 string tests should pass");
}

/// Run datetime operator tests
#[test]
fn test_datetime_operators() {
    let path = test_data_dir().join("CqlDateTimeOperatorsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse datetime tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    assert!(result.total > 0);
    // Some datetime tests pass
    assert!(result.passed > 0, "At least some datetime tests should pass");
}

/// Run interval operator tests
#[test]
fn test_interval_operators() {
    let path = test_data_dir().join("CqlIntervalOperatorsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse interval tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    assert!(result.total > 0);
    // With Interval constructor and operators, we should pass many tests
    assert!(result.passed > 140, "At least 140 interval tests should pass");
}

/// Run list operator tests
#[test]
fn test_list_operators() {
    let path = test_data_dir().join("CqlListOperatorsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse list tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    assert!(result.total > 0);
    // Some list tests pass
    assert!(result.passed > 20, "At least 20 list tests should pass");
}

/// Run aggregate function tests
#[test]
fn test_aggregate_functions() {
    let path = test_data_dir().join("CqlAggregateFunctionsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse aggregate tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    assert!(result.total > 0);
    assert!(result.passed > 30, "At least 30 aggregate tests should pass");
}

/// Run type operator tests
#[test]
fn test_type_operators() {
    let path = test_data_dir().join("CqlTypeOperatorsTest.xml");
    let suite = parse_test_file(&path).expect("Failed to parse type tests");

    let runner = SpecTestRunner::new();
    let result = runner.run_suite(&suite);

    println!("\n{}", generate_report(&[result.clone()]));
    assert!(result.total > 0);
    // With 'as' type casting support, we should pass some tests
    assert!(result.passed > 5, "At least 5 type tests should pass");
}

/// Generate a full compliance report
#[test]
fn generate_compliance_report() {
    let dir = test_data_dir();
    let runner = SpecTestRunner::new();
    let mut all_results = Vec::new();

    let entries = std::fs::read_dir(&dir).expect("Failed to read test directory");

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "xml").unwrap_or(false) {
            if let Ok(suite) = parse_test_file(&path) {
                let result = runner.run_suite(&suite);
                all_results.push(result);
            }
        }
    }

    let report = generate_report(&all_results);
    println!("\n{}", report);

    // Write report to file
    let report_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("compliance_report.md");
    std::fs::write(&report_path, &report).expect("Failed to write report");
    println!("Report written to: {:?}", report_path);
}
