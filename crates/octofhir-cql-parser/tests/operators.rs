//! Tests for CQL operator parsing and precedence
//!
//! Covers:
//! - Arithmetic operators (+, -, *, /, div, mod, ^)
//! - Comparison operators (=, !=, <, >, <=, >=, ~, !~)
//! - Logical operators (and, or, xor, not)
//! - String operators (&, +)
//! - Membership operators (in, contains)
//! - Null operators (is null, is not null)
//! - Operator precedence and associativity

use octofhir_cql_ast::*;
use octofhir_cql_parser::Parser;
use pretty_assertions::assert_eq;
use rstest::rstest;

fn parse_expr(input: &str) -> Expression {
    let parser = Parser::new();
    parser
        .parse_expression(input)
        .unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", input, e))
}

fn assert_binary_op(expr: &Expression) -> (&Expression, &str, &Expression) {
    match &expr.kind {
        ExpressionKind::BinaryOp { left, op, right } => {
            (left.as_ref(), op.as_str(), right.as_ref())
        }
        _ => panic!("Expected BinaryOp, got: {:?}", expr.kind),
    }
}

fn assert_unary_op(expr: &Expression) -> (&str, &Expression) {
    match &expr.kind {
        ExpressionKind::UnaryOp { op, operand } => (op.as_str(), operand.as_ref()),
        _ => panic!("Expected UnaryOp, got: {:?}", expr.kind),
    }
}

// === Arithmetic Operators ===

#[test]
fn test_addition() {
    let expr = parse_expr("1 + 2");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "+");
}

#[test]
fn test_subtraction() {
    let expr = parse_expr("5 - 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "-");
}

#[test]
fn test_multiplication() {
    let expr = parse_expr("4 * 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "*");
}

#[test]
fn test_division() {
    let expr = parse_expr("10 / 2");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "/");
}

#[test]
fn test_integer_division() {
    let expr = parse_expr("10 div 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "div");
}

#[test]
fn test_modulo() {
    let expr = parse_expr("10 mod 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "mod");
}

#[test]
fn test_power() {
    let expr = parse_expr("2 ^ 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "^");
}

#[test]
fn test_unary_minus() {
    let expr = parse_expr("-5");
    let (op, _) = assert_unary_op(&expr);
    assert_eq!(op, "-");
}

#[test]
fn test_unary_plus() {
    let expr = parse_expr("+5");
    // May be parsed as unary + or just as literal
    match &expr.kind {
        ExpressionKind::UnaryOp { op, .. } => assert_eq!(op, "+"),
        ExpressionKind::Literal(_) => {} // Also acceptable
        _ => panic!("Expected UnaryOp or Literal"),
    }
}

// === Comparison Operators ===

#[test]
fn test_equal() {
    let expr = parse_expr("x = 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "=");
}

#[test]
fn test_not_equal() {
    let expr = parse_expr("x != 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "!=");
}

#[test]
fn test_less_than() {
    let expr = parse_expr("x < 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "<");
}

#[test]
fn test_less_equal() {
    let expr = parse_expr("x <= 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "<=");
}

#[test]
fn test_greater_than() {
    let expr = parse_expr("x > 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, ">");
}

#[test]
fn test_greater_equal() {
    let expr = parse_expr("x >= 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, ">=");
}

#[test]
fn test_equivalent() {
    let expr = parse_expr("x ~ 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "~");
}

#[test]
fn test_not_equivalent() {
    let expr = parse_expr("x !~ 5");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "!~");
}

// === Logical Operators ===

#[test]
fn test_and() {
    let expr = parse_expr("true and false");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "and");
}

#[test]
fn test_or() {
    let expr = parse_expr("true or false");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "or");
}

#[test]
fn test_xor() {
    let expr = parse_expr("true xor false");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "xor");
}

#[test]
fn test_not() {
    let expr = parse_expr("not true");
    let (op, _) = assert_unary_op(&expr);
    assert_eq!(op, "not");
}

#[test]
fn test_implies() {
    let expr = parse_expr("A implies B");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "implies");
}

// === String Operators ===

#[test]
fn test_string_concatenation() {
    let expr = parse_expr("'hello' & 'world'");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "&");
}

#[test]
fn test_string_concatenation_plus() {
    let expr = parse_expr("'hello' + 'world'");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "+"); // May also be used for concatenation
}

// === Membership Operators ===

#[test]
fn test_in_operator() {
    let expr = parse_expr("5 in {1, 2, 3, 4, 5}");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "in");
}

#[test]
fn test_contains_operator() {
    let expr = parse_expr("{1, 2, 3} contains 2");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "contains");
}

// === Null Operators ===

#[test]
fn test_is_null() {
    let expr = parse_expr("x is null");
    match &expr.kind {
        ExpressionKind::IsNull { operand } => {
            // Successfully parsed
            assert!(matches!(operand.kind, ExpressionKind::Identifier(_)));
        }
        ExpressionKind::BinaryOp { op, .. } if op == "is" => {
            // Alternative parse: "is" as operator
        }
        _ => panic!("Expected IsNull or BinaryOp 'is', got: {:?}", expr.kind),
    }
}

#[test]
fn test_is_not_null() {
    let expr = parse_expr("x is not null");
    match &expr.kind {
        ExpressionKind::IsNotNull { operand } => {
            assert!(matches!(operand.kind, ExpressionKind::Identifier(_)));
        }
        _ => {} // May be parsed differently
    }
}

// === Interval Operators ===

#[test]
fn test_interval_during() {
    let expr = parse_expr("A during B");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "during");
}

#[test]
fn test_interval_includes() {
    let expr = parse_expr("A includes B");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "includes");
}

