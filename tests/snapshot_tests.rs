//! Snapshot tests for CQL parser and compiler
//!
//! Uses insta for snapshot testing of:
//! - AST output
//! - ELM JSON output
//! - Error messages
//! - Formatted code

use insta::{assert_snapshot, assert_yaml_snapshot};
use octofhir_cql_parser::Parser;

// === AST Snapshots ===

#[test]
fn snapshot_ast_literals() {
    let parser = Parser::new();
    let expr = parser.parse_expression("42").unwrap();
    assert_yaml_snapshot!("ast_integer_literal", expr);

    let expr = parser.parse_expression("3.14").unwrap();
    assert_yaml_snapshot!("ast_decimal_literal", expr);

    let expr = parser.parse_expression("'hello'").unwrap();
    assert_yaml_snapshot!("ast_string_literal", expr);

    let expr = parser.parse_expression("true").unwrap();
    assert_yaml_snapshot!("ast_boolean_literal", expr);
}

#[test]
fn snapshot_ast_arithmetic() {
    let parser = Parser::new();

    let expr = parser.parse_expression("1 + 2").unwrap();
    assert_yaml_snapshot!("ast_addition", expr);

    let expr = parser.parse_expression("1 + 2 * 3").unwrap();
    assert_yaml_snapshot!("ast_precedence", expr);

    let expr = parser.parse_expression("(1 + 2) * 3").unwrap();
    assert_yaml_snapshot!("ast_parentheses", expr);
}

#[test]
fn snapshot_ast_comparison() {
    let parser = Parser::new();

    let expr = parser.parse_expression("x > 5").unwrap();
    assert_yaml_snapshot!("ast_greater_than", expr);

    let expr = parser.parse_expression("x = y").unwrap();
    assert_yaml_snapshot!("ast_equal", expr);

    let expr = parser.parse_expression("x >= 10 and y < 20").unwrap();
    assert_yaml_snapshot!("ast_compound_comparison", expr);
}

#[test]
fn snapshot_ast_logical() {
    let parser = Parser::new();

    let expr = parser.parse_expression("true and false").unwrap();
    assert_yaml_snapshot!("ast_and", expr);

    let expr = parser.parse_expression("true or false").unwrap();
    assert_yaml_snapshot!("ast_or", expr);

    let expr = parser.parse_expression("not true").unwrap();
    assert_yaml_snapshot!("ast_not", expr);
}

#[test]
fn snapshot_ast_query_simple() {
    let parser = Parser::new();
    let expr = parser.parse_expression("[Observation] O").unwrap();
    assert_yaml_snapshot!("ast_simple_query", expr);
}

#[test]
fn snapshot_ast_query_with_where() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "[Observation] O where O.status = 'final'"
    ).unwrap();
    assert_yaml_snapshot!("ast_query_with_where", expr);
}

#[test]
fn snapshot_ast_query_complex() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "[Observation] O
         where O.status = 'final'
         return Tuple { code: O.code, value: O.value }
         sort by O.effectiveDateTime desc"
    ).unwrap();
    assert_yaml_snapshot!("ast_complex_query", expr);
}

#[test]
fn snapshot_ast_list() {
    let parser = Parser::new();

    let expr = parser.parse_expression("{1, 2, 3, 4, 5}").unwrap();
    assert_yaml_snapshot!("ast_integer_list", expr);

    let expr = parser.parse_expression("{}").unwrap();
    assert_yaml_snapshot!("ast_empty_list", expr);
}

#[test]
fn snapshot_ast_tuple() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "Tuple { id: '123', value: 42, active: true }"
    ).unwrap();
    assert_yaml_snapshot!("ast_tuple", expr);
}

#[test]
fn snapshot_ast_interval() {
    let parser = Parser::new();

    let expr = parser.parse_expression("Interval[1, 10]").unwrap();
    assert_yaml_snapshot!("ast_closed_interval", expr);

    let expr = parser.parse_expression("Interval(1, 10)").unwrap();
    assert_yaml_snapshot!("ast_open_interval", expr);

    let expr = parser.parse_expression("Interval[1, 10)").unwrap();
    assert_yaml_snapshot!("ast_half_open_interval", expr);
}

// === Library Snapshots ===

#[test]
fn snapshot_library_minimal() {
    let parser = Parser::new();
    let lib = parser.parse_library("library Test version '1.0.0'").unwrap();
    assert_yaml_snapshot!("library_minimal", lib);
}

