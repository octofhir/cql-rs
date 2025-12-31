//! CQL Spec Test Runner
//!
//! Runs CQFramework specification tests by:
//! 1. Parsing CQL expressions
//! 2. Converting AST to ELM
//! 3. Evaluating ELM expressions
//! 4. Comparing results against expected outputs

use crate::spec_tests::xml_parser::{ExpectedOutput, InvalidType, OutputType, TestCase, TestSuite};
use octofhir_cql_elm::AstToElmConverter;
use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_types::CqlValue;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Result of running a single test
#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_name: String,
    pub group_name: String,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    pub error: Option<String>,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

/// Summary of running a test suite
#[derive(Debug, Clone)]
pub struct SuiteResult {
    pub suite_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<TestResult>,
}

/// Capabilities that this implementation supports
#[derive(Debug, Default)]
pub struct ImplementationCapabilities {
    capabilities: HashMap<String, bool>,
}

impl ImplementationCapabilities {
    pub fn new() -> Self {
        let mut caps = HashMap::new();
        // Core capabilities we support
        caps.insert("arithmetic".to_string(), true);
        caps.insert("comparison".to_string(), true);
        caps.insert("logical".to_string(), true);
        caps.insert("string".to_string(), true);
        caps.insert("datetime".to_string(), true);
        caps.insert("interval".to_string(), true);
        caps.insert("list".to_string(), true);
        caps.insert("aggregate".to_string(), true);

        // Capabilities we don't yet support
        caps.insert("ucum-unit-conversion-support".to_string(), false);
        caps.insert("precision-operators-for-decimal-and-date-time-types".to_string(), true);

        Self { capabilities: caps }
    }

    pub fn supports(&self, code: &str) -> bool {
        self.capabilities.get(code).copied().unwrap_or(true)
    }

    pub fn add_capability(&mut self, code: &str, supported: bool) {
        self.capabilities.insert(code.to_string(), supported);
    }
}

/// Test runner for CQL specification tests
pub struct SpecTestRunner {
    engine: CqlEngine,
    capabilities: ImplementationCapabilities,
}

impl Default for SpecTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecTestRunner {
    pub fn new() -> Self {
        Self {
            engine: CqlEngine::new(),
            capabilities: ImplementationCapabilities::new(),
        }
    }

    pub fn with_capabilities(mut self, caps: ImplementationCapabilities) -> Self {
        self.capabilities = caps;
        self
    }

