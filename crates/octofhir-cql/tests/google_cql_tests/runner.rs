//! Google CQL Test Runner
//!
//! Runs tests extracted from google/cql repository.

use octofhir_cql_elm::AstToElmConverter;
use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_types::CqlValue;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Test case from Google CQL tests
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTestCase {
    pub name: String,
    pub cql: String,
    pub expected: Option<ExpectedValue>,
}

/// Expected value can be various types
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ExpectedValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Typed(TypedValue),
    Raw(RawValue),
}

/// Raw value without explicit type
#[derive(Debug, Clone, Deserialize)]
pub struct RawValue {
    pub raw: String,
}

/// Typed value with explicit type
#[derive(Debug, Clone, Deserialize)]
pub struct TypedValue {
    #[serde(rename = "type")]
    pub value_type: String,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub raw: Option<String>,
}

/// Test file structure
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTestFile {
    pub source: String,
    pub functions: HashMap<String, Vec<GoogleTestCase>>,
}

/// Result of running a single test
#[derive(Debug, Clone)]
pub struct TestResult {
    pub function_name: String,
    pub test_name: String,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    pub error: Option<String>,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

/// Summary of running a test file
#[derive(Debug, Clone)]
pub struct FileResult {
    pub file_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<TestResult>,
}

/// Test runner for Google CQL tests
pub struct GoogleTestRunner {
    engine: CqlEngine,
}

impl Default for GoogleTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl GoogleTestRunner {
    pub fn new() -> Self {
        Self {
            engine: CqlEngine::new(),
        }
    }

    /// Load test file from JSON
    pub fn load_test_file(path: &Path) -> Result<GoogleTestFile, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Run all tests in a file
    pub fn run_file(&self, test_file: &GoogleTestFile) -> FileResult {
        let mut results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for (func_name, tests) in &test_file.functions {
            for test in tests {
                let result = self.run_test(func_name, test);

                if result.skipped {
                    skipped += 1;
                } else if result.passed {
                    passed += 1;
                } else {
                    failed += 1;
                }

                results.push(result);
            }
        }

        FileResult {
            file_name: test_file.source.clone(),
            total: results.len(),
            passed,
            failed,
            skipped,
            results,
        }
    }

    /// Run a single test
    pub fn run_test(&self, func_name: &str, test: &GoogleTestCase) -> TestResult {
        // Check for unsupported features
        if let Some(reason) = self.should_skip(&test.cql) {
            return TestResult {
                function_name: func_name.to_string(),
                test_name: test.name.clone(),
                passed: false,
                expected: format_expected(&test.expected),
                actual: String::new(),
                error: None,
                skipped: true,
                skip_reason: Some(reason),
            };
        }

        // Evaluate the expression
        match self.evaluate_expression(&test.cql) {
            Ok(result) => {
                let expected_str = format_expected(&test.expected);
                let actual_str = format_value(&result);
                let passed = compare_result(&result, &test.expected);

                TestResult {
                    function_name: func_name.to_string(),
                    test_name: test.name.clone(),
                    passed,
                    expected: expected_str,
                    actual: actual_str,
                    error: None,
                    skipped: false,
                    skip_reason: None,
                }
            }
            Err(e) => TestResult {
                function_name: func_name.to_string(),
                test_name: test.name.clone(),
                passed: false,
                expected: format_expected(&test.expected),
                actual: String::new(),
                error: Some(e),
                skipped: false,
                skip_reason: None,
            },
        }
    }

    fn should_skip(&self, cql: &str) -> Option<String> {
        // Skip UCUM unit conversion tests
        if cql.contains("convert") && cql.contains("to") {
            return Some("UCUM conversion not supported".to_string());
        }

        // Skip terminology tests
        if cql.contains("InValueSet") || cql.contains("AnyInValueSet") {
            return Some("Terminology operations not supported".to_string());
        }

        None
    }

