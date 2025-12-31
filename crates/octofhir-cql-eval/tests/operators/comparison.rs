//! Comparison Operator Tests
//!
//! Tests for: Equal, NotEqual, Equivalent, Less, Greater, LessOrEqual, GreaterOrEqual
//! All operators implement three-valued logic (true/false/null)

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_eval::operators::comparison::{cql_compare, cql_equal, cql_equivalent};
use octofhir_cql_elm::{BinaryExpression, Element, Expression, Literal, NullLiteral};
use octofhir_cql_types::{CqlCode, CqlDate, CqlDateTime, CqlList, CqlQuantity, CqlTime, CqlType, CqlValue};
use rust_decimal::Decimal;
use std::cmp::Ordering;

// ============================================================================
// Test Helpers
// ============================================================================

fn engine() -> CqlEngine {
    CqlEngine::new()
}

fn ctx() -> EvaluationContext {
    EvaluationContext::new()
}

fn int_expr(i: i32) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
        value: Some(i.to_string()),
    }))
}

fn decimal_expr(d: &str) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Decimal".to_string(),
        value: Some(d.to_string()),
    }))
}

fn string_expr(s: &str) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
        value: Some(s.to_string()),
    }))
}

fn bool_expr(b: bool) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
        value: Some(b.to_string()),
    }))
}

fn null_expr() -> Box<Expression> {
    Box::new(Expression::Null(NullLiteral { element: Element::default() }))
}

fn make_binary(left: Box<Expression>, right: Box<Expression>) -> BinaryExpression {
    BinaryExpression {
        element: Element::default(),
        operand: vec![left, right],
    }
}

// ============================================================================
// Equal (=) Tests - Three-valued logic
// ============================================================================

