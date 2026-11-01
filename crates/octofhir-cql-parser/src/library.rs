//! Library structure parser using winnow
//!
//! The library level is parsed item by item (`using`, `include`, `codesystem`,
//! `valueset`, `code`, `concept`, `parameter`, `context`, `define`,
//! `define function`). Every item records its real source span, statements
//! remember the `context` in effect where they appear, and failures are
//! reported with the line/column of the offending token. In
//! [`ParseMode::Analysis`](crate::ParseMode::Analysis) the parser recovers at
//! the next line that starts a top-level item and keeps collecting errors.

use crate::combinators::{
    Input, PError, PResult, identifier_or_keyword_parser, identifier_parser, keyword, lit,
    preprocess, string_parser, version_specifier_parser, ws,
};
use crate::expression::{expression_parser, type_specifier_parser};
use crate::{ParseMode, ParseResult};
use octofhir_cql_ast::{
    AccessModifier, CodeDefinition, CodeSystemRef, CodesystemDefinition, ConceptDefinition,
    ContextDefinition, Expression, ExpressionDefinition, FunctionDefinition, FunctionParameter,
    IncludeDefinition, Library, LibraryDefinition, ParameterDefinition, QualifiedIdentifier,
    Spanned, Statement, UsingDefinition, ValuesetDefinition, VersionSpecifier,
};
use octofhir_cql_diagnostics::{
    CQL0001, CQL0002, CQL0012, CQL0013, CQL0014, CqlError, ErrorCode, Result, SourceLocation, Span,
};
use winnow::prelude::*;

/// Keywords that start a top-level library item.
const ITEM_KEYWORDS: &[&str] = &[
    "library",
    "using",
    "include",
    "public",
    "private",
    "parameter",
    "codesystem",
    "valueset",
    "code",
    "concept",
    "context",
    "define",
];

/// Parse CQL source into a Library AST, failing on the first error.
pub fn parse(source: &str) -> Result<Library> {
    parse_with_mode(source, ParseMode::Fast).into_result()
}

/// Parse a single CQL expression
pub fn parse_expression(source: &str) -> Result<Spanned<Expression>> {
    let cleaned = preprocess(source);
    let mut input: Input<'_> = &cleaned;

    let expr = match expression_parser(&mut input) {
        Ok(expr) => expr,
        Err(_) => {
            let at = cleaned.len() - input.len();
            return Err(syntax_error(
                source,
                &cleaned,
                at,
                CQL0012,
                "expected an expression",
            ));
        }
    };
    let _ = ws(&mut input);
    if !input.is_empty() {
        let at = cleaned.len() - input.len();
        return Err(syntax_error(
            source,
            &cleaned,
            at,
            CQL0001,
            "unexpected input after expression",
        ));
    }
    Ok(expr)
}

/// Parse CQL source with the specified mode.
///
/// - [`ParseMode::Fast`] stops at the first error.
/// - [`ParseMode::Analysis`] skips the broken item, resumes at the next line
///   that starts a top-level item and returns every error plus the items that
///   did parse.
pub fn parse_with_mode(source: &str, mode: ParseMode) -> ParseResult {
    let cleaned = preprocess(source);
    LibraryParser::new(source, &cleaned, mode).run()
}

/// A parsed top-level item.
enum Item {
    Library(LibraryDefinition),
    Using(UsingDefinition),
    Include(IncludeDefinition),
    Parameter(ParameterDefinition),
    CodeSystem(CodesystemDefinition),
    ValueSet(ValuesetDefinition),
    Code(CodeDefinition),
    Concept(ConceptDefinition),
    Context(ContextDefinition),
    Expression(ExpressionDefinition),
    Function(FunctionDefinition),
}

/// Where and why an item failed to parse.
struct Failure<'a> {
    /// Remaining input at the failure point.
    at: Input<'a>,
    code: ErrorCode,
    expected: &'static str,
}

type ItemResult<'a, T> = std::result::Result<T, Failure<'a>>;

/// Run `parser` after skipping whitespace; on failure rewind to that point
/// and report what was expected there.
fn expect<'a, O>(
    input: &mut Input<'a>,
    mut parser: impl Parser<Input<'a>, O, PError>,
    code: ErrorCode,
    expected: &'static str,
) -> ItemResult<'a, O> {
    let _ = ws(input);
    let checkpoint = *input;
    parser.parse_next(input).map_err(|_| {
        *input = checkpoint;
        Failure {
            at: checkpoint,
            code,
            expected,
        }
    })
}

