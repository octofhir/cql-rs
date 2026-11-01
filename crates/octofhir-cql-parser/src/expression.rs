//! Expression parser using recursive descent with precedence climbing
//!
//! This parser uses a simple precedence climbing approach instead of Pratt parsing
//! to avoid complex symbol names that can cause macOS linker issues.

use crate::combinators::{
    Input, PResult, boolean_parser, identifier_or_keyword_parser, identifier_parser, keyword, lit,
    number_parser, padded_keyword, quantity_literal_parser, string_parser, temporal_literal_parser,
    ws,
};
use bigdecimal::BigDecimal;
use octofhir_cql_ast::{
    AggregateClause, AsCastExpr, BetweenExpr, BinaryOp, BinaryOpExpr, CaseExpr, CaseItem,
    ChoiceTypeSpecifier, CodeComparator, CodeSelectorExpr, ConceptSelectorExpr, ConvertExpr,
    DateConstructorExpr, DateTimeComponent, DateTimeComponentExpr, DateTimeConstructorExpr,
    DifferenceBetweenExpr, DurationBetweenExpr, Expression, FunctionRefExpr, Identifier,
    IdentifierRef, IfExpr, IndexerExpr, InstanceElement, InstanceExpr, IntervalExpr, IntervalOp,
    IntervalOpExpr, IntervalTypeSpecifier, IsNullExpr, IsTypeExpr, LetClause, ListExpr,
    ListTypeSpecifier, Literal, MinMaxValueExpr, NamedTypeSpecifier, PropertyAccess,
    QualifiedIdentifier, QuantityLiteral, Query, QuerySource, RelationshipClause, RelationshipKind,
    Retrieve, ReturnClause, SameAsExpr, SameOrAfterExpr, SameOrBeforeExpr, SortClause,
    SortDirection, SortItem, Spanned, TemporalPrecision, TimeConstructorExpr, TupleElement,
    TupleElementDefinition, TupleExpr, TupleTypeSpecifier, TypeSpecifier, UnaryOp, UnaryOpExpr,
};
use octofhir_cql_diagnostics::Span;
use winnow::combinator::{alt, opt, separated};
use winnow::error::ContextError;
use winnow::prelude::*;

/// Parse a CQL expression (entry point) - returns expression with spans
/// Note: Span tracking is currently placeholder (0, 0) - proper span tracking
/// can be added later when needed.
pub fn expression_parser<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    expression_with_dummy_spans(input)
}

/// Parse expression without proper span tracking (placeholder spans)
/// This is the top-level expression parser that tries query expressions first.
fn expression_with_dummy_spans<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    ws.parse_next(input)?;
    // Only try query_expression when input starts with patterns that could be queries:
    // - "from" keyword for multi-source queries
    // - "(" or "{" or "[" for single-source queries with alias
    let maybe_query = input.starts_with("from")
        || input.starts_with('(')
        || input.starts_with('{')
        || input.starts_with('[')
        || input.starts_with('"')
        || input.starts_with('`')
        || input
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_');

    if maybe_query {
        let checkpoint = *input;
        if let Ok(query) = query_expression(input) {
            return Ok(query);
        }
        // Restore input position if query parsing failed
        *input = checkpoint;
    }
    implies_expression(input)
}

/// Parse expression without trying query syntax.
/// Used for nested contexts like function arguments where query syntax doesn't apply.
fn inner_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    ws.parse_next(input)?;
    implies_expression(input)
}

/// Helper to wrap expression in dummy span
fn dummy_span<T>(inner: T) -> Spanned<T> {
    Spanned::new(inner, Span::new(0, 0))
}

/// Parse a type specifier: named (`Integer`, `FHIR.Patient`), `List<T>`,
/// `Interval<T>`, `Choice<T1, T2, ...>` and `Tuple { name Type, ... }`.
pub fn type_specifier_parser<'a>(input: &mut Input<'a>) -> PResult<TypeSpecifier> {
    let type_name = identifier_or_keyword_parser(input)?;
    let name = type_name.name.clone();
    let lower_name = name.to_lowercase();

    ws.parse_next(input)?;

    if lower_name == "tuple" {
        let checkpoint = *input;
        if lit("{").parse_next(input).is_ok() {
            let elements: Vec<TupleElementDefinition> = separated(
                0..,
                |input: &mut Input<'a>| {
                    ws.parse_next(input)?;
                    let element = identifier_or_keyword_parser(input)?;
                    ws.parse_next(input)?;
                    let element_type = type_specifier_parser(input)?;
                    ws.parse_next(input)?;
                    Ok(TupleElementDefinition::new(element, Some(element_type)))
                },
                ",",
            )
            .parse_next(input)?;
            ws.parse_next(input)?;
            lit("}").parse_next(input)?;
            return Ok(TypeSpecifier::Tuple(TupleTypeSpecifier::new(elements)));
        }
        *input = checkpoint;
    }

    if lit("<").parse_next(input).is_ok() {
        let arguments: Vec<TypeSpecifier> = separated(
            1..,
            |input: &mut Input<'a>| {
                ws.parse_next(input)?;
                let ts = type_specifier_parser(input)?;
                ws.parse_next(input)?;
                Ok(ts)
            },
            ",",
        )
        .parse_next(input)?;
        lit(">").parse_next(input)?;

        let mut arguments = arguments.into_iter();
        return match lower_name.as_str() {
            "list" if arguments.len() == 1 => Ok(TypeSpecifier::List(ListTypeSpecifier::new(
                arguments.next().expect("one argument"),
            ))),
            "interval" if arguments.len() == 1 => Ok(TypeSpecifier::Interval(
                IntervalTypeSpecifier::new(arguments.next().expect("one argument")),
            )),
            "choice" => Ok(TypeSpecifier::Choice(ChoiceTypeSpecifier::new(
                arguments.collect(),
            ))),
            _ => Err(ContextError::new()),
        };
    }

    // Qualified name (e.g., System.String, FHIR.Patient)
    let checkpoint = *input;
    if lit(".").parse_next(input).is_ok() {
        if let Ok(inner_name) = identifier_or_keyword_parser(input) {
            return Ok(TypeSpecifier::Named(NamedTypeSpecifier {
                namespace: Some(name),
                name: inner_name.name,
            }));
        }
        *input = checkpoint;
    }

    Ok(TypeSpecifier::Named(NamedTypeSpecifier {
        namespace: None,
        name,
    }))
}

/// Parse implies expression (lowest precedence, right-associative)
fn implies_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = or_expression(input)?;

    if padded_keyword("implies").parse_next(input).is_ok() {
        let right = implies_expression(input)?;
        left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
            left: Box::new(left),
            op: BinaryOp::Implies,
            right: Box::new(right),
        }));
    }

    Ok(left)
}

