//! Tests for parsing CQL literal values
//!
//! Covers all CQL literal types:
//! - Integers
//! - Decimals
//! - Strings
//! - Booleans
//! - Null
//! - Dates
//! - DateTimes
//! - Times
//! - Quantities

use octofhir_cql_ast::{Expression, Literal};
use octofhir_cql_parser::parse_expression;
use rstest::rstest;

fn parse_expr(input: &str) -> Expression {
    parse_expression(input)
        .unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", input, e))
        .inner
}

fn assert_literal(expr: &Expression) -> &Literal {
    match expr {
        Expression::Literal(lit) => lit,
        _ => panic!("Expected Literal, got: {:?}", expr),
    }
}

#[test]
fn test_integer_positive() {
    let expr = parse_expr("42");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::Integer(42)));
}

#[test]
fn test_integer_negative() {
    let expr = parse_expr("-42");
    // Negative is parsed as unary minus operator
    match &expr {
        Expression::UnaryOp(unary) => {
            assert!(matches!(unary.op, octofhir_cql_ast::UnaryOp::Negate));
            let lit = assert_literal(&unary.operand.inner);
            assert!(matches!(lit, Literal::Integer(42)));
        }
        _ => panic!("Expected UnaryOp, got: {:?}", expr),
    }
}

#[test]
fn test_integer_zero() {
    let expr = parse_expr("0");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::Integer(0)));
}

#[test]
fn test_decimal_basic() {
    let expr = parse_expr("3.14");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Decimal(d) => {
            let expected = "3.14".parse::<rust_decimal::Decimal>().unwrap();
            assert_eq!(*d, expected);
        }
        _ => panic!("Expected Decimal literal, got: {:?}", lit),
    }
}

#[test]
fn test_decimal_leading_zero() {
    let expr = parse_expr("0.5");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Decimal(d) => {
            let expected = "0.5".parse::<rust_decimal::Decimal>().unwrap();
            assert_eq!(*d, expected);
        }
        _ => panic!("Expected Decimal literal, got: {:?}", lit),
    }
}

#[test]
fn test_string_empty() {
    let expr = parse_expr("''");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::String(s) if s.is_empty()));
}

#[test]
fn test_string_simple() {
    let expr = parse_expr("'hello'");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::String(s) if s == "hello"));
}

#[test]
fn test_string_with_spaces() {
    let expr = parse_expr("'hello world'");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::String(s) if s == "hello world"));
}

#[test]
fn test_boolean_true() {
    let expr = parse_expr("true");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::Boolean(true)));
}

#[test]
fn test_boolean_false() {
    let expr = parse_expr("false");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::Boolean(false)));
}

#[test]
fn test_null() {
    let expr = parse_expr("null");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::Null));
}

#[test]
fn test_date_full() {
    let expr = parse_expr("@2024-03-15");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Date(date) => {
            assert_eq!(date.year, 2024);
            assert_eq!(date.month, Some(3));
            assert_eq!(date.day, Some(15));
        }
        _ => panic!("Expected Date literal, got: {:?}", lit),
    }
}

#[test]
fn test_date_year_month() {
    let expr = parse_expr("@2024-03");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Date(date) => {
            assert_eq!(date.year, 2024);
            assert_eq!(date.month, Some(3));
            assert_eq!(date.day, None);
        }
        _ => panic!("Expected Date literal, got: {:?}", lit),
    }
}

#[test]
fn test_date_year_only() {
    let expr = parse_expr("@2024");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Date(date) => {
            assert_eq!(date.year, 2024);
            assert_eq!(date.month, None);
            assert_eq!(date.day, None);
        }
        _ => panic!("Expected Date literal, got: {:?}", lit),
    }
}

#[test]
fn test_datetime_full() {
    let expr = parse_expr("@2024-03-15T10:30:00");
    let lit = assert_literal(&expr);
    match lit {
        Literal::DateTime(dt) => {
            assert_eq!(dt.date.year, 2024);
            assert_eq!(dt.date.month, Some(3));
            assert_eq!(dt.date.day, Some(15));
            assert_eq!(dt.hour, Some(10));
            assert_eq!(dt.minute, Some(30));
            assert_eq!(dt.second, Some(0));
        }
        _ => panic!("Expected DateTime literal, got: {:?}", lit),
    }
}

