//! Expression parser using recursive descent with precedence climbing
//!
//! This parser uses a simple precedence climbing approach instead of Pratt parsing
//! to avoid complex symbol names that can cause macOS linker issues.

use crate::combinators::{
    boolean_parser, identifier_or_keyword_parser, identifier_parser, keyword, lit, number_parser,
    padded_keyword, quantity_literal_parser, string_parser, temporal_literal_parser, ws, Input,
    PResult,
};
use octofhir_cql_ast::{
    AggregateClause, AsCastExpr, BetweenExpr, BinaryOp, BinaryOpExpr, CaseExpr, CaseItem, ConvertExpr,
    DateConstructorExpr, DateTimeComponent, DateTimeComponentExpr, DateTimeConstructorExpr,
    DifferenceBetweenExpr, DurationBetweenExpr, Expression, FunctionRefExpr, Identifier,
    IdentifierRef, IfExpr, IndexerExpr, InstanceElement, InstanceExpr, IntervalExpr, IntervalOp,
    IntervalOpExpr, IntervalTypeSpecifier, IsNullExpr, IsTypeExpr, ListExpr, ListTypeSpecifier,
    Literal, MinMaxValueExpr, NamedTypeSpecifier, PropertyAccess, QuantityLiteral, Query,
    QuerySource, Retrieve, ReturnClause, SameAsExpr, SameOrAfterExpr, SameOrBeforeExpr, SortClause,
    SortDirection, SortItem, Spanned, TemporalPrecision, TimeConstructorExpr, TupleElement,
    TupleExpr, TypeSpecifier, UnaryOp, UnaryOpExpr,
};
use rust_decimal::Decimal;
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
        || input.starts_with('[');

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

/// Parse a type specifier, handling generics like List<String>, Interval<Date>
fn type_specifier_parser<'a>(input: &mut Input<'a>) -> PResult<TypeSpecifier> {
    // Get the base type name (could be qualified like System.String)
    let type_name = identifier_or_keyword_parser(input)?;
    let name = type_name.name.clone();

    ws.parse_next(input)?;

    // Check for generic type parameter
    if lit("<").parse_next(input).is_ok() {
        ws.parse_next(input)?;
        // Parse the type argument
        let inner_type = type_specifier_parser(input)?;
        ws.parse_next(input)?;
        lit(">").parse_next(input)?;

        // Determine if this is List or Interval based on name
        let lower_name = name.to_lowercase();
        if lower_name == "list" {
            return Ok(TypeSpecifier::List(ListTypeSpecifier::new(inner_type)));
        } else if lower_name == "interval" {
            return Ok(TypeSpecifier::Interval(IntervalTypeSpecifier::new(inner_type)));
        }
        // For other generic types, just use the base name (could be extended later)
    }

    // Check for qualified name (e.g., System.String)
    if lit(".").parse_next(input).is_ok() {
        let inner_name = identifier_or_keyword_parser(input)?;
        return Ok(TypeSpecifier::Named(NamedTypeSpecifier {
            namespace: Some(name),
            name: inner_name.name,
        }));
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
            Some(IntervalOp::Starts)
        } else if padded_keyword("ends").parse_next(input).is_ok() {
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
    if padded_keyword("start").parse_next(input).is_ok() {
        if padded_keyword("of").parse_next(input).is_ok() {
            let operand = unary_expression(input)?;
            return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
                library: None,
                name: Identifier::new("Start"),
                arguments: vec![operand],
            })));
        }
    }

    if padded_keyword("end").parse_next(input).is_ok() {
        if padded_keyword("of").parse_next(input).is_ok() {
            let operand = unary_expression(input)?;
            return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
                library: None,
                name: Identifier::new("End"),
                arguments: vec![operand],
            })));
        }
    }

    if padded_keyword("width").parse_next(input).is_ok() {
        if padded_keyword("of").parse_next(input).is_ok() {
            let operand = unary_expression(input)?;
            return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
                library: None,
                name: Identifier::new("Width"),
                arguments: vec![operand],
            })));
        }
    }

    // singleton from - extracts single element from list
    if padded_keyword("singleton").parse_next(input).is_ok() {
        if padded_keyword("from").parse_next(input).is_ok() {
            let operand = unary_expression(input)?;
            return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
                op: UnaryOp::SingletonFrom,
                operand: Box::new(operand),
            })));
        }
    }

    // point from - extracts point from unit interval
    if padded_keyword("point").parse_next(input).is_ok() {
        if padded_keyword("from").parse_next(input).is_ok() {
            let operand = unary_expression(input)?;
            return Ok(dummy_span(Expression::FunctionRef(FunctionRefExpr {
                library: None,
                name: Identifier::new("PointFrom"),
                arguments: vec![operand],
            })));
        }
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
    if padded_keyword("predecessor").parse_next(input).is_ok() {
        if padded_keyword("of").parse_next(input).is_ok() {
            let operand = unary_expression(input)?;
            return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
                op: UnaryOp::Predecessor,
                operand: Box::new(operand),
            })));
        }
    }

    // successor of X
    if padded_keyword("successor").parse_next(input).is_ok() {
        if padded_keyword("of").parse_next(input).is_ok() {
            let operand = unary_expression(input)?;
            return Ok(dummy_span(Expression::UnaryOp(UnaryOpExpr {
                op: UnaryOp::Successor,
                operand: Box::new(operand),
            })));
        }
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
                    QuantityLiteral::new(Decimal::ONE).with_unit(unit)
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

    if keyword("millisecond").parse_next(input).is_ok() || keyword("milliseconds").parse_next(input).is_ok() {
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
        Ok(dummy_span(Expression::DurationBetween(DurationBetweenExpr {
            precision,
            low: Box::new(low),
            high: Box::new(high),
        })))
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
    .map_err(|e: ContextError| {
        *input = checkpoint;
        e
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
            let prop = identifier_parser(input)?;
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
            let q = quantity_literal_parser.parse_next(input).map_err(|_: ContextError| {
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
        // Regular identifiers and function calls (must come last)
        |input: &mut Input<'a>| identifier_or_function_call(input),
    ))
    .parse_next(input)
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
    let type_name = identifier_parser(input)?;
    ws.parse_next(input)?;
    lit("]").parse_next(input)?;

    let type_spec = TypeSpecifier::Named(NamedTypeSpecifier {
        namespace: None,
        name: type_name.name.clone(),
    });

    Ok(dummy_span(Expression::Retrieve(Box::new(Retrieve::new(
        dummy_span(type_spec),
    )))))
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
    let mut id = identifier_or_keyword_parser(input)?;
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
                    let expr = inner_expression(input)?;
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
    let alias = identifier_parser(input).map_err(|_| {
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

    Err(ContextError::new())
}

/// Parse comma-separated query sources for multi-source queries
fn parse_query_sources<'a>(input: &mut Input<'a>) -> PResult<Vec<QuerySource>> {
    let sources: Vec<QuerySource> = separated(
        1..,
        |input: &mut Input<'a>| {
            ws.parse_next(input)?;
            let expr = query_source_expression(input)?;
            ws.parse_next(input)?;
            let alias = identifier_parser(input)?;
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
                        let direction = parse_sort_direction(input).unwrap_or(SortDirection::Ascending);
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

