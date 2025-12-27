//! Aggregate Function Tests
//!
//! Tests for: Count, Sum, Avg, Min, Max, Median, Mode, Product, GeometricMean,
//! Variance, PopulationVariance, StdDev, PopulationStdDev, AllTrue, AnyTrue

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_elm::{AggregateExpression, Element, Expression, ListExpression, Literal, NullLiteral};
use octofhir_cql_types::{CqlList, CqlType, CqlValue};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

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

fn make_int_list_expr(values: &[i32]) -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(values.iter().map(|&i| int_expr(i)).collect()),
    }))
}

fn make_decimal_list_expr(values: &[&str]) -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(values.iter().map(|&d| decimal_expr(d)).collect()),
    }))
}

fn make_bool_list_expr(values: &[bool]) -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(values.iter().map(|&b| bool_expr(b)).collect()),
    }))
}

fn make_aggregate(source: Box<Expression>) -> AggregateExpression {
    AggregateExpression {
        element: Element::default(),
        source: Some(source),
        path: None,
        iteration: None,
        starting: None,
    }
}

// ============================================================================
// Count Tests
// ============================================================================

#[test]
fn test_count_integers() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[1, 2, 3, 4, 5]));
    let result = e.eval_count(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_count_with_nulls() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(1), null_expr(), int_expr(3), null_expr()]),
    }));

    let expr = make_aggregate(list);
    let result = e.eval_count(&expr, &mut c).unwrap();
    // Count excludes nulls
    assert_eq!(result, CqlValue::Integer(2));
}

#[test]
fn test_count_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_count(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_count_null_source() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(null_expr());
    let result = e.eval_count(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

// ============================================================================
// Sum Tests
// ============================================================================

#[test]
fn test_sum_integers() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[1, 2, 3, 4, 5]));
    let result = e.eval_sum(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(15));
}

#[test]
fn test_sum_with_nulls() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(1), null_expr(), int_expr(3), null_expr()]),
    }));

    let expr = make_aggregate(list);
    let result = e.eval_sum(&expr, &mut c).unwrap();
    // Sum ignores nulls
    assert_eq!(result, CqlValue::Integer(4));
}

#[test]
fn test_sum_decimals() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_decimal_list_expr(&["1.5", "2.5", "3.0"]));
    let result = e.eval_sum(&expr, &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        assert_eq!(d, Decimal::from_str_exact("7.0").unwrap());
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_sum_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_sum(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_sum_null_source() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(null_expr());
    let result = e.eval_sum(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Avg Tests
// ============================================================================

#[test]
fn test_avg_integers() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[1, 2, 3, 4, 5]));
    let result = e.eval_avg(&expr, &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        // Average of 1, 2, 3, 4, 5 is 3
        assert_eq!(d, Decimal::from(3));
    } else {
        panic!("Expected Decimal, got {:?}", result);
    }
}

#[test]
fn test_avg_with_nulls() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(2), null_expr(), int_expr(4)]),
    }));

    let expr = make_aggregate(list);
    let result = e.eval_avg(&expr, &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        // Average of 2 and 4 is 3
        assert_eq!(d, Decimal::from(3));
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_avg_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_avg(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Min Tests
// ============================================================================

#[test]
fn test_min_integers() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[5, 3, 8, 1, 4]));
    let result = e.eval_min(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(1));
}

#[test]
fn test_min_with_nulls() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(5), null_expr(), int_expr(3)]),
    }));

    let expr = make_aggregate(list);
    let result = e.eval_min(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_min_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_min(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Max Tests
// ============================================================================

#[test]
fn test_max_integers() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[5, 3, 8, 1, 4]));
    let result = e.eval_max(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(8));
}

#[test]
fn test_max_with_nulls() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(5), null_expr(), int_expr(3)]),
    }));

    let expr = make_aggregate(list);
    let result = e.eval_max(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_max_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_max(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Median Tests
// ============================================================================

#[test]
fn test_median_odd_count() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[5, 1, 3]));
    let result = e.eval_median(&expr, &mut c).unwrap();
    // Sorted: 1, 3, 5 -> median is 3
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_median_even_count() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[1, 2, 3, 4]));
    let result = e.eval_median(&expr, &mut c).unwrap();
    // Sorted: 1, 2, 3, 4 -> median is average of 2 and 3 = 2.5
    if let CqlValue::Decimal(d) = result {
        assert_eq!(d, Decimal::from_str_exact("2.5").unwrap());
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_median_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_median(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Mode Tests
// ============================================================================

#[test]
fn test_mode_single_mode() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[1, 2, 2, 3, 2, 4]));
    let result = e.eval_mode(&expr, &mut c).unwrap();
    // 2 appears most frequently
    assert_eq!(result, CqlValue::Integer(2));
}