    fn evaluate_expression(&self, expr: &str) -> Result<CqlValue, String> {
        // Wrap expression in a minimal library for parsing
        let cql = format!("library Test version '1.0'\ndefine Result: {}", expr);

        // Parse the CQL to AST
        let ast = octofhir_cql_parser::parse(&cql)
            .map_err(|e| format!("Parse error: {:?}", e))?;

        // Convert AST to ELM
        let mut converter = AstToElmConverter::new();
        let elm_library = converter.convert_library(&ast);

        // Set up evaluation context with the library
        let mut ctx = EvaluationContext::new()
            .with_library(elm_library.clone());

        // Evaluate the "Result" expression
        self.engine.evaluate_expression(&elm_library, "Result", &mut ctx)
            .map_err(|e| format!("Evaluation error: {:?}", e))
    }
}

fn format_expected(expected: &Option<ExpectedValue>) -> String {
    match expected {
        None => "null".to_string(),
        Some(ExpectedValue::Null) => "null".to_string(),
        Some(ExpectedValue::Bool(b)) => b.to_string(),
        Some(ExpectedValue::Int(i)) => i.to_string(),
        Some(ExpectedValue::Float(f)) => {
            if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                f.to_string()
            }
        }
        Some(ExpectedValue::String(s)) => format!("'{}'", s),
        Some(ExpectedValue::Raw(r)) => format!("(raw) {}", r.raw),
        Some(ExpectedValue::Typed(t)) => {
            match t.value_type.as_str() {
                "Long" => {
                    if let Some(v) = &t.value {
                        format!("{}L", v)
                    } else {
                        "null".to_string()
                    }
                }
                "Quantity" => {
                    let val = t.value.as_ref().map(|v| v.to_string()).unwrap_or_default();
                    let unit = t.unit.as_ref().map(|u| u.as_str()).unwrap_or("");
                    format!("{} '{}'", val, unit)
                }
                _ => t.raw.clone().unwrap_or_else(|| format!("{:?}", t)),
            }
        }
    }
}

fn format_value(value: &CqlValue) -> String {
    match value {
        CqlValue::Null => "null".to_string(),
        CqlValue::Boolean(b) => b.to_string(),
        CqlValue::Integer(i) => i.to_string(),
        CqlValue::Long(l) => format!("{}L", l),
        CqlValue::Decimal(d) => {
            let normalized = d.normalize();
            let s = normalized.to_string();
            if s.contains('.') {
                s
            } else {
                format!("{}.0", s)
            }
        }
        CqlValue::String(s) => format!("'{}'", s),
        CqlValue::Date(d) => format!("@{}", d),
        CqlValue::DateTime(dt) => format!("@{}", dt),
        CqlValue::Time(t) => format!("@T{}", t),
        CqlValue::Quantity(q) => format!("{}", q),
        CqlValue::Code(c) => format!("Code '{}'", c.code),
        CqlValue::Concept(c) => format!("Concept with {} codes", c.codes.len()),
        CqlValue::Interval(i) => format!("Interval {}", i),
        CqlValue::List(l) => {
            if l.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> = l.iter().map(format_value).collect();
                format!("{{{}}}", items.join(", "))
            }
        }
        CqlValue::Tuple(t) => format!("Tuple with {} elements", t.elements.len()),
        CqlValue::Ratio(r) => format!("Ratio({:?}:{:?})", r.numerator, r.denominator),
    }
}

