//! Common parser combinators for CQL

use chumsky::prelude::*;
use octofhir_cql_ast::{
    DateLiteral, DateTimeLiteral, Identifier, Literal, QualifiedIdentifier, QuantityLiteral,
    RatioLiteral, TimeLiteral, VersionSpecifier,
};
use rust_decimal::Decimal;
use std::str::FromStr;

/// Parse a CQL identifier (not a keyword)
pub fn identifier_parser<'a>(
) -> impl Parser<'a, &'a str, Identifier, extra::Err<Rich<'a, char>>> + Clone {
    // Quoted identifier: "identifier"
    let quoted = just('"')
        .ignore_then(none_of("\"").repeated().collect::<String>())
        .then_ignore(just('"'))
        .map(Identifier::quoted);

    // Regular identifier: starts with letter or _, followed by alphanumeric or _
    let regular = text::ident().map(|s: &str| Identifier::new(s));

    quoted.or(regular)
}

/// Parse a qualified identifier (e.g., "Library.Definition")
pub fn qualified_identifier_parser<'a>(
) -> impl Parser<'a, &'a str, QualifiedIdentifier, extra::Err<Rich<'a, char>>> + Clone {
    identifier_parser()
        .then(just('.').ignore_then(identifier_parser()).or_not())
        .map(|(first, second)| match second {
            Some(name) => QualifiedIdentifier::qualified(first.name, name),
            None => QualifiedIdentifier::simple(first),
        })
}

/// Parse a version specifier (a string literal)
pub fn version_specifier_parser<'a>(
) -> impl Parser<'a, &'a str, VersionSpecifier, extra::Err<Rich<'a, char>>> + Clone {
    string_parser().map(|s| VersionSpecifier::new(s))
}

/// Parse a string literal (single-quoted)
pub fn string_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    just('\'')
        .ignore_then(
            choice((just("''").to('\''), none_of("'\\")))
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just('\''))
}

/// Parse an integer literal
pub fn integer_parser<'a>() -> impl Parser<'a, &'a str, i32, extra::Err<Rich<'a, char>>> + Clone {
    text::int(10).map(|s: &str| s.parse().unwrap_or(0))
}

/// Parse a decimal literal
pub fn decimal_parser<'a>() -> impl Parser<'a, &'a str, Decimal, extra::Err<Rich<'a, char>>> + Clone
{
    text::int(10)
        .then(just('.').then(text::digits(10)))
        .to_slice()
        .map(|s: &str| Decimal::from_str(s).unwrap_or_default())
}

/// Parse a number (decimal or integer) returning a Literal
pub fn number_parser<'a>() -> impl Parser<'a, &'a str, Literal, extra::Err<Rich<'a, char>>> + Clone {
    // Try decimal first (has dot), then integer
    let decimal = text::int(10)
        .then(just('.'))
        .then(text::digits(10))
        .to_slice()
        .map(|s: &str| Literal::Decimal(Decimal::from_str(s).unwrap_or_default()));

    let long = text::int(10)
        .then(one_of("Ll"))
        .to_slice()
        .map(|s: &str| {
            let num_str = s.trim_end_matches(['L', 'l']);
            Literal::Long(num_str.parse().unwrap_or(0))
        });

    let integer = text::int(10).map(|s: &str| Literal::Integer(s.parse().unwrap_or(0)));

    decimal.or(long).or(integer)
}

/// Parse a boolean literal
pub fn boolean_parser<'a>() -> impl Parser<'a, &'a str, bool, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        text::keyword("true").to(true),
        text::keyword("false").to(false),
    ))
}

/// Parse a 2-digit number
fn two_digits<'a>() -> impl Parser<'a, &'a str, u8, extra::Err<Rich<'a, char>>> + Clone {
    text::digits(10)
        .exactly(2)
        .to_slice()
        .map(|s: &str| s.parse().unwrap_or(0))
}

/// Parse a 4-digit year
fn four_digit_year<'a>() -> impl Parser<'a, &'a str, i32, extra::Err<Rich<'a, char>>> + Clone {
    text::digits(10)
        .exactly(4)
        .to_slice()
        .map(|s: &str| s.parse().unwrap_or(0))
}

/// Parse milliseconds (1-3 digits)
fn milliseconds<'a>() -> impl Parser<'a, &'a str, u16, extra::Err<Rich<'a, char>>> + Clone {
    text::digits(10)
        .at_least(1)
        .at_most(3)
        .to_slice()
        .map(|s: &str| {
            let num: u16 = s.parse().unwrap_or(0);
            // Pad to 3 digits: "1" -> 100, "12" -> 120, "123" -> 123
            match s.len() {
                1 => num * 100,
                2 => num * 10,
                _ => num,
            }
        })
}

