//! Common parser combinators for CQL using winnow

use octofhir_cql_ast::{
    DateLiteral, DateTimeLiteral, Identifier, Literal, QualifiedIdentifier, QuantityLiteral,
    RatioLiteral, TimeLiteral, VersionSpecifier,
};
use rust_decimal::Decimal;
use std::str::FromStr;
use winnow::ascii::{digit1, multispace0};
use winnow::combinator::{alt, delimited, opt, preceded, repeat};
use winnow::error::ContextError;
use winnow::prelude::*;
use winnow::stream::{AsChar, Offset};
use winnow::token::{any, literal, none_of, one_of, take_while};

/// Input type for our parsers - simple &str with position tracking via checkpoint
pub type Input<'a> = &'a str;

/// Error type for our parsers
pub type PError = ContextError;

/// Result type for our parsers
pub type PResult<O> = Result<O, PError>;

/// State to track original input for position calculation
/// Note: Currently unused but will be needed for proper span tracking
#[allow(dead_code)]
pub struct ParseState<'a> {
    pub original: &'a str,
}

#[allow(dead_code)]
impl<'a> ParseState<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { original: input }
    }

    pub fn position(&self, current: &str) -> usize {
        self.original.offset_from(&current)
    }
}

/// Parse optional whitespace
pub fn ws<'a>(input: &mut Input<'a>) -> PResult<()> {
    multispace0.void().parse_next(input)
}

/// Parse a literal string with proper type annotations
pub fn lit<'a>(s: &'static str) -> impl Parser<Input<'a>, &'a str, PError> {
    literal(s)
}

/// Parse a keyword case-insensitively (must not be followed by alphanumeric)
pub fn keyword<'a>(kw: &'static str) -> impl Parser<Input<'a>, (), PError> {
    move |input: &mut Input<'a>| {
        let checkpoint = *input;
        // Case-insensitive keyword matching - keywords are ASCII-only so byte length is OK
        // but we need to check if we have enough characters first
        let input_chars: String = input.chars().take(kw.len()).collect();
        if input_chars.len() < kw.len() {
            return Err(ContextError::new());
        }
        if !input_chars.eq_ignore_ascii_case(kw) {
            return Err(ContextError::new());
        }
        // Advance past the keyword - calculate actual byte length consumed
        let byte_len: usize = input.char_indices().take(kw.len()).last().map_or(0, |(i, c)| i + c.len_utf8());
        *input = &input[byte_len..];
        // Peek at next char - if alphanumeric or _, it's not a keyword
        if let Some(c) = input.chars().next() {
            if c.is_alphanum() || c == '_' {
                *input = checkpoint;
                return Err(ContextError::new());
            }
        }
        Ok(())
    }
}

/// Parse a padded keyword (with whitespace before and after)
pub fn padded_keyword<'a>(kw: &'static str) -> impl Parser<Input<'a>, (), PError> {
    move |input: &mut Input<'a>| {
        ws.parse_next(input)?;
        keyword(kw).parse_next(input)?;
        ws.parse_next(input)?;
        Ok(())
    }
}

/// Parse a CQL identifier (not a keyword)
pub fn identifier_parser<'a>(input: &mut Input<'a>) -> PResult<Identifier> {
    alt((
        // Quoted identifier: "identifier"
        delimited(
            '"',
            take_while(0.., |c: char| c != '"').map(|s: &str| Identifier::quoted(s.to_string())),
            '"',
        ),
        // Regular identifier: starts with letter or _, followed by alphanumeric or _
        (
            one_of(|c: char| c.is_alphabetic() || c == '_'),
            take_while(0.., |c: char| c.is_alphanumeric() || c == '_'),
        )
            .take()
            .verify(|s: &str| !is_keyword(s))
            .map(|s: &str| Identifier::new(s)),
    ))
    .parse_next(input)
}

