//! Tests for CQL query parsing
//!
//! Covers:
//! - Retrieve expressions
//! - Property access
//! - If-then-else expressions
//! - Lists

use octofhir_cql_ast::Expression;
use octofhir_cql_parser::parse_expression;
use rstest::rstest;

fn parse_expr(input: &str) -> Expression {
    parse_expression(input)
        .unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", input, e))
        .inner
}

// === Retrieve Expressions ===

#[test]
fn test_simple_retrieve() {
    let expr = parse_expr("[Patient]");
    match &expr {
        Expression::Retrieve(r) => {
            match &r.data_type.inner {
                octofhir_cql_ast::TypeSpecifier::Named(named) => {
                    assert_eq!(named.name, "Patient");
                }
                _ => panic!("Expected Named type specifier"),
            }
        }
        _ => panic!("Expected Retrieve, got: {:?}", expr),
    }
}

#[test]
fn test_retrieve_observation() {
    let expr = parse_expr("[Observation]");
    match &expr {
        Expression::Retrieve(r) => {
            match &r.data_type.inner {
                octofhir_cql_ast::TypeSpecifier::Named(named) => {
                    assert_eq!(named.name, "Observation");
                }
                _ => panic!("Expected Named type specifier"),
            }
        }
        _ => panic!("Expected Retrieve"),
    }
}

// === Property Access ===

#[test]
fn test_property_access() {
    let expr = parse_expr("Patient.name");
    match &expr {
        Expression::Property(prop) => {
            assert_eq!(prop.property.name, "name");
        }
        _ => panic!("Expected Property, got: {:?}", expr),
    }
}

#[test]
fn test_chained_property_access() {
    let expr = parse_expr("Patient.name.family");
    match &expr {
        Expression::Property(prop) => {
            assert_eq!(prop.property.name, "family");
            // Source should be another property access
            match &prop.source.inner {
                Expression::Property(inner) => {
                    assert_eq!(inner.property.name, "name");
                }
                _ => panic!("Expected nested Property"),
            }
        }
        _ => panic!("Expected Property"),
    }
}

// === If-Then-Else ===

#[test]
fn test_if_then_else() {
    let expr = parse_expr("if true then 1 else 2");
    match &expr {
        Expression::If(if_expr) => {
            // Condition should be true
            assert!(matches!(&if_expr.condition.inner, Expression::Literal(octofhir_cql_ast::Literal::Boolean(true))));
            // Then should be 1
            assert!(matches!(&if_expr.then_expr.inner, Expression::Literal(octofhir_cql_ast::Literal::Integer(1))));
            // Else should be 2
            assert!(matches!(&if_expr.else_expr.inner, Expression::Literal(octofhir_cql_ast::Literal::Integer(2))));
        }
        _ => panic!("Expected If, got: {:?}", expr),
    }
}

#[test]
fn test_nested_if() {
    let expr = parse_expr("if a then if b then 1 else 2 else 3");
    match &expr {
        Expression::If(if_expr) => {
            // Then branch should be another if
            assert!(matches!(&if_expr.then_expr.inner, Expression::If(_)));
        }
        _ => panic!("Expected If"),
    }
}

// === Lists ===

#[test]
fn test_empty_list() {
    let expr = parse_expr("{}");
    match &expr {
        Expression::List(list) => {
            assert_eq!(list.elements.len(), 0);
        }
        _ => panic!("Expected List, got: {:?}", expr),
    }
}

