//! Library structure parser

use chumsky::prelude::*;

use crate::combinators::{
    identifier_parser, preprocess, qualified_identifier_parser, version_specifier_parser,
};
use crate::expression::expression_parser;
use octofhir_cql_ast::{
    AccessModifier, ContextDefinition, ExpressionDefinition, Library, LibraryDefinition,
    ParameterDefinition, Spanned, Statement, UsingDefinition,
};
use octofhir_cql_diagnostics::{CqlError, Result, Span, CQL0001};

/// Parse CQL source into a Library AST
pub fn parse(source: &str) -> Result<Library> {
    let cleaned = preprocess(source);
    let parser = library_parser();

    parser
        .parse(&cleaned)
        .into_result()
        .map_err(|errs| {
            let errors: Vec<CqlError> = errs
                .into_iter()
                .map(|e| CqlError::parse(CQL0001, format!("Parse error: {}", e), source))
                .collect();
            if errors.len() == 1 {
                errors.into_iter().next().unwrap()
            } else {
                CqlError::Multiple(errors)
            }
        })
}

/// Parse a single CQL expression
pub fn parse_expression(source: &str) -> Result<Spanned<octofhir_cql_ast::Expression>> {
    let cleaned = preprocess(source);
    let parser = expression_parser().padded().then_ignore(end());

    parser
        .parse(&cleaned)
        .into_result()
        .map_err(|errs| {
            let errors: Vec<CqlError> = errs
                .into_iter()
                .map(|e| CqlError::parse(CQL0001, format!("Parse error: {}", e), source))
                .collect();
            if errors.len() == 1 {
                errors.into_iter().next().unwrap()
            } else {
                CqlError::Multiple(errors)
            }
        })
}

/// Parse CQL source with specified mode
///
/// In Fast mode: fails on first error (default behavior)
/// In Analysis mode: collects all errors and returns partial AST if possible
pub fn parse_with_mode(source: &str, mode: crate::ParseMode) -> crate::ParseResult {
    let cleaned = preprocess(source);
    let parser = library_parser();

    match mode {
        crate::ParseMode::Fast => {
            // Fast mode: same as parse(), but returns ParseResult
            match parser.parse(&cleaned).into_result() {
                Ok(library) => crate::ParseResult::success(library),
                Err(errs) => {
                    let errors: Vec<CqlError> = errs
                        .into_iter()
                        .map(|e| CqlError::parse(CQL0001, format!("Parse error: {}", e), source))
                        .collect();
                    crate::ParseResult::error(errors)
                }
            }
        }
        crate::ParseMode::Analysis => {
            // Analysis mode: collect all errors
            let result = parser.parse(&cleaned);

            // Collect all errors from the parse result
            let errors: Vec<CqlError> = result
                .errors()
                .map(|e| CqlError::parse(CQL0001, format!("Parse error: {}", e), source))
                .collect();

            // Try to get a partial or complete AST
            match result.into_output() {
                Some(library) => crate::ParseResult {
                    library: Some(library),
                    errors,
                },
                None => crate::ParseResult {
                    library: None,
                    errors,
                },
            }
        }
    }
}

/// Library parser
fn library_parser<'a>() -> impl Parser<'a, &'a str, Library, extra::Err<Rich<'a, char>>> {
    let lib_def = library_definition().or_not();
    let using_defs = using_definition().repeated().collect::<Vec<_>>();
    let context_defs = context_definition().repeated().collect::<Vec<_>>();
    let param_defs = parameter_definition().repeated().collect::<Vec<_>>();
    let expr_defs = expression_definition().repeated().collect::<Vec<_>>();

    lib_def
        .then(using_defs)
        .then(context_defs)
        .then(param_defs)
        .then(expr_defs)
        .padded()
        .then_ignore(end())
        .map(|((((lib, usings), contexts), params), exprs)| {
            let mut library = Library::new();
            library.definition = lib;
            library.usings = usings;
            library.contexts = contexts;
            library.parameters = params;
            library.statements = exprs
                .into_iter()
                .map(|e| Spanned::new(Statement::ExpressionDef(e.inner), e.span))
                .collect();
            library
        })
}

/// Parse library definition
fn library_definition<'a>(
) -> impl Parser<'a, &'a str, LibraryDefinition, extra::Err<Rich<'a, char>>> + Clone {
    text::keyword("library")
        .padded()
        .ignore_then(qualified_identifier_parser())
        .then(
            text::keyword("version")
                .padded()
                .ignore_then(version_specifier_parser())
                .or_not(),
        )
        .map(|(name, version)| LibraryDefinition { name, version })
}

