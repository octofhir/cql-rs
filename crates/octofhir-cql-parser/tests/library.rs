//! Tests for parsing complete CQL libraries
//!
//! Covers:
//! - Library declaration
//! - Using declarations (models, code systems, value sets, libraries)
//! - Include statements
//! - Code system definitions
//! - Value set definitions
//! - Code definitions
//! - Concept definitions
//! - Parameter definitions
//! - Context declarations
//! - Define statements (expressions and functions)

use octofhir_cql_ast::*;
use octofhir_cql_parser::Parser;
use pretty_assertions::assert_eq;

fn parse_lib(input: &str) -> Library {
    let parser = Parser::new();
    parser
        .parse_library(input)
        .unwrap_or_else(|e| panic!("Failed to parse library: {:?}", e))
}

// === Library Declaration ===

#[test]
fn test_minimal_library() {
    let lib = parse_lib(
        "library Test version '1.0.0'"
    );

    assert_eq!(lib.identifier.id, "Test");
    assert_eq!(lib.identifier.version, Some("1.0.0".to_string()));
}

#[test]
fn test_library_without_version() {
    let lib = parse_lib(
        "library Test"
    );

    assert_eq!(lib.identifier.id, "Test");
    assert_eq!(lib.identifier.version, None);
}

#[test]
fn test_library_with_qualified_name() {
    let lib = parse_lib(
        "library org.example.Test version '1.0.0'"
    );

    assert!(lib.identifier.id.contains("Test"));
}

// === Using Declarations ===

#[test]
fn test_using_fhir_model() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         using FHIR version '4.0.1'"
    );

    assert_eq!(lib.usings.len(), 1);
    assert_eq!(lib.usings[0].local_identifier, "FHIR");
    assert_eq!(lib.usings[0].version, Some("4.0.1".to_string()));
}

#[test]
fn test_multiple_using_declarations() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         using FHIR version '4.0.1'
         using QDM version '5.6'"
    );

    assert_eq!(lib.usings.len(), 2);
    assert_eq!(lib.usings[0].local_identifier, "FHIR");
    assert_eq!(lib.usings[1].local_identifier, "QDM");
}

// === Include Statements ===

#[test]
fn test_include_statement() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         include FHIRHelpers version '4.0.1'"
    );

    assert_eq!(lib.includes.len(), 1);
    assert_eq!(lib.includes[0].local_identifier, "FHIRHelpers");
    assert_eq!(lib.includes[0].version, Some("4.0.1".to_string()));
}

#[test]
fn test_include_with_alias() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         include FHIRHelpers version '4.0.1' called FH"
    );

    assert_eq!(lib.includes.len(), 1);
    assert_eq!(lib.includes[0].local_identifier, "FHIRHelpers");
    assert_eq!(lib.includes[0].called, Some("FH".to_string()));
}

#[test]
fn test_multiple_includes() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         include FHIRHelpers version '4.0.1'
         include Common version '1.0.0' called C"
    );

    assert_eq!(lib.includes.len(), 2);
}

// === Code System Definitions ===

#[test]
fn test_codesystem_definition() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         codesystem \"LOINC\": 'http://loinc.org'"
    );

    assert_eq!(lib.code_systems.len(), 1);
    assert_eq!(lib.code_systems[0].name, "LOINC");
    assert_eq!(lib.code_systems[0].id, "http://loinc.org");
}

#[test]
fn test_codesystem_with_version() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         codesystem \"LOINC\": 'http://loinc.org' version '2.73'"
    );

    assert_eq!(lib.code_systems.len(), 1);
    assert_eq!(lib.code_systems[0].version, Some("2.73".to_string()));
}

#[test]
fn test_multiple_codesystems() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         codesystem \"LOINC\": 'http://loinc.org'
         codesystem \"SNOMED\": 'http://snomed.info/sct'"
    );

    assert_eq!(lib.code_systems.len(), 2);
}

// === Value Set Definitions ===

#[test]
fn test_valueset_definition() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         valueset \"Blood Pressure\": 'http://example.org/fhir/ValueSet/blood-pressure'"
    );

    assert_eq!(lib.value_sets.len(), 1);
    assert_eq!(lib.value_sets[0].name, "Blood Pressure");
    assert_eq!(lib.value_sets[0].id, "http://example.org/fhir/ValueSet/blood-pressure");
}