/// Parse an identifier that allows keywords (for tuple element names, property names, etc.)
/// In CQL, certain contexts allow keywords to be used as identifiers.
pub fn identifier_or_keyword_parser<'a>(input: &mut Input<'a>) -> PResult<Identifier> {
    alt((
        // Quoted identifier: "identifier"
        delimited(
            '"',
            take_while(0.., |c: char| c != '"').map(|s: &str| Identifier::quoted(s.to_string())),
            '"',
        ),
        // Regular identifier OR keyword: starts with letter or _, followed by alphanumeric or _
        (
            one_of(|c: char| c.is_alphabetic() || c == '_'),
            take_while(0.., |c: char| c.is_alphanumeric() || c == '_'),
        )
            .take()
            .map(|s: &str| Identifier::new(s)),
    ))
    .parse_next(input)
}

/// Parse a qualified identifier (e.g., "Library.Definition")
pub fn qualified_identifier_parser<'a>(input: &mut Input<'a>) -> PResult<QualifiedIdentifier> {
    let first = identifier_parser.parse_next(input)?;
    let second = opt(preceded('.', identifier_parser)).parse_next(input)?;

    Ok(match second {
        Some(name) => QualifiedIdentifier::qualified(first.name, name),
        None => QualifiedIdentifier::simple(first),
    })
}

/// Parse a version specifier (a string literal)
pub fn version_specifier_parser<'a>(input: &mut Input<'a>) -> PResult<VersionSpecifier> {
    string_parser.map(VersionSpecifier::new).parse_next(input)
}

/// Parse a string literal (single-quoted)
/// Character or two-character escape sequence from a string
enum StringChar {
    Single(char),
    Escape(char, char), // For unknown escapes like \d, keep both chars
}

pub fn string_parser<'a>(input: &mut Input<'a>) -> PResult<String> {
    delimited(
        '\'',
        repeat(
            0..,
            alt((
                // Escaped quote '' (CQL style)
                "''".map(|_| StringChar::Single('\'')),
                // Backslash escape sequences
                preceded(
                    '\\',
                    alt((
                        '\\'.map(|_| StringChar::Single('\\')),
                        '\''.map(|_| StringChar::Single('\'')),
                        'n'.map(|_| StringChar::Single('\n')),
                        'r'.map(|_| StringChar::Single('\r')),
                        't'.map(|_| StringChar::Single('\t')),
                        'f'.map(|_| StringChar::Single('\x0C')),
                        // Keep unknown escape sequences as-is (e.g., \d, \s for regex)
                        any.map(|c| StringChar::Escape('\\', c)),
                    )),
                ),
                // Any char except quote and backslash
                none_of(['\'', '\\']).map(StringChar::Single),
            )),
        )
        .fold(String::new, |mut acc: String, sc: StringChar| {
            match sc {
                StringChar::Single(c) => acc.push(c),
                StringChar::Escape(c1, c2) => {
                    acc.push(c1);
                    acc.push(c2);
                }
            }
            acc
        }),
        '\'',
    )
    .parse_next(input)
}

/// Parse an integer literal
#[allow(dead_code)]
pub fn integer_parser<'a>(input: &mut Input<'a>) -> PResult<i32> {
    digit1
        .map(|s: &str| s.parse().unwrap_or(0))
        .parse_next(input)
}

/// Parse a decimal literal
#[allow(dead_code)]
pub fn decimal_parser<'a>(input: &mut Input<'a>) -> PResult<Decimal> {
    (digit1, '.', digit1)
        .take()
        .map(|s: &str| Decimal::from_str(s).unwrap_or_default())
        .parse_next(input)
}

/// Parse a number (decimal, long, or integer) returning a Literal
pub fn number_parser<'a>(input: &mut Input<'a>) -> PResult<Literal> {
    alt((
        // Decimal: digits.digits
        (digit1, '.', digit1)
            .take()
            .map(|s: &str| Literal::Decimal(Decimal::from_str(s).unwrap_or_default())),
        // Long: digitsL or digitsl
        (digit1, one_of(['L', 'l']))
            .take()
            .map(|s: &str| {
                let num_str = s.trim_end_matches(['L', 'l']);
                Literal::Long(num_str.parse().unwrap_or(0))
            }),
        // Integer: digits
        digit1.map(|s: &str| Literal::Integer(s.parse().unwrap_or(0))),
    ))
    .parse_next(input)
}

