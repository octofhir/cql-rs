//! Tests for CQL operator parsing and precedence
//!
//! Covers:
//! - Arithmetic operators (+, -, *, /, div, mod, ^)
//! - Comparison operators (=, !=, <, >, <=, >=, ~, !~)
//! - Logical operators (and, or, xor, not)
//! - Membership operators (in, contains)
//! - Operator precedence and associativity

use octofhir_cql_ast::{BinaryOp, Expression, UnaryOp};
use octofhir_cql_parser::parse_expression;
use rstest::rstest;

fn parse_expr(input: &str) -> Expression {
    parse_expression(input)
        .unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", input, e))
        .inner
}

fn assert_binary_op(expr: &Expression) -> (&Expression, BinaryOp, &Expression) {
    match expr {
        Expression::BinaryOp(binop) => (&binop.left.inner, binop.op.clone(), &binop.right.inner),
        _ => panic!("Expected BinaryOp, got: {:?}", expr),
    }
}

fn assert_unary_op(expr: &Expression) -> (UnaryOp, &Expression) {
    match expr {
        Expression::UnaryOp(unary) => (unary.op.clone(), &unary.operand.inner),
        _ => panic!("Expected UnaryOp, got: {:?}", expr),
    }
}

// === Arithmetic Operators ===

#[test]
fn test_addition() {
    let expr = parse_expr("1 + 2");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Add));
}

#[test]
fn test_subtraction() {
    let expr = parse_expr("5 - 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Subtract));
}

#[test]
fn test_multiplication() {
    let expr = parse_expr("4 * 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Multiply));
}

#[test]
fn test_division() {
    let expr = parse_expr("10 / 2");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Divide));
}

#[test]
fn test_integer_division() {
    let expr = parse_expr("10 div 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::TruncatedDivide));
}

#[test]
fn test_modulo() {
    let expr = parse_expr("10 mod 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Modulo));
}

#[test]
fn test_power() {
    let expr = parse_expr("2 ^ 3");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Power));
}

#[test]
fn test_unary_minus() {
    let expr = parse_expr("-5");
    let (op, _) = assert_unary_op(&expr);
    assert!(matches!(op, UnaryOp::Negate));
}

#[test]
fn test_unary_plus() {
    let expr = parse_expr("+5");
    let (op, _) = assert_unary_op(&expr);
    assert!(matches!(op, UnaryOp::Plus));
}

// === Comparison Operators ===

#[test]
fn test_equal() {
    let expr = parse_expr("a = b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Equal));
}

#[test]
fn test_not_equal() {
    let expr = parse_expr("a != b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::NotEqual));
}

#[test]
fn test_less_than() {
    let expr = parse_expr("a < b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Less));
}

#[test]
fn test_greater_than() {
    let expr = parse_expr("a > b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Greater));
}

#[test]
fn test_less_or_equal() {
    let expr = parse_expr("a <= b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::LessOrEqual));
}

#[test]
fn test_greater_or_equal() {
    let expr = parse_expr("a >= b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::GreaterOrEqual));
}

#[test]
fn test_equivalent() {
    let expr = parse_expr("a ~ b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Equivalent));
}

#[test]
fn test_not_equivalent() {
    let expr = parse_expr("a !~ b");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::NotEquivalent));
}

// === Logical Operators ===

#[test]
fn test_and() {
    let expr = parse_expr("true and false");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::And));
}

#[test]
fn test_or() {
    let expr = parse_expr("true or false");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Or));
}

#[test]
fn test_xor() {
    let expr = parse_expr("true xor false");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Xor));
}

#[test]
fn test_not() {
    let expr = parse_expr("not true");
    let (op, _) = assert_unary_op(&expr);
    assert!(matches!(op, UnaryOp::Not));
}

#[test]
fn test_implies() {
    let expr = parse_expr("true implies false");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Implies));
}

// === Membership Operators ===

