//! Expression parser using Pratt parsing for operator precedence

use chumsky::extra;
use chumsky::pratt::{infix, left, prefix, right};
use chumsky::prelude::*;

use crate::combinators::{
    boolean_parser, identifier_parser, number_parser, quantity_literal_parser,
    ratio_literal_parser, string_parser, temporal_literal_parser,
};
use octofhir_cql_ast::{
    AsCastExpr, BinaryOp, BinaryOpExpr, Expression, FunctionRefExpr, Identifier, IdentifierRef,
    IfExpr, IndexerExpr, IntervalExpr, IntervalOp, IntervalOpExpr, ListExpr, Literal,
    NamedTypeSpecifier, PropertyAccess, Retrieve, Spanned, TupleElement, TupleExpr, TypeSpecifier,
    UnaryOp, UnaryOpExpr,
};
use octofhir_cql_diagnostics::Span;

/// Create an error expression for recovery
fn error_expr(span: Span) -> Spanned<Expression> {
    Spanned::new(Expression::Error, span)
}

/// Helper to create span from chumsky's SimpleSpan
fn make_span(s: SimpleSpan<usize>) -> Span {
    Span::new(s.start, s.end)
}

/// Postfix operation type
enum PostfixOp {
    Property(Identifier),
    Indexer(Spanned<Expression>),
    AsCast(Identifier),
}

