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
//! - Code literals
//! - Concept literals

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

fn assert_literal(expr: &Expression) -> &Literal {
    match &expr.kind {
        ExpressionKind::Literal(lit) => lit,
        _ => panic!("Expected Literal, got: {:?}", expr.kind),
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
    // Negative is typically parsed as unary minus operator
    match &expr.kind {
        ExpressionKind::UnaryOp { op, operand } => {
            assert_eq!(op, "-");
            let lit = assert_literal(operand);
            assert!(matches!(lit, Literal::Integer(42)));
        }
        ExpressionKind::Literal(Literal::Integer(-42)) => {
            // Also valid if parser handles it as a literal
        }
        _ => panic!("Expected UnaryOp or negative Integer literal, got: {:?}", expr.kind),
    }
}

#[test]
fn test_integer_zero() {
    let expr = parse_expr("0");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::Integer(0)));
}

#[test]
fn test_integer_large() {
    let expr = parse_expr("9223372036854775807");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::Integer(9223372036854775807)));
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
fn test_decimal_trailing_zeros() {
    let expr = parse_expr("1.500");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Decimal(d) => {
            let expected = "1.500".parse::<rust_decimal::Decimal>().unwrap();
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
fn test_string_with_escape_quote() {
    let expr = parse_expr("'It\\'s working'");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::String(s) if s == "It's working"));
}

#[test]
fn test_string_with_escape_backslash() {
    let expr = parse_expr("'C:\\\\path\\\\file'");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::String(s) if s == "C:\\path\\file"));
}

#[test]
fn test_string_with_unicode() {
    let expr = parse_expr("'Hello 世界 🌍'");
    let lit = assert_literal(&expr);
    assert!(matches!(lit, Literal::String(s) if s == "Hello 世界 🌍"));
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
        Literal::Date { year, month, day } => {
            assert_eq!(*year, 2024);
            assert_eq!(*month, Some(3));
            assert_eq!(*day, Some(15));
        }
        _ => panic!("Expected Date literal, got: {:?}", lit),
    }
}

#[test]
fn test_date_year_month() {
    let expr = parse_expr("@2024-03");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Date { year, month, day } => {
            assert_eq!(*year, 2024);
            assert_eq!(*month, Some(3));
            assert_eq!(*day, None);
        }
        _ => panic!("Expected Date literal, got: {:?}", lit),
    }
}

#[test]
fn test_date_year_only() {
    let expr = parse_expr("@2024");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Date { year, month, day } => {
            assert_eq!(*year, 2024);
            assert_eq!(*month, None);
            assert_eq!(*day, None);
        }
        _ => panic!("Expected Date literal, got: {:?}", lit),
    }
}

#[test]
fn test_datetime_full() {
    let expr = parse_expr("@2024-03-15T10:30:00.123Z");
    let lit = assert_literal(&expr);
    match lit {
        Literal::DateTime { .. } => {
            // Successful parse of a datetime
        }
        _ => panic!("Expected DateTime literal, got: {:?}", lit),
    }
}

#[test]
fn test_datetime_with_timezone() {
    let expr = parse_expr("@2024-03-15T10:30:00+05:00");
    let lit = assert_literal(&expr);
    match lit {
        Literal::DateTime { timezone_offset, .. } => {
            assert_eq!(*timezone_offset, Some(300)); // +5 hours = 300 minutes
        }
        _ => panic!("Expected DateTime literal, got: {:?}", lit),
    }
}

#[test]
fn test_time_full() {
    let expr = parse_expr("@T10:30:00.123");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Time { hour, minute, second, millisecond } => {
            assert_eq!(*hour, 10);
            assert_eq!(*minute, 30);
            assert_eq!(*second, Some(0));
            assert_eq!(*millisecond, Some(123));
        }
        _ => panic!("Expected Time literal, got: {:?}", lit),
    }
}

#[test]
fn test_time_hour_minute() {
    let expr = parse_expr("@T10:30");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Time { hour, minute, second, millisecond } => {
            assert_eq!(*hour, 10);
            assert_eq!(*minute, 30);
            assert_eq!(*second, None);
            assert_eq!(*millisecond, None);
        }
        _ => panic!("Expected Time literal, got: {:?}", lit),
    }
}

#[test]
fn test_quantity_with_unit() {
    let expr = parse_expr("5 'mg'");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Quantity { value, unit } => {
            let expected = "5".parse::<rust_decimal::Decimal>().unwrap();
            assert_eq!(*value, expected);
            assert_eq!(unit, "mg");
        }
        _ => panic!("Expected Quantity literal, got: {:?}", lit),
    }
}