    /// Run all tests in a suite
    pub fn run_suite(&self, suite: &TestSuite) -> SuiteResult {
        let mut results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for group in &suite.groups {
            for test in &group.tests {
                let result = self.run_test(test, &group.name);

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

        SuiteResult {
            suite_name: suite.name.clone(),
            total: results.len(),
            passed,
            failed,
            skipped,
            results,
        }
    }

    /// Run a single test
    pub fn run_test(&self, test: &TestCase, group_name: &str) -> TestResult {
        // Check if we support all required capabilities
        for cap in &test.capabilities {
            if !self.capabilities.supports(&cap.code) {
                return TestResult {
                    test_name: test.name.clone(),
                    group_name: group_name.to_string(),
                    passed: false,
                    expected: String::new(),
                    actual: String::new(),
                    error: None,
                    skipped: true,
                    skip_reason: Some(format!("Missing capability: {}", cap.code)),
                };
            }
        }

        // Handle tests expecting invalid expressions
        if let Some(invalid_type) = &test.invalid {
            return self.run_invalid_test(test, group_name, invalid_type);
        }

        // Parse and evaluate the expression
        match self.evaluate_expression(&test.expression) {
            Ok(result) => {
                let expected = test.outputs.first()
                    .map(|o| o.value.clone())
                    .unwrap_or_default();
                let actual = format_value(&result);
                let passed = self.compare_result(&result, &test.outputs);

                TestResult {
                    test_name: test.name.clone(),
                    group_name: group_name.to_string(),
                    passed,
                    expected,
                    actual,
                    error: None,
                    skipped: false,
                    skip_reason: None,
                }
            }
            Err(e) => TestResult {
                test_name: test.name.clone(),
                group_name: group_name.to_string(),
                passed: false,
                expected: test.outputs.first()
                    .map(|o| o.value.clone())
                    .unwrap_or_default(),
                actual: String::new(),
                error: Some(e),
                skipped: false,
                skip_reason: None,
            },
        }
    }

    fn run_invalid_test(&self, test: &TestCase, group_name: &str, invalid_type: &InvalidType) -> TestResult {
        match self.evaluate_expression(&test.expression) {
            Ok(result) => {
                // Expression should have failed but didn't
                TestResult {
                    test_name: test.name.clone(),
                    group_name: group_name.to_string(),
                    passed: false,
                    expected: format!("Error ({:?})", invalid_type),
                    actual: format_value(&result),
                    error: Some("Expected error but expression succeeded".to_string()),
                    skipped: false,
                    skip_reason: None,
                }
            }
            Err(e) => {
                // Check if the error type matches
                let error_matches = match invalid_type {
                    InvalidType::Syntax => e.contains("parse") || e.contains("syntax"),
                    InvalidType::Semantic => e.contains("type") || e.contains("semantic"),
                    InvalidType::Execution => e.contains("runtime") || e.contains("execution"),
                    InvalidType::True => true, // Any error is acceptable
                    InvalidType::False => false, // Should not be invalid
                };

                TestResult {
                    test_name: test.name.clone(),
                    group_name: group_name.to_string(),
                    passed: error_matches,
                    expected: format!("Error ({:?})", invalid_type),
                    actual: format!("Error: {}", e),
                    error: if error_matches { None } else { Some(e) },
                    skipped: false,
                    skip_reason: None,
                }
            }
        }
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

    fn compare_result(&self, result: &CqlValue, outputs: &[ExpectedOutput]) -> bool {
        if outputs.is_empty() {
            return result.is_null();
        }

        let expected = &outputs[0];

        // Compare based on output type
        match &expected.output_type {
            Some(OutputType::Integer) => {
                if let CqlValue::Integer(i) = result {
                    expected.value.parse::<i32>().map(|e| *i == e).unwrap_or(false)
                } else {
                    false
                }
            }
            Some(OutputType::Decimal) => {
                if let CqlValue::Decimal(d) = result {
                    expected.value.parse::<Decimal>().map(|e| *d == e).unwrap_or(false)
                } else {
                    false
                }
            }
            Some(OutputType::Boolean) => {
                if let CqlValue::Boolean(b) = result {
                    expected.value.parse::<bool>().map(|e| *b == e).unwrap_or(false)
                } else {
                    false
                }
            }
            Some(OutputType::String) => {
                if let CqlValue::String(s) = result {
                    s == &expected.value
                } else {
                    false
                }
            }
            _ => {
                // Generic comparison - format and compare strings
                // Normalize whitespace for list comparisons as test expectations are inconsistent
                let actual = format_value(result);
                let expected_normalized = normalize_list_format(&expected.value);
                let actual_normalized = normalize_list_format(&actual);
                actual_normalized == expected_normalized
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
            // For CQL, preserve full scale for boundary operations but normalize for regular operations
            // Check if this looks like a boundary result (scale = 8 with trailing zeros)
            let scale = d.scale();
            if scale == 8 {
                // Likely a boundary operation - preserve scale
                let s = d.to_string();
                if s.contains('.') { s } else { format!("{}.0", s) }
            } else {
                // Regular operation - normalize but ensure at least one decimal place
                let normalized = d.normalize();
                let s = normalized.to_string();
                if s.contains('.') { s } else { format!("{}.0", s) }
            }
        }
        CqlValue::String(s) => {
            // Escape quotes using unicode escapes per CQL spec
            let mut escaped = String::with_capacity(s.len());
            for c in s.chars() {
                match c {
                    '\'' => escaped.push_str("\\u0027"),
                    '"' => escaped.push_str("\\u0022"),
                    _ => escaped.push(c),
                }
            }
            format!("'{}'", escaped)
        }
        CqlValue::Date(d) => format!("@{}", d),
        CqlValue::DateTime(dt) => format!("@{}", dt),
        CqlValue::Time(t) => format!("@T{}", t),
        CqlValue::Quantity(q) => format!("{}", q),
        CqlValue::Code(c) => format!("Code {{ code: '{}' }}", c.code),
        CqlValue::Concept(c) => {
            let codes_str: Vec<String> = c.codes.iter()
                .map(|code| format!("Code {{ code: '{}' }}", code.code))
                .collect();
            format!("Concept {{ codes: {} }}", codes_str.join(", "))
        }
        CqlValue::Interval(i) => format!("Interval {}", i),
        CqlValue::List(l) => {
            if l.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> = l.iter().map(format_value).collect();
                format!("{{{}}}", items.join(", "))  // Use ", " to match most test expectations
            }
        }
        CqlValue::Tuple(t) => {
            // Format as { name: value, ... } without Tuple prefix
            let elements: Vec<String> = t.iter()
                .map(|(name, val)| format!("{}: {}", name, format_value(val)))
                .collect();
            format!("{{ {} }}", elements.join(", "))
        }
        CqlValue::Ratio(r) => format!("Ratio({:?}:{:?})", r.numerator, r.denominator),
    }
}

/// Normalize format for comparison
/// Test expectations are inconsistent with whitespace and prefixes:
/// - Lists: `{ null }` vs `{null}`, `{1,2,3}` vs `{1, 2, 3}`
/// - Intervals: `Interval[ 1, 10 ]` vs `Interval[1, 10]`
/// - Tuples: `Tuple { id: 1 }` vs `{ id: 1 }`
/// - Quantities: `5 'ml'` vs `5'ml'`
fn normalize_list_format(s: &str) -> String {
    // Collapse all whitespace (spaces, tabs, newlines) to single spaces
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    // First strip common prefixes that are inconsistently used
    let s = s.replace("Tuple { ", "{ ").replace("Tuple{ ", "{ ");
    // Normalize Interval space (Interval [ → Interval[)
    let s = s.replace("Interval [ ", "Interval[").replace("Interval ( ", "Interval(");
    // Normalize quantity space (remove space before unit quotes)
    let s = s.replace(" '", "'");
    // Normalize quote escapes: convert unicode escapes to backslash escapes
    // Both \u0027 and \' represent single quote, both \u0022 and \" represent double quote
    let s = s.replace("\\u0027", "\\'").replace("\\u0022", "\\\"");
    // Normalize timezone: Z is equivalent to +00:00
    let s = s.replace("+00:00", "Z");
    // Normalize decimal precision: 1.00 is equivalent to 1.0
    let s = normalize_decimal_precision(&s);

    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' | '[' | '(' => {
                result.push(c);
                // Skip whitespace after opening bracket
                while chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            '}' | ']' | ')' => {
                // Remove trailing whitespace before closing bracket
                while result.ends_with(' ') {
                    result.pop();
                }
                result.push(c);
            }
            ',' => {
                result.push(',');
                // Skip all whitespace after comma
                while chars.peek() == Some(&' ') {
                    chars.next();
                }
                // Add single space after comma for consistency
                let next = chars.peek();
                if next != Some(&'}') && next != Some(&']') && next != Some(&')') {
                    result.push(' ');
                }
            }
            ' ' => {
                // Only add space if not right before comma or closing bracket
                let next = chars.peek();
                if next != Some(&',') && next != Some(&'}') && next != Some(&']') && next != Some(&')') {
                    result.push(' ');
                }
            }
            _ => result.push(c),
        }
    }

    result
}

/// Normalize decimal precision by removing trailing zeros after decimal point
/// 1.00 -> 1.0, 1.000 -> 1.0, but keep at least one decimal place
fn normalize_decimal_precision(s: &str) -> String {
    use regex::Regex;
    // Match decimal numbers with trailing zeros
    let re = Regex::new(r"(\d+\.\d*?)0+(\D|$)").unwrap();
    let mut result = s.to_string();
    // Keep applying until no more changes (for nested cases)
    loop {
        let new_result = re.replace_all(&result, |caps: &regex::Captures| {
            let num_part = &caps[1];
            let suffix = &caps[2];
            // Ensure at least one digit after decimal
            if num_part.ends_with('.') {
                format!("{}0{}", num_part, suffix)
            } else {
                format!("{}{}", num_part, suffix)
            }
        }).to_string();
        if new_result == result {
            break;
        }
        result = new_result;
    }
    result
}

/// Generate a compliance report
pub fn generate_report(results: &[SuiteResult]) -> String {
    let mut report = String::new();

    report.push_str("# CQL Specification Compliance Report\n\n");

    let total_tests: usize = results.iter().map(|r| r.total).sum();
    let total_passed: usize = results.iter().map(|r| r.passed).sum();
    let total_failed: usize = results.iter().map(|r| r.failed).sum();
    let total_skipped: usize = results.iter().map(|r| r.skipped).sum();

    report.push_str("## Summary\n\n");
    report.push_str(&format!("| Metric | Count |\n"));
    report.push_str(&format!("|--------|-------|\n"));
    report.push_str(&format!("| Total Tests | {} |\n", total_tests));
    report.push_str(&format!("| Passed | {} ({:.1}%) |\n",
        total_passed,
        if total_tests > 0 { total_passed as f64 / total_tests as f64 * 100.0 } else { 0.0 }
    ));
    report.push_str(&format!("| Failed | {} |\n", total_failed));
    report.push_str(&format!("| Skipped | {} |\n", total_skipped));
    report.push_str("\n");

    report.push_str("## Results by Suite\n\n");

    for suite in results {
        report.push_str(&format!("### {}\n\n", suite.suite_name));
        report.push_str(&format!("- Passed: {}/{}\n", suite.passed, suite.total));
        report.push_str(&format!("- Failed: {}\n", suite.failed));
        report.push_str(&format!("- Skipped: {}\n\n", suite.skipped));

        // List failed tests
        let failed: Vec<_> = suite.results.iter()
            .filter(|r| !r.passed && !r.skipped)
            .collect();

        if !failed.is_empty() {
            report.push_str("#### Failed Tests\n\n");
            for result in failed {
                report.push_str(&format!("- **{}::{}**\n", result.group_name, result.test_name));
                report.push_str(&format!("  - Expected: `{}`\n", result.expected));
                report.push_str(&format!("  - Actual: `{}`\n", result.actual));
                if let Some(err) = &result.error {
                    report.push_str(&format!("  - Error: {}\n", err));
                }
            }
            report.push_str("\n");
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_tests::xml_parser::parse_test_xml;

    #[test]
    fn test_runner_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tests name="BasicTest" version="1.0">
    <group name="Arithmetic">
        <test name="Add1">
            <expression>1 + 1</expression>
            <output>2</output>
        </test>
    </group>
</tests>"#;

        let suite = parse_test_xml(xml).unwrap();
        let runner = SpecTestRunner::new();
        let result = runner.run_suite(&suite);

        assert_eq!(result.suite_name, "BasicTest");
        assert_eq!(result.total, 1);
    }

    #[test]
    fn test_capability_check() {
        let caps = ImplementationCapabilities::new();
        assert!(caps.supports("arithmetic"));
        assert!(!caps.supports("ucum-unit-conversion-support"));
    }
}