#[test]
fn test_simple_list() {
    let expr = parse_expr("{1, 2, 3}");
    match &expr {
        Expression::List(list) => {
            assert_eq!(list.elements.len(), 3);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_mixed_list() {
    let expr = parse_expr("{1, 'hello', true}");
    match &expr {
        Expression::List(list) => {
            assert_eq!(list.elements.len(), 3);
        }
        _ => panic!("Expected List"),
    }
}

// === Function Calls ===

#[test]
fn test_function_call_no_args() {
    let expr = parse_expr("AgeInYears()");
    match &expr {
        Expression::FunctionRef(func) => {
            assert_eq!(func.name.name, "AgeInYears");
            assert_eq!(func.arguments.len(), 0);
        }
        _ => panic!("Expected FunctionRef, got: {:?}", expr),
    }
}

#[test]
fn test_function_call_with_args() {
    let expr = parse_expr("AgeInYears(birthDate)");
    match &expr {
        Expression::FunctionRef(func) => {
            assert_eq!(func.name.name, "AgeInYears");
            assert_eq!(func.arguments.len(), 1);
        }
        _ => panic!("Expected FunctionRef"),
    }
}

#[test]
fn test_function_call_multiple_args() {
    let expr = parse_expr("Coalesce(a, b, c)");
    match &expr {
        Expression::FunctionRef(func) => {
            assert_eq!(func.name.name, "Coalesce");
            assert_eq!(func.arguments.len(), 3);
        }
        _ => panic!("Expected FunctionRef"),
    }
}

// Note: In CQL, exists, flatten, distinct, etc. are operators, not functions
// So `Exists({1, 2})` is parsed as the exists operator applied to the list

#[test]
fn test_exists_operator() {
    // Exists is a unary operator, not a function
    let expr = parse_expr("exists {1, 2}");
    match &expr {
        Expression::UnaryOp(op) => {
            assert!(matches!(op.op, octofhir_cql_ast::UnaryOp::Exists));
            assert!(matches!(&op.operand.inner, Expression::List(_)));
        }
        _ => panic!("Expected UnaryOp, got: {:?}", expr),
    }
}

#[test]
fn test_exists_operator_with_parens() {
    // Exists({1, 2}) is the same as exists {1, 2}
    let expr = parse_expr("Exists({1, 2})");
    match &expr {
        Expression::UnaryOp(op) => {
            assert!(matches!(op.op, octofhir_cql_ast::UnaryOp::Exists));
            assert!(matches!(&op.operand.inner, Expression::List(_)));
        }
        _ => panic!("Expected UnaryOp, got: {:?}", expr),
    }
}

#[test]
fn test_flatten_operator() {
    // Flatten is also a unary operator
    let expr = parse_expr("Flatten({{1,2}, {3,4}})");
    match &expr {
        Expression::UnaryOp(op) => {
            assert!(matches!(op.op, octofhir_cql_ast::UnaryOp::Flatten));
        }
        _ => panic!("Expected UnaryOp, got: {:?}", expr),
    }
}

#[test]
fn test_custom_function_with_list_literal() {
    // Custom functions (not keywords) with list arguments should work
    let expr = parse_expr("MyFunc({1, 2})");
    match &expr {
        Expression::FunctionRef(func) => {
            assert_eq!(func.name.name, "MyFunc");
            assert_eq!(func.arguments.len(), 1);
            assert!(matches!(&func.arguments[0].inner, Expression::List(_)));
        }
        _ => panic!("Expected FunctionRef, got: {:?}", expr),
    }
}

// === Intervals ===

#[test]
fn test_interval_closed() {
    let expr = parse_expr("Interval[1, 10]");
    match &expr {
        Expression::Interval(interval) => {
            assert!(interval.low_closed);
            assert!(interval.high_closed);
        }
        _ => panic!("Expected Interval, got: {:?}", expr),
    }
}

#[test]
fn test_interval_open() {
    let expr = parse_expr("Interval(1, 10)");
    match &expr {
        Expression::Interval(interval) => {
            assert!(!interval.low_closed);
            assert!(!interval.high_closed);
        }
        _ => panic!("Expected Interval"),
    }
}

#[test]
fn test_interval_half_open() {
    let expr = parse_expr("Interval[1, 10)");
    match &expr {
        Expression::Interval(interval) => {
            assert!(interval.low_closed);
            assert!(!interval.high_closed);
        }
        _ => panic!("Expected Interval"),
    }
}

// === Complex Expressions ===

#[rstest]
#[case("[Patient]", true)]
#[case("Patient.name", true)]
#[case("if true then 1 else 2", true)]
#[case("{1, 2, 3}", true)]
#[case("AgeInYears()", true)]
#[case("Interval[1, 10]", true)]
fn test_various_expressions(#[case] input: &str, #[case] should_parse: bool) {
    let result = parse_expression(input);
    assert_eq!(result.is_ok(), should_parse, "Parse result for '{}' unexpected", input);
}

// === Query Expressions ===

#[test]
fn test_query_with_alias() {
    let expr = parse_expr("({1, 2, 3}) l");
    match &expr {
        Expression::Query(query) => {
            assert_eq!(query.sources.len(), 1);
            assert_eq!(query.sources[0].alias.name, "l");
        }
        _ => panic!("Expected Query, got: {:?}", expr),
    }
}

#[test]
fn test_query_sort_ascending() {
    let expr = parse_expr("({1, 3, 2}) l sort ascending");
    match &expr {
        Expression::Query(query) => {
            assert!(query.sort_clause.is_some());
        }
        _ => panic!("Expected Query, got: {:?}", expr),
    }
}

#[test]
fn test_query_sort_desc() {
    let expr = parse_expr("({1, 3, 2}) l sort desc");
    match &expr {
        Expression::Query(query) => {
            assert!(query.sort_clause.is_some());
        }
        _ => panic!("Expected Query, got: {:?}", expr),
    }
}

#[test]
fn test_query_from_multi_source() {
    let expr = parse_expr("from ({2, 3}) A, ({5, 6}) B");
    match &expr {
        Expression::Query(query) => {
            assert_eq!(query.sources.len(), 2);
            assert_eq!(query.sources[0].alias.name, "A");
            assert_eq!(query.sources[1].alias.name, "B");
        }
        _ => panic!("Expected Query, got: {:?}", expr),
    }
}

#[test]
fn test_query_aggregate() {
    let expr = parse_expr("({1, 2, 3, 3, 4}) L aggregate A starting 1: A * L");
    match &expr {
        Expression::Query(query) => {
            assert!(query.aggregate_clause.is_some());
        }
        _ => panic!("Expected Query, got: {:?}", expr),
    }
}