/// Parse or/xor expression
fn or_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = and_expression(input)?;

    loop {
        let op = if padded_keyword("or").parse_next(input).is_ok() {
            Some(BinaryOp::Or)
        } else if padded_keyword("xor").parse_next(input).is_ok() {
            Some(BinaryOp::Xor)
        } else {
            None
        };

        if let Some(op) = op {
            let right = and_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse and expression
fn and_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = membership_expression(input)?;

    loop {
        if padded_keyword("and").parse_next(input).is_ok() {
            let right = membership_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse membership expression (in, contains, between)
fn membership_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = equality_expression(input)?;

    loop {
        ws.parse_next(input)?;
        let checkpoint = *input;

        // Check for "between X and Y"
        if padded_keyword("between").parse_next(input).is_ok() {
            let low = equality_expression(input)?;
            ws.parse_next(input)?;
            if keyword("and").parse_next(input).is_ok() {
                ws.parse_next(input)?;
                let high = equality_expression(input)?;
                left = dummy_span(Expression::Between(BetweenExpr {
                    operand: Box::new(left),
                    low: Box::new(low),
                    high: Box::new(high),
                }));
                continue;
            } else {
                *input = checkpoint;
            }
        }

        let op = if padded_keyword("in").parse_next(input).is_ok() {
            Some(BinaryOp::In)
        } else if padded_keyword("contains").parse_next(input).is_ok() {
            Some(BinaryOp::Contains)
        } else {
            None
        };

        if let Some(op) = op {
            let right = equality_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse equality expression (=, !=, ~, !~)
fn equality_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = relational_expression(input)?;

    loop {
        ws.parse_next(input)?;

        let op = if lit("!=").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::NotEqual)
        } else if lit("!~").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::NotEquivalent)
        } else if lit("=").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Equal)
        } else if lit("~").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Equivalent)
        } else {
            None
        };

        if let Some(op) = op {
            let right = relational_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse relational expression (<, >, <=, >=)
fn relational_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = interval_operator_expression(input)?;

    loop {
        ws.parse_next(input)?;

        let op = if lit("<=").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::LessOrEqual)
        } else if lit(">=").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::GreaterOrEqual)
        } else if lit("<").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Less)
        } else if lit(">").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Greater)
        } else {
            None
        };

        if let Some(op) = op {
            let right = interval_operator_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse optional temporal precision
fn parse_temporal_precision<'a>(input: &mut Input<'a>) -> PResult<Option<TemporalPrecision>> {
    let checkpoint = *input;

    let precision = alt((
        keyword("year").value(TemporalPrecision::Year),
        keyword("month").value(TemporalPrecision::Month),
        keyword("week").value(TemporalPrecision::Week),
        keyword("day").value(TemporalPrecision::Day),
        keyword("hour").value(TemporalPrecision::Hour),
        keyword("minute").value(TemporalPrecision::Minute),
        keyword("second").value(TemporalPrecision::Second),
        keyword("millisecond").value(TemporalPrecision::Millisecond),
    ))
    .parse_next(input);

    match precision {
        Ok(p) => {
            ws.parse_next(input)?;
            // Must be followed by "of" for precision to apply
            if keyword("of").parse_next(input).is_ok() {
                ws.parse_next(input)?;
                Ok(Some(p))
            } else {
                *input = checkpoint;
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

/// Parse interval/temporal operators (after, before, same, meets, overlaps, etc.)
fn interval_operator_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = union_expression(input)?;

    loop {
        ws.parse_next(input)?;
        let checkpoint = *input;

        // Compound timing phrases (`starts before end of`, `3 days or less after`,
        // `within 30 days of`, `ends during`, ...)
        if let Some(timing) = compound_timing_phrase(input, &left)? {
            left = timing;
            continue;
        }
        *input = checkpoint;

        // Try parsing "same X or before/after", "same X as", or "on or before/after X"
        if padded_keyword("same").parse_next(input).is_ok() {
            // Parse optional precision
            let precision = alt((
                keyword("year").value(TemporalPrecision::Year),
                keyword("month").value(TemporalPrecision::Month),
                keyword("week").value(TemporalPrecision::Week),
                keyword("day").value(TemporalPrecision::Day),
                keyword("hour").value(TemporalPrecision::Hour),
                keyword("minute").value(TemporalPrecision::Minute),
                keyword("second").value(TemporalPrecision::Second),
                keyword("millisecond").value(TemporalPrecision::Millisecond),
            ))
            .parse_next(input)
            .ok();

            ws.parse_next(input)?;

            // "same X or before", "same X or after", or "same X as"
            if padded_keyword("or").parse_next(input).is_ok() {
                if padded_keyword("before").parse_next(input).is_ok() {
                    let right = union_expression(input)?;
                    left = dummy_span(Expression::SameOrBefore(SameOrBeforeExpr {
                        left: Box::new(left),
                        right: Box::new(right),
                        precision,
                    }));
                    continue;
                } else if padded_keyword("after").parse_next(input).is_ok() {
                    let right = union_expression(input)?;
                    left = dummy_span(Expression::SameOrAfter(SameOrAfterExpr {
                        left: Box::new(left),
                        right: Box::new(right),
                        precision,
                    }));
                    continue;
                }
            } else if padded_keyword("as").parse_next(input).is_ok() {
                let right = union_expression(input)?;
                left = dummy_span(Expression::SameAs(SameAsExpr {
                    left: Box::new(left),
                    right: Box::new(right),
                    precision,
                }));
                continue;
            }
            *input = checkpoint;
            break;
        }

        // "on or before/after X" - alternate syntax for same or before/after
        if padded_keyword("on").parse_next(input).is_ok() {
            if padded_keyword("or").parse_next(input).is_ok() {
                if padded_keyword("before").parse_next(input).is_ok() {
                    // Parse optional precision after the keyword
                    let precision = alt((
                        keyword("year").value(TemporalPrecision::Year),
                        keyword("month").value(TemporalPrecision::Month),
                        keyword("week").value(TemporalPrecision::Week),
                        keyword("day").value(TemporalPrecision::Day),
                        keyword("hour").value(TemporalPrecision::Hour),
                        keyword("minute").value(TemporalPrecision::Minute),
                        keyword("second").value(TemporalPrecision::Second),
                        keyword("millisecond").value(TemporalPrecision::Millisecond),
                    ))
                    .parse_next(input)
                    .ok();
                    // Skip "of" if present
                    let _ = padded_keyword("of").parse_next(input);
                    let right = union_expression(input)?;
                    left = dummy_span(Expression::SameOrBefore(SameOrBeforeExpr {
                        left: Box::new(left),
                        right: Box::new(right),
                        precision,
                    }));
                    continue;
                } else if padded_keyword("after").parse_next(input).is_ok() {
                    // Parse optional precision after the keyword
                    let precision = alt((
                        keyword("year").value(TemporalPrecision::Year),
                        keyword("month").value(TemporalPrecision::Month),
                        keyword("week").value(TemporalPrecision::Week),
                        keyword("day").value(TemporalPrecision::Day),
                        keyword("hour").value(TemporalPrecision::Hour),
                        keyword("minute").value(TemporalPrecision::Minute),
                        keyword("second").value(TemporalPrecision::Second),
                        keyword("millisecond").value(TemporalPrecision::Millisecond),
                    ))
                    .parse_next(input)
                    .ok();
                    // Skip "of" if present
                    let _ = padded_keyword("of").parse_next(input);
                    let right = union_expression(input)?;
                    left = dummy_span(Expression::SameOrAfter(SameOrAfterExpr {
                        left: Box::new(left),
                        right: Box::new(right),
                        precision,
                    }));
                    continue;
                }
            }
            *input = checkpoint;
            break;
        }

        // Try parsing interval operators with optional precision
        // Multi-word operators first (properly includes, included in, properly included in)
        let checkpoint = *input;
        let op = if padded_keyword("properly").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            if padded_keyword("includes").parse_next(input).is_ok() {
                Some(IntervalOp::ProperlyIncludes)
            } else if padded_keyword("included").parse_next(input).is_ok() {
                ws.parse_next(input)?;
                if padded_keyword("in").parse_next(input).is_ok() {
                    Some(IntervalOp::ProperlyIncludedIn)
                } else {
                    *input = checkpoint;
                    None
                }
            } else {
                *input = checkpoint;
                None
            }
        } else if padded_keyword("included").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            if padded_keyword("in").parse_next(input).is_ok() {
                Some(IntervalOp::IncludedIn)
            } else {
                *input = checkpoint;
                None
            }
        } else if padded_keyword("after").parse_next(input).is_ok() {
            Some(IntervalOp::After)
        } else if padded_keyword("before").parse_next(input).is_ok() {
            Some(IntervalOp::Before)
        } else if padded_keyword("meets").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            if padded_keyword("before").parse_next(input).is_ok() {
                Some(IntervalOp::MeetsBefore)
            } else if padded_keyword("after").parse_next(input).is_ok() {
                Some(IntervalOp::MeetsAfter)
            } else {
                Some(IntervalOp::Meets)
            }
        } else if padded_keyword("overlaps").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            if padded_keyword("before").parse_next(input).is_ok() {
                Some(IntervalOp::OverlapsBefore)
            } else if padded_keyword("after").parse_next(input).is_ok() {
                Some(IntervalOp::OverlapsAfter)
            } else {
                Some(IntervalOp::Overlaps)
            }
        } else if padded_keyword("starts").parse_next(input).is_ok() {
            if let Some(timing) = try_parse_timing_phrase(input, IntervalOp::Starts, left.clone())?
            {
                left = timing;
                continue;
            }
            Some(IntervalOp::Starts)
        } else if padded_keyword("ends").parse_next(input).is_ok() {
            if let Some(timing) = try_parse_timing_phrase(input, IntervalOp::Ends, left.clone())? {
                left = timing;
                continue;
            }
            Some(IntervalOp::Ends)
        } else if padded_keyword("during").parse_next(input).is_ok() {
            Some(IntervalOp::During)
        } else if padded_keyword("includes").parse_next(input).is_ok() {
            Some(IntervalOp::Includes)
        } else {
            None
        };

        if let Some(op) = op {
            // Parse optional precision (e.g., "after year of")
            let precision = parse_temporal_precision(input)?;
            let right = union_expression(input)?;
            left = dummy_span(Expression::IntervalOp(IntervalOpExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                precision,
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Which point of the left operand a timing phrase compares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TimingBoundary {
    Start,
    End,
    Occurs,
}

/// Quantity offset of a before/after phrase.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OffsetKind {
    /// `3 days before`
    Exactly,
    /// `3 days or more before`
    OrMore,
    /// `3 days or less before`
    OrLess,
    /// `less than 3 days before`
    LessThan,
    /// `more than 3 days before`
    MoreThan,
}

fn point_call(name: &str, operand: Spanned<Expression>) -> Spanned<Expression> {
    dummy_span(Expression::FunctionRef(FunctionRefExpr {
        library: None,
        name: Identifier::new(name),
        arguments: vec![operand],
    }))
}

fn binary(
    op: BinaryOp,
    left: Spanned<Expression>,
    right: Spanned<Expression>,
) -> Spanned<Expression> {
    dummy_span(Expression::BinaryOp(BinaryOpExpr {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }))
}

fn interval_op(
    op: IntervalOp,
    left: Spanned<Expression>,
    right: Spanned<Expression>,
    precision: Option<TemporalPrecision>,
) -> Spanned<Expression> {
    dummy_span(Expression::IntervalOp(IntervalOpExpr {
        left: Box::new(left),
        op,
        right: Box::new(right),
        precision,
    }))
}

fn same_or_before(
    left: Spanned<Expression>,
    right: Spanned<Expression>,
    precision: Option<TemporalPrecision>,
) -> Spanned<Expression> {
    dummy_span(Expression::SameOrBefore(SameOrBeforeExpr {
        left: Box::new(left),
        right: Box::new(right),
        precision,
    }))
}

fn same_or_after(
    left: Spanned<Expression>,
    right: Spanned<Expression>,
    precision: Option<TemporalPrecision>,
) -> Spanned<Expression> {
    dummy_span(Expression::SameOrAfter(SameOrAfterExpr {
        left: Box::new(left),
        right: Box::new(right),
        precision,
    }))
}

/// Parse a temporal precision keyword (`day`, `days`, ...) without `of`.
fn precision_keyword<'a>(input: &mut Input<'a>) -> Option<TemporalPrecision> {
    let checkpoint = *input;
    let _ = ws.parse_next(input);
    let precision = alt((
        alt((keyword("years"), keyword("year"))).value(TemporalPrecision::Year),
        alt((keyword("months"), keyword("month"))).value(TemporalPrecision::Month),
        alt((keyword("weeks"), keyword("week"))).value(TemporalPrecision::Week),
        alt((keyword("days"), keyword("day"))).value(TemporalPrecision::Day),
        alt((keyword("hours"), keyword("hour"))).value(TemporalPrecision::Hour),
        alt((keyword("minutes"), keyword("minute"))).value(TemporalPrecision::Minute),
        alt((keyword("seconds"), keyword("second"))).value(TemporalPrecision::Second),
        alt((keyword("milliseconds"), keyword("millisecond")))
            .value(TemporalPrecision::Millisecond),
    ))
    .parse_next(input)
    .ok();
    if precision.is_none() {
        *input = checkpoint;
    }
    precision
}

/// Parse the compound CQL 1.5 timing phrases (spec §5.6.2, grammar rule
/// `intervalOperatorPhrase`) that the simple operator loop does not cover:
///
/// - `(starts|ends|occurs) <relationship> ...` with any relationship
/// - quantity offsets: `3 days [or more|or less] before|after`,
///   `less than|more than 3 days before|after`
/// - `[on or] before|after [or on]`
/// - `[properly] within 3 days of`
/// - `(starts|ends|occurs) [properly] during|included in`
/// - `(starts|ends|occurs) same <precision> as|or before|or after`
///
/// Returns `Ok(None)` (input untouched by the caller's checkpoint) when no
/// compound phrase starts here.
fn compound_timing_phrase<'a>(
    input: &mut Input<'a>,
    left: &Spanned<Expression>,
) -> PResult<Option<Spanned<Expression>>> {
    let start = *input;

    let boundary = if padded_keyword("starts").parse_next(input).is_ok() {
        Some(TimingBoundary::Start)
    } else if padded_keyword("ends").parse_next(input).is_ok() {
        Some(TimingBoundary::End)
    } else if padded_keyword("occurs").parse_next(input).is_ok() {
        Some(TimingBoundary::Occurs)
    } else {
        None
    };

    // Quantity offset
    let mut offset: Option<(Spanned<Expression>, OffsetKind)> = None;
    {
        let checkpoint = *input;
        let exclusive = if padded_keyword("less").parse_next(input).is_ok()
            && padded_keyword("than").parse_next(input).is_ok()
        {
            Some(OffsetKind::LessThan)
        } else {
            *input = checkpoint;
            if padded_keyword("more").parse_next(input).is_ok()
                && padded_keyword("than").parse_next(input).is_ok()
            {
                Some(OffsetKind::MoreThan)
            } else {
                *input = checkpoint;
                None
            }
        };
        let _ = ws.parse_next(input);
        match quantity_literal_parser(input) {
            Ok(q) if q.unit.is_some() => {
                let quantity = dummy_span(Expression::Literal(Literal::Quantity(q)));
                let kind = match exclusive {
                    Some(kind) => kind,
                    None => {
                        let qualifier = *input;
                        if padded_keyword("or").parse_next(input).is_ok() {
                            if padded_keyword("more").parse_next(input).is_ok() {
                                OffsetKind::OrMore
                            } else if padded_keyword("less").parse_next(input).is_ok() {
                                OffsetKind::OrLess
                            } else {
                                *input = qualifier;
                                OffsetKind::Exactly
                            }
                        } else {
                            OffsetKind::Exactly
                        }
                    }
                };
                offset = Some((quantity, kind));
            }
            _ => {
                if exclusive.is_some() {
                    *input = start;
                    return Ok(None);
                }
                *input = checkpoint;
            }
        }
    }

    let left_point = match boundary {
        Some(TimingBoundary::Start) => point_call("Start", left.clone()),
        Some(TimingBoundary::End) => point_call("End", left.clone()),
        Some(TimingBoundary::Occurs) | None => left.clone(),
    };

    // `[properly] within <quantity> of`
    {
        let checkpoint = *input;
        let properly = padded_keyword("properly").parse_next(input).is_ok();
        if offset.is_none() && padded_keyword("within").parse_next(input).is_ok() {
            let _ = ws.parse_next(input);
            let q = quantity_literal_parser(input)?;
            padded_keyword("of").parse_next(input)?;
            let right = union_expression(input)?;
            let quantity = dummy_span(Expression::Literal(Literal::Quantity(q)));
            let low = binary(BinaryOp::Subtract, right.clone(), quantity.clone());
            let high = binary(BinaryOp::Add, right, quantity);
            let expr = if properly {
                binary(
                    BinaryOp::And,
                    interval_op(IntervalOp::After, left_point.clone(), low, None),
                    interval_op(IntervalOp::Before, left_point, high, None),
                )
            } else {
                binary(
                    BinaryOp::And,
                    same_or_after(left_point.clone(), low, None),
                    same_or_before(left_point, high, None),
                )
            };
            return Ok(Some(expr));
        }
        *input = checkpoint;
    }

    // `[on or] before|after [or on]`
    let checkpoint = *input;
    let mut inclusive = padded_keyword("on").parse_next(input).is_ok()
        && padded_keyword("or").parse_next(input).is_ok();
    if !inclusive {
        *input = checkpoint;
    }
    let direction = if padded_keyword("before").parse_next(input).is_ok() {
        Some(IntervalOp::Before)
    } else if padded_keyword("after").parse_next(input).is_ok() {
        Some(IntervalOp::After)
    } else {
        None
    };

    if let Some(direction) = direction {
        let or_on = *input;
        if padded_keyword("or").parse_next(input).is_ok()
            && padded_keyword("on").parse_next(input).is_ok()
        {
            inclusive = true;
        } else {
            *input = or_on;
        }
        // Simple `X before Y` / `X on or before Y` stay with the operator loop.
        if boundary.is_none() && offset.is_none() {
            *input = start;
            return Ok(None);
        }
        let precision = parse_temporal_precision(input)?;
        let right = union_expression(input)?;
        let before = direction == IntervalOp::Before;
        let right_point = right;

        let expr = match offset {
            None => {
                if inclusive {
                    if before {
                        same_or_before(left_point, right_point, precision)
                    } else {
                        same_or_after(left_point, right_point, precision)
                    }
                } else {
                    interval_op(direction, left_point, right_point, precision)
                }
            }
            Some((quantity, kind)) => {
                // Reference point shifted by the offset: right - q (before) / right + q (after)
                let shifted = binary(
                    if before {
                        BinaryOp::Subtract
                    } else {
                        BinaryOp::Add
                    },
                    right_point.clone(),
                    quantity,
                );
                // "beyond the shifted point" and "between the shifted point and the reference"
                let beyond_inclusive = |l: Spanned<Expression>, r: Spanned<Expression>| {
                    if before {
                        same_or_before(l, r, precision)
                    } else {
                        same_or_after(l, r, precision)
                    }
                };
                let beyond_strict = |l: Spanned<Expression>, r: Spanned<Expression>| {
                    interval_op(direction, l, r, precision)
                };
                let toward_inclusive = |l: Spanned<Expression>, r: Spanned<Expression>| {
                    if before {
                        same_or_after(l, r, precision)
                    } else {
                        same_or_before(l, r, precision)
                    }
                };
                let toward_strict = |l: Spanned<Expression>, r: Spanned<Expression>| {
                    interval_op(
                        if before {
                            IntervalOp::After
                        } else {
                            IntervalOp::Before
                        },
                        l,
                        r,
                        precision,
                    )
                };
                // Relation to the reference point itself (inclusive with `on or`)
                let near_reference = |l: Spanned<Expression>, r: Spanned<Expression>| {
                    if inclusive {
                        beyond_inclusive(l, r)
                    } else {
                        beyond_strict(l, r)
                    }
                };
                match kind {
                    OffsetKind::Exactly => dummy_span(Expression::SameAs(SameAsExpr {
                        left: Box::new(left_point),
                        right: Box::new(shifted),
                        precision,
                    })),
                    OffsetKind::OrMore => beyond_inclusive(left_point, shifted),
                    OffsetKind::MoreThan => beyond_strict(left_point, shifted),
                    OffsetKind::OrLess => binary(
                        BinaryOp::And,
                        toward_inclusive(left_point.clone(), shifted),
                        near_reference(left_point, right_point),
                    ),
                    OffsetKind::LessThan => binary(
                        BinaryOp::And,
                        toward_strict(left_point.clone(), shifted),
                        near_reference(left_point, right_point),
                    ),
                }
            }
        };
        return Ok(Some(expr));
    }

    if offset.is_some() {
        // An offset must be followed by before/after.
        *input = start;
        return Ok(None);
    }

    let Some(boundary) = boundary else {
        *input = start;
        return Ok(None);
    };

    // `(starts|ends|occurs) [properly] during|included in`
    let checkpoint = *input;
    let properly = padded_keyword("properly").parse_next(input).is_ok();
    let membership = if padded_keyword("during").parse_next(input).is_ok()
        || (padded_keyword("included").parse_next(input).is_ok()
            && padded_keyword("in").parse_next(input).is_ok())
    {
        true
    } else {
        *input = checkpoint;
        false
    };
    if membership {
        let precision = parse_temporal_precision(input)?;
        let right = union_expression(input)?;
        let op = match (boundary, properly) {
            (TimingBoundary::Occurs, false) => IntervalOp::During,
            (TimingBoundary::Occurs, true) => IntervalOp::ProperlyIncludedIn,
            (_, false) => IntervalOp::During,
            (_, true) => IntervalOp::ProperlyIncludedIn,
        };
        return Ok(Some(interval_op(op, left_point, right, precision)));
    }

    // `(starts|ends|occurs) same <precision> as|or before|or after`
    if padded_keyword("same").parse_next(input).is_ok() {
        let precision = precision_keyword(input);
        let expr = if padded_keyword("as").parse_next(input).is_ok() {
            let right = union_expression(input)?;
            dummy_span(Expression::SameAs(SameAsExpr {
                left: Box::new(left_point),
                right: Box::new(right),
                precision,
            }))
        } else if padded_keyword("or").parse_next(input).is_ok() {
            if padded_keyword("before").parse_next(input).is_ok() {
                same_or_before(left_point, union_expression(input)?, precision)
            } else if padded_keyword("after").parse_next(input).is_ok() {
                same_or_after(left_point, union_expression(input)?, precision)
            } else {
                *input = start;
                return Ok(None);
            }
        } else {
            *input = start;
            return Ok(None);
        };
        return Ok(Some(expr));
    }

    // Plain `X starts Y` / `X ends Y` stay with the operator loop.
    *input = start;
    Ok(None)
}

/// Helper to parse complex timing phrases like "starts 1 day or less on or after"
/// Synthesizes AST using standard operators (SameOrAfter, SameOrBefore, Add)
fn try_parse_timing_phrase<'a>(
    input: &mut Input<'a>,
    boundary: IntervalOp,
    left: Spanned<Expression>,
) -> PResult<Option<Spanned<Expression>>> {
    let checkpoint = *input;
    ws.parse_next(input)?;

    // 1. Parse Quantity (e.g. "1 day")
    let quantity = if let Ok(q) = quantity_literal_parser(input) {
        dummy_span(Expression::Literal(Literal::Quantity(q)))
    } else {
        *input = checkpoint;
        return Ok(None);
    };

    ws.parse_next(input)?;

    // 2. Parse Offset Qualifier ("or less")
    // Note: We currently only support "or less" for Issue32Interval compatibility
    let _is_or_less = if padded_keyword("or").parse_next(input).is_ok() {
        if padded_keyword("less").parse_next(input).is_ok() {
            true
        } else {
            *input = checkpoint;
            return Ok(None);
        }
    } else {
        *input = checkpoint;
        return Ok(None);
    };

    ws.parse_next(input)?;

    // 3. Parse Direction ("on or after")
    // Note: We currently only support "on or after" for Issue32Interval compatibility
    if padded_keyword("on").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        if padded_keyword("or").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            if padded_keyword("after").parse_next(input).is_ok() {
                // Match: on or after (>=)
            } else {
                *input = checkpoint;
                return Ok(None);
            }
        } else {
            *input = checkpoint;
            return Ok(None);
        }
    } else {
        *input = checkpoint;
        return Ok(None);
    };

    // 4. Parse Precision ("day of")
    let precision = parse_temporal_precision(input)?;

    // 5. Parse Anchor ("start of", "end of" - optional)
    let anchor_checkpoint = *input;
    ws.parse_next(input)?;
    let right_anchor_start = if keyword("start").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        if keyword("of").parse_next(input).is_ok() {
            true
        } else {
            *input = anchor_checkpoint;
            true // default
        }
    } else if keyword("end").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        if keyword("of").parse_next(input).is_ok() {
            false
        } else {
            *input = anchor_checkpoint;
            true // default
        }
    } else {
        *input = anchor_checkpoint;
        true // default to Start
    };

    // 6. Right operand expression
    let right_expr = union_expression(input)?;

    // Construct logic:
    // "starts 1 day or less on or after" ->
    // Start(Left) >= Start(Right) AND Start(Left) <= Start(Right) + 1 day

    // Left Point
    let left_point = dummy_span(Expression::FunctionRef(FunctionRefExpr {
        library: None,
        name: Identifier::new(if boundary == IntervalOp::Starts {
            "Start"
        } else {
            "End"
        }),
        arguments: vec![left],
    }));

    // Right Point
    let right_point = dummy_span(Expression::FunctionRef(FunctionRefExpr {
        library: None,
        name: Identifier::new(if right_anchor_start { "Start" } else { "End" }),
        arguments: vec![right_expr],
    }));

    // Right Point + Quantity
    let right_plus_quantity = dummy_span(Expression::BinaryOp(BinaryOpExpr {
        op: BinaryOp::Add,
        left: Box::new(right_point.clone()),
        right: Box::new(quantity),
    }));

    // Condition 1: Left >= Right (SameOrAfter)
    let cond1 = dummy_span(Expression::SameOrAfter(SameOrAfterExpr {
        left: Box::new(left_point.clone()),
        right: Box::new(right_point),
        precision,
    }));

    // Condition 2: Left <= Right + Quantity (SameOrBefore)
    let cond2 = dummy_span(Expression::SameOrBefore(SameOrBeforeExpr {
        left: Box::new(left_point),
        right: Box::new(right_plus_quantity),
        precision,
    }));

    // Combine with AND
    let and_expr = dummy_span(Expression::BinaryOp(BinaryOpExpr {
        op: BinaryOp::And,
        left: Box::new(cond1),
        right: Box::new(cond2),
    }));

    Ok(Some(and_expr))
}