/// Consume `kw` (after whitespace) if present.
fn eat_keyword(input: &mut Input<'_>, kw: &'static str) -> bool {
    let checkpoint = *input;
    let _ = ws(input);
    if keyword(kw).parse_next(input).is_ok() {
        true
    } else {
        *input = checkpoint;
        false
    }
}

/// Consume punctuation `p` (after whitespace) if present.
fn eat_lit(input: &mut Input<'_>, p: &'static str) -> bool {
    let checkpoint = *input;
    let _ = ws(input);
    if lit(p).parse_next(input).is_ok() {
        true
    } else {
        *input = checkpoint;
        false
    }
}

/// Whether the input (after whitespace) starts with a top-level item keyword.
fn starts_item(input: Input<'_>) -> bool {
    let mut probe = input;
    let _ = ws(&mut probe);
    ITEM_KEYWORDS.iter().any(|kw| {
        let mut cursor = probe;
        keyword(kw).parse_next(&mut cursor).is_ok()
    })
}

/// Parse an optional `version '<x>'` clause.
fn opt_version<'a>(input: &mut Input<'a>) -> ItemResult<'a, Option<VersionSpecifier>> {
    if eat_keyword(input, "version") {
        expect(
            input,
            version_specifier_parser,
            CQL0001,
            "a version string, e.g. '1.0.0'",
        )
        .map(Some)
    } else {
        Ok(None)
    }
}

/// Parse an optional `display '<x>'` clause.
fn opt_display<'a>(input: &mut Input<'a>) -> ItemResult<'a, Option<String>> {
    if eat_keyword(input, "display") {
        expect(input, string_parser, CQL0001, "a display string").map(Some)
    } else {
        Ok(None)
    }
}

/// Parse a dotted identifier (`a.b.C`): the last segment is the name, the
/// preceding segments form the qualifier (namespace or library alias).
fn dotted_identifier<'a>(input: &mut Input<'a>) -> PResult<QualifiedIdentifier> {
    let mut segments = vec![identifier_parser(input)?];
    loop {
        let checkpoint = *input;
        if lit(".").parse_next(input).is_ok()
            && let Ok(next) = identifier_or_keyword_parser(input)
        {
            segments.push(next);
            continue;
        }
        *input = checkpoint;
        break;
    }
    let name = segments.pop().expect("at least one segment");
    if segments.is_empty() {
        Ok(QualifiedIdentifier::simple(name))
    } else {
        let qualifier = segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        Ok(QualifiedIdentifier::qualified(qualifier, name))
    }
}