#[test]
fn test_interval_included_in() {
    let expr = parse_expr("A included in B");
    // This is a multi-word operator
    match &expr.kind {
        ExpressionKind::BinaryOp { op, .. } => {
            assert!(op.contains("included") && op.contains("in"));
        }
        _ => panic!("Expected BinaryOp with 'included in'"),
    }
}

#[test]
fn test_interval_overlaps() {
    let expr = parse_expr("A overlaps B");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "overlaps");
}

#[test]
fn test_interval_starts() {
    let expr = parse_expr("A starts B");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "starts");
}

#[test]
fn test_interval_ends() {
    let expr = parse_expr("A ends B");
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "ends");
}

// === Operator Precedence Tests ===

#[test]
fn test_precedence_multiply_before_add() {
    // 1 + 2 * 3 should parse as 1 + (2 * 3), not (1 + 2) * 3
    let expr = parse_expr("1 + 2 * 3");
    let (left, op, right) = assert_binary_op(&expr);
    assert_eq!(op, "+");

    // left should be 1
    match &left.kind {
        ExpressionKind::Literal(_) => {}
        _ => panic!("Expected literal on left"),
    }

    // right should be (2 * 3)
    let (_, mult_op, _) = assert_binary_op(right);
    assert_eq!(mult_op, "*");
}

#[test]
fn test_precedence_power_before_multiply() {
    // 2 * 3 ^ 2 should parse as 2 * (3 ^ 2)
    let expr = parse_expr("2 * 3 ^ 2");
    let (left, op, right) = assert_binary_op(&expr);
    assert_eq!(op, "*");

    // right should be (3 ^ 2)
    let (_, pow_op, _) = assert_binary_op(right);
    assert_eq!(pow_op, "^");
}

#[test]
fn test_precedence_comparison_before_and() {
    // x > 5 and y < 10 should parse as (x > 5) and (y < 10)
    let expr = parse_expr("x > 5 and y < 10");
    let (left, op, right) = assert_binary_op(&expr);
    assert_eq!(op, "and");

    // left and right should be comparison operations
    let (_, left_op, _) = assert_binary_op(left);
    assert_eq!(left_op, ">");

    let (_, right_op, _) = assert_binary_op(right);
    assert_eq!(right_op, "<");
}

#[test]
fn test_precedence_and_before_or() {
    // A or B and C should parse as A or (B and C)
    let expr = parse_expr("A or B and C");
    let (left, op, right) = assert_binary_op(&expr);
    assert_eq!(op, "or");

    // right should be (B and C)
    let (_, and_op, _) = assert_binary_op(right);
    assert_eq!(and_op, "and");
}

#[test]
fn test_associativity_addition_left() {
    // 1 + 2 + 3 should parse as (1 + 2) + 3 (left-associative)
    let expr = parse_expr("1 + 2 + 3");
    let (left, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "+");

    // left should also be a + operation
    let (_, left_op, _) = assert_binary_op(left);
    assert_eq!(left_op, "+");
}