/// Parse union expression (| or 'union')
fn union_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = additive_expression(input)?;

    loop {
        ws.parse_next(input)?;
        // Support both | symbol and 'union' keyword
        if lit("|").parse_next(input).is_ok() || keyword("union").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            let right = additive_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op: BinaryOp::Union,
                right: Box::new(right),
            }));
        } else if keyword("except").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            let right = additive_expression(input)?;
            // Except is a set difference operation
            left = dummy_span(Expression::FunctionRef(FunctionRefExpr {
                library: None,
                name: Identifier::new("Except"),
                arguments: vec![left, right],
            }));
        } else if keyword("intersect").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            let right = additive_expression(input)?;
            // Intersect is a set intersection operation
            left = dummy_span(Expression::FunctionRef(FunctionRefExpr {
                library: None,
                name: Identifier::new("Intersect"),
                arguments: vec![left, right],
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse additive expression (+, -, &)
fn additive_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = multiplicative_expression(input)?;

    loop {
        ws.parse_next(input)?;

        let op = if lit("+").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Add)
        } else if lit("-").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Subtract)
        } else if lit("&").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Concatenate)
        } else {
            None
        };

        if let Some(op) = op {
            let right = multiplicative_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse multiplicative expression (*, /, div, mod)
fn multiplicative_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut left = power_expression(input)?;

    loop {
        ws.parse_next(input)?;

        let op = if lit("*").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Multiply)
        } else if lit("/").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            Some(BinaryOp::Divide)
        } else if padded_keyword("div").parse_next(input).is_ok() {
            Some(BinaryOp::TruncatedDivide)
        } else if padded_keyword("mod").parse_next(input).is_ok() {
            Some(BinaryOp::Modulo)
        } else {
            None
        };

        if let Some(op) = op {
            let right = power_expression(input)?;
            left = dummy_span(Expression::BinaryOp(BinaryOpExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            }));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse power expression (^, right-associative)
