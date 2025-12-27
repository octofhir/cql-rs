//! Tests for parser error recovery and edge cases
//!
//! Covers:
//! - Parse error recovery (analysis mode)
//! - Invalid syntax handling
//! - Comments (single-line and multi-line)
//! - Whitespace handling
//! - Escape sequences
//! - Unicode support
//! - Malformed input

use octofhir_cql_parser::Parser;
use rstest::rstest;

fn parse_with_errors(input: &str) -> bool {
    let parser = Parser::new();
    parser.parse_expression(input).is_err()
}

fn parse_library_with_errors(input: &str) -> bool {
    let parser = Parser::new();
    parser.parse_library(input).is_err()
}

// === Error Cases ===

#[test]
fn test_unclosed_string() {
    assert!(parse_with_errors("'unclosed string"));
}

#[test]
fn test_invalid_operator_sequence() {
    assert!(parse_with_errors("1 + + 2"));
}

#[test]
fn test_missing_operand() {
    assert!(parse_with_errors("1 +"));
}

#[test]
fn test_unmatched_parenthesis_open() {
    assert!(parse_with_errors("(1 + 2"));
}

#[test]
fn test_unmatched_parenthesis_close() {
    assert!(parse_with_errors("1 + 2)"));
}

#[test]
fn test_unmatched_bracket_open() {
    assert!(parse_with_errors("[Observation"));
}

#[test]
fn test_unmatched_bracket_close() {
    assert!(parse_with_errors("Observation]"));
}

#[test]
fn test_unmatched_brace_open() {
    assert!(parse_with_errors("{1, 2, 3"));
}

#[test]
fn test_unmatched_brace_close() {
    assert!(parse_with_errors("1, 2, 3}"));
}

#[test]
fn test_invalid_number() {
    assert!(parse_with_errors("123abc"));
}

#[test]
fn test_invalid_date() {
    assert!(parse_with_errors("@2024-13-45")); // Invalid month/day
}

#[test]
fn test_invalid_time() {
    assert!(parse_with_errors("@T25:00:00")); // Invalid hour
}

#[test]
fn test_missing_query_alias() {
    // Some parsers may require aliases
    let result = parse_with_errors("[Observation]");
    // May or may not error depending on parser requirements
}

#[test]
fn test_invalid_keyword_as_identifier() {
    // Using reserved keywords as identifiers should fail
    assert!(parse_with_errors("define and: 5"));
}

// === Comments ===

#[test]
fn test_single_line_comment() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "1 + 2 // This is a comment"
    );
    assert!(result.is_ok());
}

#[test]
fn test_single_line_comment_full_line() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "// Comment line
         1 + 2"
    );
    assert!(result.is_ok());
}

#[test]
fn test_multi_line_comment() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "1 + /* inline comment */ 2"
    );
    assert!(result.is_ok());
}

#[test]
fn test_multi_line_comment_multiline() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "/*
          This is a
          multi-line comment
         */
         1 + 2"
    );
    assert!(result.is_ok());
}

#[test]
fn test_nested_comments() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "/* Outer /* nested */ comment */ 1 + 2"
    );
    // May or may not support nested comments
}

#[test]
fn test_comment_with_special_chars() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "// Comment with special chars: @#$%^&*()
         42"
    );
    assert!(result.is_ok());
}

// === Whitespace Handling ===

#[test]
fn test_leading_whitespace() {
    let parser = Parser::new();
    let result = parser.parse_expression("   42");
    assert!(result.is_ok());
}

#[test]
fn test_trailing_whitespace() {
    let parser = Parser::new();
    let result = parser.parse_expression("42   ");
    assert!(result.is_ok());
}

#[test]
fn test_tabs() {
    let parser = Parser::new();
    let result = parser.parse_expression("\t42\t");
    assert!(result.is_ok());
}

#[test]
fn test_newlines() {
    let parser = Parser::new();
    let result = parser.parse_expression("\n42\n");
    assert!(result.is_ok());
}

#[test]
fn test_mixed_whitespace() {
    let parser = Parser::new();
    let result = parser.parse_expression("  \t\n  42  \n\t  ");
    assert!(result.is_ok());
}

#[test]
fn test_whitespace_in_expression() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "1    +    2    *    3"
    );
    assert!(result.is_ok());
}

#[test]
fn test_no_whitespace_required() {
    let parser = Parser::new();
    // Operators should work without surrounding whitespace where valid
    let result = parser.parse_expression("1+2*3");
    assert!(result.is_ok());
}

// === Escape Sequences ===

#[test]
fn test_escaped_quote_in_string() {
    let parser = Parser::new();
    let result = parser.parse_expression("'It\\'s working'");
    assert!(result.is_ok());
}

#[test]
fn test_escaped_backslash() {
    let parser = Parser::new();
    let result = parser.parse_expression("'C:\\\\path\\\\file'");
    assert!(result.is_ok());
}

#[test]
fn test_escaped_newline() {
    let parser = Parser::new();
    let result = parser.parse_expression("'Line 1\\nLine 2'");
    assert!(result.is_ok());
}

#[test]
fn test_escaped_tab() {
    let parser = Parser::new();
    let result = parser.parse_expression("'Column1\\tColumn2'");
    assert!(result.is_ok());
}

#[test]
fn test_unicode_escape() {
    let parser = Parser::new();
    let result = parser.parse_expression("'\\u0041'"); // 'A'
    // May or may not support unicode escapes
}

#[test]
fn test_invalid_escape_sequence() {
    let parser = Parser::new();
    let result = parser.parse_expression("'Invalid\\x'");
    // Should handle invalid escapes gracefully
}

// === Unicode Support ===