#[test]
fn test_mode_all_unique() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[1, 2, 3, 4]));
    let result = e.eval_mode(&expr, &mut c).unwrap();
    // All appear once, returns first (1)
    assert_eq!(result, CqlValue::Integer(1));
}

#[test]
fn test_mode_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_mode(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Product Tests
// ============================================================================

#[test]
fn test_product_integers() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[2, 3, 4]));
    let result = e.eval_product(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(24));
}

#[test]
fn test_product_with_zero() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[2, 0, 4]));
    let result = e.eval_product(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_product_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_product(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// GeometricMean Tests
// ============================================================================

#[test]
fn test_geometric_mean() {
    let e = engine();
    let mut c = ctx();

    // Geometric mean of 2, 8 = sqrt(16) = 4
    let expr = make_aggregate(make_int_list_expr(&[2, 8]));
    let result = e.eval_geometric_mean(&expr, &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        let value = d.to_f64().unwrap();
        assert!((value - 4.0).abs() < 0.0001);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_geometric_mean_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_geometric_mean(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Variance Tests
// ============================================================================

#[test]
fn test_variance() {
    let e = engine();
    let mut c = ctx();

    // Variance of 2, 4, 4, 4, 5, 5, 7, 9
    // Mean = 5, Variance = ((2-5)^2 + (4-5)^2*3 + (5-5)^2*2 + (7-5)^2 + (9-5)^2) / 7
    let expr = make_aggregate(make_int_list_expr(&[2, 4, 4, 4, 5, 5, 7, 9]));
    let result = e.eval_variance(&expr, &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        let value = d.to_f64().unwrap();
        // Sample variance should be approximately 4.571
        assert!(value > 4.0 && value < 5.0);
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn test_variance_single_element() {
    let e = engine();
    let mut c = ctx();

    // Variance of a single element is null (n-1 = 0)
    let expr = make_aggregate(make_int_list_expr(&[5]));
    let result = e.eval_variance(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Population Variance Tests
// ============================================================================

#[test]
fn test_population_variance() {
    let e = engine();
    let mut c = ctx();

    // Population variance of 2, 4, 4, 4, 5, 5, 7, 9
    let expr = make_aggregate(make_int_list_expr(&[2, 4, 4, 4, 5, 5, 7, 9]));
    let result = e.eval_population_variance(&expr, &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        let value = d.to_f64().unwrap();
        // Population variance should be 4.0
        assert!(value > 3.9 && value < 4.1);
    } else {
        panic!("Expected Decimal");
    }
}

// ============================================================================
// StdDev Tests
// ============================================================================

#[test]
fn test_stddev() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_int_list_expr(&[2, 4, 4, 4, 5, 5, 7, 9]));
    let result = e.eval_stddev(&expr, &mut c).unwrap();
    if let CqlValue::Decimal(d) = result {
        let value = d.to_f64().unwrap();
        // StdDev should be sqrt(variance) ≈ 2.138
        assert!(value > 2.0 && value < 2.3);
    } else {
        panic!("Expected Decimal");
    }
}

// ============================================================================
// AllTrue Tests
// ============================================================================

#[test]
fn test_all_true() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_bool_list_expr(&[true, true, true]));
    let result = e.eval_all_true(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_all_true_with_false() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_bool_list_expr(&[true, false, true]));
    let result = e.eval_all_true(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_all_true_with_null() {
    let e = engine();
    let mut c = ctx();

    // Nulls are ignored - if all non-null values are true, result is true
    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![bool_expr(true), null_expr(), bool_expr(true)]),
    }));

    let expr = make_aggregate(list);
    let result = e.eval_all_true(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_all_true_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_all_true(&expr, &mut c).unwrap();
    // Empty list returns true (vacuous truth)
    assert_eq!(result, CqlValue::Boolean(true));
}

// ============================================================================
// AnyTrue Tests
// ============================================================================

#[test]
fn test_any_true() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_bool_list_expr(&[false, true, false]));
    let result = e.eval_any_true(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_any_true_all_false() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(make_bool_list_expr(&[false, false, false]));
    let result = e.eval_any_true(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_any_true_with_null() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![bool_expr(false), null_expr(), bool_expr(true)]),
    }));

    let expr = make_aggregate(list);
    let result = e.eval_any_true(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_any_true_empty() {
    let e = engine();
    let mut c = ctx();

    let list = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }));

    let expr = make_aggregate(list);
    let result = e.eval_any_true(&expr, &mut c).unwrap();
    // Empty list returns false
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_any_true_null_source() {
    let e = engine();
    let mut c = ctx();

    let expr = make_aggregate(null_expr());
    let result = e.eval_any_true(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}