#[test]
fn test_valueset_with_version() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         valueset \"Blood Pressure\": 'http://example.org/fhir/ValueSet/blood-pressure' version '1.0.0'"
    );

    assert_eq!(lib.value_sets.len(), 1);
    assert_eq!(lib.value_sets[0].version, Some("1.0.0".to_string()));
}

// === Code Definitions ===

#[test]
fn test_code_definition() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         codesystem \"LOINC\": 'http://loinc.org'
         code \"Systolic BP\": '8480-6' from \"LOINC\""
    );

    assert_eq!(lib.codes.len(), 1);
    assert_eq!(lib.codes[0].name, "Systolic BP");
    assert_eq!(lib.codes[0].code, "8480-6");
}

#[test]
fn test_code_with_display() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         codesystem \"LOINC\": 'http://loinc.org'
         code \"Systolic BP\": '8480-6' from \"LOINC\" display 'Systolic blood pressure'"
    );

    assert_eq!(lib.codes.len(), 1);
    assert_eq!(lib.codes[0].display, Some("Systolic blood pressure".to_string()));
}

// === Concept Definitions ===

#[test]
fn test_concept_definition() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         codesystem \"LOINC\": 'http://loinc.org'
         code \"Systolic BP LOINC\": '8480-6' from \"LOINC\"
         concept \"Blood Pressure Systolic\": { \"Systolic BP LOINC\" }"
    );

    assert_eq!(lib.concepts.len(), 1);
    assert_eq!(lib.concepts[0].name, "Blood Pressure Systolic");
}

// === Parameter Definitions ===

#[test]
fn test_parameter_definition() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         parameter \"Measurement Period\" Interval<DateTime>"
    );

    assert_eq!(lib.parameters.len(), 1);
    assert_eq!(lib.parameters[0].name, "Measurement Period");
}

#[test]
fn test_parameter_with_default() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         parameter \"Minimum Age\" Integer default 18"
    );

    assert_eq!(lib.parameters.len(), 1);
    assert_eq!(lib.parameters[0].name, "Minimum Age");
    assert!(lib.parameters[0].default.is_some());
}

#[test]
fn test_multiple_parameters() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         parameter \"Measurement Period\" Interval<DateTime>
         parameter \"Minimum Age\" Integer default 18
         parameter \"Include Active\" Boolean default true"
    );

    assert_eq!(lib.parameters.len(), 3);
}

// === Context Declaration ===

#[test]
fn test_context_patient() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         using FHIR version '4.0.1'
         context Patient"
    );

    assert_eq!(lib.statements.context, Some("Patient".to_string()));
}

#[test]
fn test_context_population() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         context Population"
    );

    assert_eq!(lib.statements.context, Some("Population".to_string()));
}

// === Define Statements ===

#[test]
fn test_simple_define() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         define \"Patient Age\": 35"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert_eq!(lib.statements.defs[0].name, "Patient Age");
    assert!(!lib.statements.defs[0].is_function);
}

#[test]
fn test_define_with_expression() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         define \"Is Adult\": AgeInYears() >= 18"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert_eq!(lib.statements.defs[0].name, "Is Adult");
}

#[test]
fn test_define_with_query() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         using FHIR version '4.0.1'

         define \"Active Conditions\":
           [Condition] C
             where C.clinicalStatus = 'active'"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert_eq!(lib.statements.defs[0].name, "Active Conditions");
}

#[test]
fn test_define_function() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         define function \"Add\"(a Integer, b Integer):
           a + b"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert_eq!(lib.statements.defs[0].name, "Add");
    assert!(lib.statements.defs[0].is_function);
    assert_eq!(lib.statements.defs[0].operands.len(), 2);
}

#[test]
fn test_define_function_multiple_params() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         define function \"CalculateBMI\"(weight Decimal, height Decimal):
           weight / (height * height)"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert!(lib.statements.defs[0].is_function);
    assert_eq!(lib.statements.defs[0].operands.len(), 2);
}

#[test]
fn test_public_define() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         public define \"Patient Age\": 35"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert_eq!(lib.statements.defs[0].access_level, AccessModifier::Public);
}

#[test]
fn test_private_define() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         private define \"Internal Calculation\": 42"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert_eq!(lib.statements.defs[0].access_level, AccessModifier::Private);
}

