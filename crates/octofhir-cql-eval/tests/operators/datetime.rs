//! DateTime Operator Tests
//!
//! Tests for: Date, DateTime, Time constructors, DateFrom, TimeFrom,
//! DateTimeComponentFrom, DurationBetween, DifferenceBetween,
//! SameAs, SameOrBefore, SameOrAfter

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_elm::{
    DateExpression, DateTimeComponentFromExpression, DateTimeExpression, DifferenceBetweenExpression,
    DurationBetweenExpression, Element, Expression, Literal, NullLiteral, SameAsExpression,
    SameOrAfterExpression, SameOrBeforeExpression, TimeExpression, UnaryExpression,
};
use octofhir_cql_types::{CqlDate, CqlDateTime, CqlTime, CqlValue};

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

fn null_expr() -> Box<Expression> {
    Box::new(Expression::Null(NullLiteral { element: Element::default() }))
}

fn date_expr(year: i32, month: u8, day: u8) -> Box<Expression> {
    Box::new(Expression::Date(DateExpression {
        element: Element::default(),
        year: int_expr(year),
        month: Some(int_expr(month as i32)),
        day: Some(int_expr(day as i32)),
    }))
}

fn datetime_expr(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Box<Expression> {
    Box::new(Expression::DateTime(DateTimeExpression {
        element: Element::default(),
        year: int_expr(year),
        month: Some(int_expr(month as i32)),
        day: Some(int_expr(day as i32)),
        hour: Some(int_expr(hour as i32)),
        minute: Some(int_expr(minute as i32)),
        second: Some(int_expr(second as i32)),
        millisecond: None,
        timezone_offset: None,
    }))
}

fn time_expr(hour: u8, minute: u8, second: u8) -> Box<Expression> {
    Box::new(Expression::Time(TimeExpression {
        element: Element::default(),
        hour: int_expr(hour as i32),
        minute: Some(int_expr(minute as i32)),
        second: Some(int_expr(second as i32)),
        millisecond: None,
    }))
}

fn make_unary(operand: Box<Expression>) -> UnaryExpression {
    UnaryExpression {
        element: Element::default(),
        operand,
    }
}

// ============================================================================
// Date Constructor Tests
// ============================================================================

#[test]
fn test_date_constructor_full() {
    let e = engine();
    let mut c = ctx();

    let expr = DateExpression {
        element: Element::default(),
        year: int_expr(2024),
        month: Some(int_expr(3)),
        day: Some(int_expr(15)),
    };

    let result = e.eval_date_constructor(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Date(CqlDate::new(2024, 3, 15)));
}

#[test]
fn test_date_constructor_year_only() {
    let e = engine();
    let mut c = ctx();

    let expr = DateExpression {
        element: Element::default(),
        year: int_expr(2024),
        month: None,
        day: None,
    };

    let result = e.eval_date_constructor(&expr, &mut c).unwrap();
    if let CqlValue::Date(d) = result {
        assert_eq!(d.year, 2024);
        assert_eq!(d.month, None);
        assert_eq!(d.day, None);
    } else {
        panic!("Expected Date");
    }
}

#[test]
fn test_date_constructor_year_month() {
    let e = engine();
    let mut c = ctx();

    let expr = DateExpression {
        element: Element::default(),
        year: int_expr(2024),
        month: Some(int_expr(6)),
        day: None,
    };

    let result = e.eval_date_constructor(&expr, &mut c).unwrap();
    if let CqlValue::Date(d) = result {
        assert_eq!(d.year, 2024);
        assert_eq!(d.month, Some(6));
        assert_eq!(d.day, None);
    } else {
        panic!("Expected Date");
    }
}

#[test]
fn test_date_constructor_null_year() {
    let e = engine();
    let mut c = ctx();

    let expr = DateExpression {
        element: Element::default(),
        year: null_expr(),
        month: Some(int_expr(1)),
        day: Some(int_expr(1)),
    };

    let result = e.eval_date_constructor(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// DateTime Constructor Tests
// ============================================================================

#[test]
fn test_datetime_constructor_full() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeExpression {
        element: Element::default(),
        year: int_expr(2024),
        month: Some(int_expr(3)),
        day: Some(int_expr(15)),
        hour: Some(int_expr(10)),
        minute: Some(int_expr(30)),
        second: Some(int_expr(45)),
        millisecond: Some(int_expr(500)),
        timezone_offset: None,
    };

    let result = e.eval_datetime_constructor(&expr, &mut c).unwrap();
    if let CqlValue::DateTime(dt) = result {
        assert_eq!(dt.year, 2024);
        assert_eq!(dt.month, Some(3));
        assert_eq!(dt.day, Some(15));
        assert_eq!(dt.hour, Some(10));
        assert_eq!(dt.minute, Some(30));
        assert_eq!(dt.second, Some(45));
        assert_eq!(dt.millisecond, Some(500));
    } else {
        panic!("Expected DateTime");
    }
}

#[test]
fn test_datetime_constructor_date_only() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeExpression {
        element: Element::default(),
        year: int_expr(2024),
        month: Some(int_expr(1)),
        day: Some(int_expr(1)),
        hour: None,
        minute: None,
        second: None,
        millisecond: None,
        timezone_offset: None,
    };

    let result = e.eval_datetime_constructor(&expr, &mut c).unwrap();
    if let CqlValue::DateTime(dt) = result {
        assert_eq!(dt.year, 2024);
        assert_eq!(dt.hour, None);
    } else {
        panic!("Expected DateTime");
    }
}

#[test]
fn test_datetime_constructor_null_year() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeExpression {
        element: Element::default(),
        year: null_expr(),
        month: None,
        day: None,
        hour: None,
        minute: None,
        second: None,
        millisecond: None,
        timezone_offset: None,
    };

    let result = e.eval_datetime_constructor(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Time Constructor Tests
// ============================================================================

#[test]
fn test_time_constructor_full() {
    let e = engine();
    let mut c = ctx();

    let expr = TimeExpression {
        element: Element::default(),
        hour: int_expr(10),
        minute: Some(int_expr(30)),
        second: Some(int_expr(45)),
        millisecond: Some(int_expr(500)),
    };

    let result = e.eval_time_constructor(&expr, &mut c).unwrap();
    if let CqlValue::Time(t) = result {
        assert_eq!(t.hour, 10);
        assert_eq!(t.minute, Some(30));
        assert_eq!(t.second, Some(45));
        assert_eq!(t.millisecond, Some(500));
    } else {
        panic!("Expected Time");
    }
}

#[test]
fn test_time_constructor_hour_only() {
    let e = engine();
    let mut c = ctx();

    let expr = TimeExpression {
        element: Element::default(),
        hour: int_expr(14),
        minute: None,
        second: None,
        millisecond: None,
    };

    let result = e.eval_time_constructor(&expr, &mut c).unwrap();
    if let CqlValue::Time(t) = result {
        assert_eq!(t.hour, 14);
        assert_eq!(t.minute, None);
    } else {
        panic!("Expected Time");
    }
}

#[test]
fn test_time_constructor_null_hour() {
    let e = engine();
    let mut c = ctx();

    let expr = TimeExpression {
        element: Element::default(),
        hour: null_expr(),
        minute: Some(int_expr(30)),
        second: None,
        millisecond: None,
    };

    let result = e.eval_time_constructor(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// DateFrom Tests
// ============================================================================

#[test]
fn test_date_from_datetime() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_date_from(&make_unary(datetime_expr(2024, 3, 15, 10, 30, 0)), &mut c).unwrap();
    if let CqlValue::Date(d) = result {
        assert_eq!(d.year, 2024);
        assert_eq!(d.month, Some(3));
        assert_eq!(d.day, Some(15));
    } else {
        panic!("Expected Date");
    }
}

#[test]
fn test_date_from_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_date_from(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// TimeFrom Tests
// ============================================================================

#[test]
fn test_time_from_datetime() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_time_from(&make_unary(datetime_expr(2024, 3, 15, 10, 30, 45)), &mut c).unwrap();
    if let CqlValue::Time(t) = result {
        assert_eq!(t.hour, 10);
        assert_eq!(t.minute, Some(30));
        assert_eq!(t.second, Some(45));
    } else {
        panic!("Expected Time");
    }
}

#[test]
fn test_time_from_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_time_from(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// DateTimeComponentFrom Tests
// ============================================================================

#[test]
fn test_datetime_component_year() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeComponentFromExpression {
        element: Element::default(),
        operand: date_expr(2024, 3, 15),
        precision: octofhir_cql_elm::DateTimePrecision::Year,
    };

    let result = e.eval_datetime_component_from(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(2024));
}

#[test]
fn test_datetime_component_month() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeComponentFromExpression {
        element: Element::default(),
        operand: date_expr(2024, 7, 15),
        precision: octofhir_cql_elm::DateTimePrecision::Month,
    };

    let result = e.eval_datetime_component_from(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(7));
}

#[test]
fn test_datetime_component_day() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeComponentFromExpression {
        element: Element::default(),
        operand: date_expr(2024, 3, 20),
        precision: octofhir_cql_elm::DateTimePrecision::Day,
    };

    let result = e.eval_datetime_component_from(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(20));
}

#[test]
fn test_datetime_component_hour() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeComponentFromExpression {
        element: Element::default(),
        operand: time_expr(14, 30, 0),
        precision: octofhir_cql_elm::DateTimePrecision::Hour,
    };

    let result = e.eval_datetime_component_from(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(14));
}

