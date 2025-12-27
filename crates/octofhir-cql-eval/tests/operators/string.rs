//! String Operator Tests
//!
//! Tests for: Concatenate, Combine, Split, SplitOnMatches, Length, Upper, Lower,
//! Substring, PositionOf, LastPositionOf, StartsWith, EndsWith, Matches, ReplaceMatches

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_elm::{
    BinaryExpression, CombineExpression, Element, Expression, LastPositionOfExpression, Literal,
    ListExpression, NaryExpression, NullLiteral, PositionOfExpression, SplitExpression,
    SubstringExpression, TernaryExpression, UnaryExpression,
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

fn string_expr(s: &str) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
        value: Some(s.to_string()),
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

fn string_list_expr(strings: &[&str]) -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(strings.iter().map(|s| string_expr(s)).collect()),
    }))
}

// ============================================================================
// Concatenate Tests
// ============================================================================

#[test]
fn test_concatenate_two_strings() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![string_expr("Hello"), string_expr(" World")],
    };

    let result = e.eval_concatenate(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("Hello World".to_string()));
}

#[test]
fn test_concatenate_multiple_strings() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![string_expr("A"), string_expr("B"), string_expr("C")],
    };

    let result = e.eval_concatenate(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("ABC".to_string()));
}

#[test]
fn test_concatenate_empty_string() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![string_expr("Hello"), string_expr("")],
    };

    let result = e.eval_concatenate(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("Hello".to_string()));
}

