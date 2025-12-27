//! List Operator Tests
//!
//! Tests for: List constructor, Exists, First, Last, Slice, IndexOf, Flatten,
//! Sort, ForEach, Distinct, SingletonFrom, and set operations (Union, Intersect, Except)

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_elm::{
    Element, Expression, FirstLastExpression, IndexOfExpression, ListExpression, Literal,
    NullLiteral, SliceExpression, UnaryExpression,
};
use octofhir_cql_types::{CqlList, CqlType, CqlValue};

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

fn string_expr(s: &str) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
        value: Some(s.to_string()),
    }))
}

fn null_expr() -> Box<Expression> {
    Box::new(Expression::Null(NullLiteral { element: Element::default() }))
}

fn make_int_list(values: &[i32]) -> CqlValue {
    CqlValue::List(CqlList {
        element_type: CqlType::Integer,
        elements: values.iter().map(|&i| CqlValue::Integer(i)).collect(),
    })
}

fn make_int_list_expr(values: &[i32]) -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(values.iter().map(|&i| int_expr(i)).collect()),
    }))
}

fn make_unary(operand: Box<Expression>) -> UnaryExpression {
    UnaryExpression {
        element: Element::default(),
        operand,
    }
}

// ============================================================================
// List Constructor Tests
// ============================================================================

#[test]
fn test_list_constructor_integers() {
    let e = engine();
    let mut c = ctx();

    let expr = ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(1), int_expr(2), int_expr(3)]),
    };

    let result = e.eval_list_constructor(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(&CqlValue::Integer(1)));
        assert_eq!(list.get(1), Some(&CqlValue::Integer(2)));
        assert_eq!(list.get(2), Some(&CqlValue::Integer(3)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_list_constructor_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    };

    let result = e.eval_list_constructor(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert!(list.is_empty());
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_list_constructor_with_nulls() {
    let e = engine();
    let mut c = ctx();

    let expr = ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(1), null_expr(), int_expr(3)]),
    };

    let result = e.eval_list_constructor(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(&CqlValue::Integer(1)));
        assert!(list.get(1).map(|v| v.is_null()).unwrap_or(false));
        assert_eq!(list.get(2), Some(&CqlValue::Integer(3)));
    } else {
        panic!("Expected List");
    }
}

// ============================================================================
// Exists Tests
// ============================================================================