/// Parse a boolean literal
pub fn boolean_parser<'a>(input: &mut Input<'a>) -> PResult<bool> {
    alt((keyword("true").value(true), keyword("false").value(false))).parse_next(input)
}

/// Parse a 2-digit number
fn two_digits<'a>(input: &mut Input<'a>) -> PResult<u8> {
    take_while(2..=2, |c: char| c.is_ascii_digit())
        .map(|s: &str| s.parse().unwrap_or(0))
        .parse_next(input)
}

/// Parse a 4-digit year
fn four_digit_year<'a>(input: &mut Input<'a>) -> PResult<i32> {
    take_while(4..=4, |c: char| c.is_ascii_digit())
        .map(|s: &str| s.parse().unwrap_or(0))
        .parse_next(input)
}

/// Parse milliseconds (1-3 digits)
fn milliseconds<'a>(input: &mut Input<'a>) -> PResult<u16> {
    take_while(1..=3, |c: char| c.is_ascii_digit())
        .map(|s: &str| {
            let num: u16 = s.parse().unwrap_or(0);
            // Pad to 3 digits: "1" -> 100, "12" -> 120, "123" -> 123
            match s.len() {
                1 => num * 100,
                2 => num * 10,
                _ => num,
            }
        })
        .parse_next(input)
}

/// Parse timezone offset (+/-hh:mm or Z)
fn timezone_offset<'a>(input: &mut Input<'a>) -> PResult<i16> {
    alt((
        'Z'.value(0i16),
        (one_of(['+', '-']), two_digits, ':', two_digits).map(|(sign, hours, _, minutes)| {
            let total = (hours as i16) * 60 + (minutes as i16);
            if sign == '-' {
                -total
            } else {
                total
            }
        }),
    ))
    .parse_next(input)
}

/// Parse a date literal: @YYYY[-MM[-DD]]
pub fn date_literal_parser<'a>(input: &mut Input<'a>) -> PResult<DateLiteral> {
    preceded(
        '@',
        (
            four_digit_year,
            opt(preceded('-', two_digits)),
            opt(preceded('-', two_digits)),
        ),
    )
    .map(|(year, month, day)| {
        let mut date = DateLiteral::new(year);
        if let Some(m) = month {
            date = date.with_month(m);
        }
        if let Some(d) = day {
            date = date.with_day(d);
        }
        date
    })
    .parse_next(input)
}