fn power_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let left = type_expression(input)?;

    ws.parse_next(input)?;
    if lit("^").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        let right = power_expression(input)?;
        Ok(dummy_span(Expression::BinaryOp(BinaryOpExpr {
            left: Box::new(left),
            op: BinaryOp::Power,
            right: Box::new(right),
        })))
    } else {
        Ok(left)
    }
}

/// Parse type expression (is null, is Type, as Type)
/// This level handles type testing operators that bind looser than unary/component extraction
fn type_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut base = unary_expression(input)?;

    loop {
        ws.parse_next(input)?;

        // Parse "as Type" expression
        if padded_keyword("as").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            let type_spec = type_specifier_parser(input)?;
            base = dummy_span(Expression::As(AsCastExpr {
                operand: Box::new(base),
                as_type: dummy_span(type_spec),
                strict: false,
            }));
            continue;
        }

        // Parse "is null", "is not null", or "is Type" expression
        if padded_keyword("is").parse_next(input).is_ok() {
            ws.parse_next(input)?;

            // Check for "is not null"
            let checkpoint = *input;
            if padded_keyword("not").parse_next(input).is_ok() {
                ws.parse_next(input)?;
                if keyword("null").parse_next(input).is_ok() {
                    // "is not null" - wrap in Not(IsNull)
                    base = dummy_span(Expression::UnaryOp(UnaryOpExpr {
                        op: UnaryOp::Not,
                        operand: Box::new(dummy_span(Expression::IsNull(IsNullExpr {
                            operand: Box::new(base),
                        }))),
                    }));
                    continue;
                }
                // Not "is not null", restore position
                *input = checkpoint;
                ws.parse_next(input)?;
            }

            // Check for "is null"
            if keyword("null").parse_next(input).is_ok() {
                base = dummy_span(Expression::IsNull(IsNullExpr {
                    operand: Box::new(base),
                }));
                continue;
            }

            // Parse type specifier (handles generics like List<String>)
            let type_spec = type_specifier_parser(input)?;
            base = dummy_span(Expression::Is(IsTypeExpr {
                operand: Box::new(base),
                is_type: dummy_span(type_spec),
            }));
            continue;
        }

        break;
    }

    Ok(base)
}

/// Parse unary expression (-, +, not, exists, component extraction)
fn unary_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    ws.parse_next(input)?;

    if lit("-").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Negate,
            operand: Box::new(operand),
        })));
    }

    if lit("+").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Plus,
            operand: Box::new(operand),
        })));
    }

    if padded_keyword("not").parse_next(input).is_ok() {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Not,
            operand: Box::new(operand),
        })));
    }

    if padded_keyword("exists").parse_next(input).is_ok() {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Exists,
            operand: Box::new(operand),
        })));
    }

    if padded_keyword("collapse").parse_next(input).is_ok() {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Collapse,
            operand: Box::new(operand),
        })));
    }

    if padded_keyword("distinct").parse_next(input).is_ok() {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Distinct,
            operand: Box::new(operand),
        })));
    }

    if padded_keyword("flatten").parse_next(input).is_ok() {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Flatten,
            operand: Box::new(operand),
        })));
    }

    // start of, end of, width of - interval boundary extractors
    if padded_keyword("start").parse_next(input).is_ok()
        && padded_keyword("of").parse_next(input).is_ok()
    {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
            library: None,
            name: Identifier::new("Start"),
            arguments: vec![operand],
        })));
    }

    if padded_keyword("end").parse_next(input).is_ok()
        && padded_keyword("of").parse_next(input).is_ok()
    {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
            library: None,
            name: Identifier::new("End"),
            arguments: vec![operand],
        })));
    }

    if padded_keyword("width").parse_next(input).is_ok()
        && padded_keyword("of").parse_next(input).is_ok()
    {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
            library: None,
            name: Identifier::new("Width"),
            arguments: vec![operand],
        })));
    }

    // singleton from - extracts single element from list
    if padded_keyword("singleton").parse_next(input).is_ok()
        && padded_keyword("from").parse_next(input).is_ok()
    {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::SingletonFrom,
            operand: Box::new(operand),
        })));
    }

    // point from - extracts point from unit interval
    if padded_keyword("point").parse_next(input).is_ok()
        && padded_keyword("from").parse_next(input).is_ok()
    {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
            library: None,
            name: Identifier::new("PointFrom"),
            arguments: vec![operand],
        })));
    }

    // minimum/maximum Type
    if padded_keyword("minimum").parse_next(input).is_ok() {
        // Allow type keywords like DateTime, Date, Time, Integer, etc.
        let type_name = alt((
            keyword("DateTime").map(|_| Identifier::new("DateTime")),
            keyword("Date").map(|_| Identifier::new("Date")),
            keyword("Time").map(|_| Identifier::new("Time")),
            identifier_parser,
        ))
        .parse_next(input)?;
        return Ok(dummy_span(Expression::MinValue(MinMaxValueExpr {
            value_type: type_name,
        })));
    }

    if padded_keyword("maximum").parse_next(input).is_ok() {
        // Allow type keywords like DateTime, Date, Time, Integer, etc.
        let type_name = alt((
            keyword("DateTime").map(|_| Identifier::new("DateTime")),
            keyword("Date").map(|_| Identifier::new("Date")),
            keyword("Time").map(|_| Identifier::new("Time")),
            identifier_parser,
        ))
        .parse_next(input)?;
        return Ok(dummy_span(Expression::MaxValue(MinMaxValueExpr {
            value_type: type_name,
        })));
    }

    // predecessor of X
    if padded_keyword("predecessor").parse_next(input).is_ok()
        && padded_keyword("of").parse_next(input).is_ok()
    {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Predecessor,
            operand: Box::new(operand),
        })));
    }

    // successor of X
    if padded_keyword("successor").parse_next(input).is_ok()
        && padded_keyword("of").parse_next(input).is_ok()
    {
        let operand = unary_expression(input)?;
        return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
            op: UnaryOp::Successor,
            operand: Box::new(operand),
        })));
    }

    // expand X [per Y] - expand takes an interval or list, with optional per clause
    if padded_keyword("expand").parse_next(input).is_ok() {
        let operand = unary_expression(input)?;
        // Optionally parse "per" clause
        ws.parse_next(input)?;
        let mut arguments = vec![operand];
        if padded_keyword("per").parse_next(input).is_ok() {
            // Parse the quantity (e.g., "day", "2 days", "hour")
            // First try singular time units (day, hour, minute, etc.) which mean "1 day", "1 hour"
            let per_value = if let Ok(unit) = parse_singular_time_unit(input) {
                // Wrap in a quantity with value 1
                dummy_span(Expression::Literal(Literal::Quantity(
                    QuantityLiteral::new(BigDecimal::from(1)).with_unit(unit),
                )))
            } else {
                unary_expression(input)?
            };
            arguments.push(per_value);
        }
        // Return as a function call with capitalized name for converter routing
        return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
            library: None,
            name: Identifier::new("Expand"),
            arguments,
        })));
    }

    // Component extraction: year from X, month from X, etc.
    if let Ok(expr) = component_extraction(input) {
        return Ok(expr);
    }

    // Duration/difference between: duration in years between X and Y
    if let Ok(expr) = duration_difference_between(input) {
        return Ok(expr);
    }

    postfix_expression(input)
}