#[test]
fn test_quantity_decimal_with_unit() {
    let expr = parse_expr("2.5 'kg'");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Quantity { value, unit } => {
            let expected = "2.5".parse::<rust_decimal::Decimal>().unwrap();
            assert_eq!(*value, expected);
            assert_eq!(unit, "kg");
        }
        _ => panic!("Expected Quantity literal, got: {:?}", lit),
    }
}

#[test]
fn test_quantity_complex_unit() {
    let expr = parse_expr("120 'mm[Hg]'");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Quantity { value, unit } => {
            let expected = "120".parse::<rust_decimal::Decimal>().unwrap();
            assert_eq!(*value, expected);
            assert_eq!(unit, "mm[Hg]");
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
    let result = Parser::new().parse_expression(input);
    assert_eq!(result.is_ok(), should_parse, "Parse result for '{}' unexpected", input);
}

#[rstest]
#[case("3.14", true)]
#[case("0.5", true)]
#[case(".5", true)] // May or may not be valid depending on CQL spec
#[case("1.", true)]
fn test_decimal_variations(#[case] input: &str, #[case] should_parse: bool) {
    let result = Parser::new().parse_expression(input);
    assert_eq!(result.is_ok(), should_parse, "Parse result for '{}' unexpected", input);
}

#[rstest]
#[case("''", "")]
#[case("'hello'", "hello")]
#[case("'Hello World!'", "Hello World!")]
#[case("'with\\'quote'", "with'quote")]
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
    match &expr.kind {
        ExpressionKind::BinaryOp { left, op, right } => {
            assert_eq!(op, "+");
            assert!(matches!(assert_literal(left), Literal::Integer(1)));
            assert!(matches!(assert_literal(right), Literal::Integer(2)));
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_multiple_literals_in_list() {
    let expr = parse_expr("{1, 2, 3, 4, 5}");
    match &expr.kind {
        ExpressionKind::List(elements) => {
            assert_eq!(elements.len(), 5);
            for (i, elem) in elements.iter().enumerate() {
                let lit = assert_literal(elem);
                assert!(matches!(lit, Literal::Integer(n) if *n == (i as i64 + 1)));
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
        let result = Parser::new().parse_expression(input);
        assert!(result.is_ok(), "Failed to parse with whitespace: '{}'", input);
    }
}

#[test]
fn test_code_literal() {
    // Code literal: Code '8480-6' from "http://loinc.org"
    let expr = parse_expr("Code '8480-6' from \"http://loinc.org\"");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Code { code, system, display } => {
            assert_eq!(code, "8480-6");
            assert_eq!(system, "http://loinc.org");
            assert_eq!(*display, None);
        }
        _ => panic!("Expected Code literal, got: {:?}", lit),
    }
}

#[test]
fn test_code_literal_with_display() {
    let expr = parse_expr("Code '8480-6' from \"http://loinc.org\" display 'Systolic BP'");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Code { code, system, display } => {
            assert_eq!(code, "8480-6");
            assert_eq!(system, "http://loinc.org");
            assert_eq!(display.as_deref(), Some("Systolic BP"));
        }
        _ => panic!("Expected Code literal, got: {:?}", lit),
    }
}

#[test]
fn test_concept_literal() {
    // Concept { Code '8480-6' from "http://loinc.org" }
    let expr = parse_expr("Concept { Code '8480-6' from \"http://loinc.org\" }");
    let lit = assert_literal(&expr);
    match lit {
        Literal::Concept { codes, display } => {
            assert_eq!(codes.len(), 1);
            assert_eq!(codes[0].code, "8480-6");
            assert_eq!(*display, None);
        }
        _ => panic!("Expected Concept literal, got: {:?}", lit),
    }
}

#[test]
fn test_concept_literal_multiple_codes() {
    let expr = parse_expr(
        "Concept {
            Code '8480-6' from \"http://loinc.org\",
            Code '271649006' from \"http://snomed.info/sct\"
        }"
    );
    let lit = assert_literal(&expr);
    match lit {
        Literal::Concept { codes, .. } => {
            assert_eq!(codes.len(), 2);
            assert_eq!(codes[0].code, "8480-6");
            assert_eq!(codes[1].code, "271649006");
        }
        _ => panic!("Expected Concept literal, got: {:?}", lit),
    }
}