fn compare_result(result: &CqlValue, expected: &Option<ExpectedValue>) -> bool {
    match expected {
        None => result.is_null(),
        Some(ExpectedValue::Null) => result.is_null(),
        Some(ExpectedValue::Bool(b)) => {
            matches!(result, CqlValue::Boolean(rb) if rb == b)
        }
        Some(ExpectedValue::Int(i)) => {
            match result {
                CqlValue::Integer(ri) => *ri as i64 == *i,
                CqlValue::Long(rl) => *rl == *i,
                _ => false,
            }
        }
        Some(ExpectedValue::Float(f)) => {
            match result {
                CqlValue::Decimal(d) => {
                    // Compare with tolerance for floating point
                    if let Ok(expected_dec) = Decimal::try_from(*f) {
                        let diff = (*d - expected_dec).abs();
                        diff < Decimal::new(1, 8) // 0.00000001 tolerance
                    } else {
                        false
                    }
                }
                CqlValue::Integer(i) => (*i as f64 - f).abs() < 0.0001,
                _ => false,
            }
        }
        Some(ExpectedValue::String(s)) => {
            matches!(result, CqlValue::String(rs) if rs == s)
        }
        Some(ExpectedValue::Raw(_)) => {
            // Raw values have complex Go types we can't parse yet
            // Fall back to string comparison, which will likely fail
            false
        }
        Some(ExpectedValue::Typed(t)) => {
            match t.value_type.as_str() {
                "Long" => {
                    if let Some(v) = &t.value {
                        if let CqlValue::Long(l) = result {
                            v.as_i64().map(|expected| *l == expected).unwrap_or(false)
                        } else {
                            false
                        }
                    } else {
                        result.is_null()
                    }
                }
                "Quantity" => {
                    // Basic quantity comparison
                    if let CqlValue::Quantity(q) = result {
                        if let Some(v) = &t.value {
                            let val_matches = v.as_f64()
                                .and_then(|expected| Decimal::try_from(expected).ok())
                                .map(|expected| (q.value - expected).abs() < Decimal::new(1, 4))
                                .unwrap_or(false);
                            val_matches
                            // Unit comparison would go here
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                _ => {
                    // Fall back to string comparison
                    format_value(result) == format_expected(expected)
                }
            }
        }
    }
}

/// Generate a compliance report for Google CQL tests
pub fn generate_report(results: &[FileResult]) -> String {
    let mut report = String::new();

    report.push_str("# Google CQL Test Compliance Report\n\n");

    let total_tests: usize = results.iter().map(|r| r.total).sum();
    let total_passed: usize = results.iter().map(|r| r.passed).sum();
    let total_failed: usize = results.iter().map(|r| r.failed).sum();
    let total_skipped: usize = results.iter().map(|r| r.skipped).sum();

    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Count |\n");
    report.push_str("|--------|-------|\n");
    report.push_str(&format!("| Total Tests | {} |\n", total_tests));
    report.push_str(&format!("| Passed | {} ({:.1}%) |\n",
        total_passed,
        if total_tests > 0 { total_passed as f64 / total_tests as f64 * 100.0 } else { 0.0 }
    ));
    report.push_str(&format!("| Failed | {} |\n", total_failed));
    report.push_str(&format!("| Skipped | {} |\n", total_skipped));
    report.push_str("\n");

    report.push_str("## Results by File\n\n");

    for file_result in results {
        report.push_str(&format!("### {}\n\n", file_result.file_name));
        report.push_str(&format!("- Passed: {}/{}\n", file_result.passed, file_result.total));
        report.push_str(&format!("- Failed: {}\n", file_result.failed));
        report.push_str(&format!("- Skipped: {}\n\n", file_result.skipped));

        // List failed tests (limit to first 20)
        let failed: Vec<_> = file_result.results.iter()
            .filter(|r| !r.passed && !r.skipped)
            .take(20)
            .collect();

        if !failed.is_empty() {
            report.push_str("#### Failed Tests (first 20)\n\n");
            for result in failed {
                report.push_str(&format!("- **{}::{} - {}**\n",
                    result.function_name, result.test_name, result.function_name));
                report.push_str(&format!("  - Expected: `{}`\n", result.expected));
                report.push_str(&format!("  - Actual: `{}`\n", result.actual));
                if let Some(err) = &result.error {
                    // Truncate long errors
                    let err_display = if err.len() > 100 {
                        format!("{}...", &err[..100])
                    } else {
                        err.clone()
                    };
                    report.push_str(&format!("  - Error: {}\n", err_display));
                }
            }
            report.push_str("\n");
        }
    }

    report
}