#[test]
fn snapshot_library_simple() {
    let parser = Parser::new();
    let lib = parser.parse_library(
        "library Test version '1.0.0'
         define \"Value\": 42
         define \"Result\": \"Value\" * 2"
    ).unwrap();
    assert_yaml_snapshot!("library_simple", lib);
}

#[test]
fn snapshot_library_with_using() {
    let parser = Parser::new();
    let lib = parser.parse_library(
        "library Test version '1.0.0'
         using FHIR version '4.0.1'
         context Patient
         define \"PatientAge\": AgeInYears()"
    ).unwrap();
    assert_yaml_snapshot!("library_with_using", lib);
}

#[test]
fn snapshot_library_with_valueset() {
    let parser = Parser::new();
    let lib = parser.parse_library(
        "library Test version '1.0.0'
         codesystem \"LOINC\": 'http://loinc.org'
         valueset \"Blood Pressure\": 'http://example.org/fhir/ValueSet/bp'
         code \"Systolic BP\": '8480-6' from \"LOINC\""
    ).unwrap();
    assert_yaml_snapshot!("library_with_valueset", lib);
}

#[test]
fn snapshot_library_realistic() {
    let parser = Parser::new();
    let lib = parser.parse_library(
        r#"library CMS146 version '2.0.0'

        using FHIR version '4.0.1'

        include FHIRHelpers version '4.0.1' called FH

        codesystem "LOINC": 'http://loinc.org'

        valueset "Pharyngitis": 'http://cts.nlm.nih.gov/fhir/ValueSet/pharyngitis'

        parameter "Measurement Period" Interval<DateTime>

        context Patient

        define "In Demographic":
          AgeInYearsAt(start of "Measurement Period") >= 2
            and AgeInYearsAt(start of "Measurement Period") < 18

        define "Pharyngitis Encounters":
          [Encounter: "Pharyngitis"] E
            where E.period during "Measurement Period"
              and E.status = 'finished'"#
    ).unwrap();
    assert_yaml_snapshot!("library_realistic", lib);
}

// === Error Message Snapshots ===

#[test]
fn snapshot_error_unclosed_string() {
    let parser = Parser::new();
    let result = parser.parse_expression("'unclosed string");
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_snapshot!("error_unclosed_string", format!("{:?}", errors));
}

#[test]
fn snapshot_error_invalid_operator() {
    let parser = Parser::new();
    let result = parser.parse_expression("1 + + 2");
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_snapshot!("error_invalid_operator", format!("{:?}", errors));
}

#[test]
fn snapshot_error_unmatched_paren() {
    let parser = Parser::new();
    let result = parser.parse_expression("(1 + 2");
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_snapshot!("error_unmatched_paren", format!("{:?}", errors));
}

// === Complex Expression Snapshots ===

#[test]
fn snapshot_nested_query() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "[Patient] P
         return {
           patient: P,
           observations: (
             [Observation] O
               where O.subject.reference = 'Patient/' + P.id
               return O.value
           )
         }"
    ).unwrap();
    assert_yaml_snapshot!("nested_query", expr);
}

#[test]
fn snapshot_function_call() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "AgeInYearsAt(@2024-01-01)"
    ).unwrap();
    assert_yaml_snapshot!("function_call", expr);
}

#[test]
fn snapshot_member_access_chain() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "Patient.name[0].given[0]"
    ).unwrap();
    assert_yaml_snapshot!("member_access_chain", expr);
}

#[test]
fn snapshot_exists_expression() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "exists ([Observation] O where O.code = '8480-6')"
    ).unwrap();
    assert_yaml_snapshot!("exists_expression", expr);
}

#[test]
fn snapshot_between_expression() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "x between 1 and 10"
    ).unwrap();
    assert_yaml_snapshot!("between_expression", expr);
}

#[test]
fn snapshot_case_expression() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "case
           when x < 0 then 'negative'
           when x = 0 then 'zero'
           else 'positive'
         end"
    ).unwrap();
    assert_yaml_snapshot!("case_expression", expr);
}

#[test]
fn snapshot_multi_source_query() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "from [Encounter] E,
         [Observation] O
         where O.encounter = E.id
         return Tuple { encounter: E, observation: O }"
    ).unwrap();
    assert_yaml_snapshot!("multi_source_query", expr);
}

#[test]
fn snapshot_with_relationship() {
    let parser = Parser::new();
    let expr = parser.parse_expression(
        "[Encounter] E
         with [Observation] O
           such that O.encounter = E.id
         return E"
    ).unwrap();
    assert_yaml_snapshot!("with_relationship", expr);
}