/// Parse a time literal: @Thh[:mm[:ss[.fff]]]
pub fn time_literal_parser<'a>(input: &mut Input<'a>) -> PResult<TimeLiteral> {
    preceded(
        "@T",
        (
            two_digits,
            opt(preceded(':', two_digits)),
            opt(preceded(':', two_digits)),
            opt(preceded('.', milliseconds)),
        ),
    )
    .map(|(hour, minute, second, ms)| {
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
    .parse_next(input)
}

/// Parse a datetime literal: @YYYY[-MM[-DD]][Thh[:mm[:ss[.fff]]]][(+|-)hh:mm|Z]
#[allow(dead_code)]
pub fn datetime_literal_parser<'a>(input: &mut Input<'a>) -> PResult<DateTimeLiteral> {
    preceded(
        '@',
        (
            four_digit_year,
            opt(preceded('-', two_digits)),
            opt(preceded('-', two_digits)),
            opt(preceded('T', two_digits)),
            opt(preceded(':', two_digits)),
            opt(preceded(':', two_digits)),
            opt(preceded('.', milliseconds)),
            opt(timezone_offset),
        ),
    )
    .map(
        |(year, month, day, hour, minute, second, ms, tz)| {
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
    .parse_next(input)
}

/// Parse any temporal literal (Date, DateTime, or Time)
pub fn temporal_literal_parser<'a>(input: &mut Input<'a>) -> PResult<Literal> {
    alt((
        // Time must be checked first (@T...)
        time_literal_parser.map(Literal::Time),
        // DateTime with time component (has 'T' after date)
        preceded(
            '@',
            (
                four_digit_year,
                opt(preceded('-', two_digits)),
                opt(preceded('-', two_digits)),
                preceded('T', two_digits),
                opt(preceded(':', two_digits)),
                opt(preceded(':', two_digits)),
                opt(preceded('.', milliseconds)),
                opt(timezone_offset),
            ),
        )
        .map(
            |(year, month, day, hour, minute, second, ms, tz)| {
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
        ),
        // Date only (no 'T')
        date_literal_parser.map(Literal::Date),
    ))
    .parse_next(input)
}

/// Parse a decimal number for quantities (can include negative)
fn quantity_value<'a>(input: &mut Input<'a>) -> PResult<Decimal> {
    let neg = opt('-').map(|s| s.is_some()).parse_next(input)?;
    let num_str = (digit1, opt(('.', digit1)))
        .take()
        .parse_next(input)?;
    let val = Decimal::from_str(num_str).unwrap_or_default();
    Ok(if neg { -val } else { val })
}

/// Parse a UCUM unit string (single-quoted)
fn quoted_unit_string<'a>(input: &mut Input<'a>) -> PResult<String> {
    string_parser(input)
}

/// Parse an unquoted duration unit keyword
fn duration_unit_keyword<'a>(input: &mut Input<'a>) -> PResult<String> {
    alt((
        keyword("years").value("year".to_string()),
        keyword("year").value("year".to_string()),
        keyword("months").value("month".to_string()),
        keyword("month").value("month".to_string()),
        keyword("weeks").value("week".to_string()),
        keyword("week").value("week".to_string()),
        keyword("days").value("day".to_string()),
        keyword("day").value("day".to_string()),
        keyword("hours").value("hour".to_string()),
        keyword("hour").value("hour".to_string()),
        keyword("minutes").value("minute".to_string()),
        keyword("minute").value("minute".to_string()),
        keyword("seconds").value("second".to_string()),
        keyword("second").value("second".to_string()),
        keyword("milliseconds").value("millisecond".to_string()),
        keyword("millisecond").value("millisecond".to_string()),
    ))
    .parse_next(input)
}

/// Parse a unit string (quoted UCUM or unquoted duration keyword)
fn unit_string<'a>(input: &mut Input<'a>) -> PResult<String> {
    alt((quoted_unit_string, duration_unit_keyword)).parse_next(input)
}

/// Parse a quantity literal: number [unit]
pub fn quantity_literal_parser<'a>(input: &mut Input<'a>) -> PResult<QuantityLiteral> {
    let value = quantity_value.parse_next(input)?;
    ws.parse_next(input)?;
    let unit = opt(unit_string).parse_next(input)?;

    let mut q = QuantityLiteral::new(value);
    if let Some(u) = unit {
        q = q.with_unit(u);
    }
    Ok(q)
}

/// Parse a ratio literal: quantity:quantity
#[allow(dead_code)]
pub fn ratio_literal_parser<'a>(input: &mut Input<'a>) -> PResult<RatioLiteral> {
    let num = quantity_literal_parser.parse_next(input)?;
    ws.parse_next(input)?;
    ':'.parse_next(input)?;
    ws.parse_next(input)?;
    let denom = quantity_literal_parser.parse_next(input)?;
    Ok(RatioLiteral::new(num, denom))
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
            // Interval/temporal operators
            | "after"
            | "before"
            | "same"
            | "meets"
            | "overlaps"
            | "starts"
            | "ends"
            | "includes"
            | "included"  // for "included in"
            | "during"
            | "properly"
            | "on"
            | "within"
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