#[test]
fn test_in() {
    let expr = parse_expr("1 in {1, 2, 3}");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::In));
}

#[test]
fn test_contains() {
    let expr = parse_expr("{1, 2, 3} contains 1");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Contains));
}

// === String Operators ===

#[test]
fn test_concatenate() {
    let expr = parse_expr("'hello' & ' world'");
    let (_, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Concatenate));
}

// === Operator Precedence ===

#[test]
fn test_multiplication_over_addition() {
    // 1 + 2 * 3 should be parsed as 1 + (2 * 3)
    let expr = parse_expr("1 + 2 * 3");
    let (left, op, right) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Add));
    // Left should be literal 1
    assert!(matches!(left, Expression::Literal(_)));
    // Right should be 2 * 3
    let (_, right_op, _) = assert_binary_op(right);
    assert!(matches!(right_op, BinaryOp::Multiply));
}

#[test]
fn test_and_over_or() {
    // true or false and false should be parsed as true or (false and false)
    let expr = parse_expr("true or false and false");
    let (_, op, right) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Or));
    // Right should be false and false
    let (_, right_op, _) = assert_binary_op(right);
    assert!(matches!(right_op, BinaryOp::And));
}

#[test]
fn test_comparison_over_logical() {
    // a < b and c > d should be parsed as (a < b) and (c > d)
    let expr = parse_expr("a < b and c > d");
    let (left, op, right) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::And));
    // Both sides should be comparisons
    let (_, left_op, _) = assert_binary_op(left);
    assert!(matches!(left_op, BinaryOp::Less));
    let (_, right_op, _) = assert_binary_op(right);
    assert!(matches!(right_op, BinaryOp::Greater));
}

#[test]
fn test_power_right_associative() {
    // 2 ^ 3 ^ 4 should be parsed as 2 ^ (3 ^ 4)
    let expr = parse_expr("2 ^ 3 ^ 4");
    let (_, op, right) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Power));
    // Right should be 3 ^ 4
    let (_, right_op, _) = assert_binary_op(right);
    assert!(matches!(right_op, BinaryOp::Power));
}

#[test]
fn test_parentheses_override_precedence() {
    // (1 + 2) * 3 should be parsed with addition first
    let expr = parse_expr("(1 + 2) * 3");
    let (left, op, _) = assert_binary_op(&expr);
    assert!(matches!(op, BinaryOp::Multiply));
    // Left should be 1 + 2
    let (_, left_op, _) = assert_binary_op(left);
    assert!(matches!(left_op, BinaryOp::Add));
}

// === Complex Expressions ===

#[rstest]
#[case("1 + 2 + 3", true)]
#[case("1 * 2 * 3", true)]
#[case("true and false or true", true)]
#[case("a = b and c != d", true)]
#[case("(a + b) * (c - d)", true)]
fn test_complex_expressions(#[case] input: &str, #[case] should_parse: bool) {
    let result = parse_expression(input);
    assert_eq!(result.is_ok(), should_parse, "Parse result for '{}' unexpected", input);
}

// === Type Assertions ===

#[test]
fn test_null_as_type() {
    let expr = parse_expr("null as Integer");
    match &expr {
        Expression::As(as_expr) => {
            assert!(matches!(&as_expr.operand.inner, Expression::Literal(octofhir_cql_ast::Literal::Null)));
        }
        _ => panic!("Expected As, got: {:?}", expr),
    }
}

#[test]
fn test_parenthesized_null_as_type() {
    let expr = parse_expr("(null as Integer)");
    match &expr {
        Expression::As(as_expr) => {
            assert!(matches!(&as_expr.operand.inner, Expression::Literal(octofhir_cql_ast::Literal::Null)));
        }
        _ => panic!("Expected As, got: {:?}", expr),
    }
}

#[test]
fn test_null_as_type_after_interval() {
    let result = parse_expression("(null as Integer) after Interval[1, 10]");
    assert!(result.is_ok(), "Failed to parse: {:?}", result);
}
