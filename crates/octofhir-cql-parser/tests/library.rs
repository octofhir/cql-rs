//! Tests for parsing complete CQL libraries
//!
//! Covers:
//! - Library declaration
//! - Using declarations
//! - Context declarations
//! - Parameter definitions
//! - Define statements

use octofhir_cql_ast::Library;
use octofhir_cql_parser::parse;
use rstest::rstest;

fn parse_lib(input: &str) -> Library {
    parse(input).unwrap_or_else(|e| panic!("Failed to parse library: {:?}", e))
}

// === Library Declaration ===

#[test]
fn test_minimal_library() {
    let lib = parse_lib("library Test version '1.0.0'");

    assert!(lib.definition.is_some());
    let def = lib.definition.unwrap();
    assert_eq!(def.name.name.name, "Test");
    assert_eq!(def.version.as_ref().map(|v| v.version.as_str()), Some("1.0.0"));
}

#[test]
fn test_library_without_version() {
    let lib = parse_lib("library Test");

    assert!(lib.definition.is_some());
    let def = lib.definition.unwrap();
    assert_eq!(def.name.name.name, "Test");
    assert_eq!(def.version, None);
}

// === Using Declarations ===

#[test]
fn test_using_fhir() {
    let lib = parse_lib(r#"
        library Test
        using FHIR version '4.0.1'
    "#);

    assert_eq!(lib.usings.len(), 1);
    assert_eq!(lib.usings[0].inner.model.name, "FHIR");
    assert_eq!(lib.usings[0].inner.version.as_ref().map(|v| v.version.as_str()), Some("4.0.1"));
}

#[test]
fn test_using_without_version() {
    let lib = parse_lib(r#"
        library Test
        using FHIR
    "#);

    assert_eq!(lib.usings.len(), 1);
    assert_eq!(lib.usings[0].inner.model.name, "FHIR");
    assert_eq!(lib.usings[0].inner.version, None);
}

// === Context Declarations ===

#[test]
fn test_context_patient() {
    let lib = parse_lib(r#"
        library Test
        context Patient
    "#);

    assert_eq!(lib.contexts.len(), 1);
    assert_eq!(lib.contexts[0].inner.context.name, "Patient");
}

// === Define Statements ===

#[test]
fn test_define_simple_expression() {
    let lib = parse_lib(r#"
        library Test
        define IsAdult: true
    "#);

    assert_eq!(lib.statements.len(), 1);
}

#[test]
fn test_define_with_boolean_expression() {
    let lib = parse_lib(r#"
        library Test
        define IsAdult: age >= 18
    "#);

    assert_eq!(lib.statements.len(), 1);
}

#[test]
fn test_multiple_defines() {
    let lib = parse_lib(r#"
        library Test
        define First: 1
        define Second: 2
        define Third: 3
    "#);

    assert_eq!(lib.statements.len(), 3);
}

// === Complete Libraries ===

#[test]
fn test_complete_library() {
    let lib = parse_lib(r#"
        library AdultCheck version '1.0.0'
        using FHIR version '4.0.1'
        context Patient
        define IsAdult: age >= 18
        define HasName: exists name
    "#);

    assert!(lib.definition.is_some());
    assert_eq!(lib.usings.len(), 1);
    assert_eq!(lib.contexts.len(), 1);
    assert_eq!(lib.statements.len(), 2);
}

// === Various Valid Libraries ===

#[rstest]
#[case("library Test", true)]
#[case("library Test version '1.0'", true)]
#[case("library Test\ndefine X: 1", true)]
#[case("library Test\nusing FHIR\ndefine X: 1", true)]
fn test_library_variations(#[case] input: &str, #[case] should_parse: bool) {
    let result = parse(input);
    assert_eq!(result.is_ok(), should_parse, "Parse result for library unexpected");
}