/// Parse a singular time unit keyword (day, hour, minute, etc.)
/// Returns the UCUM-compatible unit string
fn parse_singular_time_unit<'a>(input: &mut Input<'a>) -> PResult<String> {
    let checkpoint = *input;

    // Try to parse a temporal unit keyword
    if keyword("year").parse_next(input).is_ok() || keyword("years").parse_next(input).is_ok() {
        return Ok("year".to_string());
    }
    *input = checkpoint;

    if keyword("month").parse_next(input).is_ok() || keyword("months").parse_next(input).is_ok() {
        return Ok("month".to_string());
    }
    *input = checkpoint;

    if keyword("week").parse_next(input).is_ok() || keyword("weeks").parse_next(input).is_ok() {
        return Ok("week".to_string());
    }
    *input = checkpoint;

    if keyword("day").parse_next(input).is_ok() || keyword("days").parse_next(input).is_ok() {
        return Ok("day".to_string());
    }
    *input = checkpoint;

    if keyword("hour").parse_next(input).is_ok() || keyword("hours").parse_next(input).is_ok() {
        return Ok("hour".to_string());
    }
    *input = checkpoint;

    if keyword("minute").parse_next(input).is_ok() || keyword("minutes").parse_next(input).is_ok() {
        return Ok("minute".to_string());
    }
    *input = checkpoint;

    if keyword("second").parse_next(input).is_ok() || keyword("seconds").parse_next(input).is_ok() {
        return Ok("second".to_string());
    }
    *input = checkpoint;

    if keyword("millisecond").parse_next(input).is_ok()
        || keyword("milliseconds").parse_next(input).is_ok()
    {
        return Ok("millisecond".to_string());
    }
    *input = checkpoint;

    Err(ContextError::new())
}

/// Parse duration/difference in X between Y and Z
/// Also supports short form: years between X and Y (equivalent to duration in years between)
fn duration_difference_between<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let checkpoint = *input;

    // Try "difference" or "duration" prefix, or short form (precision between)
    let is_difference = if keyword("difference").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        // Expect "in"
        if keyword("in").parse_next(input).is_err() {
            *input = checkpoint;
            return Err(ContextError::new());
        }
        ws.parse_next(input)?;
        true
    } else if keyword("duration").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        // Expect "in"
        if keyword("in").parse_next(input).is_err() {
            *input = checkpoint;
            return Err(ContextError::new());
        }
        ws.parse_next(input)?;
        false
    } else {
        // Short form: years between X and Y (no difference/duration prefix, defaults to duration)
        false
    };

    // Parse precision
    let precision = alt((
        keyword("years").value(TemporalPrecision::Year),
        keyword("year").value(TemporalPrecision::Year),
        keyword("months").value(TemporalPrecision::Month),
        keyword("month").value(TemporalPrecision::Month),
        keyword("weeks").value(TemporalPrecision::Week),
        keyword("week").value(TemporalPrecision::Week),
        keyword("days").value(TemporalPrecision::Day),
        keyword("day").value(TemporalPrecision::Day),
        keyword("hours").value(TemporalPrecision::Hour),
        keyword("hour").value(TemporalPrecision::Hour),
        keyword("minutes").value(TemporalPrecision::Minute),
        keyword("minute").value(TemporalPrecision::Minute),
        keyword("seconds").value(TemporalPrecision::Second),
        keyword("second").value(TemporalPrecision::Second),
        keyword("milliseconds").value(TemporalPrecision::Millisecond),
        keyword("millisecond").value(TemporalPrecision::Millisecond),
    ))
    .parse_next(input)
    .map_err(|_: ContextError| {
        *input = checkpoint;
        ContextError::new()
    })?;

    ws.parse_next(input)?;

    // Expect "between"
    if keyword("between").parse_next(input).is_err() {
        *input = checkpoint;
        return Err(ContextError::new());
    }

    ws.parse_next(input)?;

    // Parse low expression - use interval_operator_expression to stop before comparison operators
    // This ensures "months between A and B = 5" is parsed as "(months between A and B) = 5"
    let low = interval_operator_expression(input)?;

    ws.parse_next(input)?;

    // Expect "and"
    if keyword("and").parse_next(input).is_err() {
        *input = checkpoint;
        return Err(ContextError::new());
    }

    ws.parse_next(input)?;

    // Parse high expression - also use interval_operator_expression
    let high = interval_operator_expression(input)?;

    if is_difference {
        Ok(dummy_span(Expression::DifferenceBetween(
            DifferenceBetweenExpr {
                precision,
                low: Box::new(low),
                high: Box::new(high),
            },
        )))
    } else {
        Ok(dummy_span(Expression::DurationBetween(
            DurationBetweenExpr {
                precision,
                low: Box::new(low),
                high: Box::new(high),
            },
        )))
    }
}

/// Parse component extraction: year from X, month from X, etc.
fn component_extraction<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let checkpoint = *input;

    let component = alt((
        keyword("year").value(DateTimeComponent::Year),
        keyword("month").value(DateTimeComponent::Month),
        keyword("day").value(DateTimeComponent::Day),
        keyword("hour").value(DateTimeComponent::Hour),
        keyword("minute").value(DateTimeComponent::Minute),
        keyword("second").value(DateTimeComponent::Second),
        keyword("millisecond").value(DateTimeComponent::Millisecond),
        keyword("timezoneoffset").value(DateTimeComponent::TimezoneOffset),
        keyword("date").value(DateTimeComponent::Date),
        keyword("time").value(DateTimeComponent::Time),
    ))
    .parse_next(input)
    .inspect_err(|_| {
        *input = checkpoint;
    })?;

    ws.parse_next(input)?;

    // Must be followed by "from" for component extraction
    if keyword("from").parse_next(input).is_err() {
        *input = checkpoint;
        return Err(ContextError::new());
    }

    ws.parse_next(input)?;

    let source = unary_expression(input)?;

    Ok(dummy_span(Expression::DateTimeComponent(
        DateTimeComponentExpr {
            source: Box::new(source),
            component,
        },
    )))
}

/// Parse postfix expression (property access, indexer, as cast)
fn postfix_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let mut base = atom(input)?;

    loop {
        ws.parse_next(input)?;

        if lit(".").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            let prop = identifier_or_keyword_parser(input)?;
            ws.parse_next(input)?;

            // Check if this is a fluent function call: .method()
            if lit("(").parse_next(input).is_ok() {
                ws.parse_next(input)?;
                let args = if lit(")").parse_next(input).is_ok() {
                    vec![base]
                } else {
                    let mut args = vec![base];
                    loop {
                        let arg = expression_with_dummy_spans(input)?;
                        args.push(arg);
                        ws.parse_next(input)?;
                        if lit(",").parse_next(input).is_ok() {
                            ws.parse_next(input)?;
                        } else {
                            break;
                        }
                    }
                    lit(")").parse_next(input)?;
                    args
                };
                base = dummy_span(Expression::FunctionRef(FunctionRefExpr {
                    library: None,
                    name: prop,
                    arguments: args,
                }));
            } else {
                base = dummy_span(Expression::Property(PropertyAccess {
                    source: Box::new(base),
                    property: prop,
                }));
            }
            continue;
        }

        if lit("[").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            let index = expression_with_dummy_spans(input)?;
            ws.parse_next(input)?;
            lit("]").parse_next(input)?;
            base = dummy_span(Expression::Indexer(IndexerExpr {
                source: Box::new(base),
                index: Box::new(index),
            }));
            continue;
        }

        // Note: "is null", "is Type", and "as Type" are handled in type_expression()
        // at a higher precedence level to ensure proper parsing of expressions like
        // "hour from @2015-02-10T is null" as "(hour from @2015-02-10T) is null"

        break;
    }

    Ok(base)
}