/// Parse a CQL expression
pub fn expression_parser<'a>(
) -> impl Parser<'a, &'a str, Spanned<Expression>, extra::Err<Rich<'a, char>>> + Clone {
    recursive(|expr| {
        // Retrieve expression: [TypeName] with recovery
        let retrieve = identifier_parser()
            .delimited_by(just('[').padded(), just(']').padded())
            .map_with(|type_name, e| {
                let span = make_span(e.span());
                let type_spec = TypeSpecifier::Named(NamedTypeSpecifier {
                    namespace: None,
                    name: type_name.name.clone(),
                });
                Spanned::new(
                    Expression::Retrieve(Box::new(Retrieve::new(Spanned::new(type_spec, span)))),
                    span,
                )
            })
            .recover_with(via_parser(
                any()
                    .and_is(just(']').not())
                    .repeated()
                    .delimited_by(just('['), just(']'))
                    .map_with(|_, e| error_expr(make_span(e.span()))),
            ));

        // Interval constructor: Interval[low, high], Interval(low, high), etc.
        // Brackets mean closed (inclusive), parentheses mean open (exclusive)
        let interval = text::keyword("Interval")
            .padded()
            .ignore_then(choice((
                // [low, high] - closed-closed
                just('[')
                    .ignore_then(expr.clone().padded())
                    .then_ignore(just(',').padded())
                    .then(expr.clone().padded())
                    .then_ignore(just(']'))
                    .map(|(low, high)| (low, true, high, true)),
                // [low, high) - closed-open
                just('[')
                    .ignore_then(expr.clone().padded())
                    .then_ignore(just(',').padded())
                    .then(expr.clone().padded())
                    .then_ignore(just(')'))
                    .map(|(low, high)| (low, true, high, false)),
                // (low, high] - open-closed
                just('(')
                    .ignore_then(expr.clone().padded())
                    .then_ignore(just(',').padded())
                    .then(expr.clone().padded())
                    .then_ignore(just(']'))
                    .map(|(low, high)| (low, false, high, true)),
                // (low, high) - open-open
                just('(')
                    .ignore_then(expr.clone().padded())
                    .then_ignore(just(',').padded())
                    .then(expr.clone().padded())
                    .then_ignore(just(')'))
                    .map(|(low, high)| (low, false, high, false)),
            )))
            .map_with(|(low, low_closed, high, high_closed), e| {
                let span = make_span(e.span());
                Spanned::new(
                    Expression::Interval(IntervalExpr {
                        low: Some(Box::new(low)),
                        low_closed,
                        high: Some(Box::new(high)),
                        high_closed,
                    }),
                    span,
                )
            });

        // Tuple element: name : value
        let tuple_element = identifier_parser()
            .padded()
            .then_ignore(just(':').padded())
            .then(expr.clone())
            .map(|(name, value)| TupleElement {
                name,
                value: Box::new(value),
            });

        // Tuple constructor: Tuple { name: value, ... }
        let tuple = text::keyword("Tuple")
            .padded()
            .ignore_then(
                tuple_element
                    .separated_by(just(',').padded())
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just('{').padded(), just('}').padded()),
            )
            .map_with(|elements, e| {
                let span = make_span(e.span());
                Spanned::new(Expression::Tuple(TupleExpr { elements }), span)
            });

        // List expression: { expr, expr, ... } with recovery
        let list = expr
            .clone()
            .separated_by(just(',').padded())
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just('{').padded(), just('}').padded())
            .map_with(|elements, e| {
                let span = make_span(e.span());
                Spanned::new(
                    Expression::List(ListExpr {
                        element_type: None,
                        elements,
                    }),
                    span,
                )
            })
            .recover_with(via_parser(
                any()
                    .and_is(just('}').not())
                    .repeated()
                    .delimited_by(just('{'), just('}'))
                    .map_with(|_, e| {
                        let span = make_span(e.span());
                        Spanned::new(
                            Expression::List(ListExpr {
                                element_type: None,
                                elements: vec![],
                            }),
                            span,
                        )
                    }),
            ));

        // Atom parsers - literals, identifiers, parenthesized expressions
        let atom = choice((
            // Null literal
            text::keyword("null").map_with(|_, e| {
                let span = make_span(e.span());
                Spanned::new(Expression::Literal(Literal::Null), span)
            }),
            // Boolean literals
            boolean_parser().map_with(|b, e| {
                let span = make_span(e.span());
                Spanned::new(Expression::Literal(Literal::Boolean(b)), span)
            }),
            // Temporal literals (@YYYY-MM-DD, @YYYY-MM-DDThh:mm:ss, @Thh:mm:ss)
            temporal_literal_parser().map_with(|lit, e| {
                let span = make_span(e.span());
                Spanned::new(Expression::Literal(lit), span)
            }),
            // Ratio literal (must be before quantity - number 'unit':number 'unit')
            ratio_literal_parser().map_with(|ratio, e| {
                let span = make_span(e.span());
                Spanned::new(Expression::Literal(Literal::Ratio(ratio)), span)
            }),
            // Quantity literal (number with unit - must check for unit)
            quantity_literal_parser()
                .try_map(|q, span| {
                    // Only accept as quantity if it has a unit
                    if q.unit.is_some() {
                        Ok(q)
                    } else {
                        Err(Rich::custom(span, "expected unit for quantity"))
                    }
                })
                .map_with(|q, e| {
                    let span = make_span(e.span());
                    Spanned::new(Expression::Literal(Literal::Quantity(q)), span)
                }),
            // Number literals
            number_parser().map_with(|lit, e| {
                let span = make_span(e.span());
                Spanned::new(Expression::Literal(lit), span)
            }),
            // String literals
            string_parser().map_with(|s, e| {
                let span = make_span(e.span());
                Spanned::new(Expression::Literal(Literal::String(s)), span)
            }),
            // If-then-else expression
            text::keyword("if")
                .padded()
                .ignore_then(expr.clone())
                .then_ignore(text::keyword("then").padded())
                .then(expr.clone())
                .then_ignore(text::keyword("else").padded())
                .then(expr.clone())
                .map_with(|((cond, then_expr), else_expr), e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::If(IfExpr {
                            condition: Box::new(cond),
                            then_expr: Box::new(then_expr),
                            else_expr: Box::new(else_expr),
                        }),
                        span,
                    )
                }),
            // Interval constructor - must be before identifier parser
            interval,
            // Tuple constructor - must be before identifier parser
            tuple,
            // Retrieve expression
            retrieve,
            // List expression
            list,
            // Parenthesized expression with recovery
            expr.clone()
                .delimited_by(just('(').padded(), just(')').padded())
                .recover_with(via_parser(
                    any()
                        .and_is(just(')').not())
                        .repeated()
                        .delimited_by(just('('), just(')'))
                        .map_with(|_, e| error_expr(make_span(e.span()))),
                )),
            // Identifier with optional function call: name or name(args)
            identifier_parser()
                .then(
                    expr.clone()
                        .separated_by(just(',').padded())
                        .allow_trailing()
                        .collect::<Vec<_>>()
                        .delimited_by(just('(').padded(), just(')').padded())
                        .or_not(),
                )
                .map_with(|(id, args), e| {
                    let span = make_span(e.span());
                    match args {
                        Some(arguments) => Spanned::new(
                            Expression::FunctionRef(FunctionRefExpr {
                                library: None,
                                name: id,
                                arguments,
                            }),
                            span,
                        ),
                        None => Spanned::new(
                            Expression::IdentifierRef(IdentifierRef { name: id }),
                            span,
                        ),
                    }
                }),
        ));

        // Postfix operations: property access (.name), indexer ([expr]), type cast (as Type)
        let postfix = atom.foldl(
            choice((
                // Property access: .name
                just('.')
                    .padded()
                    .ignore_then(identifier_parser())
                    .map(|id| PostfixOp::Property(id)),
                // Indexer: [expr]
                expr.clone()
                    .delimited_by(just('[').padded(), just(']').padded())
                    .map(|idx| PostfixOp::Indexer(idx)),
                // Type cast: as Type
                text::keyword("as")
                    .padded()
                    .ignore_then(identifier_parser())
                    .map(|type_name| PostfixOp::AsCast(type_name)),
            ))
            .repeated(),
            |base, op| {
                let base_span = base.span;
                match op {
                    PostfixOp::Property(prop) => {
                        let new_span = Span::new(base_span.start, base_span.end + prop.name.len());
                        Spanned::new(
                            Expression::Property(PropertyAccess {
                                source: Box::new(base),
                                property: prop,
                            }),
                            new_span,
                        )
                    }
                    PostfixOp::Indexer(index) => {
                        let new_span = Span::new(base_span.start, index.span.end);
                        Spanned::new(
                            Expression::Indexer(IndexerExpr {
                                source: Box::new(base),
                                index: Box::new(index),
                            }),
                            new_span,
                        )
                    }
                    PostfixOp::AsCast(type_name) => {
                        let new_span = Span::new(base_span.start, base_span.end + type_name.name.len() + 4);
                        let type_spec = TypeSpecifier::Named(NamedTypeSpecifier {
                            namespace: None,
                            name: type_name.name.clone(),
                        });
                        Spanned::new(
                            Expression::As(AsCastExpr {
                                operand: Box::new(base),
                                as_type: Spanned::new(type_spec, new_span),
                                strict: false,
                            }),
                            new_span,
                        )
                    }
                }
            },
        );

        // Layer 1: Unary operators (highest precedence)
        let with_unary = postfix.pratt((
            prefix(12, just('-').padded(), |_, operand: Spanned<Expression>, e| {
                let span = make_span(e.span());
                Spanned::new(
                    Expression::UnaryOp(UnaryOpExpr {
                        op: UnaryOp::Negate,
                        operand: Box::new(operand),
                    }),
                    span,
                )
            }),
            prefix(12, just('+').padded(), |_, operand: Spanned<Expression>, e| {
                let span = make_span(e.span());
                Spanned::new(
                    Expression::UnaryOp(UnaryOpExpr {
                        op: UnaryOp::Plus,
                        operand: Box::new(operand),
                    }),
                    span,
                )
            }),
            prefix(
                12,
                text::keyword("not").padded(),
                |_, operand: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::UnaryOp(UnaryOpExpr {
                            op: UnaryOp::Not,
                            operand: Box::new(operand),
                        }),
                        span,
                    )
                },
            ),
            prefix(
                12,
                text::keyword("exists").padded(),
                |_, operand: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::UnaryOp(UnaryOpExpr {
                            op: UnaryOp::Exists,
                            operand: Box::new(operand),
                        }),
                        span,
                    )
                },
            ),
        ));

        // Layer 2: High precedence binary operators (multiplicative, power)
        let with_high = with_unary.pratt((
            // Power - precedence 11, right-associative
            infix(
                right(11),
                just('^').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Power,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            // Multiplicative - precedence 10
            infix(
                left(10),
                just('*').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Multiply,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(10),
                just('/').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Divide,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(10),
                text::keyword("div").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::TruncatedDivide,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(10),
                text::keyword("mod").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Modulo,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
        ));

        // Layer 3: Additive and concatenation operators
        let with_additive = with_high.pratt((
            infix(
                left(9),
                just('+').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Add,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(9),
                just('-').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Subtract,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(9),
                just('&').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Concatenate,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
        ));

        // Layer 4: Comparison and equality operators
        let with_comparison = with_additive.pratt((
            // Union - precedence 7
            infix(
                left(7),
                just('|').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Union,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            // Relational - precedence 6
            infix(
                left(6),
                just("<=").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::LessOrEqual,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                just(">=").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::GreaterOrEqual,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                just('<').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Less,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                just('>').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Greater,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            // Equality - precedence 5
            infix(
                left(5),
                just('=').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Equal,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(5),
                just("!=").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::NotEqual,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(5),
                just("!~").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::NotEquivalent,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(5),
                just('~').padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Equivalent,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            // Interval operators - precedence 6
            infix(
                left(6),
                text::keyword("after").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::After,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                text::keyword("before").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::Before,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                text::keyword("meets").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::Meets,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                text::keyword("overlaps").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::Overlaps,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                text::keyword("starts").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::Starts,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                text::keyword("ends").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::Ends,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                text::keyword("during").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::During,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(6),
                text::keyword("includes").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::IntervalOp(IntervalOpExpr {
                            left: Box::new(left),
                            op: IntervalOp::Includes,
                            right: Box::new(right),
                            precision: None,
                        }),
                        span,
                    )
                },
            ),
        ));

        // Layer 5: Membership and logical operators
        with_comparison.pratt((
            // Membership - precedence 4
            infix(
                left(4),
                text::keyword("in").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::In,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(4),
                text::keyword("contains").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Contains,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            // Logical AND - precedence 3
            infix(
                left(3),
                text::keyword("and").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::And,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            // Logical XOR and OR - precedence 2
            infix(
                left(2),
                text::keyword("xor").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Xor,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            infix(
                left(2),
                text::keyword("or").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Or,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
            // Logical implies - precedence 1 (lowest), right-associative
            infix(
                right(1),
                text::keyword("implies").padded(),
                |left: Spanned<Expression>, _, right: Spanned<Expression>, e| {
                    let span = make_span(e.span());
                    Spanned::new(
                        Expression::BinaryOp(BinaryOpExpr {
                            left: Box::new(left),
                            op: BinaryOp::Implies,
                            right: Box::new(right),
                        }),
                        span,
                    )
                },
            ),
        ))
    })
}