#[test]
fn test_datetime_component_null() {
    let e = engine();
    let mut c = ctx();

    let expr = DateTimeComponentFromExpression {
        element: Element::default(),
        operand: null_expr(),
        precision: octofhir_cql_elm::DateTimePrecision::Year,
    };

    let result = e.eval_datetime_component_from(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// DurationBetween Tests
// ============================================================================

#[test]
fn test_duration_between_dates_days() {
    let e = engine();
    let mut c = ctx();

    let expr = DurationBetweenExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 1, 1), date_expr(2024, 1, 10)],
        precision: octofhir_cql_elm::DateTimePrecision::Day,
    };

    let result = e.eval_duration_between(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(9));
}

#[test]
fn test_duration_between_dates_months() {
    let e = engine();
    let mut c = ctx();

    let expr = DurationBetweenExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 1, 15), date_expr(2024, 4, 15)],
        precision: octofhir_cql_elm::DateTimePrecision::Month,
    };

    let result = e.eval_duration_between(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_duration_between_dates_years() {
    let e = engine();
    let mut c = ctx();

    let expr = DurationBetweenExpression {
        element: Element::default(),
        operand: vec![date_expr(2020, 1, 1), date_expr(2024, 1, 1)],
        precision: octofhir_cql_elm::DateTimePrecision::Year,
    };

    let result = e.eval_duration_between(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(4));
}

#[test]
fn test_duration_between_null() {
    let e = engine();
    let mut c = ctx();

    let expr = DurationBetweenExpression {
        element: Element::default(),
        operand: vec![null_expr(), date_expr(2024, 1, 1)],
        precision: octofhir_cql_elm::DateTimePrecision::Day,
    };

    let result = e.eval_duration_between(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_duration_between_times_hours() {
    let e = engine();
    let mut c = ctx();

    let expr = DurationBetweenExpression {
        element: Element::default(),
        operand: vec![time_expr(10, 0, 0), time_expr(14, 0, 0)],
        precision: octofhir_cql_elm::DateTimePrecision::Hour,
    };

    let result = e.eval_duration_between(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(4));
}

#[test]
fn test_duration_between_times_minutes() {
    let e = engine();
    let mut c = ctx();

    let expr = DurationBetweenExpression {
        element: Element::default(),
        operand: vec![time_expr(10, 0, 0), time_expr(10, 45, 0)],
        precision: octofhir_cql_elm::DateTimePrecision::Minute,
    };

    let result = e.eval_duration_between(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(45));
}

// ============================================================================
// DifferenceBetween Tests
// ============================================================================

#[test]
fn test_difference_between_dates() {
    let e = engine();
    let mut c = ctx();

    let expr = DifferenceBetweenExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 1, 1), date_expr(2024, 1, 15)],
        precision: octofhir_cql_elm::DateTimePrecision::Day,
    };

    let result = e.eval_difference_between(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(14));
}