#[test]
fn test_concatenate_with_null_returns_null() {
    let e = engine();
    let mut c = ctx();

    let expr = NaryExpression {
        element: Element::default(),
        operand: vec![string_expr("Hello"), null_expr()],
    };

    let result = e.eval_concatenate(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Combine Tests
// ============================================================================

#[test]
fn test_combine_with_separator() {
    let e = engine();
    let mut c = ctx();

    let expr = CombineExpression {
        element: Element::default(),
        source: string_list_expr(&["a", "b", "c"]),
        separator: Some(string_expr(",")),
    };

    let result = e.eval_combine(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("a,b,c".to_string()));
}

#[test]
fn test_combine_without_separator() {
    let e = engine();
    let mut c = ctx();

    let expr = CombineExpression {
        element: Element::default(),
        source: string_list_expr(&["a", "b", "c"]),
        separator: None,
    };

    let result = e.eval_combine(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("abc".to_string()));
}

#[test]
fn test_combine_null_source() {
    let e = engine();
    let mut c = ctx();

    let expr = CombineExpression {
        element: Element::default(),
        source: null_expr(),
        separator: Some(string_expr(",")),
    };

    let result = e.eval_combine(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Split Tests
// ============================================================================

#[test]
fn test_split_by_comma() {
    let e = engine();
    let mut c = ctx();

    let expr = SplitExpression {
        element: Element::default(),
        string_to_split: string_expr("a,b,c"),
        separator: Some(string_expr(",")),
    };

    let result = e.eval_split(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(&CqlValue::string("a")));
        assert_eq!(list.get(1), Some(&CqlValue::string("b")));
        assert_eq!(list.get(2), Some(&CqlValue::string("c")));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_split_no_separator_found() {
    let e = engine();
    let mut c = ctx();

    let expr = SplitExpression {
        element: Element::default(),
        string_to_split: string_expr("hello"),
        separator: Some(string_expr(",")),
    };

    let result = e.eval_split(&expr, &mut c).unwrap();
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0), Some(&CqlValue::string("hello")));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_split_null_string() {
    let e = engine();
    let mut c = ctx();

    let expr = SplitExpression {
        element: Element::default(),
        string_to_split: null_expr(),
        separator: Some(string_expr(",")),
    };

    let result = e.eval_split(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Length Tests
// ============================================================================

#[test]
fn test_length_string() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_string_length(&make_unary(string_expr("hello")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(5));
}

#[test]
fn test_length_empty_string() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_string_length(&make_unary(string_expr("")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_length_unicode() {
    let e = engine();
    let mut c = ctx();

    // Unicode characters should be counted properly
    let result = e.eval_string_length(&make_unary(string_expr("cafe")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(4));
}

#[test]
fn test_length_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_string_length(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Upper Tests
// ============================================================================

#[test]
fn test_upper() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_upper(&make_unary(string_expr("hello")), &mut c).unwrap();
    assert_eq!(result, CqlValue::String("HELLO".to_string()));
}

#[test]
fn test_upper_mixed() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_upper(&make_unary(string_expr("HeLLo WoRLd")), &mut c).unwrap();
    assert_eq!(result, CqlValue::String("HELLO WORLD".to_string()));
}

#[test]
fn test_upper_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_upper(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Lower Tests
// ============================================================================

#[test]
fn test_lower() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_lower(&make_unary(string_expr("HELLO")), &mut c).unwrap();
    assert_eq!(result, CqlValue::String("hello".to_string()));
}

#[test]
fn test_lower_mixed() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_lower(&make_unary(string_expr("HeLLo WoRLd")), &mut c).unwrap();
    assert_eq!(result, CqlValue::String("hello world".to_string()));
}

#[test]
fn test_lower_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_lower(&make_unary(null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// StartsWith Tests
// ============================================================================

#[test]
fn test_starts_with_true() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_starts_with(&make_binary(string_expr("Hello, World!"), string_expr("Hello")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_starts_with_false() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_starts_with(&make_binary(string_expr("Hello, World!"), string_expr("World")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_starts_with_empty_prefix() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_starts_with(&make_binary(string_expr("Hello"), string_expr("")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_starts_with_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_starts_with(&make_binary(string_expr("Hello"), null_expr()), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// EndsWith Tests
// ============================================================================

#[test]
fn test_ends_with_true() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ends_with(&make_binary(string_expr("Hello, World!"), string_expr("World!")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_ends_with_false() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ends_with(&make_binary(string_expr("Hello, World!"), string_expr("Hello")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_ends_with_empty_suffix() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ends_with(&make_binary(string_expr("Hello"), string_expr("")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_ends_with_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_ends_with(&make_binary(null_expr(), string_expr("World")), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// PositionOf Tests
// ============================================================================

#[test]
fn test_position_of_found() {
    let e = engine();
    let mut c = ctx();

    let expr = PositionOfExpression {
        element: Element::default(),
        pattern: string_expr("World"),
        string: string_expr("Hello, World!"),
    };

    let result = e.eval_position_of(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(7));
}

#[test]
fn test_position_of_not_found() {
    let e = engine();
    let mut c = ctx();

    let expr = PositionOfExpression {
        element: Element::default(),
        pattern: string_expr("xyz"),
        string: string_expr("Hello, World!"),
    };

    let result = e.eval_position_of(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-1));
}

#[test]
fn test_position_of_empty_pattern() {
    let e = engine();
    let mut c = ctx();

    let expr = PositionOfExpression {
        element: Element::default(),
        pattern: string_expr(""),
        string: string_expr("Hello"),
    };

    let result = e.eval_position_of(&expr, &mut c).unwrap();
    // Empty string is found at position 0
    assert_eq!(result, CqlValue::Integer(0));
}

#[test]
fn test_position_of_null() {
    let e = engine();
    let mut c = ctx();

    let expr = PositionOfExpression {
        element: Element::default(),
        pattern: null_expr(),
        string: string_expr("Hello"),
    };

    let result = e.eval_position_of(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// LastPositionOf Tests
// ============================================================================

#[test]
fn test_last_position_of() {
    let e = engine();
    let mut c = ctx();

    let expr = LastPositionOfExpression {
        element: Element::default(),
        pattern: string_expr("l"),
        string: string_expr("Hello"),
    };

    let result = e.eval_last_position_of(&expr, &mut c).unwrap();
    // Last 'l' is at position 3
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_last_position_of_not_found() {
    let e = engine();
    let mut c = ctx();

    let expr = LastPositionOfExpression {
        element: Element::default(),
        pattern: string_expr("z"),
        string: string_expr("Hello"),
    };

    let result = e.eval_last_position_of(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(-1));
}

// ============================================================================
// Substring Tests
// ============================================================================

#[test]
fn test_substring_with_length() {
    let e = engine();
    let mut c = ctx();

    let expr = SubstringExpression {
        element: Element::default(),
        string_to_sub: string_expr("Hello, World!"),
        start_index: int_expr(7),
        length: Some(int_expr(5)),
    };

    let result = e.eval_substring(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("World".to_string()));
}

#[test]
fn test_substring_without_length() {
    let e = engine();
    let mut c = ctx();

    let expr = SubstringExpression {
        element: Element::default(),
        string_to_sub: string_expr("Hello, World!"),
        start_index: int_expr(7),
        length: None,
    };

    let result = e.eval_substring(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("World!".to_string()));
}

#[test]
fn test_substring_from_start() {
    let e = engine();
    let mut c = ctx();

    let expr = SubstringExpression {
        element: Element::default(),
        string_to_sub: string_expr("Hello"),
        start_index: int_expr(0),
        length: Some(int_expr(2)),
    };

    let result = e.eval_substring(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("He".to_string()));
}

#[test]
fn test_substring_negative_start_returns_null() {
    let e = engine();
    let mut c = ctx();

    let expr = SubstringExpression {
        element: Element::default(),
        string_to_sub: string_expr("Hello"),
        start_index: int_expr(-1),
        length: None,
    };

    let result = e.eval_substring(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_substring_start_past_end_returns_null() {
    let e = engine();
    let mut c = ctx();

    let expr = SubstringExpression {
        element: Element::default(),
        string_to_sub: string_expr("Hello"),
        start_index: int_expr(100),
        length: None,
    };

    let result = e.eval_substring(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_substring_null_string() {
    let e = engine();
    let mut c = ctx();

    let expr = SubstringExpression {
        element: Element::default(),
        string_to_sub: null_expr(),
        start_index: int_expr(0),
        length: None,
    };

    let result = e.eval_substring(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Matches Tests
// ============================================================================

#[test]
fn test_matches_true() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_matches(&make_binary(string_expr("test@example.com"), string_expr(r"^\w+@\w+\.\w+$")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_matches_false() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_matches(&make_binary(string_expr("not-an-email"), string_expr(r"^\w+@\w+\.\w+$")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(false));
}

#[test]
fn test_matches_simple_pattern() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_matches(&make_binary(string_expr("hello123"), string_expr(r"\d+")), &mut c).unwrap();
    assert_eq!(result, CqlValue::Boolean(true));
}

#[test]
fn test_matches_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_matches(&make_binary(null_expr(), string_expr(r"\d+")), &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// ReplaceMatches Tests
// ============================================================================

#[test]
fn test_replace_matches() {
    let e = engine();
    let mut c = ctx();

    let expr = TernaryExpression {
        element: Element::default(),
        operand: vec![string_expr("abc123def456"), string_expr(r"\d+"), string_expr("X")],
    };

    let result = e.eval_replace_matches(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("abcXdefX".to_string()));
}

#[test]
fn test_replace_matches_no_match() {
    let e = engine();
    let mut c = ctx();

    let expr = TernaryExpression {
        element: Element::default(),
        operand: vec![string_expr("hello"), string_expr(r"\d+"), string_expr("X")],
    };

    let result = e.eval_replace_matches(&expr, &mut c).unwrap();
    assert_eq!(result, CqlValue::String("hello".to_string()));
}

#[test]
fn test_replace_matches_null() {
    let e = engine();
    let mut c = ctx();

    let expr = TernaryExpression {
        element: Element::default(),
        operand: vec![null_expr(), string_expr(r"\d+"), string_expr("X")],
    };

    let result = e.eval_replace_matches(&expr, &mut c).unwrap();
    assert!(result.is_null());
}

// ============================================================================
// Indexer Tests
// ============================================================================

#[test]
fn test_indexer_string() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_indexer(&make_binary(string_expr("Hello"), int_expr(0)), &mut c).unwrap();
    assert_eq!(result, CqlValue::String("H".to_string()));
}

#[test]
fn test_indexer_string_middle() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_indexer(&make_binary(string_expr("Hello"), int_expr(2)), &mut c).unwrap();
    assert_eq!(result, CqlValue::String("l".to_string()));
}

#[test]
fn test_indexer_out_of_bounds() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_indexer(&make_binary(string_expr("Hello"), int_expr(10)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_indexer_negative() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_indexer(&make_binary(string_expr("Hello"), int_expr(-1)), &mut c).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_indexer_null() {
    let e = engine();
    let mut c = ctx();

    let result = e.eval_indexer(&make_binary(null_expr(), int_expr(0)), &mut c).unwrap();
    assert!(result.is_null());
}