/// Parse atom (highest precedence: literals, identifiers, parenthesized expressions)
fn atom<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    ws.parse_next(input)?;

    alt((
        keyword("null").map(|_| dummy_span(Expression::Literal(Literal::Null))),
        |input: &mut Input<'a>| {
            let b = boolean_parser(input)?;
            Ok(dummy_span(Expression::Literal(Literal::Boolean(b))))
        },
        |input: &mut Input<'a>| {
            let lit = temporal_literal_parser(input)?;
            Ok(dummy_span(Expression::Literal(lit)))
        },
        |input: &mut Input<'a>| {
            let checkpoint = *input;
            let q = quantity_literal_parser
                .parse_next(input)
                .map_err(|_: ContextError| {
                    *input = checkpoint;
                    ContextError::new()
                })?;
            if q.unit.is_none() {
                *input = checkpoint;
                return Err(ContextError::new());
            }
            Ok(dummy_span(Expression::Literal(Literal::Quantity(q))))
        },
        |input: &mut Input<'a>| {
            let lit = number_parser(input)?;
            Ok(dummy_span(Expression::Literal(lit)))
        },
        |input: &mut Input<'a>| {
            let s = string_parser(input)?;
            Ok(dummy_span(Expression::Literal(Literal::String(s))))
        },
        |input: &mut Input<'a>| if_expression(input),
        |input: &mut Input<'a>| case_expression(input),
        |input: &mut Input<'a>| convert_expression(input),
        |input: &mut Input<'a>| cast_expression(input),
        |input: &mut Input<'a>| interval_constructor(input),
        |input: &mut Input<'a>| retrieve_expression(input),
        |input: &mut Input<'a>| list_expression(input),
        |input: &mut Input<'a>| {
            lit("(").parse_next(input)?;
            ws.parse_next(input)?;
            let expr = expression_with_dummy_spans(input)?;
            ws.parse_next(input)?;
            lit(")").parse_next(input)?;
            Ok(dummy_span(expr.inner))
        },
        // Temporal constructors (keyword-named functions) - grouped to reduce alt count
        |input: &mut Input<'a>| {
            alt((
                |input: &mut Input<'a>| datetime_constructor(input),
                |input: &mut Input<'a>| date_constructor(input),
                |input: &mut Input<'a>| time_constructor(input),
                |input: &mut Input<'a>| now_function(input),
                |input: &mut Input<'a>| today_function(input),
                |input: &mut Input<'a>| timeofday_function(input),
            ))
            .parse_next(input)
        },
        // Tuple literal
        |input: &mut Input<'a>| tuple_expression(input),
        // Code / Concept selectors (`Code '1' from "CS"`, `Concept { ... }`)
        |input: &mut Input<'a>| selector_expression(input),
        // Regular identifiers and function calls (must come last)
        |input: &mut Input<'a>| identifier_or_function_call(input),
    ))
    .parse_next(input)
}

/// Parse a code selector body after the `Code` keyword:
/// `'<code>' from <codesystem> [display '<text>']`.
fn code_selector_body<'a>(input: &mut Input<'a>) -> PResult<CodeSelectorExpr> {
    ws.parse_next(input)?;
    let code = string_parser(input)?;
    padded_keyword("from").parse_next(input)?;
    let first = identifier_parser(input)?;
    let checkpoint = *input;
    let codesystem = if lit(".").parse_next(input).is_ok() {
        match identifier_or_keyword_parser(input) {
            Ok(name) => QualifiedIdentifier::qualified(first.name, name),
            Err(_) => {
                *input = checkpoint;
                QualifiedIdentifier::simple(first)
            }
        }
    } else {
        QualifiedIdentifier::simple(first)
    };
    let display = opt_display_clause(input)?;
    Ok(CodeSelectorExpr {
        code,
        codesystem,
        display,
    })
}

/// Parse an optional `display '<text>'` clause.
fn opt_display_clause<'a>(input: &mut Input<'a>) -> PResult<Option<String>> {
    let checkpoint = *input;
    if padded_keyword("display").parse_next(input).is_ok() {
        return string_parser(input).map(Some);
    }
    *input = checkpoint;
    Ok(None)
}

/// Whether the input continues with `Code '` (the start of a code selector).
fn at_code_selector(input: Input<'_>) -> bool {
    let mut probe = input;
    ws.parse_next(&mut probe).is_ok()
        && keyword("Code").parse_next(&mut probe).is_ok()
        && ws.parse_next(&mut probe).is_ok()
        && probe.starts_with('\'')
}

/// Parse a Code selector or a Concept selector of code selectors.
///
/// `Code { ... }` and `Concept { codes: ... }` instance selectors are left to
/// [`identifier_or_function_call`].
fn selector_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let checkpoint = *input;
    if at_code_selector(input) {
        ws.parse_next(input)?;
        keyword("Code").parse_next(input)?;
        let code = code_selector_body(input)?;
        return Ok(dummy_span(Expression::CodeSelector(code)));
    }

    if keyword("Concept").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        if lit("{").parse_next(input).is_ok() && at_code_selector(input) {
            let codes: Vec<CodeSelectorExpr> = separated(
                1..,
                |input: &mut Input<'a>| {
                    ws.parse_next(input)?;
                    keyword("Code").parse_next(input)?;
                    let code = code_selector_body(input)?;
                    ws.parse_next(input)?;
                    Ok(code)
                },
                ",",
            )
            .parse_next(input)?;
            ws.parse_next(input)?;
            lit("}").parse_next(input)?;
            let display = opt_display_clause(input)?;
            return Ok(dummy_span(Expression::ConceptSelector(
                ConceptSelectorExpr { codes, display },
            )));
        }
    }

    *input = checkpoint;
    Err(ContextError::new())
}

/// Parse if expression
fn if_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    padded_keyword("if").parse_next(input)?;
    let condition = expression_with_dummy_spans(input)?;
    padded_keyword("then").parse_next(input)?;
    let then_expr = expression_with_dummy_spans(input)?;
    padded_keyword("else").parse_next(input)?;
    let else_expr = expression_with_dummy_spans(input)?;

    Ok(dummy_span(Expression::If(IfExpr {
        condition: Box::new(condition),
        then_expr: Box::new(then_expr),
        else_expr: Box::new(else_expr),
    })))
}

/// Parse case expression
/// Two forms:
/// 1. Standard: case when <cond> then <result> ... else <else> end
/// 2. Selected: case <comparand> when <value> then <result> ... else <else> end
fn case_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    padded_keyword("case").parse_next(input)?;
    ws.parse_next(input)?;

    // Check for comparand (selected case form)
    let checkpoint = input.checkpoint();
    let comparand = if padded_keyword("when").parse_next(input).is_ok() {
        // No comparand - restore position for when clause parsing
        input.reset(&checkpoint);
        None
    } else {
        // Try to parse comparand
        input.reset(&checkpoint);
        let comp = expression_with_dummy_spans(input).ok();
        if comp.is_some() {
            ws.parse_next(input)?;
        }
        comp.map(Box::new)
    };

    // Parse case items (when-then pairs)
    let mut items = Vec::new();
    loop {
        ws.parse_next(input)?;
        if padded_keyword("when").parse_next(input).is_err() {
            break;
        }
        ws.parse_next(input)?;
        let when_expr = expression_with_dummy_spans(input)?;
        ws.parse_next(input)?;
        padded_keyword("then").parse_next(input)?;
        ws.parse_next(input)?;
        let then_expr = expression_with_dummy_spans(input)?;

        items.push(CaseItem {
            when: Box::new(when_expr),
            then: Box::new(then_expr),
        });
    }

    // Parse optional else clause
    ws.parse_next(input)?;
    let else_expr = if padded_keyword("else").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        let e = expression_with_dummy_spans(input)?;
        Some(Box::new(e))
    } else {
        None
    };

    ws.parse_next(input)?;
    padded_keyword("end").parse_next(input)?;

    Ok(dummy_span(Expression::Case(CaseExpr {
        comparand,
        items,
        else_expr,
    })))
}

/// Parse convert expression: convert <operand> to <type>
fn convert_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    padded_keyword("convert").parse_next(input)?;
    ws.parse_next(input)?;
    let operand = unary_expression(input)?;
    ws.parse_next(input)?;
    padded_keyword("to").parse_next(input)?;
    ws.parse_next(input)?;
    let type_name = identifier_or_keyword_parser(input)?;
    let type_spec = TypeSpecifier::Named(NamedTypeSpecifier {
        namespace: None,
        name: type_name.name.clone(),
    });

    Ok(dummy_span(Expression::Convert(ConvertExpr {
        operand: Box::new(operand),
        to_type: dummy_span(type_spec),
    })))
}

/// Parse cast expression: cast <operand> as <type>
fn cast_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    padded_keyword("cast").parse_next(input)?;
    ws.parse_next(input)?;
    let operand = unary_expression(input)?;
    ws.parse_next(input)?;
    padded_keyword("as").parse_next(input)?;
    ws.parse_next(input)?;
    let type_name = identifier_or_keyword_parser(input)?;
    let type_spec = TypeSpecifier::Named(NamedTypeSpecifier {
        namespace: None,
        name: type_name.name.clone(),
    });

    Ok(dummy_span(Expression::As(AsCastExpr {
        operand: Box::new(operand),
        as_type: dummy_span(type_spec),
        strict: true, // cast is strict
    })))
}

/// Parse interval constructor
fn interval_constructor<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    padded_keyword("Interval").parse_next(input)?;
    let low_closed = alt((lit("[").value(true), lit("(").value(false))).parse_next(input)?;
    ws.parse_next(input)?;
    let low = expression_with_dummy_spans(input)?;
    ws.parse_next(input)?;
    lit(",").parse_next(input)?;
    ws.parse_next(input)?;
    let high = expression_with_dummy_spans(input)?;
    ws.parse_next(input)?;
    let high_closed = alt((lit("]").value(true), lit(")").value(false))).parse_next(input)?;

    Ok(dummy_span(Expression::Interval(IntervalExpr {
        low: Some(Box::new(low)),
        low_closed,
        high: Some(Box::new(high)),
        high_closed,
    })))
}

/// Parse retrieve expression
fn retrieve_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    lit("[").parse_next(input)?;
    ws.parse_next(input)?;

    // Optional related context: `[Mother -> Observation]`
    let checkpoint = *input;
    let context = match identifier_parser(input) {
        Ok(ctx) => {
            ws.parse_next(input)?;
            if lit("->").parse_next(input).is_ok() {
                ws.parse_next(input)?;
                Some(ctx)
            } else {
                *input = checkpoint;
                None
            }
        }
        Err(_) => None,
    };

    // Named type, optionally model-qualified (`FHIR.Condition`)
    let first = identifier_or_keyword_parser(input)?;
    let type_spec = {
        let checkpoint = *input;
        if lit(".").parse_next(input).is_ok()
            && let Ok(name) = identifier_or_keyword_parser(input)
        {
            TypeSpecifier::Named(NamedTypeSpecifier {
                namespace: Some(first.name),
                name: name.name,
            })
        } else {
            *input = checkpoint;
            TypeSpecifier::Named(NamedTypeSpecifier {
                namespace: None,
                name: first.name,
            })
        }
    };
    ws.parse_next(input)?;

    let mut retrieve = Retrieve::new(dummy_span(type_spec));
    retrieve.context = context;

    // Optional terminology filter: `: "Value Set"` or `: code in "Value Set"`
    if lit(":").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        let checkpoint = *input;
        let path_and_comparator = (|| -> PResult<(String, CodeComparator)> {
            let mut path = identifier_or_keyword_parser(input)?.name;
            while lit(".").parse_next(input).is_ok() {
                path.push('.');
                path.push_str(&identifier_or_keyword_parser(input)?.name);
            }
            ws.parse_next(input)?;
            let comparator = alt((
                keyword("in").value(CodeComparator::In),
                lit("~").value(CodeComparator::Equivalent),
                lit("=").value(CodeComparator::Equal),
            ))
            .parse_next(input)?;
            Ok((path, comparator))
        })();
        match path_and_comparator {
            Ok((path, comparator)) => {
                retrieve.code_path = Some(path);
                retrieve.code_comparator = Some(comparator);
            }
            Err(_) => *input = checkpoint,
        }
        ws.parse_next(input)?;
        let terminology = inner_expression(input)?;
        retrieve.codes = Some(Box::new(terminology));
        ws.parse_next(input)?;
    }

    lit("]").parse_next(input)?;

    Ok(dummy_span(Expression::Retrieve(Box::new(retrieve))))
}