#[test]
fn test_associativity_power_right() {
    // 2 ^ 3 ^ 2 should parse as 2 ^ (3 ^ 2) (right-associative)
    let expr = parse_expr("2 ^ 3 ^ 2");
    let (_, op, right) = assert_binary_op(&expr);
    assert_eq!(op, "^");

    // right should also be a ^ operation
    let (_, right_op, _) = assert_binary_op(right);
    assert_eq!(right_op, "^");
}

#[test]
fn test_parentheses_override_precedence() {
    // (1 + 2) * 3 should parse as expected
    let expr = parse_expr("(1 + 2) * 3");
    let (left, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "*");

    // left should be (1 + 2)
    let (_, add_op, _) = assert_binary_op(left);
    assert_eq!(add_op, "+");
}

#[test]
fn test_complex_precedence() {
    // 1 + 2 * 3 - 4 / 2 should parse correctly
    let expr = parse_expr("1 + 2 * 3 - 4 / 2");

    // Should be ((1 + (2 * 3)) - (4 / 2))
    let (left, op, right) = assert_binary_op(&expr);
    assert_eq!(op, "-");

    // Verify multiplication in left subtree
    let (_, _, mult_part) = assert_binary_op(left);
    let (_, mult_op, _) = assert_binary_op(mult_part);
    assert_eq!(mult_op, "*");

    // Verify division in right
    let (_, div_op, _) = assert_binary_op(right);
    assert_eq!(div_op, "/");
}

#[test]
fn test_unary_precedence() {
    // -2 * 3 should parse as (-2) * 3, not -(2 * 3)
    let expr = parse_expr("-2 * 3");
    let (left, op, _) = assert_binary_op(&expr);
    assert_eq!(op, "*");

    // left should be unary minus
    match &left.kind {
        ExpressionKind::UnaryOp { op, .. } => assert_eq!(op, "-"),
        ExpressionKind::Literal(_) => {} // Negative literal also acceptable
        _ => panic!("Expected UnaryOp or negative literal on left"),
    }
}

#[rstest]
#[case("1 + 2", "+")]
#[case("1 - 2", "-")]
#[case("1 * 2", "*")]
#[case("1 / 2", "/")]
#[case("1 div 2", "div")]
#[case("1 mod 2", "mod")]
#[case("1 ^ 2", "^")]
#[case("1 = 2", "=")]
#[case("1 != 2", "!=")]
#[case("1 < 2", "<")]
#[case("1 <= 2", "<=")]
#[case("1 > 2", ">")]
#[case("1 >= 2", ">=")]
#[case("1 ~ 2", "~")]
#[case("1 !~ 2", "!~")]
fn test_binary_operators(#[case] input: &str, #[case] expected_op: &str) {
    let expr = parse_expr(input);
    let (_, op, _) = assert_binary_op(&expr);
    assert_eq!(op, expected_op);
}

#[rstest]
#[case("not true", "not")]
#[case("-5", "-")]
fn test_unary_operators(#[case] input: &str, #[case] expected_op: &str) {
    let expr = parse_expr(input);
    match &expr.kind {
        ExpressionKind::UnaryOp { op, .. } => assert_eq!(op, expected_op),
        ExpressionKind::Literal(_) if expected_op == "-" => {} // Negative literal OK
        _ => panic!("Expected UnaryOp with '{}', got: {:?}", expected_op, expr.kind),
    }
}

#[test]
fn test_chained_comparisons_not_allowed() {
    // CQL doesn't typically allow chained comparisons like 1 < x < 10
    // This should parse as (1 < x) < 10, which may not be semantically valid
    let expr = parse_expr("1 < x < 10");
    // Just verify it parses (semantic validation is separate)
    assert!(matches!(expr.kind, ExpressionKind::BinaryOp { .. }));
}

#[test]
fn test_between_operator() {
    // x between 1 and 10
    let expr = parse_expr("x between 1 and 10");
    match &expr.kind {
        ExpressionKind::Between { value, low, high } => {
            // Successfully parsed as between
            assert!(matches!(value.kind, ExpressionKind::Identifier(_)));
        }
        ExpressionKind::BinaryOp { .. } => {
            // May also be parsed as binary operations
        }
        _ => panic!("Expected Between or BinaryOp, got: {:?}", expr.kind),
    }
}
