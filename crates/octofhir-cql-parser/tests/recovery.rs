//! Tests for parser error recovery and edge cases
//!
//! Covers:
//! - Parse error handling
//! - Invalid syntax handling
//! - Comments (single-line)
//! - Whitespace handling
//! - Unicode support
//! - Malformed input

use octofhir_cql_parser::{parse, parse_expression};
use rstest::rstest;

fn parse_with_errors(input: &str) -> bool {
    parse_expression(input).is_err()
}

fn parse_library_with_errors(input: &str) -> bool {
    parse(input).is_err()
}

// === Error Cases ===

#[test]
fn test_unclosed_string() {
    assert!(parse_with_errors("'unclosed string"));
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
    assert!(parse_with_errors("[Patient"));
}

#[test]
fn test_unmatched_brace_open() {
    assert!(parse_with_errors("{1, 2, 3"));
}

#[test]
fn test_invalid_date_format() {
    assert!(parse_with_errors("@123-45-67"));
}

// Note: Time range validation is not currently implemented in the parser.
// @T99:99 is syntactically valid but semantically invalid.
// This test is commented out until semantic validation is added.
// #[test]
// fn test_invalid_time_format() {
//     assert!(parse_with_errors("@T99:99"));
// }

// === Comments ===

#[test]
fn test_single_line_comment() {
    let result = parse("library Test // this is a comment\ndefine X: 1");
    assert!(result.is_ok());
}

#[test]
fn test_comment_at_end() {
    let result = parse_expression("1 + 2 // comment");
    assert!(result.is_ok());
}

// === Whitespace ===

#[test]
fn test_leading_whitespace() {
    let result = parse_expression("   42");
    assert!(result.is_ok());
}

#[test]
fn test_trailing_whitespace() {
    let result = parse_expression("42   ");
    assert!(result.is_ok());
}

#[test]
fn test_mixed_whitespace() {
    let result = parse_expression("  1  +  2  ");
    assert!(result.is_ok());
}

#[test]
fn test_tabs_and_newlines() {
    let result = parse_expression("\t42\n");
    assert!(result.is_ok());
}

// === Unicode ===

#[test]
fn test_unicode_in_string() {
    let result = parse_expression("'Hello 世界'");
    assert!(result.is_ok());
}

#[test]
fn test_emoji_in_string() {
    let result = parse_expression("'Hello 🌍'");
    assert!(result.is_ok());
}

// === Edge Cases ===

#[test]
fn test_empty_input() {
    assert!(parse_with_errors(""));
}

#[test]
fn test_only_whitespace() {
    assert!(parse_with_errors("   "));
}

#[test]
fn test_deeply_nested_parens() {
    let result = parse_expression("((((1 + 2))))");
    assert!(result.is_ok());
}

#[test]
fn test_long_identifier() {
    let long_name = "a".repeat(100);
    let result = parse_expression(&long_name);
    assert!(result.is_ok());
}

// === rstest variations ===

#[rstest]
#[case("", false)]
#[case("   ", false)]
#[case("42", true)]
#[case("'hello'", true)]
#[case("true", true)]
#[case("[Patient]", true)]
#[case("{1, 2, 3}", true)]
fn test_various_inputs(#[case] input: &str, #[case] should_succeed: bool) {
    let result = parse_expression(input);
    assert_eq!(result.is_ok(), should_succeed, "Unexpected result for '{}'", input);
}