/// Parse list expression
fn list_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    lit("{").parse_next(input)?;
    ws.parse_next(input)?;

    // Use inner_expression to avoid query parsing for list elements
    let elements: Vec<Spanned<Expression>> =
        separated(0.., inner_expression, ",").parse_next(input)?;

    ws.parse_next(input)?;
    opt(",").parse_next(input)?;
    ws.parse_next(input)?;
    lit("}").parse_next(input)?;

    Ok(dummy_span(Expression::List(ListExpr {
        element_type: None,
        elements,
    })))
}

/// Parse identifier, qualified identifier, function call, or instance selector
fn identifier_or_function_call<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    // Use identifier_or_keyword_parser for the initial identifier since it might be a namespace
    // like "System" followed by a type that is also a keyword (e.g., "ValueSet", "Code")
    let mut id = identifier_or_keyword_parser
        .verify(|id: &Identifier| id.quoted || !is_structural_keyword(&id.name))
        .parse_next(input)?;
    ws.parse_next(input)?;

    // Check for qualified identifier (e.g., System.ValueSet)
    // Type names like ValueSet, Code, Concept can be keywords
    let mut namespace: Option<Identifier> = None;
    let checkpoint = *input;
    if lit(".").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        // Use identifier_or_keyword_parser since type names can be keywords
        if let Ok(second_id) = identifier_or_keyword_parser(input) {
            ws.parse_next(input)?;
            // Check if this is followed by { (instance selector) or ( (function call)
            // If not, revert - this is property access
            let next_checkpoint = *input;
            if lit("{").parse_next(input).is_ok() || lit("(").parse_next(input).is_ok() {
                *input = next_checkpoint;
                namespace = Some(id);
                id = second_id;
            } else {
                *input = checkpoint;
            }
        } else {
            *input = checkpoint;
        }
    }

    // Check for function call
    if lit("(").parse_next(input).is_ok() {
        ws.parse_next(input)?;

        // Check for empty argument list first
        let args = if lit(")").parse_next(input).is_ok() {
            Vec::new()
        } else {
            // Use inner_expression to avoid query parsing for function arguments
            let args: Vec<Spanned<Expression>> = separated(
                1..,
                |input: &mut Input<'a>| {
                    ws.parse_next(input)?;
                    let expr = expression_with_dummy_spans(input)?;
                    ws.parse_next(input)?;
                    Ok(expr)
                },
                ",",
            )
            .parse_next(input)?;
            ws.parse_next(input)?;
            opt(",").parse_next(input)?;
            ws.parse_next(input)?;
            lit(")").parse_next(input)?;
            args
        };

        Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
            library: namespace,
            name: id,
            arguments: args,
        })))
    // Check for instance selector (type selector): Type { field: value, ... }
    } else if lit("{").parse_next(input).is_ok() {
        ws.parse_next(input)?;

        // Parse instance elements
        // Note: Use identifier_or_keyword_parser since field names can be keywords like 'code'
        let mut elements = Vec::new();
        if lit("}").parse_next(input).is_err() {
            loop {
                ws.parse_next(input)?;
                let field_name = identifier_or_keyword_parser(input)?;
                ws.parse_next(input)?;
                lit(":").parse_next(input)?;
                ws.parse_next(input)?;
                let value = expression_with_dummy_spans(input)?;
                elements.push(InstanceElement {
                    name: field_name,
                    value: Box::new(value),
                });
                ws.parse_next(input)?;
                if lit(",").parse_next(input).is_ok() {
                    ws.parse_next(input)?;
                } else {
                    break;
                }
            }
            lit("}").parse_next(input)?;
        }

        let type_spec = TypeSpecifier::Named(NamedTypeSpecifier {
            namespace: namespace.map(|ns| ns.name),
            name: id.name,
        });

        Ok(dummy_span(Expression::Instance(InstanceExpr {
            class_type: dummy_span(type_spec),
            elements,
        })))
    } else {
        Ok(dummy_span(Expression::IdentifierRef(IdentifierRef {
            name: id,
        })))
    }
}

/// Parse DateTime constructor: DateTime(year, month?, day?, hour?, minute?, second?, millisecond?, timezone?)
fn datetime_constructor<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    keyword("DateTime").parse_next(input)?;
    ws.parse_next(input)?;
    lit("(").parse_next(input)?;
    ws.parse_next(input)?;

    let args: Vec<Spanned<Expression>> = separated(
        1..,
        |input: &mut Input<'a>| {
            ws.parse_next(input)?;
            let expr = expression_with_dummy_spans(input)?;
            ws.parse_next(input)?;
            Ok(expr)
        },
        ",",
    )
    .parse_next(input)?;

    ws.parse_next(input)?;
    lit(")").parse_next(input)?;

    let mut args_iter = args.into_iter();
    let year = args_iter.next().ok_or_else(ContextError::new)?;

    Ok(dummy_span(Expression::DateTime(DateTimeConstructorExpr {
        year: Box::new(year),
        month: args_iter.next().map(Box::new),
        day: args_iter.next().map(Box::new),
        hour: args_iter.next().map(Box::new),
        minute: args_iter.next().map(Box::new),
        second: args_iter.next().map(Box::new),
        millisecond: args_iter.next().map(Box::new),
        timezone_offset: args_iter.next().map(Box::new),
    })))
}

/// Parse Date constructor: Date(year, month?, day?)
fn date_constructor<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    keyword("Date").parse_next(input)?;
    ws.parse_next(input)?;
    lit("(").parse_next(input)?;
    ws.parse_next(input)?;

    let args: Vec<Spanned<Expression>> = separated(
        1..,
        |input: &mut Input<'a>| {
            ws.parse_next(input)?;
            let expr = expression_with_dummy_spans(input)?;
            ws.parse_next(input)?;
            Ok(expr)
        },
        ",",
    )
    .parse_next(input)?;

    ws.parse_next(input)?;
    lit(")").parse_next(input)?;

    let mut args_iter = args.into_iter();
    let year = args_iter.next().ok_or_else(ContextError::new)?;

    Ok(dummy_span(Expression::Date(DateConstructorExpr {
        year: Box::new(year),
        month: args_iter.next().map(Box::new),
        day: args_iter.next().map(Box::new),
    })))
}

/// Parse Time constructor: Time(hour, minute?, second?, millisecond?)
fn time_constructor<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    keyword("Time").parse_next(input)?;
    ws.parse_next(input)?;
    lit("(").parse_next(input)?;
    ws.parse_next(input)?;

    let args: Vec<Spanned<Expression>> = separated(
        1..,
        |input: &mut Input<'a>| {
            ws.parse_next(input)?;
            let expr = expression_with_dummy_spans(input)?;
            ws.parse_next(input)?;
            Ok(expr)
        },
        ",",
    )
    .parse_next(input)?;

    ws.parse_next(input)?;
    lit(")").parse_next(input)?;

    let mut args_iter = args.into_iter();
    let hour = args_iter.next().ok_or_else(ContextError::new)?;

    Ok(dummy_span(Expression::Time(TimeConstructorExpr {
        hour: Box::new(hour),
        minute: args_iter.next().map(Box::new),
        second: args_iter.next().map(Box::new),
        millisecond: args_iter.next().map(Box::new),
    })))
}

/// Parse Now() function
fn now_function<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    keyword("Now").parse_next(input)?;
    ws.parse_next(input)?;
    lit("(").parse_next(input)?;
    ws.parse_next(input)?;
    lit(")").parse_next(input)?;
    Ok(dummy_span(Expression::Now))
}

/// Parse Today() function
fn today_function<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    keyword("Today").parse_next(input)?;
    ws.parse_next(input)?;
    lit("(").parse_next(input)?;
    ws.parse_next(input)?;
    lit(")").parse_next(input)?;
    Ok(dummy_span(Expression::Today))
}

/// Parse TimeOfDay() function
fn timeofday_function<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    keyword("TimeOfDay").parse_next(input)?;
    ws.parse_next(input)?;
    lit("(").parse_next(input)?;
    ws.parse_next(input)?;
    lit(")").parse_next(input)?;
    Ok(dummy_span(Expression::TimeOfDay))
}

/// Parse Tuple literal: Tuple { name : value, ... }
fn tuple_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    keyword("Tuple").parse_next(input)?;
    ws.parse_next(input)?;
    lit("{").parse_next(input)?;
    ws.parse_next(input)?;

    // Parse tuple elements - use identifier_or_keyword_parser to allow keywords as element names
    let elements: Vec<TupleElement> = separated(
        0..,
        |input: &mut Input<'a>| {
            ws.parse_next(input)?;
            let name = identifier_or_keyword_parser(input)?;
            ws.parse_next(input)?;
            lit(":").parse_next(input)?;
            ws.parse_next(input)?;
            let value = expression_with_dummy_spans(input)?;
            ws.parse_next(input)?;
            Ok(TupleElement {
                name,
                value: Box::new(value),
            })
        },
        ",",
    )
    .parse_next(input)?;

    ws.parse_next(input)?;
    opt(",").parse_next(input)?;
    ws.parse_next(input)?;
    lit("}").parse_next(input)?;

    Ok(dummy_span(Expression::Tuple(TupleExpr { elements })))
}