// ============================================================================
// SameAs Tests
// ============================================================================

#[test]
fn test_same_as_dates_true() {
    let e = engine();
    let mut c = ctx();

    let expr = SameAsExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 15), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_as(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_same_as_dates_false() {
    let e = engine();
    let mut c = ctx();

    let expr = SameAsExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 15), date_expr(2024, 3, 16)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_as(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_same_as_dates_same_month() {
    let e = engine();
    let mut c = ctx();

    // Different days but same month
    let expr = SameAsExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 15), date_expr(2024, 3, 20)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Month),
    };

    let result = e.eval_same_as(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_same_as_null() {
    let e = engine();
    let mut c = ctx();

    let expr = SameAsExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 15), null_expr()],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_as(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// SameOrBefore Tests
// ============================================================================

#[test]
fn test_same_or_before_same() {
    let e = engine();
    let mut c = ctx();

    let expr = SameOrBeforeExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 15), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_or_before(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_same_or_before_before() {
    let e = engine();
    let mut c = ctx();

    let expr = SameOrBeforeExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 10), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_or_before(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_same_or_before_after() {
    let e = engine();
    let mut c = ctx();

    let expr = SameOrBeforeExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 20), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_or_before(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

// ============================================================================
// SameOrAfter Tests
// ============================================================================

#[test]
fn test_same_or_after_same() {
    let e = engine();
    let mut c = ctx();

    let expr = SameOrAfterExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 15), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_or_after(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_same_or_after_after() {
    let e = engine();
    let mut c = ctx();

    let expr = SameOrAfterExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 20), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_or_after(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_same_or_after_before() {
    let e = engine();
    let mut c = ctx();

    let expr = SameOrAfterExpression {
        element: Element::default(),
        operand: vec![date_expr(2024, 3, 10), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_or_after(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_same_or_after_null() {
    let e = engine();
    let mut c = ctx();

    let expr = SameOrAfterExpression {
        element: Element::default(),
        operand: vec![null_expr(), date_expr(2024, 3, 15)],
        precision: Some(octofhir_cql_elm::DateTimePrecision::Day),
    };

    let result = e.eval_same_or_after(&expr, &mut c).unwrap();
    assert!(result.is_null());
}