#[test]
fn test_fluent_define() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         define fluent function \"double\"(value Integer):
           value * 2"
    );

    assert_eq!(lib.statements.defs.len(), 1);
    assert!(lib.statements.defs[0].fluent);
}

// === Complete Library Examples ===

#[test]
fn test_complete_library_structure() {
    let lib = parse_lib(
        "library CMS146 version '2.0.0'

         using FHIR version '4.0.1'

         include FHIRHelpers version '4.0.1' called FH

         codesystem \"LOINC\": 'http://loinc.org'
         codesystem \"SNOMED\": 'http://snomed.info/sct'

         valueset \"Pharyngitis\": 'http://cts.nlm.nih.gov/fhir/ValueSet/2.16.840.1.113883.3.464.1003.102.12.1011'

         code \"Strep Test\": '6557-3' from \"LOINC\"

         parameter \"Measurement Period\" Interval<DateTime>

         context Patient

         define \"In Demographic\":
           AgeInYearsAt(start of \"Measurement Period\") >= 2
             and AgeInYearsAt(start of \"Measurement Period\") < 18

         define \"Pharyngitis Encounters\":
           [Encounter: \"Pharyngitis\"] E
             where E.period during \"Measurement Period\"
               and E.status = 'finished'"
    );

    // Verify all sections
    assert_eq!(lib.identifier.id, "CMS146");
    assert_eq!(lib.usings.len(), 1);
    assert_eq!(lib.includes.len(), 1);
    assert_eq!(lib.code_systems.len(), 2);
    assert_eq!(lib.value_sets.len(), 1);
    assert_eq!(lib.codes.len(), 1);
    assert_eq!(lib.parameters.len(), 1);
    assert_eq!(lib.statements.context, Some("Patient".to_string()));
    assert_eq!(lib.statements.defs.len(), 2);
}

#[test]
fn test_library_with_comments() {
    let lib = parse_lib(
        "// This is a test library
         library Test version '1.0.0'

         // Patient context
         context Patient

         // Calculate patient age
         define \"Patient Age\": AgeInYears()"
    );

    assert_eq!(lib.identifier.id, "Test");
    assert_eq!(lib.statements.defs.len(), 1);
}

#[test]
fn test_library_with_multiline_comments() {
    let lib = parse_lib(
        "/*
          Test Library
          Version 1.0.0
          Author: Test
         */
         library Test version '1.0.0'

         /*
          Calculate patient age in years
         */
         define \"Patient Age\": AgeInYears()"
    );

    assert_eq!(lib.identifier.id, "Test");
    assert_eq!(lib.statements.defs.len(), 1);
}

#[test]
fn test_library_with_annotations() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         @description: 'Calculates patient age'
         @version: '1.0.0'
         define \"Patient Age\": AgeInYears()"
    );

    // Annotations should be preserved if the parser supports them
    assert_eq!(lib.statements.defs.len(), 1);
}

#[test]
fn test_empty_library() {
    let lib = parse_lib(
        "library Test version '1.0.0'"
    );

    assert_eq!(lib.identifier.id, "Test");
    assert!(lib.statements.defs.is_empty());
}

#[test]
fn test_library_define_order() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         define \"A\": 1
         define \"B\": \"A\" + 1
         define \"C\": \"B\" + 1"
    );

    assert_eq!(lib.statements.defs.len(), 3);
    assert_eq!(lib.statements.defs[0].name, "A");
    assert_eq!(lib.statements.defs[1].name, "B");
    assert_eq!(lib.statements.defs[2].name, "C");
}

#[test]
fn test_library_with_complex_types() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         parameter \"Intervals\" List<Interval<DateTime>>
         parameter \"Codes\" List<Code>
         parameter \"Tuples\" List<Tuple { id: String, value: Integer }>"
    );

    assert_eq!(lib.parameters.len(), 3);
}

#[test]
fn test_qualified_identifier_reference() {
    let lib = parse_lib(
        "library Test version '1.0.0'

         include Common version '1.0.0' called C

         define \"Use Common Function\":
           C.\"HelperFunction\"()"
    );

    assert_eq!(lib.includes.len(), 1);
    assert_eq!(lib.statements.defs.len(), 1);
}