/// Parse a query expression
/// Handles two main forms:
/// 1. `from <source>, <source>, ... [clauses]` - multi-source query
/// 2. `<expr> <alias> [clauses]` - single-source query (e.g., `({1,2,3}) L sort desc`)
fn query_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let checkpoint = *input;

    // Try "from" keyword for multi-source queries
    if padded_keyword("from").parse_next(input).is_ok() {
        // Parse multiple aliased sources separated by commas
        let sources = parse_query_sources(input)?;
        let query = parse_query_clauses(sources, input)?;
        return Ok(dummy_span(Expression::Query(Box::new(query))));
    }

    // Reset and try single-source query pattern: <queryable-expr> <alias> [clauses]
    *input = checkpoint;

    // Parse a queryable expression (list, parenthesized expr, or retrieve)
    let source_expr = query_source_expression(input)?;

    ws.parse_next(input)?;

    // Must be followed by an alias identifier (not a keyword or operator)
    let alias = query_alias(input).map_err(|_| {
        *input = checkpoint;
        ContextError::new()
    })?;

    // Now we have <expr> <alias>, parse optional query clauses
    let source = QuerySource::new(Box::new(source_expr), alias);
    let query = parse_query_clauses(vec![source], input)?;

    Ok(dummy_span(Expression::Query(Box::new(query))))
}

/// Parse a queryable source expression (list, parenthesized expr, or retrieve)
fn query_source_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    ws.parse_next(input)?;

    // Parenthesized expression (most common for inline queries like `(4) l`)
    if lit("(").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        let expr = implies_expression(input)?;
        ws.parse_next(input)?;
        lit(")").parse_next(input)?;
        return Ok(dummy_span(expr.inner));
    }

    // List expression
    if let Ok(list) = list_expression(input) {
        return Ok(list);
    }

    // Retrieve expression
    if let Ok(retrieve) = retrieve_expression(input) {
        return Ok(retrieve);
    }

    // Qualified identifier expression: `"Def"`, `Lib."Def"`, `Alias.path.to`
    qualified_identifier_expression(input)
}

/// Keywords that structure a library or a query and can never start an
/// operand, so `define A: 1 +` fails at the dangling operator instead of
/// swallowing the next `define`.
fn is_structural_keyword(word: &str) -> bool {
    matches!(
        word,
        "library"
            | "using"
            | "include"
            | "called"
            | "parameter"
            | "default"
            | "codesystem"
            | "codesystems"
            | "valueset"
            | "context"
            | "define"
            | "function"
            | "fluent"
            | "public"
            | "private"
            | "returns"
            | "external"
            | "where"
            | "return"
            | "sort"
            | "such"
            | "that"
            | "with"
            | "without"
            | "then"
            | "else"
            | "when"
            | "and"
            | "or"
            | "xor"
            | "implies"
            | "between"
    )
}

/// Unquoted words that begin an expression construct (`minimum Integer`,
/// `predecessor of x`, `year from x`, `duration in days between ...`) or
/// continue one (`of`, `per`), and so can never be a query source or alias.
fn opens_expression_construct(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "minimum"
            | "maximum"
            | "predecessor"
            | "successor"
            | "expand"
            | "duration"
            | "difference"
            | "of"
            | "per"
            | "year"
            | "years"
            | "month"
            | "months"
            | "week"
            | "weeks"
            | "day"
            | "days"
            | "hour"
            | "hours"
            | "minute"
            | "minutes"
            | "second"
            | "seconds"
            | "millisecond"
            | "milliseconds"
            | "timezone"
            | "timezoneoffset"
            | "occurs"
    )
}

/// Parse a query alias: an identifier that is not a keyword and does not
/// open an expression construct.
fn query_alias<'a>(input: &mut Input<'a>) -> PResult<Identifier> {
    identifier_parser
        .verify(|id: &Identifier| id.quoted || !opens_expression_construct(&id.name))
        .parse_next(input)
}

/// Parse a (possibly qualified) identifier path used as a query source:
/// `"Qualifying Encounters"`, `Common."Active Conditions"`, `Patient.name`.
///
/// The first segment must not be a keyword, so expressions such as
/// `exists "X"` are never mistaken for an aliased query source.
fn qualified_identifier_expression<'a>(input: &mut Input<'a>) -> PResult<Spanned<Expression>> {
    let first = identifier_parser
        .verify(|id: &Identifier| id.quoted || !opens_expression_construct(&id.name))
        .parse_next(input)?;
    let mut expr = dummy_span(Expression::IdentifierRef(IdentifierRef { name: first }));
    loop {
        let checkpoint = *input;
        ws.parse_next(input)?;
        if lit(".").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            if let Ok(property) = identifier_or_keyword_parser(input) {
                expr = dummy_span(Expression::Property(PropertyAccess {
                    source: Box::new(expr),
                    property,
                }));
                continue;
            }
        }
        *input = checkpoint;
        break;
    }
    Ok(expr)
}

/// Parse comma-separated query sources for multi-source queries
fn parse_query_sources<'a>(input: &mut Input<'a>) -> PResult<Vec<QuerySource>> {
    let sources: Vec<QuerySource> = separated(
        1..,
        |input: &mut Input<'a>| {
            ws.parse_next(input)?;
            let expr = query_source_expression(input)?;
            ws.parse_next(input)?;
            let alias = query_alias(input)?;
            ws.parse_next(input)?;
            Ok(QuerySource::new(Box::new(expr), alias))
        },
        ",",
    )
    .parse_next(input)?;

    Ok(sources)
}

/// Parse query clauses (where, return, sort, aggregate, let, with, without)
fn parse_query_clauses<'a>(sources: Vec<QuerySource>, input: &mut Input<'a>) -> PResult<Query> {
    let mut query = if sources.len() == 1 {
        Query::new(sources.into_iter().next().unwrap())
    } else {
        Query::multi(sources)
    };

    loop {
        ws.parse_next(input)?;

        // Parse let clause: let a: expr, b: expr
        if padded_keyword("let").parse_next(input).is_ok() {
            let lets: Vec<LetClause> = separated(
                1..,
                |input: &mut Input<'a>| {
                    ws.parse_next(input)?;
                    let identifier = identifier_parser(input)?;
                    ws.parse_next(input)?;
                    lit(":").parse_next(input)?;
                    let expression = expression_with_dummy_spans(input)?;
                    ws.parse_next(input)?;
                    Ok(LetClause::new(identifier, Box::new(expression)))
                },
                ",",
            )
            .parse_next(input)?;
            query.lets.extend(lets);
            continue;
        }

        // Parse inclusion clauses: with/without <source> <alias> such that <expr>
        let relationship = if padded_keyword("with").parse_next(input).is_ok() {
            Some(RelationshipKind::With)
        } else if padded_keyword("without").parse_next(input).is_ok() {
            Some(RelationshipKind::Without)
        } else {
            None
        };
        if let Some(kind) = relationship {
            let source = query_source_expression(input)?;
            ws.parse_next(input)?;
            let alias = query_alias(input)?;
            padded_keyword("such").parse_next(input)?;
            padded_keyword("that").parse_next(input)?;
            let condition = implies_expression(input)?;
            query.relationships.push(RelationshipClause {
                kind,
                source: QuerySource::new(Box::new(source), alias),
                such_that: Some(Box::new(condition)),
            });
            continue;
        }

        // Parse return clause
        if padded_keyword("return").parse_next(input).is_ok() {
            let distinct = padded_keyword("distinct").parse_next(input).is_ok();
            let all = if !distinct {
                padded_keyword("all").parse_next(input).is_ok()
            } else {
                false
            };
            let expr = implies_expression(input)?;
            query.return_clause = Some(ReturnClause {
                distinct,
                all,
                expression: Box::new(expr),
            });
            continue;
        }

        // Parse sort clause: sort [by <expr>] (asc|ascending|desc|descending)
        if padded_keyword("sort").parse_next(input).is_ok() {
            ws.parse_next(input)?;

            // Check for "by" keyword for sort by expression
            let has_by = padded_keyword("by").parse_next(input).is_ok();

            if has_by {
                // Sort by specific expression(s)
                let items: Vec<SortItem> = separated(
                    1..,
                    |input: &mut Input<'a>| {
                        ws.parse_next(input)?;
                        let expr = implies_expression(input)?;
                        ws.parse_next(input)?;
                        let direction =
                            parse_sort_direction(input).unwrap_or(SortDirection::Ascending);
                        Ok(SortItem::new(Some(Box::new(expr)), direction))
                    },
                    ",",
                )
                .parse_next(input)?;
                query.sort_clause = Some(SortClause::new(items));
            } else {
                // Simple sort without "by" - just direction (sort asc, sort desc)
                let direction = parse_sort_direction(input).unwrap_or(SortDirection::Ascending);
                query.sort_clause = Some(SortClause::single(SortItem::new(None, direction)));
            }
            continue;
        }

        // Parse aggregate clause: aggregate [distinct|all] <id> [starting <expr>]: <expr>
        if padded_keyword("aggregate").parse_next(input).is_ok() {
            ws.parse_next(input)?;
            let distinct = padded_keyword("distinct").parse_next(input).is_ok();
            let _all = if !distinct {
                padded_keyword("all").parse_next(input).is_ok()
            } else {
                false
            };
            ws.parse_next(input)?;
            let identifier = identifier_parser(input)?;
            ws.parse_next(input)?;

            // Optional starting clause
            let starting = if padded_keyword("starting").parse_next(input).is_ok() {
                ws.parse_next(input)?;
                let expr = implies_expression(input)?;
                Some(Box::new(expr))
            } else {
                None
            };

            ws.parse_next(input)?;
            lit(":").parse_next(input)?;
            ws.parse_next(input)?;
            let expression = implies_expression(input)?;

            query.aggregate_clause = Some(AggregateClause {
                distinct,
                identifier,
                starting,
                lets: Vec::new(),
                expression: Box::new(expression),
            });
            continue;
        }

        // Parse where clause
        if padded_keyword("where").parse_next(input).is_ok() {
            let expr = implies_expression(input)?;
            query.where_clause = Some(Box::new(expr));
            continue;
        }

        break;
    }

    Ok(query)
}

/// Parse sort direction (asc, ascending, desc, descending)
fn parse_sort_direction<'a>(input: &mut Input<'a>) -> Option<SortDirection> {
    ws.parse_next(input).ok()?;

    if keyword("desc").parse_next(input).is_ok() || keyword("descending").parse_next(input).is_ok()
    {
        Some(SortDirection::Desc)
    } else if keyword("asc").parse_next(input).is_ok()
        || keyword("ascending").parse_next(input).is_ok()
    {
        Some(SortDirection::Asc)
    } else {
        None
    }
}
