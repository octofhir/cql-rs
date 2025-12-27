//! Evaluation test helpers
//!
//! Utilities for testing CQL expression evaluation including context setup,
//! assertion helpers, and integration with mock providers.

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use octofhir_cql_eval::context::{EvaluationContext, EvaluationContextBuilder};
use octofhir_cql_eval::engine::EvaluationEngine;
use octofhir_cql_elm::{Expression as ElmExpression, Library};
use octofhir_cql_types::CqlValue;
use std::sync::Arc;

use crate::mocks::{MockDataProvider, MockTerminologyProvider};

/// Create a default evaluation context for testing
pub fn test_context() -> EvaluationContext {
    EvaluationContext::new()
}

/// Create a context with a specific timestamp
pub fn test_context_with_timestamp(timestamp: DateTime<FixedOffset>) -> EvaluationContext {
    EvaluationContext::with_timestamp(timestamp)
}

/// Create a context with timestamp set to a known value (2024-01-01 12:00:00 UTC)
pub fn test_context_fixed_time() -> EvaluationContext {
    let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .unwrap()
        .fixed_offset();
    EvaluationContext::with_timestamp(timestamp)
}

/// Create a context with mock providers
pub fn test_context_with_mocks() -> (
    EvaluationContext,
    Arc<MockTerminologyProvider>,
    Arc<MockDataProvider>,
) {
    let terminology = Arc::new(MockTerminologyProvider::new());
    let data_provider = Arc::new(MockDataProvider::new());

    let context = EvaluationContextBuilder::new()
        .terminology_provider(terminology.clone())
        .data_provider(data_provider.clone())
        .build();

    (context, terminology, data_provider)
}

/// Evaluate an ELM expression and return the result
pub fn eval_elm_expression(
    expr: &ElmExpression,
    context: &mut EvaluationContext,
) -> Result<CqlValue, Box<dyn std::error::Error>> {
    let engine = EvaluationEngine::new();
    engine.evaluate_expression(expr, context)
}

/// Evaluate an ELM expression and expect success
#[track_caller]
pub fn eval_elm_expression_ok(expr: &ElmExpression, context: &mut EvaluationContext) -> CqlValue {
    eval_elm_expression(expr, context).expect("Evaluation failed")
}

/// Evaluate an ELM expression and expect an error
#[track_caller]
pub fn eval_elm_expression_err(expr: &ElmExpression, context: &mut EvaluationContext) -> String {
    match eval_elm_expression(expr, context) {
        Ok(val) => panic!("Expected error but got value: {:?}", val),
        Err(e) => e.to_string(),
    }
}

/// Assert that a value is an integer with expected value
#[track_caller]
pub fn assert_integer(value: &CqlValue, expected: i64) {
    match value {
        CqlValue::Integer(val) => assert_eq!(*val, expected, "Integer value mismatch"),
        _ => panic!("Expected Integer, got: {:?}", value),
    }
}

/// Assert that a value is a decimal with expected value
#[track_caller]
pub fn assert_decimal(value: &CqlValue, expected: &str) {
    match value {
        CqlValue::Decimal(val) => {
            let expected_dec = expected.parse::<rust_decimal::Decimal>()
                .expect("Invalid expected decimal");
            assert_eq!(*val, expected_dec, "Decimal value mismatch");
        }
        _ => panic!("Expected Decimal, got: {:?}", value),
    }
}

/// Assert that a value is a string with expected value
#[track_caller]
pub fn assert_string(value: &CqlValue, expected: &str) {
    match value {
        CqlValue::String(val) => assert_eq!(val, expected, "String value mismatch"),
        _ => panic!("Expected String, got: {:?}", value),
    }
}

/// Assert that a value is a boolean with expected value
#[track_caller]
pub fn assert_boolean(value: &CqlValue, expected: bool) {
    match value {
        CqlValue::Boolean(val) => assert_eq!(*val, expected, "Boolean value mismatch"),
        _ => panic!("Expected Boolean, got: {:?}", value),
    }
}

/// Assert that a value is null
#[track_caller]
pub fn assert_null(value: &CqlValue) {
    match value {
        CqlValue::Null => {}
        _ => panic!("Expected Null, got: {:?}", value),
    }
}

