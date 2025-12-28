//! Library structure parser using winnow

use crate::combinators::{
    identifier_parser, keyword, lit, padded_keyword, preprocess, qualified_identifier_parser,
    version_specifier_parser, ws, Input, PResult,
};
use crate::expression::expression_parser;
use octofhir_cql_ast::{
    AccessModifier, ContextDefinition, ExpressionDefinition, Library, LibraryDefinition,
    ParameterDefinition, Spanned, Statement, UsingDefinition,
};
use octofhir_cql_diagnostics::{CqlError, Result, Span, CQL0001};
use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ContextError;
use winnow::prelude::*;

/// Helper to create dummy span (0, 0) - placeholder until proper span tracking is implemented
fn dummy_span<T>(inner: T) -> Spanned<T> {
    Spanned::new(inner, Span::new(0, 0))
}

/// Parse CQL source into a Library AST
pub fn parse(source: &str) -> Result<Library> {
    let cleaned = preprocess(source);
    let mut input: &str = &cleaned;

    library_parser(&mut input)
        .map_err(|e| CqlError::parse(CQL0001, format!("Parse error: {:?}", e), source))
}

/// Parse a single CQL expression
pub fn parse_expression(source: &str) -> Result<Spanned<octofhir_cql_ast::Expression>> {
    let cleaned = preprocess(source);
    let mut input: &str = &cleaned;

    let expr = expression_parser(&mut input)
        .map_err(|e| CqlError::parse(CQL0001, format!("Parse error: {:?}", e), source))?;
    ws(&mut input).ok();
    eof::<_, ContextError>.parse_next(&mut input)
        .map_err(|e| CqlError::parse(CQL0001, format!("Parse error: {:?}", e), source))?;
    Ok(expr)
}

/// Parse CQL source with specified mode
///
/// In Fast mode: fails on first error (default behavior)
/// In Analysis mode: collects all errors and returns partial AST if possible
pub fn parse_with_mode(source: &str, mode: crate::ParseMode) -> crate::ParseResult {
    let cleaned = preprocess(source);
    let mut input: &str = &cleaned;

    match mode {
        crate::ParseMode::Fast => {
            // Fast mode: same as parse(), but returns ParseResult
            match library_parser(&mut input) {
                Ok(library) => crate::ParseResult::success(library),
                Err(e) => {
                    let error = CqlError::parse(CQL0001, format!("Parse error: {:?}", e), source);
                    crate::ParseResult::error(vec![error])
                }
            }
        }
        crate::ParseMode::Analysis => {
            // Analysis mode: try to collect errors and return partial AST
            // For now, fall back to fast mode behavior
            // TODO: Implement proper error recovery when winnow's unstable-recover is stable
            match library_parser(&mut input) {
                Ok(library) => crate::ParseResult::success(library),
                Err(e) => {
                    let error = CqlError::parse(CQL0001, format!("Parse error: {:?}", e), source);
                    crate::ParseResult::error(vec![error])
                }
            }
        }
    }
}

/// Library parser
fn library_parser<'a>(input: &mut Input<'a>) -> PResult<Library> {
    ws.parse_next(input)?;
    let lib_def = opt(library_definition).parse_next(input)?;
    let usings: Vec<Spanned<UsingDefinition>> = repeat(0.., using_definition).parse_next(input)?;
    let contexts: Vec<Spanned<ContextDefinition>> = repeat(0.., context_definition).parse_next(input)?;
    let params: Vec<Spanned<ParameterDefinition>> = repeat(0.., parameter_definition).parse_next(input)?;
    let exprs: Vec<Spanned<ExpressionDefinition>> = repeat(0.., expression_definition).parse_next(input)?;
    ws.parse_next(input)?;
    eof.parse_next(input)?;

    let mut library = Library::new();
    library.definition = lib_def;
    library.usings = usings;
    library.contexts = contexts;
    library.parameters = params;
    library.statements = exprs
        .into_iter()
        .map(|e| Spanned::new(Statement::ExpressionDef(e.inner), e.span))
        .collect();

    Ok(library)
}

/// Parse library definition
fn library_definition<'a>(input: &mut Input<'a>) -> PResult<LibraryDefinition> {
    padded_keyword("library").parse_next(input)?;
    let name = qualified_identifier_parser(input)?;
    ws.parse_next(input)?;
    let version = opt(|input: &mut Input<'a>| {
        padded_keyword("version").parse_next(input)?;
        version_specifier_parser(input)
    })
    .parse_next(input)?;

    Ok(LibraryDefinition { name, version })
}

/// Parse using definition
fn using_definition<'a>(input: &mut Input<'a>) -> PResult<Spanned<UsingDefinition>> {
    padded_keyword("using").parse_next(input)?;
    let model = identifier_parser(input)?;
    ws.parse_next(input)?;
    let version = opt(|input: &mut Input<'a>| {
        padded_keyword("version").parse_next(input)?;
        version_specifier_parser(input)
    })
    .parse_next(input)?;

    Ok(dummy_span(UsingDefinition { model, version }))
}

/// Parse context definition
fn context_definition<'a>(input: &mut Input<'a>) -> PResult<Spanned<ContextDefinition>> {
    padded_keyword("context").parse_next(input)?;
    let context = identifier_parser(input)?;

    Ok(dummy_span(ContextDefinition { context }))
}

/// Parse access modifier
fn access_modifier<'a>(input: &mut Input<'a>) -> PResult<AccessModifier> {
    ws.parse_next(input)?;
    alt((
        keyword("public").value(AccessModifier::Public),
        keyword("private").value(AccessModifier::Private),
    ))
    .parse_next(input)
}

/// Parse parameter definition
fn parameter_definition<'a>(input: &mut Input<'a>) -> PResult<Spanned<ParameterDefinition>> {
    let access = opt(access_modifier)
        .map(|a: Option<AccessModifier>| a.unwrap_or(AccessModifier::Public))
        .parse_next(input)?;

    padded_keyword("parameter").parse_next(input)?;
    let name = identifier_parser(input)?;
    ws.parse_next(input)?;

    let default = opt(|input: &mut Input<'a>| {
        padded_keyword("default").parse_next(input)?;
        expression_parser(input)
    })
    .parse_next(input)?;

    Ok(dummy_span(ParameterDefinition {
        access,
        name,
        type_specifier: None,
        default: default.map(Box::new),
    }))
}

/// Parse expression definition
fn expression_definition<'a>(input: &mut Input<'a>) -> PResult<Spanned<ExpressionDefinition>> {
    let access = opt(access_modifier)
        .map(|a: Option<AccessModifier>| a.unwrap_or(AccessModifier::Public))
        .parse_next(input)?;

    padded_keyword("define").parse_next(input)?;
    let name = identifier_parser(input)?;
    ws.parse_next(input)?;
    lit(":").parse_next(input)?;
    ws.parse_next(input)?;
    let expr = expression_parser(input)?;

    Ok(dummy_span(ExpressionDefinition {
        access,
        name,
        expression: Box::new(expr),
    }))
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
    fn test_parse_date_lit() {
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
    fn test_parse_datetime_lit() {
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
    fn test_parse_time_lit() {
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
    fn test_parse_quantity_lit() {
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
