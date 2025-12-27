//! Arithmetic Operator Tests
//!
//! Tests for: Add, Subtract, Multiply, Divide, TruncatedDivide, Modulo, Power,
//! Negate, Successor, Predecessor, Abs, Ceiling, Floor, Round, Truncate,
//! Exp, Ln, Log, MinValue, MaxValue

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_elm::{BinaryExpression, Element, Expression, Literal, NullLiteral, UnaryExpression};
use octofhir_cql_types::{CqlQuantity, CqlValue};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

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

fn long_expr(l: i64) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Long".to_string(),
        value: Some(l.to_string()),
    }))
}

fn decimal_expr(d: &str) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Decimal".to_string(),
        value: Some(d.to_string()),
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

fn make_unary(operand: Box<Expression>) -> UnaryExpression {
    UnaryExpression {
        element: Element::default(),
        operand,
    }
}

// ============================================================================
// Add Tests
// ============================================================================

#[test]
fn test_add_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(int_expr(2), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_add_negative_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(int_expr(-5), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-2));
}

#[test]
fn test_add_longs() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(long_expr(1000000000000), long_expr(2000000000000)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Long(3000000000000));
}

#[test]
fn test_add_integer_and_long() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(int_expr(5), long_expr(10)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Long(15));
}

#[test]
fn test_add_decimals() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(decimal_expr("1.5"), decimal_expr("2.5")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from_str_exact("4.0").unwrap()));
}

#[test]
fn test_add_integer_and_decimal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(int_expr(5), decimal_expr("2.5")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from_str_exact("7.5").unwrap()));
}