/// Assert that a value is a list with expected length
#[track_caller]
pub fn assert_list_len(value: &CqlValue, expected_len: usize) -> &[CqlValue] {
    match value {
        CqlValue::List(list) => {
            assert_eq!(
                list.elements.len(),
                expected_len,
                "List length mismatch"
            );
            &list.elements
        }
        _ => panic!("Expected List, got: {:?}", value),
    }
}

/// Assert that a value is an empty list
#[track_caller]
pub fn assert_empty_list(value: &CqlValue) {
    assert_list_len(value, 0);
}

/// Assert that a value is a tuple and get its fields
#[track_caller]
pub fn assert_tuple(value: &CqlValue) -> &octofhir_cql_types::CqlTuple {
    match value {
        CqlValue::Tuple(tuple) => tuple,
        _ => panic!("Expected Tuple, got: {:?}", value),
    }
}

/// Assert that a value is an interval
#[track_caller]
pub fn assert_interval(value: &CqlValue) -> &octofhir_cql_types::CqlInterval {
    match value {
        CqlValue::Interval(interval) => interval,
        _ => panic!("Expected Interval, got: {:?}", value),
    }
}

/// Assert that a value is a code
#[track_caller]
pub fn assert_code(value: &CqlValue) -> &octofhir_cql_types::CqlCode {
    match value {
        CqlValue::Code(code) => code,
        _ => panic!("Expected Code, got: {:?}", value),
    }
}

/// Assert that a value is a concept
#[track_caller]
pub fn assert_concept(value: &CqlValue) -> &octofhir_cql_types::CqlConcept {
    match value {
        CqlValue::Concept(concept) => concept,
        _ => panic!("Expected Concept, got: {:?}", value),
    }
}

/// Assert that a value is a date
#[track_caller]
pub fn assert_date(value: &CqlValue) -> &octofhir_cql_types::CqlDate {
    match value {
        CqlValue::Date(date) => date,
        _ => panic!("Expected Date, got: {:?}", value),
    }
}

/// Assert that a value is a datetime
#[track_caller]
pub fn assert_datetime(value: &CqlValue) -> &octofhir_cql_types::CqlDateTime {
    match value {
        CqlValue::DateTime(dt) => dt,
        _ => panic!("Expected DateTime, got: {:?}", value),
    }
}

/// Assert that a value is a time
#[track_caller]
pub fn assert_time(value: &CqlValue) -> &octofhir_cql_types::CqlTime {
    match value {
        CqlValue::Time(time) => time,
        _ => panic!("Expected Time, got: {:?}", value),
    }
}

/// Assert that a value is a quantity
#[track_caller]
pub fn assert_quantity(value: &CqlValue) -> &octofhir_cql_types::CqlQuantity {
    match value {
        CqlValue::Quantity(qty) => qty,
        _ => panic!("Expected Quantity, got: {:?}", value),
    }
}

/// Assert that two values are equal (handles null semantics)
#[track_caller]
pub fn assert_values_equal(actual: &CqlValue, expected: &CqlValue) {
    assert_eq!(actual, expected, "Values do not match");
}

#[cfg(test)]
mod tests {
    use super::*;
    use octofhir_cql_types::{CqlCode, CqlDate, CqlList};

    #[test]
    fn test_assert_integer() {
        let val = CqlValue::integer(42);
        assert_integer(&val, 42);
    }

    #[test]
    fn test_assert_string() {
        let val = CqlValue::string("hello");
        assert_string(&val, "hello");
    }

    #[test]
    fn test_assert_boolean() {
        let val = CqlValue::boolean(true);
        assert_boolean(&val, true);
    }

    #[test]
    fn test_assert_null() {
        let val = CqlValue::Null;
        assert_null(&val);
    }

    #[test]
    fn test_assert_list() {
        let val = CqlValue::List(CqlList {
            elements: vec![CqlValue::integer(1), CqlValue::integer(2)],
        });
        let elements = assert_list_len(&val, 2);
        assert_integer(&elements[0], 1);
        assert_integer(&elements[1], 2);
    }

    #[test]
    fn test_assert_empty_list() {
        let val = CqlValue::List(CqlList { elements: vec![] });
        assert_empty_list(&val);
    }

    #[test]
    fn test_context_creation() {
        let ctx = test_context();
        assert!(ctx.context_type.is_none());
    }

    #[test]
    fn test_context_with_mocks() {
        let (ctx, terminology, data_provider) = test_context_with_mocks();
        assert!(ctx.terminology_provider().is_some());
        assert!(ctx.data_provider().is_some());
    }
}