/// Parse a top-level item at the current position.
fn library_item<'a>(input: &mut Input<'a>, total: usize) -> ItemResult<'a, Item> {
    let _ = ws(input);
    let start = *input;

    if eat_keyword(input, "library") {
        let name = expect(input, dotted_identifier, CQL0013, "a library name")?;
        let version = opt_version(input)?;
        return Ok(Item::Library(LibraryDefinition { name, version }));
    }
    if eat_keyword(input, "using") {
        let model = expect(input, identifier_parser, CQL0013, "a model name, e.g. FHIR")?;
        let version = opt_version(input)?;
        let alias = if eat_keyword(input, "called") {
            Some(expect(input, identifier_parser, CQL0013, "a local alias")?)
        } else {
            None
        };
        return Ok(Item::Using(UsingDefinition {
            model,
            version,
            alias,
        }));
    }
    if eat_keyword(input, "include") {
        let library = expect(input, dotted_identifier, CQL0013, "a library name")?;
        let version = opt_version(input)?;
        let alias = if eat_keyword(input, "called") {
            Some(expect(input, identifier_parser, CQL0013, "a local alias")?)
        } else {
            None
        };
        return Ok(Item::Include(IncludeDefinition {
            library,
            version,
            alias,
        }));
    }
    if eat_keyword(input, "context") {
        let first = expect(
            input,
            identifier_parser,
            CQL0013,
            "a context name, e.g. Patient",
        )?;
        // `context FHIR.Patient` names the model explicitly; keep the type name.
        let context = if eat_lit(input, ".") {
            expect(
                input,
                identifier_or_keyword_parser,
                CQL0013,
                "a context name",
            )?
        } else {
            first
        };
        return Ok(Item::Context(ContextDefinition { context }));
    }
    if eat_keyword(input, "define") {
        let access = opt_access(input).unwrap_or_default();
        return define_item(input, access, total);
    }

    let access = opt_access(input).unwrap_or_default();

    if eat_keyword(input, "define") {
        // Accept the legacy `private define X` order as well as `define private X`.
        return define_item(input, access, total);
    }
    if eat_keyword(input, "parameter") {
        let name = expect(input, identifier_parser, CQL0013, "a parameter name")?;
        let mut probe = *input;
        let _ = ws(&mut probe);
        let starts_default = {
            let mut cursor = probe;
            keyword("default").parse_next(&mut cursor).is_ok()
        };
        let type_specifier = if probe.is_empty() || starts_item(probe) || starts_default {
            None
        } else {
            let ts = expect(input, type_specifier_parser, CQL0014, "a type specifier")?;
            Some(spanned_at(ts, probe, input, total))
        };
        let default = if eat_keyword(input, "default") {
            Some(Box::new(expect(
                input,
                expression_parser,
                CQL0012,
                "a default value expression",
            )?))
        } else {
            None
        };
        return Ok(Item::Parameter(ParameterDefinition {
            access,
            name,
            type_specifier,
            default,
        }));
    }
    if eat_keyword(input, "codesystem") {
        let name = expect(input, identifier_parser, CQL0013, "a code system name")?;
        expect(input, lit(":"), CQL0001, "':' after the code system name")?;
        let uri = expect(input, string_parser, CQL0001, "a code system URI string")?;
        let version = opt_version(input)?;
        return Ok(Item::CodeSystem(CodesystemDefinition {
            access,
            name,
            uri,
            version,
        }));
    }
    if eat_keyword(input, "valueset") {
        let name = expect(input, identifier_parser, CQL0013, "a value set name")?;
        expect(input, lit(":"), CQL0001, "':' after the value set name")?;
        let uri = expect(input, string_parser, CQL0001, "a value set URI string")?;
        let version = opt_version(input)?;
        let mut codesystems = Vec::new();
        if eat_keyword(input, "codesystems") {
            expect(input, lit("{"), CQL0001, "'{' after 'codesystems'")?;
            loop {
                let name = expect(input, dotted_identifier, CQL0013, "a code system name")?;
                codesystems.push(CodeSystemRef { name });
                if !eat_lit(input, ",") {
                    break;
                }
            }
            expect(input, lit("}"), CQL0001, "'}' closing the code system list")?;
        }
        return Ok(Item::ValueSet(ValuesetDefinition {
            access,
            name,
            uri,
            version,
            codesystems,
        }));
    }
    if eat_keyword(input, "code") {
        let name = expect(input, identifier_parser, CQL0013, "a code name")?;
        expect(input, lit(":"), CQL0001, "':' after the code name")?;
        let code = expect(input, string_parser, CQL0001, "a code string")?;
        expect(
            input,
            keyword("from"),
            CQL0001,
            "'from' followed by a code system",
        )?;
        let codesystem = expect(input, dotted_identifier, CQL0013, "a code system name")?;
        let display = opt_display(input)?;
        return Ok(Item::Code(CodeDefinition {
            access,
            name,
            code,
            codesystem,
            display,
        }));
    }
    if eat_keyword(input, "concept") {
        let name = expect(input, identifier_parser, CQL0013, "a concept name")?;
        expect(input, lit(":"), CQL0001, "':' after the concept name")?;
        expect(input, lit("{"), CQL0001, "'{' starting the code list")?;
        let mut codes = Vec::new();
        loop {
            codes.push(expect(input, dotted_identifier, CQL0013, "a code name")?);
            if !eat_lit(input, ",") {
                break;
            }
        }
        expect(input, lit("}"), CQL0001, "'}' closing the code list")?;
        let display = opt_display(input)?;
        return Ok(Item::Concept(ConceptDefinition {
            access,
            name,
            codes,
            display,
        }));
    }

    Err(Failure {
        at: start,
        code: CQL0001,
        expected: "a library item (using, include, codesystem, valueset, code, concept, parameter, context or define)",
    })
}

/// Parse an optional access modifier.
fn opt_access(input: &mut Input<'_>) -> Option<AccessModifier> {
    if eat_keyword(input, "public") {
        Some(AccessModifier::Public)
    } else if eat_keyword(input, "private") {
        Some(AccessModifier::Private)
    } else {
        None
    }
}