#[test]
fn test_time_full() {
    let expr = parse_expr("@T10:30:00");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Time(time) => {
            assert_eq!(time.hour, 10);
            assert_eq!(time.minute, Some(30));
            assert_eq!(time.second, Some(0));
        }
        _ => panic!("Expected Time literal, got: {:?}", lit),
    }
}

#[test]
fn test_time_hour_minute() {
    let expr = parse_expr("@T10:30");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Time(time) => {
            assert_eq!(time.hour, 10);
            assert_eq!(time.minute, Some(30));
            assert_eq!(time.second, None);
        }
        _ => panic!("Expected Time literal, got: {:?}", lit),
    }
}

#[test]
fn test_quantity_with_unit() {
    let expr = parse_expr("5 'mg'");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Quantity(q) => {
            let expected = "5".parse::<rust_decimal::Decimal>().unwrap();
            assert_eq!(q.value, expected);
            assert_eq!(q.unit.as_deref(), Some("mg"));
        }
        _ => panic!("Expected Quantity literal, got: {:?}", lit),
    }
}

#[test]
fn test_quantity_decimal_with_unit() {
    let expr = parse_expr("2.5 'kg'");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Quantity(q) => {
            let expected = "2.5".parse::<rust_decimal::Decimal>().unwrap();
            assert_eq!(q.value, expected);
            assert_eq!(q.unit.as_deref(), Some("kg"));
        }
        _ => panic!("Expected Quantity literal, got: {:?}", lit),
    }
}

#[rstest]
#[case("42", true)]
#[case("-1", true)]
#[case("0", true)]
#[case("999999", true)]
fn test_integer_variations(#[case] input: &str, #[case] should_parse: bool) {
    let result = parse_expression(input);
    assert_eq!(result.is_ok(), should_parse, "Parse result for '{}' unexpected", input);
}

#[rstest]
#[case("3.14", true)]
#[case("0.5", true)]
fn test_decimal_variations(#[case] input: &str, #[case] should_parse: bool) {
    let result = parse_expression(input);
    assert_eq!(result.is_ok(), should_parse, "Parse result for '{}' unexpected", input);
}

#[rstest]
#[case("''", "")]
#[case("'hello'", "hello")]
#[case("'Hello World!'", "Hello World!")]
fn test_string_variations(#[case] input: &str, #[case] expected: &str) {
    let expr = parse_expr(input);
    let lit = assert_literal(&expr);
    match lit {
        Literal::String(s) => assert_eq!(s, expected),
        _ => panic!("Expected String literal"),
    }
}

#[test]
fn test_literal_as_expression() {
    // Literals should work in expression contexts
    let expr = parse_expr("1 + 2");
    match &expr {
        Expression::BinaryOp(binop) => {
            assert!(matches!(binop.op, octofhir_cql_ast::BinaryOp::Add));
            assert!(matches!(assert_literal(&binop.left.inner), Literal::Integer(1)));
            assert!(matches!(assert_literal(&binop.right.inner), Literal::Integer(2)));
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_multiple_literals_in_list() {
    let expr = parse_expr("{1, 2, 3, 4, 5}");
    match &expr {
        Expression::List(list) => {
            assert_eq!(list.elements.len(), 5);
            for (i, elem) in list.elements.iter().enumerate() {
                let lit = assert_literal(&elem.inner);
                assert!(matches!(lit, Literal::Integer(n) if *n == (i as i32 + 1)));
            }
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_literals_with_whitespace() {
    // Should handle various whitespace
    let inputs = vec![
        "  42  ",
        "\t42\t",
        "\n42\n",
        "  'hello'  ",
    ];

    for input in inputs {
        let result = parse_expression(input);
        assert!(result.is_ok(), "Failed to parse with whitespace: '{}'", input);
    }
}