/// Parse using definition
fn using_definition<'a>(
) -> impl Parser<'a, &'a str, Spanned<UsingDefinition>, extra::Err<Rich<'a, char>>> + Clone {
    text::keyword("using")
        .padded()
        .ignore_then(identifier_parser())
        .then(
            text::keyword("version")
                .padded()
                .ignore_then(version_specifier_parser())
                .or_not(),
        )
        .map_with(|(model, version), e| {
            Spanned::new(
                UsingDefinition { model, version },
                Span::from(e.span().start..e.span().end),
            )
        })
}

/// Parse context definition
fn context_definition<'a>(
) -> impl Parser<'a, &'a str, Spanned<ContextDefinition>, extra::Err<Rich<'a, char>>> + Clone {
    text::keyword("context")
        .padded()
        .ignore_then(identifier_parser())
        .map_with(|ctx, e| {
            Spanned::new(
                ContextDefinition { context: ctx },
                Span::from(e.span().start..e.span().end),
            )
        })
}

/// Parse parameter definition
fn parameter_definition<'a>(
) -> impl Parser<'a, &'a str, Spanned<ParameterDefinition>, extra::Err<Rich<'a, char>>> + Clone {
    let access = choice((
        text::keyword("public").to(AccessModifier::Public),
        text::keyword("private").to(AccessModifier::Private),
    ))
    .padded()
    .or_not()
    .map(|a| a.unwrap_or(AccessModifier::Public));

    access
        .then_ignore(text::keyword("parameter").padded())
        .then(identifier_parser())
        .then(
            text::keyword("default")
                .padded()
                .ignore_then(expression_parser())
                .or_not(),
        )
        .map_with(|((access, name), default), e| {
            Spanned::new(
                ParameterDefinition {
                    access,
                    name,
                    type_specifier: None,
                    default: default.map(Box::new),
                },
                Span::from(e.span().start..e.span().end),
            )
        })
}