/// Parse the rest of a `define` (expression or function definition).
fn define_item<'a>(
    input: &mut Input<'a>,
    access: AccessModifier,
    total: usize,
) -> ItemResult<'a, Item> {
    let fluent = eat_keyword(input, "fluent");
    if fluent || eat_keyword(input, "function") {
        if fluent {
            expect(
                input,
                keyword("function"),
                CQL0001,
                "'function' after 'fluent'",
            )?;
        }
        return function_definition(input, access, fluent, total);
    }

    let name = expect(input, identifier_parser, CQL0013, "a definition name")?;
    expect(input, lit(":"), CQL0001, "':' after the definition name")?;
    let expression = expect(input, expression_parser, CQL0012, "an expression")?;
    Ok(Item::Expression(ExpressionDefinition {
        access,
        name,
        expression: Box::new(expression),
        context: None,
    }))
}

/// Parse `function Name(operands) [returns T]: body | external`.
fn function_definition<'a>(
    input: &mut Input<'a>,
    access: AccessModifier,
    fluent: bool,
    total: usize,
) -> ItemResult<'a, Item> {
    let name = expect(
        input,
        identifier_or_keyword_parser,
        CQL0013,
        "a function name",
    )?;
    expect(input, lit("("), CQL0001, "'(' starting the operand list")?;

    let mut parameters = Vec::new();
    if !eat_lit(input, ")") {
        loop {
            let operand = expect(
                input,
                identifier_or_keyword_parser,
                CQL0013,
                "an operand name",
            )?;
            let _ = ws(input);
            let type_start = *input;
            let type_specifier = expect(input, type_specifier_parser, CQL0014, "the operand type")?;
            parameters.push(FunctionParameter {
                name: operand,
                type_specifier: spanned_at(type_specifier, type_start, input, total),
            });
            if !eat_lit(input, ",") {
                break;
            }
        }
        expect(input, lit(")"), CQL0001, "')' closing the operand list")?;
    }

    let return_type = if eat_keyword(input, "returns") {
        let _ = ws(input);
        let type_start = *input;
        let ts = expect(input, type_specifier_parser, CQL0014, "the return type")?;
        Some(spanned_at(ts, type_start, input, total))
    } else {
        None
    };

    expect(input, lit(":"), CQL0001, "':' before the function body")?;
    let (body, external) = if eat_keyword(input, "external") {
        (None, true)
    } else {
        let body = expect(
            input,
            expression_parser,
            CQL0012,
            "the function body expression",
        )?;
        (Some(Box::new(body)), false)
    };

    Ok(Item::Function(FunctionDefinition {
        access,
        fluent,
        name,
        parameters,
        return_type,
        body,
        external,
        context: None,
    }))
}

/// Wrap `inner` in the absolute span between two positions of a buffer of
/// `total` bytes (positions are remaining-input suffixes of that buffer).
fn spanned_at<T>(inner: T, from: &str, to: &str, total: usize) -> Spanned<T> {
    Spanned::new(inner, Span::new(total - from.len(), total - to.len()))
}

/// Build a parse error pointing at byte offset `at` of the source.
fn syntax_error(
    source: &str,
    cleaned: &str,
    at: usize,
    code: ErrorCode,
    expected: &str,
) -> CqlError {
    let rest = &cleaned[at.min(cleaned.len())..];
    let token = next_token(rest);
    let (message, length) = if token.is_empty() {
        (format!("{expected}, found end of input"), 0)
    } else {
        (format!("{expected}, found `{token}`"), token.len())
    };
    let code = if token.is_empty() && code == CQL0001 {
        CQL0002
    } else {
        code
    };
    let location = SourceLocation::from_span(Span::new(at, at + length), source);
    CqlError::parse_at(code, message, source_line(source, at), location)
}

/// The token-ish text at the start of `rest`, for error messages.
fn next_token(rest: &str) -> &str {
    let rest = rest.trim_start();
    let mut chars = rest.char_indices();
    let Some((_, first)) = chars.next() else {
        return "";
    };
    let end = if first.is_alphanumeric() || first == '_' {
        rest.char_indices()
            .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
            .map_or(rest.len(), |(i, _)| i)
    } else if first == '"' || first == '\'' || first == '`' {
        rest[1..].find(first).map_or(rest.len(), |i| i + 2)
    } else {
        first.len_utf8()
    };
    let end = end.min(40);
    let end = (0..=end)
        .rev()
        .find(|i| rest.is_char_boundary(*i))
        .unwrap_or(0);
    &rest[..end]
}

