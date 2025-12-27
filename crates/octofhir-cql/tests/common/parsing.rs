//! Parsing test helpers
//!
//! Utilities for testing CQL parsing, including assertion helpers
//! and utilities for working with parse results and diagnostics.

use octofhir_cql_parser::Parser;
use octofhir_cql_ast::*;
use octofhir_cql_diagnostics::Diagnostic;

/// Parse CQL expression and return the result
pub fn parse_expression(input: &str) -> Result<Expression, Vec<Diagnostic>> {
    let parser = Parser::new();
    parser.parse_expression(input)
}

/// Parse CQL expression and expect success
pub fn parse_expression_ok(input: &str) -> Expression {
    parse_expression(input).expect(&format!("Failed to parse expression: {}", input))
}

/// Parse CQL expression and expect error
pub fn parse_expression_err(input: &str) -> Vec<Diagnostic> {
    match parse_expression(input) {
        Ok(_) => panic!("Expected parse error but got success for: {}", input),
        Err(diagnostics) => diagnostics,
    }
}

/// Parse CQL library and return the result
pub fn parse_library(input: &str) -> Result<Library, Vec<Diagnostic>> {
    let parser = Parser::new();
    parser.parse_library(input)
}

/// Parse CQL library and expect success
pub fn parse_library_ok(input: &str) -> Library {
    parse_library(input).expect(&format!("Failed to parse library: {}", input))
}

/// Parse CQL library and expect error
pub fn parse_library_err(input: &str) -> Vec<Diagnostic> {
    match parse_library(input) {
        Ok(_) => panic!("Expected parse error but got success"),
        Err(diagnostics) => diagnostics,
    }
}

/// Assert that an expression is a literal
#[track_caller]
pub fn assert_literal(expr: &Expression) -> &Literal {
    match &expr.kind {
        ExpressionKind::Literal(lit) => lit,
        _ => panic!("Expected Literal, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is a specific literal type
#[track_caller]
pub fn assert_integer_literal(expr: &Expression, expected: i64) {
    let lit = assert_literal(expr);
    match lit {
        Literal::Integer(val) => assert_eq!(*val, expected),
        _ => panic!("Expected Integer literal, got: {:?}", lit),
    }
}

/// Assert that an expression is a string literal
#[track_caller]
pub fn assert_string_literal(expr: &Expression, expected: &str) {
    let lit = assert_literal(expr);
    match lit {
        Literal::String(val) => assert_eq!(val, expected),
        _ => panic!("Expected String literal, got: {:?}", lit),
    }
}

/// Assert that an expression is a boolean literal
#[track_caller]
pub fn assert_boolean_literal(expr: &Expression, expected: bool) {
    let lit = assert_literal(expr);
    match lit {
        Literal::Boolean(val) => assert_eq!(*val, expected),
        _ => panic!("Expected Boolean literal, got: {:?}", lit),
    }
}

/// Assert that an expression is a binary operation
#[track_caller]
pub fn assert_binary_op(expr: &Expression) -> (&Expression, &str, &Expression) {
    match &expr.kind {
        ExpressionKind::BinaryOp { left, op, right } => (left.as_ref(), op.as_str(), right.as_ref()),
        _ => panic!("Expected BinaryOp, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is a unary operation
#[track_caller]
pub fn assert_unary_op(expr: &Expression) -> (&str, &Expression) {
    match &expr.kind {
        ExpressionKind::UnaryOp { op, operand } => (op.as_str(), operand.as_ref()),
        _ => panic!("Expected UnaryOp, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is an identifier reference
#[track_caller]
pub fn assert_identifier(expr: &Expression, expected: &str) {
    match &expr.kind {
        ExpressionKind::Identifier(name) => assert_eq!(name, expected),
        _ => panic!("Expected Identifier, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is a function call
#[track_caller]
pub fn assert_function_call(expr: &Expression) -> (&str, &[Expression]) {
    match &expr.kind {
        ExpressionKind::FunctionCall { name, arguments } => (name.as_str(), arguments.as_slice()),
        _ => panic!("Expected FunctionCall, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is a query
#[track_caller]
pub fn assert_query(expr: &Expression) -> &Query {
    match &expr.kind {
        ExpressionKind::Query(query) => query,
        _ => panic!("Expected Query, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is a member access
#[track_caller]
pub fn assert_member_access(expr: &Expression) -> (&Expression, &str) {
    match &expr.kind {
        ExpressionKind::MemberAccess { object, member } => (object.as_ref(), member.as_str()),
        _ => panic!("Expected MemberAccess, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is a list
#[track_caller]
pub fn assert_list(expr: &Expression) -> &[Expression] {
    match &expr.kind {
        ExpressionKind::List(elements) => elements.as_slice(),
        _ => panic!("Expected List, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is a tuple
#[track_caller]
pub fn assert_tuple(expr: &Expression) -> &[(String, Expression)] {
    match &expr.kind {
        ExpressionKind::Tuple(elements) => elements.as_slice(),
        _ => panic!("Expected Tuple, got: {:?}", expr.kind),
    }
}

/// Assert that an expression is an interval
#[track_caller]
pub fn assert_interval(expr: &Expression) -> (&Expression, &Expression, bool, bool) {
    match &expr.kind {
        ExpressionKind::Interval { low, high, low_closed, high_closed } => {
            (low.as_ref(), high.as_ref(), *low_closed, *high_closed)
        }
        _ => panic!("Expected Interval, got: {:?}", expr.kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer_literal() {
        let expr = parse_expression_ok("42");
        assert_integer_literal(&expr, 42);
    }

    #[test]
    fn test_parse_string_literal() {
        let expr = parse_expression_ok("'hello'");
        assert_string_literal(&expr, "hello");
    }

    #[test]
    fn test_parse_boolean_literal() {
        let expr = parse_expression_ok("true");
        assert_boolean_literal(&expr, true);
    }

    #[test]
    fn test_parse_binary_op() {
        let expr = parse_expression_ok("1 + 2");
        let (left, op, right) = assert_binary_op(&expr);
        assert_eq!(op, "+");
        assert_integer_literal(left, 1);
        assert_integer_literal(right, 2);
    }

    #[test]
    fn test_parse_identifier() {
        let expr = parse_expression_ok("PatientAge");
        assert_identifier(&expr, "PatientAge");
    }

    #[test]
    #[should_panic(expected = "Failed to parse expression")]
    fn test_parse_error() {
        parse_expression_ok("1 + + 2");
    }
}