#[test]
fn test_exists_non_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(make_int_list_expr(&[1, 2, 3]));
    let result = e.eval_exists(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_exists_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    })));

    let result = e.eval_exists(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_exists_null() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(null_expr());
    let result = e.eval_exists(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_exists_single_value() {
    let e = engine();
    let mut c = ctx();

    // Single value is treated as a list with one element
    let expr = make_unary(int_expr(5));
    let result = e.eval_exists(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

// ============================================================================
// First Tests
// ============================================================================

#[test]
fn test_first_non_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = FirstLastExpression {
        element: Element::default(),
        source: make_int_list_expr(&[10, 20, 30]),
        order_by: None,
    };

    let result = e.eval_first(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(10));
}

#[test]
fn test_first_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = FirstLastExpression {
        element: Element::default(),
        source: Box::new(Expression::List(ListExpression {
            element: Element::default(),
            type_specifier: None,
            elements: None,
        })),
        order_by: None,
    };

    let result = e.eval_first(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_first_null() {
    let e = engine();
    let mut c = ctx();

    let expr = FirstLastExpression {
        element: Element::default(),
        source: null_expr(),
        order_by: None,
    };

    let result = e.eval_first(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Last Tests
// ============================================================================

#[test]
fn test_last_non_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = FirstLastExpression {
        element: Element::default(),
        source: make_int_list_expr(&[10, 20, 30]),
        order_by: None,
    };

    let result = e.eval_last(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(30));
}

#[test]
fn test_last_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = FirstLastExpression {
        element: Element::default(),
        source: Box::new(Expression::List(ListExpression {
            element: Element::default(),
            type_specifier: None,
            elements: None,
        })),
        order_by: None,
    };

    let result = e.eval_last(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Slice Tests
// ============================================================================

#[test]
fn test_slice_middle() {
    let e = engine();
    let mut c = ctx();

    let expr = SliceExpression {
        element: Element::default(),
        source: make_int_list_expr(&[1, 2, 3, 4, 5]),
        start_index: int_expr(1),
        end_index: Some(int_expr(4)),
    };

    let result = e.eval_slice(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(&CqlValue::Integer(2)));
        assert_eq!(list.get(1), Some(&CqlValue::Integer(3)));
        assert_eq!(list.get(2), Some(&CqlValue::Integer(4)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_slice_from_start() {
    let e = engine();
    let mut c = ctx();

    let expr = SliceExpression {
        element: Element::default(),
        source: make_int_list_expr(&[1, 2, 3, 4, 5]),
        start_index: int_expr(0),
        end_index: Some(int_expr(3)),
    };

    let result = e.eval_slice(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(&CqlValue::Integer(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_slice_to_end() {
    let e = engine();
    let mut c = ctx();

    let expr = SliceExpression {
        element: Element::default(),
        source: make_int_list_expr(&[1, 2, 3, 4, 5]),
        start_index: int_expr(2),
        end_index: None,
    };

    let result = e.eval_slice(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(&CqlValue::Integer(3)));
        assert_eq!(list.get(2), Some(&CqlValue::Integer(5)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_slice_null_source() {
    let e = engine();
    let mut c = ctx();

    let expr = SliceExpression {
        element: Element::default(),
        source: null_expr(),
        start_index: int_expr(0),
        end_index: Some(int_expr(3)),
    };

    let result = e.eval_slice(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// IndexOf Tests
// ============================================================================

#[test]
fn test_index_of_found() {
    let e = engine();
    let mut c = ctx();

    let expr = IndexOfExpression {
        element: Element::default(),
        source: make_int_list_expr(&[10, 20, 30, 40]),
        element_to_find: int_expr(30),
    };

    let result = e.eval_index_of(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(2));
}

#[test]
fn test_index_of_not_found() {
    let e = engine();
    let mut c = ctx();

    let expr = IndexOfExpression {
        element: Element::default(),
        source: make_int_list_expr(&[10, 20, 30, 40]),
        element_to_find: int_expr(50),
    };

    let result = e.eval_index_of(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-1));
}

#[test]
fn test_index_of_first_occurrence() {
    let e = engine();
    let mut c = ctx();

    let expr = IndexOfExpression {
        element: Element::default(),
        source: make_int_list_expr(&[10, 20, 10, 20]),
        element_to_find: int_expr(20),
    };

    let result = e.eval_index_of(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(1)); // Returns first occurrence
}

// ============================================================================
// Flatten Tests
// ============================================================================

#[test]
fn test_flatten() {
    let e = engine();
    let mut c = ctx();

    // Create a list of lists: [[1, 2], [3, 4]]
    let inner1 = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(1), int_expr(2)]),
    }));
    let inner2 = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![int_expr(3), int_expr(4)]),
    }));
    let outer = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![inner1, inner2]),
    }));

    let result = e.eval_flatten(&make_unary(outer), &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 4);
        assert_eq!(list.get(0), Some(&CqlValue::Integer(1)));
        assert_eq!(list.get(1), Some(&CqlValue::Integer(2)));
        assert_eq!(list.get(2), Some(&CqlValue::Integer(3)));
        assert_eq!(list.get(3), Some(&CqlValue::Integer(4)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_flatten_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_flatten(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Distinct Tests
// ============================================================================

#[test]
fn test_distinct() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(make_int_list_expr(&[1, 2, 1, 3, 2, 4]));
    let result = e.eval_distinct(&expr, &mut c).unwrap();

    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 4);
        // Should have 1, 2, 3, 4 in order of first occurrence
        assert_eq!(list.get(0), Some(&CqlValue::Integer(1)));
        assert_eq!(list.get(1), Some(&CqlValue::Integer(2)));
        assert_eq!(list.get(2), Some(&CqlValue::Integer(3)));
        assert_eq!(list.get(3), Some(&CqlValue::Integer(4)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_distinct_already_unique() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(make_int_list_expr(&[1, 2, 3]));
    let result = e.eval_distinct(&expr, &mut c).unwrap();

    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_distinct_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_distinct(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// SingletonFrom Tests
// ============================================================================

#[test]
fn test_singleton_from_single() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(make_int_list_expr(&[42]));
    let result = e.eval_singleton_from(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(42));
}

#[test]
fn test_singleton_from_empty() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    })));

    let result = e.eval_singleton_from(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_singleton_from_multiple_error() {
    let e = engine();
    let mut c = ctx();

    let expr = make_unary(make_int_list_expr(&[1, 2, 3]));
    let result = e.eval_singleton_from(&expr, &mut c);
    assert!(result.is_err());
}

#[test]
fn test_singleton_from_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_singleton_from(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// List Operations Tests (unit tests for CqlList methods)
// ============================================================================

#[test]
fn test_list_len() {
    let list = make_int_list(&[1, 2, 3, 4, 5]);
    if let CqlValue::List(l) = list {
        assert_eq!(l.len(), 5);
    }
}

#[test]
fn test_list_is_empty() {
    let empty = CqlValue::List(CqlList::new(CqlType::Integer));
    if let CqlValue::List(l) = empty {
        assert!(l.is_empty());
    }

    let non_empty = make_int_list(&[1]);
    if let CqlValue::List(l) = non_empty {
        assert!(!l.is_empty());
    }
}

#[test]
fn test_list_get() {
    let list = make_int_list(&[10, 20, 30]);
    if let CqlValue::List(l) = list {
        assert_eq!(l.get(0), Some(&CqlValue::Integer(10)));
        assert_eq!(l.get(1), Some(&CqlValue::Integer(20)));
        assert_eq!(l.get(2), Some(&CqlValue::Integer(30)));
        assert_eq!(l.get(3), None);
    }
}

#[test]
fn test_list_first_last() {
    let list = make_int_list(&[10, 20, 30]);
    if let CqlValue::List(l) = list {
        assert_eq!(l.first(), Some(&CqlValue::Integer(10)));
        assert_eq!(l.last(), Some(&CqlValue::Integer(30)));
    }
}

#[test]
fn test_list_first_last_empty() {
    let empty = CqlValue::List(CqlList::new(CqlType::Integer));
    if let CqlValue::List(l) = empty {
        assert_eq!(l.first(), None);
        assert_eq!(l.last(), None);
    }
}