/// Parse timezone offset (+/-hh:mm or Z)
fn timezone_offset<'a>() -> impl Parser<'a, &'a str, i16, extra::Err<Rich<'a, char>>> + Clone {
    let z = just('Z').to(0i16);

    let offset = one_of("+-")
        .then(two_digits())
        .then_ignore(just(':'))
        .then(two_digits())
        .map(|((sign, hours), minutes)| {
            let total = (hours as i16) * 60 + (minutes as i16);
            if sign == '-' { -total } else { total }
        });

    z.or(offset)
}

/// Parse a date literal: @YYYY[-MM[-DD]]
pub fn date_literal_parser<'a>(
) -> impl Parser<'a, &'a str, DateLiteral, extra::Err<Rich<'a, char>>> + Clone {
    just('@')
        .ignore_then(four_digit_year())
        .then(just('-').ignore_then(two_digits()).or_not())
        .then(just('-').ignore_then(two_digits()).or_not())
        .map(|((year, month), day)| {
            let mut date = DateLiteral::new(year);
            if let Some(m) = month {
                date = date.with_month(m);
            }
            if let Some(d) = day {
                date = date.with_day(d);
            }
            date
        })
}

/// Parse a time literal: @Thh[:mm[:ss[.fff]]]
pub fn time_literal_parser<'a>(
) -> impl Parser<'a, &'a str, TimeLiteral, extra::Err<Rich<'a, char>>> + Clone {
    just("@T")
        .ignore_then(two_digits())
        .then(just(':').ignore_then(two_digits()).or_not())
        .then(just(':').ignore_then(two_digits()).or_not())
        .then(just('.').ignore_then(milliseconds()).or_not())
        .map(|(((hour, minute), second), ms)| {
            let mut time = TimeLiteral::new(hour);
            if let Some(m) = minute {
                time = time.with_minute(m);
            }
            if let Some(s) = second {
                time = time.with_second(s);
            }
            if let Some(ms) = ms {
                time = time.with_millisecond(ms);
            }
            time
        })
}

/// Parse a datetime literal: @YYYY[-MM[-DD]][Thh[:mm[:ss[.fff]]]][(+|-)hh:mm|Z]
pub fn datetime_literal_parser<'a>(
) -> impl Parser<'a, &'a str, DateTimeLiteral, extra::Err<Rich<'a, char>>> + Clone {
    just('@')
        .ignore_then(four_digit_year())
        .then(just('-').ignore_then(two_digits()).or_not())
        .then(just('-').ignore_then(two_digits()).or_not())
        .then(just('T').ignore_then(two_digits()).or_not())
        .then(just(':').ignore_then(two_digits()).or_not())
        .then(just(':').ignore_then(two_digits()).or_not())
        .then(just('.').ignore_then(milliseconds()).or_not())
        .then(timezone_offset().or_not())
        .map(
            |(((((((year, month), day), hour), minute), second), ms), tz)| {
                let mut date = DateLiteral::new(year);
                if let Some(m) = month {
                    date = date.with_month(m);
                }
                if let Some(d) = day {
                    date = date.with_day(d);
                }

                let mut dt = DateTimeLiteral::new(date);
                if let Some(h) = hour {
                    dt.hour = Some(h);
                }
                if let Some(m) = minute {
                    dt.minute = Some(m);
                }
                if let Some(s) = second {
                    dt.second = Some(s);
                }
                if let Some(ms) = ms {
                    dt.millisecond = Some(ms);
                }
                if let Some(tz) = tz {
                    dt.timezone_offset = Some(tz);
                }
                dt
            },
        )
}

/// Parse any temporal literal (Date, DateTime, or Time)
pub fn temporal_literal_parser<'a>(
) -> impl Parser<'a, &'a str, Literal, extra::Err<Rich<'a, char>>> + Clone {
    // Time must be checked first (@T...)
    let time = time_literal_parser().map(Literal::Time);

    // DateTime includes time component (has 'T' after date)
    let datetime_with_time = just('@')
        .ignore_then(four_digit_year())
        .then(just('-').ignore_then(two_digits()).or_not())
        .then(just('-').ignore_then(two_digits()).or_not())
        .then_ignore(just('T'))
        .then(two_digits())
        .then(just(':').ignore_then(two_digits()).or_not())
        .then(just(':').ignore_then(two_digits()).or_not())
        .then(just('.').ignore_then(milliseconds()).or_not())
        .then(timezone_offset().or_not())
        .map(
            |(((((((year, month), day), hour), minute), second), ms), tz)| {
                let mut date = DateLiteral::new(year);
                if let Some(m) = month {
                    date = date.with_month(m);
                }
                if let Some(d) = day {
                    date = date.with_day(d);
                }

                let mut dt = DateTimeLiteral::new(date);
                dt.hour = Some(hour);
                if let Some(m) = minute {
                    dt.minute = Some(m);
                }
                if let Some(s) = second {
                    dt.second = Some(s);
                }
                if let Some(ms) = ms {
                    dt.millisecond = Some(ms);
                }
                if let Some(tz) = tz {
                    dt.timezone_offset = Some(tz);
                }
                Literal::DateTime(dt)
            },
        );

    // Date only (no 'T')
    let date_only = date_literal_parser().map(Literal::Date);

    // Try in order: time (@T), datetime with time, date only
    time.or(datetime_with_time).or(date_only)
}