#[test]
fn test_equal_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_equal(&make_binary(int_expr(5), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_equal_integers_false() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_equal(&make_binary(int_expr(5), int_expr(6)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_equal_cross_type_numeric() {
    let e = engine();
    let mut c = ctx();

    // Integer 5 equals Decimal 5.0
    let result = e.eval_equal(&make_binary(int_expr(5), decimal_expr("5.0")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_equal_strings() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_equal(&make_binary(string_expr("hello"), string_expr("hello")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_equal_strings_case_sensitive() {
    let e = engine();
    let mut c = ctx();

    // Equal is case-sensitive
    let result = e.eval_equal(&make_binary(string_expr("Hello"), string_expr("hello")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_equal_null_left_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_equal(&make_binary(null_expr(), int_expr(5)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_equal_null_right_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_equal(&make_binary(int_expr(5), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_equal_both_null_returns_null() {
    let e = engine();
    let mut c = ctx();

    // Per CQL spec, null = null returns null, not true
    let result = e.eval_equal(&make_binary(null_expr(), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_equal_booleans() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_equal(&make_binary(bool_expr(true), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));

    let result = e.eval_equal(&make_binary(bool_expr(true), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

// ============================================================================
// NotEqual (!=) Tests
// ============================================================================

#[test]
fn test_not_equal_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_not_equal(&make_binary(int_expr(5), int_expr(6)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_not_equal_same_value() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_not_equal(&make_binary(int_expr(5), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_not_equal_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_not_equal(&make_binary(int_expr(5), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Equivalent (~) Tests
// ============================================================================

#[test]
fn test_equivalent_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_equivalent(&make_binary(int_expr(5), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_equivalent_null_null_returns_true() {
    let e = engine();
    let mut c = ctx();

    // Unlike Equal, Equivalent treats null ~ null as true
    let result = e.eval_equivalent(&make_binary(null_expr(), null_expr()), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_equivalent_null_value_returns_false() {
    let e = engine();
    let mut c = ctx();

    // null ~ non-null returns false
    let result = e.eval_equivalent(&make_binary(null_expr(), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_equivalent_strings_case_insensitive() {
    let e = engine();
    let mut c = ctx();

    // Equivalent is case-insensitive for strings
    let result = e.eval_equivalent(&make_binary(string_expr("Hello"), string_expr("HELLO")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

// ============================================================================
// cql_equal function tests
// ============================================================================

#[test]
fn test_cql_equal_integers() {
    assert_eq!(cql_equal(&CqlValue::Integer(5), &CqlValue::Integer(5)).unwrap(), Some(true));
    assert_eq!(cql_equal(&CqlValue::Integer(5), &CqlValue::Integer(6)).unwrap(), Some(false));
}

#[test]
fn test_cql_equal_cross_type_numeric() {
    assert_eq!(cql_equal(&CqlValue::Integer(5), &CqlValue::Long(5)).unwrap(), Some(true));
    assert_eq!(cql_equal(&CqlValue::Integer(5), &CqlValue::Decimal(Decimal::from(5))).unwrap(), Some(true));
    assert_eq!(cql_equal(&CqlValue::Long(5), &CqlValue::Decimal(Decimal::from(5))).unwrap(), Some(true));
}

#[test]
fn test_cql_equal_dates() {
    let d1 = CqlValue::Date(CqlDate::new(2024, 1, 15));
    let d2 = CqlValue::Date(CqlDate::new(2024, 1, 15));
    let d3 = CqlValue::Date(CqlDate::new(2024, 1, 16));

    assert_eq!(cql_equal(&d1, &d2).unwrap(), Some(true));
    assert_eq!(cql_equal(&d1, &d3).unwrap(), Some(false));
}

#[test]
fn test_cql_equal_lists() {
    let list1 = CqlValue::List(CqlList {
        element_type: CqlType::Integer,
        elements: vec![CqlValue::Integer(1), CqlValue::Integer(2)],
    });
    let list2 = CqlValue::List(CqlList {
        element_type: CqlType::Integer,
        elements: vec![CqlValue::Integer(1), CqlValue::Integer(2)],
    });
    let list3 = CqlValue::List(CqlList {
        element_type: CqlType::Integer,
        elements: vec![CqlValue::Integer(1), CqlValue::Integer(3)],
    });

    assert_eq!(cql_equal(&list1, &list2).unwrap(), Some(true));
    assert_eq!(cql_equal(&list1, &list3).unwrap(), Some(false));
}

#[test]
fn test_cql_equal_lists_different_length() {
    let list1 = CqlValue::List(CqlList {
        element_type: CqlType::Integer,
        elements: vec![CqlValue::Integer(1), CqlValue::Integer(2)],
    });
    let list2 = CqlValue::List(CqlList {
        element_type: CqlType::Integer,
        elements: vec![CqlValue::Integer(1)],
    });

    assert_eq!(cql_equal(&list1, &list2).unwrap(), Some(false));
}

#[test]
fn test_cql_equal_codes() {
    let code1 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", Some("1.0"), Some("Test")));
    let code2 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", Some("1.0"), Some("Test")));
    let code3 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", Some("2.0"), Some("Test")));

    // Equal requires all fields to match including version
    assert_eq!(cql_equal(&code1, &code2).unwrap(), Some(true));
    assert_eq!(cql_equal(&code1, &code3).unwrap(), Some(false));
}

// ============================================================================
// cql_equivalent function tests
// ============================================================================

#[test]
fn test_cql_equivalent_codes_ignores_version() {
    let code1 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", Some("1.0"), Some("Test")));
    let code2 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", Some("2.0"), Some("Different")));

    // Equivalent only compares code and system
    assert!(cql_equivalent(&code1, &code2).unwrap());
}

#[test]
fn test_cql_equivalent_codes_different_code() {
    let code1 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", None::<String>, None::<String>));
    let code2 = CqlValue::Code(CqlCode::new("456", "http://snomed.info/sct", None::<String>, None::<String>));

    assert!(!cql_equivalent(&code1, &code2).unwrap());
}

#[test]
fn test_cql_equivalent_strings() {
    // Strings are case-insensitive for equivalence
    assert!(cql_equivalent(
        &CqlValue::String("Hello".to_string()),
        &CqlValue::String("HELLO".to_string())
    ).unwrap());

    assert!(cql_equivalent(
        &CqlValue::String("Test".to_string()),
        &CqlValue::String("test".to_string())
    ).unwrap());
}

// ============================================================================
// Less (<) Tests
// ============================================================================

#[test]
fn test_less_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less(&make_binary(int_expr(3), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_less_integers_false() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less(&make_binary(int_expr(5), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_less_integers_equal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less(&make_binary(int_expr(5), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_less_decimals() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less(&make_binary(decimal_expr("2.5"), decimal_expr("3.5")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_less_strings() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less(&make_binary(string_expr("abc"), string_expr("abd")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_less_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less(&make_binary(int_expr(3), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Greater (>) Tests
// ============================================================================

#[test]
fn test_greater_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater(&make_binary(int_expr(5), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_greater_integers_false() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater(&make_binary(int_expr(3), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_greater_integers_equal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater(&make_binary(int_expr(5), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_greater_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater(&make_binary(null_expr(), int_expr(3)), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// LessOrEqual (<=) Tests
// ============================================================================

#[test]
fn test_less_or_equal_less() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less_or_equal(&make_binary(int_expr(3), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_less_or_equal_equal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less_or_equal(&make_binary(int_expr(5), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_less_or_equal_greater() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less_or_equal(&make_binary(int_expr(5), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_less_or_equal_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_less_or_equal(&make_binary(int_expr(5), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// GreaterOrEqual (>=) Tests
// ============================================================================

#[test]
fn test_greater_or_equal_greater() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater_or_equal(&make_binary(int_expr(5), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_greater_or_equal_equal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater_or_equal(&make_binary(int_expr(5), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_greater_or_equal_less() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater_or_equal(&make_binary(int_expr(3), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_greater_or_equal_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_greater_or_equal(&make_binary(null_expr(), int_expr(3)), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// cql_compare function tests
// ============================================================================

#[test]
fn test_cql_compare_integers() {
    assert_eq!(
        cql_compare(&CqlValue::Integer(5), &CqlValue::Integer(3)).unwrap(),
        Some(Ordering::Greater)
    );
    assert_eq!(
        cql_compare(&CqlValue::Integer(3), &CqlValue::Integer(5)).unwrap(),
        Some(Ordering::Less)
    );
    assert_eq!(
        cql_compare(&CqlValue::Integer(5), &CqlValue::Integer(5)).unwrap(),
        Some(Ordering::Equal)
    );
}

#[test]
fn test_cql_compare_decimals() {
    assert_eq!(
        cql_compare(&CqlValue::Decimal(Decimal::from_str_exact("5.5").unwrap()), &CqlValue::Decimal(Decimal::from_str_exact("3.3").unwrap())).unwrap(),
        Some(Ordering::Greater)
    );
}

#[test]
fn test_cql_compare_cross_type_numeric() {
    assert_eq!(
        cql_compare(&CqlValue::Integer(5), &CqlValue::Decimal(Decimal::from_str_exact("3.0").unwrap())).unwrap(),
        Some(Ordering::Greater)
    );
}

#[test]
fn test_cql_compare_strings() {
    assert_eq!(
        cql_compare(&CqlValue::String("abc".to_string()), &CqlValue::String("abd".to_string())).unwrap(),
        Some(Ordering::Less)
    );
}

#[test]
fn test_cql_compare_quantities_same_unit() {
    let q1 = CqlValue::Quantity(CqlQuantity {
        value: Decimal::from(5),
        unit: Some("kg".to_string()),
    });
    let q2 = CqlValue::Quantity(CqlQuantity {
        value: Decimal::from(3),
        unit: Some("kg".to_string()),
    });

    assert_eq!(cql_compare(&q1, &q2).unwrap(), Some(Ordering::Greater));
}
