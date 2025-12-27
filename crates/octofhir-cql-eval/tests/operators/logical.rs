//! Logical Operator Tests
//!
//! Tests for: And, Or, Xor, Implies, Not, IsNull, IsTrue, IsFalse, Coalesce, If, Case
//! All operators implement three-valued logic per CQL specification

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_elm::{
    BinaryExpression, CaseExpression, CaseItem, Element, Expression, IfExpression, Literal,
    NaryExpression, NullLiteral, UnaryExpression,
};
use octofhir_cql_types::CqlValue;

// ============================================================================
// Test Helpers
// ============================================================================

fn engine() -> CqlEngine {
    CqlEngine::new()
}

fn ctx() -> EvaluationContext {
    EvaluationContext::new()
}

fn bool_expr(b: bool) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
        value: Some(b.to_string()),
    }))
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
// And Tests - Three-Valued Logic
// ============================================================================

/// And Truth Table:
/// | A     | B     | A and B |
/// |-------|-------|---------|
/// | true  | true  | true    |
/// | true  | false | false   |
/// | true  | null  | null    |
/// | false | true  | false   |
/// | false | false | false   |
/// | false | null  | false   | <- false dominates null
/// | null  | true  | null    |
/// | null  | false | false   | <- false dominates null
/// | null  | null  | null    |