/// Parse a decimal number for quantities (can include negative)
fn quantity_value<'a>() -> impl Parser<'a, &'a str, Decimal, extra::Err<Rich<'a, char>>> + Clone {
    let sign = just('-').or_not().map(|s| s.is_some());

    let decimal = text::int(10)
        .then(just('.').then(text::digits(10)).or_not())
        .to_slice();

    sign.then(decimal).map(|(neg, s): (bool, &str)| {
        let val = Decimal::from_str(s).unwrap_or_default();
        if neg { -val } else { val }
    })
}

/// Parse a UCUM unit string (single-quoted)
fn unit_string<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    string_parser()
}

/// Parse a quantity literal: number [unit]
pub fn quantity_literal_parser<'a>(
) -> impl Parser<'a, &'a str, QuantityLiteral, extra::Err<Rich<'a, char>>> + Clone {
    quantity_value()
        .then(unit_string().padded().or_not())
        .map(|(value, unit)| {
            let mut q = QuantityLiteral::new(value);
            if let Some(u) = unit {
                q = q.with_unit(u);
            }
            q
        })
}

/// Parse a ratio literal: quantity:quantity
pub fn ratio_literal_parser<'a>(
) -> impl Parser<'a, &'a str, RatioLiteral, extra::Err<Rich<'a, char>>> + Clone {
    quantity_literal_parser()
        .then_ignore(just(':').padded())
        .then(quantity_literal_parser())
        .map(|(num, denom)| RatioLiteral::new(num, denom))
}

/// Check if a string is a CQL keyword
pub fn is_keyword(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "library"
            | "version"
            | "using"
            | "include"
            | "called"
            | "parameter"
            | "default"
            | "public"
            | "private"
            | "codesystem"
            | "valueset"
            | "code"
            | "concept"
            | "from"
            | "display"
            | "context"
            | "define"
            | "function"
            | "fluent"
            | "external"
            | "returns"
            | "null"
            | "true"
            | "false"
            | "and"
            | "or"
            | "xor"
            | "not"
            | "implies"
            | "is"
            | "as"
            | "cast"
            | "between"
            | "in"
            | "contains"
            | "if"
            | "then"
            | "else"
            | "case"
            | "when"
            | "exists"
            | "flatten"
            | "distinct"
            | "collapse"
            | "singleton"
            | "such"
            | "that"
            | "with"
            | "without"
            | "where"
            | "return"
            | "all"
            | "sort"
            | "by"
            | "asc"
            | "ascending"
            | "desc"
            | "descending"
            | "let"
            | "aggregate"
            | "starting"
            | "union"
            | "intersect"
            | "except"
            | "interval"
            | "point"
            | "start"
            | "end"
            | "width"
            | "size"
            | "now"
            | "today"
            | "timeofday"
            | "date"
            | "datetime"
            | "time"
            | "div"
            | "mod"
    )
}

/// Preprocess input: strip comments and normalize whitespace
pub fn preprocess(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut in_string = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_string => {
                in_string = true;
                result.push(ch);
            }
            '\'' if in_string => {
                // Check for escaped quote
                if chars.peek() == Some(&'\'') {
                    result.push(ch);
                    result.push(chars.next().unwrap());
                } else {
                    in_string = false;
                    result.push(ch);
                }
            }
            '/' if !in_string => {
                match chars.peek() {
                    Some('/') => {
                        // Single-line comment - skip to end of line
                        chars.next();
                        result.push(' ');
                        for c in chars.by_ref() {
                            if c == '\n' {
                                result.push(' ');
                                break;
                            }
                        }
                    }
                    Some('*') => {
                        // Multi-line comment
                        chars.next();
                        result.push(' ');
                        let mut prev = '\0';
                        for c in chars.by_ref() {
                            if prev == '*' && c == '/' {
                                break;
                            }
                            prev = c;
                        }
                    }
                    _ => result.push(ch),
                }
            }
            '\n' | '\r' | '\t' if !in_string => {
                result.push(' ');
            }
            _ => result.push(ch),
        }
    }

    result
}