#[test]
fn test_add_null_left() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(null_expr(), int_expr(5)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_add_null_right() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(int_expr(5), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_add_both_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_add(&make_binary(null_expr(), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Subtract Tests
// ============================================================================

#[test]
fn test_subtract_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_subtract(&make_binary(int_expr(10), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(7));
}

#[test]
fn test_subtract_negative_result() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_subtract(&make_binary(int_expr(3), int_expr(10)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-7));
}

#[test]
fn test_subtract_decimals() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_subtract(&make_binary(decimal_expr("5.5"), decimal_expr("2.3")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from_str_exact("3.2").unwrap()));
}

#[test]
fn test_subtract_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_subtract(&make_binary(int_expr(10), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Multiply Tests
// ============================================================================

#[test]
fn test_multiply_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_multiply(&make_binary(int_expr(4), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(20));
}

#[test]
fn test_multiply_by_zero() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_multiply(&make_binary(int_expr(100), int_expr(0)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_multiply_negative() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_multiply(&make_binary(int_expr(-3), int_expr(4)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-12));
}

#[test]
fn test_multiply_decimals() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_multiply(&make_binary(decimal_expr("2.5"), decimal_expr("4.0")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from_str_exact("10.0").unwrap()));
}

#[test]
fn test_multiply_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_multiply(&make_binary(null_expr(), int_expr(5)), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Divide Tests
// ============================================================================

#[test]
fn test_divide_integers() {
    let e = engine();
    let mut c = ctx();

    // Integer division always returns Decimal
    let result = e.eval_divide(&make_binary(int_expr(10), int_expr(4)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from_str_exact("2.5").unwrap()));
}

#[test]
fn test_divide_exact() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_divide(&make_binary(int_expr(10), int_expr(2)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from(5)));
}

#[test]
fn test_divide_by_zero_returns_null() {
    let e = engine();
    let mut c = ctx();

    // CQL specifies division by zero returns null
    let result = e.eval_divide(&make_binary(int_expr(10), int_expr(0)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_divide_decimals() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_divide(&make_binary(decimal_expr("7.5"), decimal_expr("2.5")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from(3)));
}

#[test]
fn test_divide_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_divide(&make_binary(int_expr(10), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// TruncatedDivide (div) Tests
// ============================================================================

#[test]
fn test_truncated_divide_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_truncated_divide(&make_binary(int_expr(10), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_truncated_divide_negative() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_truncated_divide(&make_binary(int_expr(-10), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-3));
}

#[test]
fn test_truncated_divide_by_zero_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_truncated_divide(&make_binary(int_expr(10), int_expr(0)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_truncated_divide_longs() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_truncated_divide(&make_binary(long_expr(100), long_expr(7)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Long(14));
}

// ============================================================================
// Modulo (mod) Tests
// ============================================================================

#[test]
fn test_modulo_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_modulo(&make_binary(int_expr(10), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(1));
}

#[test]
fn test_modulo_even_division() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_modulo(&make_binary(int_expr(10), int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_modulo_by_zero_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_modulo(&make_binary(int_expr(10), int_expr(0)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_modulo_negative() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_modulo(&make_binary(int_expr(-10), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-1));
}

// ============================================================================
// Power (^) Tests
// ============================================================================

#[test]
fn test_power_integers() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_power(&make_binary(int_expr(2), int_expr(3)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(8));
}

#[test]
fn test_power_zero() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_power(&make_binary(int_expr(5), int_expr(0)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(1));
}

#[test]
fn test_power_one() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_power(&make_binary(int_expr(5), int_expr(1)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_power_null_propagation() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_power(&make_binary(int_expr(2), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Negate Tests
// ============================================================================

#[test]
fn test_negate_positive() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_negate(&make_unary(int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-5));
}

#[test]
fn test_negate_negative() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_negate(&make_unary(int_expr(-5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_negate_zero() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_negate(&make_unary(int_expr(0)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_negate_decimal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_negate(&make_unary(decimal_expr("3.14")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from_str_exact("-3.14").unwrap()));
}

#[test]
fn test_negate_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_negate(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Abs Tests
// ============================================================================

#[test]
fn test_abs_positive() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_abs(&make_unary(int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_abs_negative() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_abs(&make_unary(int_expr(-5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_abs_zero() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_abs(&make_unary(int_expr(0)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_abs_decimal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_abs(&make_unary(decimal_expr("-3.14")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Decimal(Decimal::from_str_exact("3.14").unwrap()));
}

#[test]
fn test_abs_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_abs(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Ceiling Tests
// ============================================================================

#[test]
fn test_ceiling_positive_decimal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ceiling(&make_unary(decimal_expr("3.1")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(4));
}

#[test]
fn test_ceiling_negative_decimal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ceiling(&make_unary(decimal_expr("-3.1")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-3));
}

#[test]
fn test_ceiling_whole_number() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ceiling(&make_unary(decimal_expr("3.0")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_ceiling_integer() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ceiling(&make_unary(int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_ceiling_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ceiling(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Floor Tests
// ============================================================================

#[test]
fn test_floor_positive_decimal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_floor(&make_unary(decimal_expr("3.9")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_floor_negative_decimal() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_floor(&make_unary(decimal_expr("-3.1")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-4));
}

#[test]
fn test_floor_whole_number() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_floor(&make_unary(decimal_expr("3.0")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_floor_integer() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_floor(&make_unary(int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_floor_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_floor(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Truncate Tests
// ============================================================================

#[test]
fn test_truncate_positive() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_truncate(&make_unary(decimal_expr("3.9")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_truncate_negative() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_truncate(&make_unary(decimal_expr("-3.9")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-3));
}

#[test]
fn test_truncate_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_truncate(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Ln (Natural Log) Tests
// ============================================================================

#[test]
fn test_ln_e() {
    let e = engine();
    let mut c = ctx();

    // ln(e) = 1 approximately (e ~ 2.718)
    let result = e.eval_ln(&make_unary(decimal_expr("2.718281828")), &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        assert!((d.to_f64().unwrap() - 1.0).abs() < 0.0001);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_ln_one() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ln(&make_unary(decimal_expr("1.0")), &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        assert!((d.to_f64().unwrap()).abs() < 0.0001);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_ln_negative_returns_null() {
    let e = engine();
    let mut c = ctx();

    // ln of negative number is undefined, returns null
    let result = e.eval_ln(&make_unary(decimal_expr("-1.0")), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_ln_zero_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ln(&make_unary(decimal_expr("0.0")), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_ln_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ln(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Exp Tests
// ============================================================================

#[test]
fn test_exp_zero() {
    let e = engine();
    let mut c = ctx();

    // e^0 = 1
    let result = e.eval_exp(&make_unary(decimal_expr("0.0")), &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        assert!((d.to_f64().unwrap() - 1.0).abs() < 0.0001);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_exp_one() {
    let e = engine();
    let mut c = ctx();

    // e^1 = e ~ 2.718
    let result = e.eval_exp(&make_unary(decimal_expr("1.0")), &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        assert!((d.to_f64().unwrap() - 2.718281828).abs() < 0.0001);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_exp_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_exp(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Log Tests
// ============================================================================

#[test]
fn test_log_base_10() {
    let e = engine();
    let mut c = ctx();

    // log_10(100) = 2
    let result = e.eval_log(&make_binary(decimal_expr("100.0"), decimal_expr("10.0")), &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        assert!((d.to_f64().unwrap() - 2.0).abs() < 0.0001);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_log_base_2() {
    let e = engine();
    let mut c = ctx();

    // log_2(8) = 3
    let result = e.eval_log(&make_binary(decimal_expr("8.0"), decimal_expr("2.0")), &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        assert!((d.to_f64().unwrap() - 3.0).abs() < 0.0001);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_log_base_one_returns_null() {
    let e = engine();
    let mut c = ctx();

    // log base 1 is undefined
    let result = e.eval_log(&make_binary(decimal_expr("10.0"), decimal_expr("1.0")), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_log_negative_value_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_log(&make_binary(decimal_expr("-10.0"), decimal_expr("10.0")), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Successor Tests
// ============================================================================

#[test]
fn test_successor_integer() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_successor(&make_unary(int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(6));
}

#[test]
fn test_successor_max_int_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_successor(&make_unary(int_expr(i32::MAX)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_successor_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_successor(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Predecessor Tests
// ============================================================================

#[test]
fn test_predecessor_integer() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_predecessor(&make_unary(int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(4));
}

#[test]
fn test_predecessor_min_int_returns_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_predecessor(&make_unary(int_expr(i32::MIN)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_predecessor_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_predecessor(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// MinValue Tests
// ============================================================================

#[test]
fn test_min_value_integer() {
    let e = engine();

    let expr = octofhir_cql_elm::MinMaxValueExpression {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
    };

    let result = e.eval_min_value(&expr).unwrap();
    assert_eq!(result, CqlValue::Integer(i32::MIN));
}

#[test]
fn test_min_value_long() {
    let e = engine();

    let expr = octofhir_cql_elm::MinMaxValueExpression {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Long".to_string(),
    };

    let result = e.eval_min_value(&expr).unwrap();
    assert_eq!(result, CqlValue::Long(i64::MIN));
}

// ============================================================================
// MaxValue Tests
// ============================================================================

#[test]
fn test_max_value_integer() {
    let e = engine();

    let expr = octofhir_cql_elm::MinMaxValueExpression {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
    };

    let result = e.eval_max_value(&expr).unwrap();
    assert_eq!(result, CqlValue::Integer(i32::MAX));
}

#[test]
fn test_max_value_long() {
    let e = engine();

    let expr = octofhir_cql_elm::MinMaxValueExpression {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Long".to_string(),
    };

    let result = e.eval_max_value(&expr).unwrap();
    assert_eq!(result, CqlValue::Long(i64::MAX));
}