/// Parse expression definition
fn expression_definition<'a>(
) -> impl Parser<'a, &'a str, Spanned<ExpressionDefinition>, extra::Err<Rich<'a, char>>> + Clone {
    let access = choice((
        text::keyword("public").to(AccessModifier::Public),
        text::keyword("private").to(AccessModifier::Private),
    ))
    .padded()
    .or_not()
    .map(|a| a.unwrap_or(AccessModifier::Public));

    access
        .then_ignore(text::keyword("define").padded())
        .then(identifier_parser())
        .then_ignore(just(':').padded())
        .then(expression_parser())
        .map_with(|((access, name), expr), e| {
            Spanned::new(
                ExpressionDefinition {
                    access,
                    name,
                    expression: Box::new(expr),
                },
                Span::from(e.span().start..e.span().end),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_library() {
        let source = r#"
            library Test version '1.0.0'
            using FHIR version '4.0.1'
            context Patient
            define IsAdult: true
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let library = result.unwrap();
        assert!(library.definition.is_some());
        assert_eq!(library.definition.as_ref().unwrap().name.name.name, "Test");
        assert_eq!(library.usings.len(), 1);
        assert_eq!(library.contexts.len(), 1);
        assert_eq!(library.statements.len(), 1);
    }

    #[test]
    fn test_parse_expression() {
        let source = "1 + 2 * 3";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    }

    #[test]
    fn test_parse_boolean_expression() {
        let source = "true and false or not true";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    }

    #[test]
    fn test_parse_comparison() {
        let source = "age >= 18";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    }

    #[test]
    fn test_parse_if_expression() {
        let source = "if true then 1 else 2";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    }

    #[test]
    fn test_parse_date_literal() {
        let source = "@2024-01-15";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::Literal(octofhir_cql_ast::Literal::Date(date)) =
            &result.unwrap().inner
        {
            assert_eq!(date.year, 2024);
            assert_eq!(date.month, Some(1));
            assert_eq!(date.day, Some(15));
        } else {
            panic!("Expected Date literal");
        }
    }

    #[test]
    fn test_parse_datetime_literal() {
        let source = "@2024-01-15T10:30:00";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::Literal(octofhir_cql_ast::Literal::DateTime(dt)) =
            &result.unwrap().inner
        {
            assert_eq!(dt.date.year, 2024);
            assert_eq!(dt.hour, Some(10));
            assert_eq!(dt.minute, Some(30));
            assert_eq!(dt.second, Some(0));
        } else {
            panic!("Expected DateTime literal");
        }
    }

    #[test]
    fn test_parse_time_literal() {
        let source = "@T14:30:00";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::Literal(octofhir_cql_ast::Literal::Time(time)) =
            &result.unwrap().inner
        {
            assert_eq!(time.hour, 14);
            assert_eq!(time.minute, Some(30));
            assert_eq!(time.second, Some(0));
        } else {
            panic!("Expected Time literal");
        }
    }

    #[test]
    fn test_parse_quantity_literal() {
        let source = "5 'mg'";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::Literal(octofhir_cql_ast::Literal::Quantity(q)) =
            &result.unwrap().inner
        {
            assert_eq!(q.value.to_string(), "5");
            assert_eq!(q.unit, Some("mg".to_string()));
        } else {
            panic!("Expected Quantity literal");
        }
    }

    #[test]
    fn test_parse_retrieve_expression() {
        let source = "[Patient]";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::Retrieve(r) = &result.unwrap().inner {
            if let octofhir_cql_ast::TypeSpecifier::Named(named) = &r.data_type.inner {
                assert_eq!(named.name, "Patient");
            } else {
                panic!("Expected Named type specifier");
            }
        } else {
            panic!("Expected Retrieve expression");
        }
    }

    #[test]
    fn test_parse_list_expression() {
        let source = "{ 1, 2, 3 }";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::List(list) = &result.unwrap().inner {
            assert_eq!(list.elements.len(), 3);
        } else {
            panic!("Expected List expression");
        }
    }

    #[test]
    fn test_parse_property_access() {
        let source = "Patient.name";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::Property(prop) = &result.unwrap().inner {
            assert_eq!(prop.property.name, "name");
        } else {
            panic!("Expected Property access");
        }
    }

    #[test]
    fn test_parse_function_call() {
        let source = "AgeInYears()";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::FunctionRef(func) = &result.unwrap().inner {
            assert_eq!(func.name.name, "AgeInYears");
            assert_eq!(func.arguments.len(), 0);
        } else {
            panic!("Expected FunctionRef");
        }
    }

    #[test]
    fn test_parse_function_call_with_args() {
        let source = "Max(1, 2, 3)";
        let result = parse_expression(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        if let octofhir_cql_ast::Expression::FunctionRef(func) = &result.unwrap().inner {
            assert_eq!(func.name.name, "Max");
            assert_eq!(func.arguments.len(), 3);
        } else {
            panic!("Expected FunctionRef");
        }
    }

    #[test]
    fn test_parse_with_mode_fast_success() {
        let source = r#"
            library Test version '1.0.0'
            define IsAdult: true
        "#;

        let result = parse_with_mode(source, crate::ParseMode::Fast);
        assert!(result.is_success());
        assert!(result.library.is_some());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_parse_with_mode_fast_error() {
        let source = r#"
            library Test version '1.0.0'
            define IsAdult: @@@invalid
        "#;

        let result = parse_with_mode(source, crate::ParseMode::Fast);
        assert!(!result.is_success());
        assert!(result.library.is_none());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_parse_with_mode_analysis_success() {
        let source = r#"
            library Test version '1.0.0'
            define IsAdult: true
        "#;

        let result = parse_with_mode(source, crate::ParseMode::Analysis);
        assert!(result.is_success());
        assert!(result.library.is_some());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_parse_with_mode_analysis_collects_errors() {
        let source = r#"
            library Test version '1.0.0'
            define IsAdult: @@@invalid
        "#;

        let result = parse_with_mode(source, crate::ParseMode::Analysis);
        // Analysis mode should have errors
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_error_recovery_with_malformed_parens() {
        // Test recovery from malformed parenthesized expression
        let source = "(invalid +++)";
        let result = parse_expression(source);
        // Should fail to parse
        assert!(result.is_err());
    }

    #[test]
    fn test_error_recovery_in_list() {
        // Test recovery from malformed list
        let source = "{ 1, 2, 3 }";
        let result = parse_expression(source);
        // Valid list should parse
        assert!(result.is_ok());

        if let octofhir_cql_ast::Expression::List(list) = &result.unwrap().inner {
            assert_eq!(list.elements.len(), 3);
        } else {
            panic!("Expected List expression");
        }
    }
}