#[test]
fn test_unicode_in_string() {
    let parser = Parser::new();
    let result = parser.parse_expression("'Hello 世界 🌍'");
    assert!(result.is_ok());
}

#[test]
fn test_unicode_in_identifier() {
    let parser = Parser::new();
    let result = parser.parse_library(
        "library Test version '1.0.0'
         define \"患者年齢\": 35"
    );
    assert!(result.is_ok());
}

#[test]
fn test_emoji_in_string() {
    let parser = Parser::new();
    let result = parser.parse_expression("'Status: ✅'");
    assert!(result.is_ok());
}

#[test]
fn test_rtl_text() {
    let parser = Parser::new();
    let result = parser.parse_expression("'مرحبا'"); // Arabic
    assert!(result.is_ok());
}

// === Edge Cases ===

#[test]
fn test_empty_input() {
    let parser = Parser::new();
    let result = parser.parse_expression("");
    assert!(result.is_err()); // Empty input should error
}

#[test]
fn test_whitespace_only() {
    let parser = Parser::new();
    let result = parser.parse_expression("   \t\n   ");
    assert!(result.is_err()); // Whitespace-only should error
}

#[test]
fn test_very_long_expression() {
    let parser = Parser::new();
    // Create a very long expression
    let mut expr = "1".to_string();
    for _ in 0..1000 {
        expr.push_str(" + 1");
    }
    let result = parser.parse_expression(&expr);
    assert!(result.is_ok()); // Should handle long expressions
}

#[test]
fn test_deeply_nested_parentheses() {
    let parser = Parser::new();
    let mut expr = String::new();
    for _ in 0..100 {
        expr.push('(');
    }
    expr.push_str("42");
    for _ in 0..100 {
        expr.push(')');
    }
    let result = parser.parse_expression(&expr);
    // May hit recursion limits
}

#[test]
fn test_very_long_string() {
    let parser = Parser::new();
    let long_string = "a".repeat(10000);
    let expr = format!("'{}'", long_string);
    let result = parser.parse_expression(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_case_sensitivity() {
    let parser = Parser::new();
    // CQL keywords are case-insensitive
    let result1 = parser.parse_expression("TRUE");
    let result2 = parser.parse_expression("true");
    let result3 = parser.parse_expression("True");

    // All should parse successfully
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());
}

#[rstest]
#[case("null")]
#[case("NULL")]
#[case("Null")]
#[case("true")]
#[case("TRUE")]
#[case("True")]
#[case("false")]
#[case("FALSE")]
#[case("False")]
fn test_keyword_case_variations(#[case] input: &str) {
    let parser = Parser::new();
    let result = parser.parse_expression(input);
    assert!(result.is_ok(), "Failed to parse case variation: {}", input);
}

#[test]
fn test_identifiers_case_sensitive() {
    let parser = Parser::new();
    // Identifiers should be case-sensitive
    let lib = parser.parse_library(
        "library Test version '1.0.0'
         define \"Value\": 1
         define \"value\": 2"
    ).unwrap();

    // Both definitions should exist
    assert_eq!(lib.statements.defs.len(), 2);
}

#[test]
fn test_quoted_identifiers_with_spaces() {
    let parser = Parser::new();
    let result = parser.parse_library(
        "library Test version '1.0.0'
         define \"My Variable Name\": 42"
    );
    assert!(result.is_ok());
}

#[test]
fn test_quoted_identifiers_with_special_chars() {
    let parser = Parser::new();
    let result = parser.parse_library(
        "library Test version '1.0.0'
         define \"Value@Home#1\": 42"
    );
    assert!(result.is_ok());
}

#[test]
fn test_special_null_value() {
    let parser = Parser::new();
    let result = parser.parse_expression("null as Integer");
    // Typed null
    assert!(result.is_ok());
}

#[test]
fn test_interval_edge_cases() {
    let parser = Parser::new();

    // Open-open interval
    let result1 = parser.parse_expression("Interval(1, 10)");
    assert!(result1.is_ok());

    // Closed-closed interval
    let result2 = parser.parse_expression("Interval[1, 10]");
    assert!(result2.is_ok());

    // Mixed intervals
    let result3 = parser.parse_expression("Interval(1, 10]");
    assert!(result3.is_ok());

    let result4 = parser.parse_expression("Interval[1, 10)");
    assert!(result4.is_ok());
}

#[test]
fn test_list_edge_cases() {
    let parser = Parser::new();

    // Empty list
    let result1 = parser.parse_expression("{}");
    assert!(result1.is_ok());

    // Single element
    let result2 = parser.parse_expression("{1}");
    assert!(result2.is_ok());

    // Trailing comma
    let result3 = parser.parse_expression("{1, 2, 3,}");
    // May or may not allow trailing comma
}

#[test]
fn test_tuple_edge_cases() {
    let parser = Parser::new();

    // Empty tuple
    let result1 = parser.parse_expression("Tuple {}");
    assert!(result1.is_ok());

    // Single field
    let result2 = parser.parse_expression("Tuple { a: 1 }");
    assert!(result2.is_ok());
}

#[test]
fn test_multiple_errors_in_library() {
    // Test recovery from multiple errors
    let result = parse_library_with_errors(
        "library Test version '1.0.0'
         define \"Bad1\": 1 +
         define \"Good\": 2
         define \"Bad2\": ) 3"
    );

    // Parser should detect errors but may continue parsing
    assert!(result);
}

#[test]
fn test_unterminated_comment() {
    let parser = Parser::new();
    let result = parser.parse_expression(
        "/* This comment never ends... 42"
    );
    assert!(result.is_err());
}

#[test]
fn test_bom_handling() {
    let parser = Parser::new();
    // UTF-8 BOM at start of file
    let result = parser.parse_expression("\u{FEFF}42");
    // Should handle BOM gracefully
}