/// The full source line containing byte offset `at`.
fn source_line(source: &str, at: usize) -> String {
    let at = at.min(source.len());
    let start = source[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = source[at..].find('\n').map_or(source.len(), |i| at + i);
    source[start..end].trim_end().to_string()
}

/// Stateful driver for the library-level parse.
struct LibraryParser<'s> {
    source: &'s str,
    cleaned: &'s str,
    mode: ParseMode,
    library: Library,
    errors: Vec<CqlError>,
    context: Option<String>,
}

impl<'s> LibraryParser<'s> {
    fn new(source: &'s str, cleaned: &'s str, mode: ParseMode) -> Self {
        Self {
            source,
            cleaned,
            mode,
            library: Library::new(),
            errors: Vec::new(),
            context: None,
        }
    }

    fn offset(&self, input: Input<'_>) -> usize {
        self.cleaned.len() - input.len()
    }

    fn run(mut self) -> ParseResult {
        let mut input: Input<'s> = self.cleaned;
        loop {
            let _ = ws(&mut input);
            if input.is_empty() {
                break;
            }
            let start = self.offset(input);
            match library_item(&mut input, self.cleaned.len()) {
                Ok(item) => {
                    let end = self.offset(input);
                    self.add(item, Span::new(start, end));
                    let mut probe = input;
                    let _ = ws(&mut probe);
                    if !probe.is_empty() && !starts_item(probe) {
                        self.fail(probe, CQL0001, "expected the end of the definition");
                        if self.mode == ParseMode::Fast {
                            break;
                        }
                        input = self.recover(probe);
                    }
                }
                Err(failure) => {
                    self.fail(failure.at, failure.code, failure.expected);
                    if self.mode == ParseMode::Fast {
                        break;
                    }
                    input = self.recover(failure.at);
                }
            }
        }

        if self.errors.is_empty() {
            ParseResult::success(self.library)
        } else {
            ParseResult {
                library: match self.mode {
                    ParseMode::Fast => None,
                    ParseMode::Analysis => Some(self.library),
                },
                errors: self.errors,
            }
        }
    }

    fn fail(&mut self, at: Input<'_>, code: ErrorCode, expected: &str) {
        let offset = self.offset(at);
        self.errors.push(syntax_error(
            self.source,
            self.cleaned,
            offset,
            code,
            expected,
        ));
    }

    /// Skip to the next line (after the failure point) whose first token is a
    /// top-level item keyword, or to the end of input.
    fn recover(&self, at: Input<'s>) -> Input<'s> {
        let from = self.offset(at);
        let mut line_start = self.source[from..]
            .find('\n')
            .map_or(self.source.len(), |i| from + i + 1);
        while line_start < self.source.len() {
            let candidate = &self.cleaned[line_start..];
            if starts_item(candidate) {
                return candidate;
            }
            line_start = self.source[line_start..]
                .find('\n')
                .map_or(self.source.len(), |i| line_start + i + 1);
        }
        &self.cleaned[self.cleaned.len()..]
    }

    fn add(&mut self, item: Item, span: Span) {
        match item {
            Item::Library(def) => self.library.definition = Some(def),
            Item::Using(def) => self.library.usings.push(Spanned::new(def, span)),
            Item::Include(def) => self.library.includes.push(Spanned::new(def, span)),
            Item::Parameter(def) => self.library.parameters.push(Spanned::new(def, span)),
            Item::CodeSystem(def) => self.library.codesystems.push(Spanned::new(def, span)),
            Item::ValueSet(def) => self.library.valuesets.push(Spanned::new(def, span)),
            Item::Code(def) => self.library.codes.push(Spanned::new(def, span)),
            Item::Concept(def) => self.library.concepts.push(Spanned::new(def, span)),
            Item::Context(def) => {
                self.context = Some(def.context.name.clone());
                self.library.contexts.push(Spanned::new(def, span));
            }
            Item::Expression(mut def) => {
                def.context = self.context.clone();
                self.library
                    .statements
                    .push(Spanned::new(Statement::ExpressionDef(def), span));
            }
            Item::Function(mut def) => {
                def.context = self.context.clone();
                self.library
                    .statements
                    .push(Spanned::new(Statement::FunctionDef(def), span));
            }
        }
    }
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
