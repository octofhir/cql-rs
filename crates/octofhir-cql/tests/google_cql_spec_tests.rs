//! Google CQL Specification Tests
//!
//! Tests extracted from google/cql repository enginetests.

mod google_cql_tests;

use google_cql_tests::{GoogleTestRunner, generate_report};
use std::path::PathBuf;

fn get_test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("google_cql_tests")
        .join("data")
}

#[test]
fn run_google_cql_tests() {
    let test_dir = get_test_data_dir();
    let runner = GoogleTestRunner::new();

    let test_files = [
        "operator_arithmetic.json",
        "operator_comparison.json",
        "operator_logic.json",
        "operator_string.json",
        "operator_list.json",
        "operator_interval.json",
        "operator_datetime.json",
        "operator_nullological.json",
        "literal.json",
        "conditional.json",
        "expression.json",
    ];

    let mut all_results = Vec::new();

    for file_name in &test_files {
        let path = test_dir.join(file_name);
        if !path.exists() {
            eprintln!("Test file not found: {}", path.display());
            continue;
        }

        match GoogleTestRunner::load_test_file(&path) {
            Ok(test_file) => {
                let result = runner.run_file(&test_file);
                eprintln!("{}: {}/{} passed ({} skipped)",
                    file_name,
                    result.passed,
                    result.total,
                    result.skipped
                );
                all_results.push(result);
            }
            Err(e) => {
                eprintln!("Failed to load {}: {}", file_name, e);
            }
        }
    }

    // Generate and print report
    let report = generate_report(&all_results);
    eprintln!("\n{}", report);

    // Calculate totals
    let total_tests: usize = all_results.iter().map(|r| r.total).sum();
    let total_passed: usize = all_results.iter().map(|r| r.passed).sum();
    let total_skipped: usize = all_results.iter().map(|r| r.skipped).sum();

    let pass_rate = if total_tests > 0 {
        total_passed as f64 / (total_tests - total_skipped) as f64 * 100.0
    } else {
        0.0
    };

    eprintln!("\nGoogle CQL Tests: {}/{} passed ({:.1}%)",
        total_passed, total_tests - total_skipped, pass_rate);

    // We don't fail the test - this is informational
    // Later we can add a threshold
}

#[test]
fn run_arithmetic_tests_only() {
    let test_dir = get_test_data_dir();
    let runner = GoogleTestRunner::new();

    let path = test_dir.join("operator_arithmetic.json");
    let test_file = GoogleTestRunner::load_test_file(&path)
        .expect("Failed to load arithmetic tests");

    let result = runner.run_file(&test_file);

    // Print failed tests for debugging
    for test_result in &result.results {
        if !test_result.passed && !test_result.skipped {
            eprintln!("FAIL: {}::{}", test_result.function_name, test_result.test_name);
            eprintln!("  Expected: {}", test_result.expected);
            eprintln!("  Actual: {}", test_result.actual);
            if let Some(err) = &test_result.error {
                eprintln!("  Error: {}", err);
            }
        }
    }

    eprintln!("\nArithmetic: {}/{} passed ({} skipped)",
        result.passed, result.total, result.skipped);
}

#[test]
fn run_comparison_tests_only() {
    let test_dir = get_test_data_dir();
    let runner = GoogleTestRunner::new();

    let path = test_dir.join("operator_comparison.json");
    let test_file = GoogleTestRunner::load_test_file(&path)
        .expect("Failed to load comparison tests");

    let result = runner.run_file(&test_file);

    // Print failed tests for debugging
    for test_result in &result.results {
        if !test_result.passed && !test_result.skipped {
            eprintln!("FAIL: {}::{}", test_result.function_name, test_result.test_name);
            eprintln!("  Expected: {}", test_result.expected);
            eprintln!("  Actual: {}", test_result.actual);
            if let Some(err) = &test_result.error {
                eprintln!("  Error: {}", err);
            }
        }
    }

    eprintln!("\nComparison: {}/{} passed ({} skipped)",
        result.passed, result.total, result.skipped);
}

#[test]
fn run_logic_tests_only() {
    let test_dir = get_test_data_dir();
    let runner = GoogleTestRunner::new();

    let path = test_dir.join("operator_logic.json");
    let test_file = GoogleTestRunner::load_test_file(&path)
        .expect("Failed to load logic tests");

    let result = runner.run_file(&test_file);

    // Print failed tests for debugging
    for test_result in &result.results {
        if !test_result.passed && !test_result.skipped {
            eprintln!("FAIL: {}::{}", test_result.function_name, test_result.test_name);
            eprintln!("  Expected: {}", test_result.expected);
            eprintln!("  Actual: {}", test_result.actual);
            if let Some(err) = &test_result.error {
                eprintln!("  Error: {}", err);
            }
        }
    }

    eprintln!("\nLogic: {}/{} passed ({} skipped)",
        result.passed, result.total, result.skipped);
}