#[test]
fn test_and_true_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_and(&make_binary(bool_expr(true), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_and_true_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_and(&make_binary(bool_expr(true), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_and_true_null() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_and(&make_binary(bool_expr(true), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_and_false_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_and(&make_binary(bool_expr(false), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_and_false_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_and(&make_binary(bool_expr(false), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_and_false_null() {
    let e = engine();
    let mut c = ctx();
    // FALSE dominates NULL in AND
    let result = e.eval_and(&make_binary(bool_expr(false), null_expr()), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_and_null_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_and(&make_binary(null_expr(), bool_expr(true)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_and_null_false() {
    let e = engine();
    let mut c = ctx();
    // FALSE dominates NULL in AND
    let result = e.eval_and(&make_binary(null_expr(), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_and_null_null() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_and(&make_binary(null_expr(), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Or Tests - Three-Valued Logic
// ============================================================================

/// Or Truth Table:
/// | A     | B     | A or B  |
/// |-------|-------|---------|
/// | true  | true  | true    |
/// | true  | false | true    |
/// | true  | null  | true    | <- true dominates null
/// | false | true  | true    |
/// | false | false | false   |
/// | false | null  | null    |
/// | null  | true  | true    | <- true dominates null
/// | null  | false | null    |
/// | null  | null  | null    |

#[test]
fn test_or_true_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_or(&make_binary(bool_expr(true), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_or_true_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_or(&make_binary(bool_expr(true), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_or_true_null() {
    let e = engine();
    let mut c = ctx();
    // TRUE dominates NULL in OR
    let result = e.eval_or(&make_binary(bool_expr(true), null_expr()), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_or_false_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_or(&make_binary(bool_expr(false), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_or_false_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_or(&make_binary(bool_expr(false), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_or_false_null() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_or(&make_binary(bool_expr(false), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_or_null_true() {
    let e = engine();
    let mut c = ctx();
    // TRUE dominates NULL in OR
    let result = e.eval_or(&make_binary(null_expr(), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_or_null_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_or(&make_binary(null_expr(), bool_expr(false)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_or_null_null() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_or(&make_binary(null_expr(), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Xor Tests
// ============================================================================

#[test]
fn test_xor_true_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_xor(&make_binary(bool_expr(true), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_xor_true_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_xor(&make_binary(bool_expr(true), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_xor_false_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_xor(&make_binary(bool_expr(false), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_xor_false_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_xor(&make_binary(bool_expr(false), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_xor_with_null() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_xor(&make_binary(bool_expr(true), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Implies Tests
// ============================================================================

/// Implies (A implies B) is equivalent to (not A) or B
/// | A     | B     | A implies B |
/// |-------|-------|-------------|
/// | true  | true  | true        |
/// | true  | false | false       |
/// | true  | null  | null        |
/// | false | true  | true        | <- false implies anything
/// | false | false | true        |
/// | false | null  | true        |
/// | null  | true  | true        | <- anything implies true
/// | null  | false | null        |
/// | null  | null  | null        |

#[test]
fn test_implies_true_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_implies(&make_binary(bool_expr(true), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_implies_true_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_implies(&make_binary(bool_expr(true), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_implies_true_null() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_implies(&make_binary(bool_expr(true), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_implies_false_anything() {
    let e = engine();
    let mut c = ctx();
    // FALSE implies anything is TRUE
    let result = e.eval_implies(&make_binary(bool_expr(false), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));

    let result = e.eval_implies(&make_binary(bool_expr(false), bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));

    let result = e.eval_implies(&make_binary(bool_expr(false), null_expr()), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_implies_null_true() {
    let e = engine();
    let mut c = ctx();
    // Anything implies TRUE is TRUE
    let result = e.eval_implies(&make_binary(null_expr(), bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_implies_null_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_implies(&make_binary(null_expr(), bool_expr(false)), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Not Tests
// ============================================================================

#[test]
fn test_not_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_not(&make_unary(bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_not_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_not(&make_unary(bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_not_null() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_not(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// IsNull Tests
// ============================================================================

#[test]
fn test_is_null_null() {
    let e = engine();
    let mut c = ctx();
    // IsNull never returns null
    let result = e.eval_is_null(&make_unary(null_expr()), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_is_null_value() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_is_null(&make_unary(bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_is_null_integer() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_is_null(&make_unary(int_expr(5)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

// ============================================================================
// IsTrue Tests
// ============================================================================

#[test]
fn test_is_true_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_is_true(&make_unary(bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_is_true_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_is_true(&make_unary(bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_is_true_null() {
    let e = engine();
    let mut c = ctx();
    // IsTrue never returns null
    let result = e.eval_is_true(&make_unary(null_expr()), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

// ============================================================================
// IsFalse Tests
// ============================================================================

#[test]
fn test_is_false_true() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_is_false(&make_unary(bool_expr(true)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_is_false_false() {
    let e = engine();
    let mut c = ctx();
    let result = e.eval_is_false(&make_unary(bool_expr(false)), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_is_false_null() {
    let e = engine();
    let mut c = ctx();
    // IsFalse never returns null
    let result = e.eval_is_false(&make_unary(null_expr()), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

// ============================================================================
// Coalesce Tests
// ============================================================================

#[test]
fn test_coalesce_first_non_null() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![null_expr(), null_expr(), int_expr(5), int_expr(10)],
    };

    let result = e.eval_coalesce(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_coalesce_first_value() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![int_expr(1), int_expr(2), int_expr(3)],
    };

    let result = e.eval_coalesce(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(1));
}

#[test]
fn test_coalesce_all_null() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![null_expr(), null_expr(), null_expr()],
    };

    let result = e.eval_coalesce(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_coalesce_single_value() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![int_expr(42)],
    };

    let result = e.eval_coalesce(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(42));
}

// ============================================================================
// If Tests
// ============================================================================

#[test]
fn test_if_true() {
    let e = engine();
    let mut c = ctx();

    let expr = IfExpression {
        element: Element::default(),
        condition: bool_expr(true),
        then: int_expr(1),
        else_clause: int_expr(2),
    };

    let result = e.eval_if(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(1));
}

#[test]
fn test_if_false() {
    let e = engine();
    let mut c = ctx();

    let expr = IfExpression {
        element: Element::default(),
        condition: bool_expr(false),
        then: int_expr(1),
        else_clause: int_expr(2),
    };

    let result = e.eval_if(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(2));
}

#[test]
fn test_if_null_condition() {
    let e = engine();
    let mut c = ctx();

    // If condition is null, else branch is taken
    let expr = IfExpression {
        element: Element::default(),
        condition: null_expr(),
        then: int_expr(1),
        else_clause: int_expr(2),
    };

    let result = e.eval_if(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(2));
}

// ============================================================================
// Case Tests
// ============================================================================

#[test]
fn test_case_first_match() {
    let e = engine();
    let mut c = ctx();

    let expr = CaseExpression {
        element: Element::default(),
        comparand: None,
        case_item: vec![
            CaseItem {
                when: bool_expr(true),
                then: int_expr(1),
            },
            CaseItem {
                when: bool_expr(true),
                then: int_expr(2),
            },
        ],
        else_clause: Some(int_expr(0)),
    };

    let result = e.eval_case(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(1));
}

#[test]
fn test_case_second_match() {
    let e = engine();
    let mut c = ctx();

    let expr = CaseExpression {
        element: Element::default(),
        comparand: None,
        case_item: vec![
            CaseItem {
                when: bool_expr(false),
                then: int_expr(1),
            },
            CaseItem {
                when: bool_expr(true),
                then: int_expr(2),
            },
        ],
        else_clause: Some(int_expr(0)),
    };

    let result = e.eval_case(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(2));
}

#[test]
fn test_case_else() {
    let e = engine();
    let mut c = ctx();

    let expr = CaseExpression {
        element: Element::default(),
        comparand: None,
        case_item: vec![
            CaseItem {
                when: bool_expr(false),
                then: int_expr(1),
            },
            CaseItem {
                when: bool_expr(false),
                then: int_expr(2),
            },
        ],
        else_clause: Some(int_expr(99)),
    };

    let result = e.eval_case(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(99));
}

#[test]
fn test_case_no_else_returns_null() {
    let e = engine();
    let mut c = ctx();

    let expr = CaseExpression {
        element: Element::default(),
        comparand: None,
        case_item: vec![CaseItem {
            when: bool_expr(false),
            then: int_expr(1),
        }],
        else_clause: None,
    };

    let result = e.eval_case(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_case_with_comparand() {
    let e = engine();
    let mut c = ctx();

    // case 5 when 1 then 'one' when 5 then 'five' else 'other' end
    let expr = CaseExpression {
        element: Element::default(),
        comparand: Some(int_expr(5)),
        case_item: vec![
            CaseItem {
                when: int_expr(1),
                then: Box::new(Expression::Literal(Literal {
                    element: Element::default(),
                    value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
                    value: Some("one".to_string()),
                })),
            },
            CaseItem {
                when: int_expr(5),
                then: Box::new(Expression::Literal(Literal {
                    element: Element::default(),
                    value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
                    value: Some("five".to_string()),
                })),
            },
        ],
        else_clause: Some(Box::new(Expression::Literal(Literal {
            element: Element::default(),
            value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
            value: Some("other".to_string()),
        }))),
    };

    let result = e.eval_case(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("five".to_string()));
}
