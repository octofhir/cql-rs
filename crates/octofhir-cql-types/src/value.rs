//! CQL Value types - runtime representation of all CQL values
//!
//! This module defines the CqlValue enum and all supporting types for
//! representing CQL values at runtime per the CQL 1.5 specification.

use indexmap::IndexMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::fmt;
// Arc<str> replaced with String for serde compatibility

use crate::CqlType;

/// The primary value type for CQL runtime values.
///
/// This enum represents all possible CQL values including primitives,
/// temporal types, clinical types, and collections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum CqlValue {
    // === Primitive Types ===
    /// Null value (represents missing/unknown)
    Null,
    /// Boolean value
    Boolean(bool),
    /// 32-bit signed integer
    Integer(i32),
    /// 64-bit signed integer (Long)
    Long(i64),
    /// Arbitrary precision decimal
    Decimal(Decimal),
    /// String value (using String for serde compatibility)
    String(String),

    // === Temporal Types ===
    /// Date with precision
    Date(CqlDate),
    /// DateTime with precision and timezone
    DateTime(CqlDateTime),
    /// Time with precision
    Time(CqlTime),

    // === Clinical Types ===
    /// Quantity with value and UCUM unit
    Quantity(CqlQuantity),
    /// Ratio of two quantities
    Ratio(CqlRatio),
    /// Code from a code system
    Code(CqlCode),
    /// Concept (collection of codes)
    Concept(CqlConcept),

    // === Collection Types ===
    /// Ordered list of values
    List(CqlList),
    /// Interval between two points
    Interval(CqlInterval),
    /// Tuple with named elements
    Tuple(CqlTuple),
}

impl CqlValue {
    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if this value is truthy (for Boolean)
    pub fn is_true(&self) -> bool {
        matches!(self, Self::Boolean(true))
    }

    /// Check if this value is falsy (for Boolean)
    pub fn is_false(&self) -> bool {
        matches!(self, Self::Boolean(false))
    }

    /// Get the CQL type of this value
    pub fn get_type(&self) -> CqlType {
        match self {
            Self::Null => CqlType::Any, // Null is compatible with any type
            Self::Boolean(_) => CqlType::Boolean,
            Self::Integer(_) => CqlType::Integer,
            Self::Long(_) => CqlType::Long,
            Self::Decimal(_) => CqlType::Decimal,
            Self::String(_) => CqlType::String,
            Self::Date(_) => CqlType::Date,
            Self::DateTime(_) => CqlType::DateTime,
            Self::Time(_) => CqlType::Time,
            Self::Quantity(_) => CqlType::Quantity,
            Self::Ratio(_) => CqlType::Ratio,
            Self::Code(_) => CqlType::Code,
            Self::Concept(_) => CqlType::Concept,
            Self::List(list) => CqlType::List(Box::new(list.element_type.clone())),
            Self::Interval(interval) => CqlType::Interval(Box::new(interval.point_type.clone())),
            Self::Tuple(tuple) => {
                let elements = tuple
                    .elements
                    .iter()
                    .map(|(name, value)| crate::TupleTypeElement {
                        name: name.to_string(),
                        element_type: value.get_type(),
                    })
                    .collect();
                CqlType::Tuple(elements)
            }
        }
    }

    /// Try to get as Boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as Integer
    pub fn as_integer(&self) -> Option<i32> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as Long
    pub fn as_long(&self) -> Option<i64> {
        match self {
            Self::Long(l) => Some(*l),
            Self::Integer(i) => Some(*i as i64), // Implicit promotion
            _ => None,
        }
    }

    /// Try to get as Decimal
    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Self::Decimal(d) => Some(*d),
            Self::Integer(i) => Some(Decimal::from(*i)),
            Self::Long(l) => Some(Decimal::from(*l)),
            _ => None,
        }
    }

    /// Try to get as String
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as List
    pub fn as_list(&self) -> Option<&CqlList> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }

    /// Try to get as Interval
    pub fn as_interval(&self) -> Option<&CqlInterval> {
        match self {
            Self::Interval(i) => Some(i),
            _ => None,
        }
    }

    /// Try to get as Tuple
    pub fn as_tuple(&self) -> Option<&CqlTuple> {
        match self {
            Self::Tuple(t) => Some(t),
            _ => None,
        }
    }

    /// Create a null value
    pub fn null() -> Self {
        Self::Null
    }

    /// Create a boolean value
    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// Create an integer value
    pub fn integer(value: i32) -> Self {
        Self::Integer(value)
    }

    /// Create a long value
    pub fn long(value: i64) -> Self {
        Self::Long(value)
    }

    /// Create a decimal value
    pub fn decimal(value: Decimal) -> Self {
        Self::Decimal(value)
    }

    /// Create a string value
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }
}

impl fmt::Display for CqlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(b) => write!(f, "{}", b),
            Self::Integer(i) => write!(f, "{}", i),
            Self::Long(l) => write!(f, "{}L", l),
            Self::Decimal(d) => {
                // Ensure at least one decimal place for CQL compliance
                let s = d.to_string();
                if s.contains('.') {
                    write!(f, "{}", s)
                } else {
                    write!(f, "{}.0", s)
                }
            }
            Self::String(s) => {
                // Escape quotes using unicode escapes per CQL spec
                write!(f, "'")?;
                for c in s.chars() {
                    match c {
                        '\'' => write!(f, "\\u0027")?,
                        '"' => write!(f, "\\u0022")?,
                        _ => write!(f, "{}", c)?,
                    }
                }
                write!(f, "'")
            }
            Self::Date(d) => write!(f, "@{}", d),
            Self::DateTime(dt) => write!(f, "@{}", dt),
            Self::Time(t) => write!(f, "@T{}", t),
            Self::Quantity(q) => write!(f, "{}", q),
            Self::Ratio(r) => write!(f, "{}", r),
            Self::Code(c) => write!(f, "{}", c),
            Self::Concept(c) => write!(f, "{}", c),
            Self::List(l) => write!(f, "{}", l),
            Self::Interval(i) => write!(f, "{}", i),
            Self::Tuple(t) => write!(f, "{}", t),
        }
    }
}

impl PartialEq for CqlValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Long(a), Self::Long(b)) => a == b,
            (Self::Decimal(a), Self::Decimal(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Date(a), Self::Date(b)) => a == b,
            (Self::DateTime(a), Self::DateTime(b)) => a == b,
            (Self::Time(a), Self::Time(b)) => a == b,
            (Self::Quantity(a), Self::Quantity(b)) => a == b,
            (Self::Ratio(a), Self::Ratio(b)) => a == b,
            (Self::Code(a), Self::Code(b)) => a == b,
            (Self::Concept(a), Self::Concept(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Interval(a), Self::Interval(b)) => a == b,
            (Self::Tuple(a), Self::Tuple(b)) => a == b,
            // Cross-type numeric comparisons
            (Self::Integer(a), Self::Long(b)) => (*a as i64) == *b,
            (Self::Long(a), Self::Integer(b)) => *a == (*b as i64),
            (Self::Integer(a), Self::Decimal(b)) => Decimal::from(*a) == *b,
            (Self::Decimal(a), Self::Integer(b)) => *a == Decimal::from(*b),
            (Self::Long(a), Self::Decimal(b)) => Decimal::from(*a) == *b,
            (Self::Decimal(a), Self::Long(b)) => *a == Decimal::from(*b),
            _ => false,
        }
    }
}

impl Eq for CqlValue {}

// ============================================================================
// Temporal Types
// ============================================================================

/// Precision for temporal values
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DateTimePrecision {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

impl DateTimePrecision {
    /// Get all precisions up to and including this one
    pub fn all_up_to(self) -> &'static [DateTimePrecision] {
        match self {
            Self::Year => &[Self::Year],
            Self::Month => &[Self::Year, Self::Month],
            Self::Day => &[Self::Year, Self::Month, Self::Day],
            Self::Hour => &[Self::Year, Self::Month, Self::Day, Self::Hour],
            Self::Minute => &[Self::Year, Self::Month, Self::Day, Self::Hour, Self::Minute],
            Self::Second => &[
                Self::Year,
                Self::Month,
                Self::Day,
                Self::Hour,
                Self::Minute,
                Self::Second,
            ],
            Self::Millisecond => &[
                Self::Year,
                Self::Month,
                Self::Day,
                Self::Hour,
                Self::Minute,
                Self::Second,
                Self::Millisecond,
            ],
        }
    }
}

impl fmt::Display for DateTimePrecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Year => write!(f, "year"),
            Self::Month => write!(f, "month"),
            Self::Day => write!(f, "day"),
            Self::Hour => write!(f, "hour"),
            Self::Minute => write!(f, "minute"),
            Self::Second => write!(f, "second"),
            Self::Millisecond => write!(f, "millisecond"),
        }
    }
}

/// Helper function to get the number of days in a month
fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            // Check for leap year
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31, // Default fallback
    }
}

/// CQL Date with precision
///
/// Represents a date with varying precision (year, month, or day).
/// Dates without full precision are called "partial dates".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CqlDate {
    /// Year component (required)
    pub year: i32,
    /// Month component (1-12, optional)
    pub month: Option<u8>,
    /// Day component (1-31, optional)
    pub day: Option<u8>,
}

impl CqlDate {
    /// Create a new date with full precision
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month: Some(month),
            day: Some(day),
        }
    }

    /// Create a year-only date
    pub fn year_only(year: i32) -> Self {
        Self {
            year,
            month: None,
            day: None,
        }
    }

    /// Create a year-month date
    pub fn year_month(year: i32, month: u8) -> Self {
        Self {
            year,
            month: Some(month),
            day: None,
        }
    }

    /// Get the precision of this date
    pub fn precision(&self) -> DateTimePrecision {
        match (&self.month, &self.day) {
            (None, _) => DateTimePrecision::Year,
            (Some(_), None) => DateTimePrecision::Month,
            (Some(_), Some(_)) => DateTimePrecision::Day,
        }
    }

    /// Convert to chrono NaiveDate (if fully precise)
    pub fn to_naive_date(&self) -> Option<chrono::NaiveDate> {
        match (self.month, self.day) {
            (Some(month), Some(day)) => {
                chrono::NaiveDate::from_ymd_opt(self.year, month as u32, day as u32)
            }
            _ => None,
        }
    }

    /// Parse from ISO 8601 date string
    /// Also handles @ prefix: @2024-01-15
    pub fn parse(s: &str) -> Option<Self> {
        // Strip @ prefix if present (used in CQL/ELM literals)
        let s = s.strip_prefix('@').unwrap_or(s);

        let parts: Vec<&str> = s.split('-').collect();
        match parts.len() {
            1 => {
                let year = parts[0].parse().ok()?;
                Some(Self::year_only(year))
            }
            2 => {
                let year = parts[0].parse().ok()?;
                let month = parts[1].parse().ok()?;
                Some(Self::year_month(year, month))
            }
            3 => {
                let year = parts[0].parse().ok()?;
                let month = parts[1].parse().ok()?;
                let day = parts[2].parse().ok()?;
                Some(Self::new(year, month, day))
            }
            _ => None,
        }
    }
}

impl fmt::Display for CqlDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.year)?;
        if let Some(month) = self.month {
            write!(f, "-{:02}", month)?;
            if let Some(day) = self.day {
                write!(f, "-{:02}", day)?;
            }
        }
        Ok(())
    }
}

impl PartialOrd for CqlDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Compare at the lowest common precision
        let cmp_year = self.year.cmp(&other.year);
        if cmp_year != Ordering::Equal {
            return Some(cmp_year);
        }

        match (self.month, other.month) {
            (None, None) => Some(Ordering::Equal),
            (None, Some(_)) | (Some(_), None) => None, // Uncertain comparison
            (Some(m1), Some(m2)) => {
                let cmp_month = m1.cmp(&m2);
                if cmp_month != Ordering::Equal {
                    return Some(cmp_month);
                }

                match (self.day, other.day) {
                    (None, None) => Some(Ordering::Equal),
                    (None, Some(_)) | (Some(_), None) => None, // Uncertain comparison
                    (Some(d1), Some(d2)) => Some(d1.cmp(&d2)),
                }
            }
        }
    }
}

/// CQL DateTime with precision and timezone
///
/// Represents a date/time with varying precision and optional timezone offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CqlDateTime {
    /// Year component (required)
    pub year: i32,
    /// Month component (1-12, optional)
    pub month: Option<u8>,
    /// Day component (1-31, optional)
    pub day: Option<u8>,
    /// Hour component (0-23, optional)
    pub hour: Option<u8>,
    /// Minute component (0-59, optional)
    pub minute: Option<u8>,
    /// Second component (0-59, optional)
    pub second: Option<u8>,
    /// Millisecond component (0-999, optional)
    pub millisecond: Option<u16>,
    /// Timezone offset in minutes (optional)
    pub timezone_offset: Option<i16>,
}

impl CqlDateTime {
    /// Create a new datetime with full precision
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        millisecond: u16,
        timezone_offset: Option<i16>,
    ) -> Self {
        Self {
            year,
            month: Some(month),
            day: Some(day),
            hour: Some(hour),
            minute: Some(minute),
            second: Some(second),
            millisecond: Some(millisecond),
            timezone_offset,
        }
    }

    /// Create a datetime from a date
    pub fn from_date(date: CqlDate) -> Self {
        Self {
            year: date.year,
            month: date.month,
            day: date.day,
            hour: None,
            minute: None,
            second: None,
            millisecond: None,
            timezone_offset: None,
        }
    }

    /// Get the precision of this datetime
    pub fn precision(&self) -> DateTimePrecision {
        if self.millisecond.is_some() {
            DateTimePrecision::Millisecond
        } else if self.second.is_some() {
            DateTimePrecision::Second
        } else if self.minute.is_some() {
            DateTimePrecision::Minute
        } else if self.hour.is_some() {
            DateTimePrecision::Hour
        } else if self.day.is_some() {
            DateTimePrecision::Day
        } else if self.month.is_some() {
            DateTimePrecision::Month
        } else {
            DateTimePrecision::Year
        }
    }

    /// Truncate this datetime to the specified precision
    /// Returns a new CqlDateTime with only components up to the given precision
    pub fn truncate_to_precision(&self, precision: DateTimePrecision) -> Self {
        match precision {
            DateTimePrecision::Year => Self {
                year: self.year,
                month: None,
                day: None,
                hour: None,
                minute: None,
                second: None,
                millisecond: None,
                timezone_offset: self.timezone_offset,
            },
            DateTimePrecision::Month => Self {
                year: self.year,
                month: self.month,
                day: None,
                hour: None,
                minute: None,
                second: None,
                millisecond: None,
                timezone_offset: self.timezone_offset,
            },
            DateTimePrecision::Day => Self {
                year: self.year,
                month: self.month,
                day: self.day,
                hour: None,
                minute: None,
                second: None,
                millisecond: None,
                timezone_offset: self.timezone_offset,
            },
            DateTimePrecision::Hour => Self {
                year: self.year,
                month: self.month,
                day: self.day,
                hour: self.hour,
                minute: None,
                second: None,
                millisecond: None,
                timezone_offset: self.timezone_offset,
            },
            DateTimePrecision::Minute => Self {
                year: self.year,
                month: self.month,
                day: self.day,
                hour: self.hour,
                minute: self.minute,
                second: None,
                millisecond: None,
                timezone_offset: self.timezone_offset,
            },
            DateTimePrecision::Second => Self {
                year: self.year,
                month: self.month,
                day: self.day,
                hour: self.hour,
                minute: self.minute,
                second: self.second,
                millisecond: None,
                timezone_offset: self.timezone_offset,
            },
            DateTimePrecision::Millisecond => self.clone(),
        }
    }

    /// Extract the date portion
    pub fn date(&self) -> CqlDate {
        CqlDate {
            year: self.year,
            month: self.month,
            day: self.day,
        }
    }

    /// Extract the time portion (if present)
    pub fn time(&self) -> Option<CqlTime> {
        self.hour.map(|hour| CqlTime {
            hour,
            minute: self.minute,
            second: self.second,
            millisecond: self.millisecond,
        })
    }

    /// Parse from ISO 8601 datetime string
    /// Supports formats like: 2024-01-15T14:30:00.000Z, 2024-01-15T14:30, etc.
    /// Also handles @ prefix: @2024-01-15T14:30:00.000Z
    pub fn parse(s: &str) -> Option<Self> {
        // Strip @ prefix if present (used in CQL/ELM literals)
        let s = s.strip_prefix('@').unwrap_or(s);

        // Handle timezone suffix
        let (datetime_str, tz_offset) = if s.ends_with('Z') {
            (&s[..s.len() - 1], Some(0i16))
        } else if let Some(plus_idx) = s.rfind('+') {
            if plus_idx > 10 {
                // Make sure it's a timezone offset, not part of date
                let tz_str = &s[plus_idx + 1..];
                let offset = Self::parse_tz_offset(tz_str, false)?;
                (&s[..plus_idx], Some(offset))
            } else {
                (s, None)
            }
        } else if let Some(minus_idx) = s.rfind('-') {
            if minus_idx > 10 {
                // Make sure it's a timezone offset
                let tz_str = &s[minus_idx + 1..];
                let offset = Self::parse_tz_offset(tz_str, true)?;
                (&s[..minus_idx], Some(offset))
            } else {
                (s, None)
            }
        } else {
            (s, None)
        };

        // Split date and time parts
        let parts: Vec<&str> = datetime_str.split('T').collect();
        let date_str = parts.first()?;
        let time_str = parts.get(1);

        // Parse date portion
        let date_parts: Vec<&str> = date_str.split('-').collect();
        let year: i32 = date_parts.first()?.parse().ok()?;
        let month: Option<u8> = date_parts.get(1).and_then(|s| s.parse().ok());
        let day: Option<u8> = date_parts.get(2).and_then(|s| s.parse().ok());

        // Parse time portion if present
        let (hour, minute, second, millisecond) = if let Some(t) = time_str {
            Self::parse_time_components(t)
        } else {
            (None, None, None, None)
        };

        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            timezone_offset: tz_offset,
        })
    }

    /// Parse timezone offset string like "05:00" or "0500"
    fn parse_tz_offset(s: &str, negative: bool) -> Option<i16> {
        let clean = s.replace(':', "");
        if clean.len() < 2 {
            return None;
        }
        let hours: i16 = clean[..2].parse().ok()?;
        let mins: i16 = if clean.len() >= 4 {
            clean[2..4].parse().ok()?
        } else {
            0
        };
        let offset = hours * 60 + mins;
        Some(if negative { -offset } else { offset })
    }

    /// Check if this DateTime has any uncertain components
    /// A component is uncertain if it's None but required for the given precision
    pub fn has_uncertainty(&self) -> bool {
        // If any component between year and the precision level is None, there's uncertainty
        // Year is always required, so we check month onwards
        self.month.is_none() ||
            (self.month.is_some() && self.day.is_none()) ||
            (self.day.is_some() && self.hour.is_none()) ||
            (self.hour.is_some() && self.minute.is_none()) ||
            (self.minute.is_some() && self.second.is_none()) ||
            (self.second.is_some() && self.millisecond.is_none())
    }

    /// Get the low (earliest) boundary of this DateTime
    /// Fills in missing components with their minimum values
    pub fn low_boundary(&self) -> CqlDateTime {
        CqlDateTime {
            year: self.year,
            month: Some(self.month.unwrap_or(1)),
            day: Some(self.day.unwrap_or(1)),
            hour: Some(self.hour.unwrap_or(0)),
            minute: Some(self.minute.unwrap_or(0)),
            second: Some(self.second.unwrap_or(0)),
            millisecond: Some(self.millisecond.unwrap_or(0)),
            timezone_offset: self.timezone_offset,
        }
    }

    /// Get the high (latest) boundary of this DateTime
    /// Fills in missing components with their maximum values
    pub fn high_boundary(&self) -> CqlDateTime {
        let month = self.month.unwrap_or(12);
        let day = self.day.unwrap_or_else(|| days_in_month(self.year, month));

        CqlDateTime {
            year: self.year,
            month: Some(month),
            day: Some(day),
            hour: Some(self.hour.unwrap_or(23)),
            minute: Some(self.minute.unwrap_or(59)),
            second: Some(self.second.unwrap_or(59)),
            millisecond: Some(self.millisecond.unwrap_or(999)),
            timezone_offset: self.timezone_offset,
        }
    }

    /// Parse time components from a time string
    fn parse_time_components(s: &str) -> (Option<u8>, Option<u8>, Option<u8>, Option<u16>) {
        // Handle milliseconds
        let (time_str, ms) = if let Some(dot_idx) = s.find('.') {
            let ms_str = &s[dot_idx + 1..];
            let ms: u16 = ms_str.parse().ok().unwrap_or(0);
            (&s[..dot_idx], Some(ms))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = time_str.split(':').collect();
        let hour: Option<u8> = parts.first().and_then(|s| s.parse().ok());
        let minute: Option<u8> = parts.get(1).and_then(|s| s.parse().ok());
        let second: Option<u8> = parts.get(2).and_then(|s| s.parse().ok());

        (hour, minute, second, ms)
    }
}

impl fmt::Display for CqlDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.year)?;
        if let Some(month) = self.month {
            write!(f, "-{:02}", month)?;
            if let Some(day) = self.day {
                write!(f, "-{:02}", day)?;
                if let Some(hour) = self.hour {
                    write!(f, "T{:02}", hour)?;
                    if let Some(minute) = self.minute {
                        write!(f, ":{:02}", minute)?;
                        if let Some(second) = self.second {
                            write!(f, ":{:02}", second)?;
                            if let Some(ms) = self.millisecond {
                                write!(f, ".{:03}", ms)?;
                            }
                        }
                    }
                    // Timezone
                    if let Some(offset) = self.timezone_offset {
                        if offset == 0 {
                            write!(f, "Z")?;
                        } else {
                            let hours = offset.abs() / 60;
                            let mins = offset.abs() % 60;
                            let sign = if offset >= 0 { '+' } else { '-' };
                            write!(f, "{}{:02}:{:02}", sign, hours, mins)?;
                        }
                    }
                } else {
                    // DateTime with date-only precision: add trailing T
                    write!(f, "T")?;
                }
            } else {
                // DateTime with year-month precision: add trailing T
                write!(f, "T")?;
            }
        } else {
            // DateTime with year-only precision: add trailing T
            write!(f, "T")?;
        }
        Ok(())
    }
}

/// CQL Time with precision
///
/// Represents a time-of-day with varying precision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CqlTime {
    /// Hour component (0-23, required)
    pub hour: u8,
    /// Minute component (0-59, optional)
    pub minute: Option<u8>,
    /// Second component (0-59, optional)
    pub second: Option<u8>,
    /// Millisecond component (0-999, optional)
    pub millisecond: Option<u16>,
}

impl CqlTime {
    /// Create a new time with full precision
    pub fn new(hour: u8, minute: u8, second: u8, millisecond: u16) -> Self {
        Self {
            hour,
            minute: Some(minute),
            second: Some(second),
            millisecond: Some(millisecond),
        }
    }

    /// Create a time with hour precision only
    pub fn hour_only(hour: u8) -> Self {
        Self {
            hour,
            minute: None,
            second: None,
            millisecond: None,
        }
    }

    /// Create a time with hour:minute precision
    pub fn hour_minute(hour: u8, minute: u8) -> Self {
        Self {
            hour,
            minute: Some(minute),
            second: None,
            millisecond: None,
        }
    }

    /// Create a time with hour:minute:second precision
    pub fn hour_minute_second(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour,
            minute: Some(minute),
            second: Some(second),
            millisecond: None,
        }
    }

    /// Get the precision of this time
    pub fn precision(&self) -> DateTimePrecision {
        if self.millisecond.is_some() {
            DateTimePrecision::Millisecond
        } else if self.second.is_some() {
            DateTimePrecision::Second
        } else if self.minute.is_some() {
            DateTimePrecision::Minute
        } else {
            DateTimePrecision::Hour
        }
    }

    /// Truncate this time to the specified precision
    /// Returns a new CqlTime with only components up to the given precision
    pub fn truncate_to_precision(&self, precision: DateTimePrecision) -> Self {
        match precision {
            DateTimePrecision::Year | DateTimePrecision::Month | DateTimePrecision::Day | DateTimePrecision::Hour => Self {
                hour: self.hour,
                minute: None,
                second: None,
                millisecond: None,
            },
            DateTimePrecision::Minute => Self {
                hour: self.hour,
                minute: self.minute,
                second: None,
                millisecond: None,
            },
            DateTimePrecision::Second => Self {
                hour: self.hour,
                minute: self.minute,
                second: self.second,
                millisecond: None,
            },
            DateTimePrecision::Millisecond => self.clone(),
        }
    }

    /// Convert to total milliseconds since midnight
    pub fn to_milliseconds(&self) -> Option<u32> {
        let hour_ms = self.hour as u32 * 3_600_000;
        let minute_ms = self.minute.unwrap_or(0) as u32 * 60_000;
        let second_ms = self.second.unwrap_or(0) as u32 * 1_000;
        let ms = self.millisecond.unwrap_or(0) as u32;
        Some(hour_ms + minute_ms + second_ms + ms)
    }

    /// Parse from ISO 8601 time string
    /// Supports formats like: 14:30:00.000, 14:30:00, 14:30, 14
    /// Also handles @T prefix: @T14:30:00.000
    /// Returns None for invalid time values (hour > 23, minute > 59, second > 59)
    pub fn parse(s: &str) -> Option<Self> {
        // Strip @T prefix if present (used in CQL/ELM literals)
        let s = s.strip_prefix("@T").unwrap_or(s);

        // Handle milliseconds
        let (time_str, ms) = if let Some(dot_idx) = s.find('.') {
            let ms_str = &s[dot_idx + 1..];
            let ms: u16 = ms_str.parse().ok().unwrap_or(0);
            if ms > 999 {
                return None;
            }
            (&s[..dot_idx], Some(ms))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = time_str.split(':').collect();
        match parts.len() {
            1 => {
                let hour: u8 = parts[0].parse().ok()?;
                if hour > 23 {
                    return None;
                }
                Some(Self {
                    hour,
                    minute: None,
                    second: None,
                    millisecond: ms,
                })
            }
            2 => {
                let hour: u8 = parts[0].parse().ok()?;
                let minute: u8 = parts[1].parse().ok()?;
                if hour > 23 || minute > 59 {
                    return None;
                }
                Some(Self {
                    hour,
                    minute: Some(minute),
                    second: None,
                    millisecond: ms,
                })
            }
            3 | _ => {
                let hour: u8 = parts.first()?.parse().ok()?;
                let minute: u8 = parts.get(1)?.parse().ok()?;
                let second: u8 = parts.get(2)?.parse().ok()?;
                if hour > 23 || minute > 59 || second > 59 {
                    return None;
                }
                Some(Self {
                    hour,
                    minute: Some(minute),
                    second: Some(second),
                    millisecond: ms,
                })
            }
        }
    }
}

impl fmt::Display for CqlTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}", self.hour)?;
        if let Some(minute) = self.minute {
            write!(f, ":{:02}", minute)?;
            if let Some(second) = self.second {
                write!(f, ":{:02}", second)?;
                if let Some(ms) = self.millisecond {
                    write!(f, ".{:03}", ms)?;
                }
            }
        }
        Ok(())
    }
}

impl PartialOrd for CqlTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let cmp_hour = self.hour.cmp(&other.hour);
        if cmp_hour != Ordering::Equal {
            return Some(cmp_hour);
        }

        match (self.minute, other.minute) {
            (None, None) => Some(Ordering::Equal),
            (None, Some(_)) | (Some(_), None) => None,
            (Some(m1), Some(m2)) => {
                let cmp_minute = m1.cmp(&m2);
                if cmp_minute != Ordering::Equal {
                    return Some(cmp_minute);
                }

                match (self.second, other.second) {
                    (None, None) => Some(Ordering::Equal),
                    (None, Some(_)) | (Some(_), None) => None,
                    (Some(s1), Some(s2)) => {
                        let cmp_second = s1.cmp(&s2);
                        if cmp_second != Ordering::Equal {
                            return Some(cmp_second);
                        }

                        match (self.millisecond, other.millisecond) {
                            (None, None) => Some(Ordering::Equal),
                            (None, Some(_)) | (Some(_), None) => None,
                            (Some(ms1), Some(ms2)) => Some(ms1.cmp(&ms2)),
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Clinical Types
// ============================================================================

/// CQL Quantity with value and UCUM unit
///
/// Quantities are used for measurements with units.
/// The unit string must be a valid UCUM unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CqlQuantity {
    /// Numeric value
    pub value: Decimal,
    /// UCUM unit string (e.g., "mg", "kg", "m/s")
    pub unit: Option<String>,
}

impl CqlQuantity {
    /// Create a new quantity
    pub fn new(value: Decimal, unit: impl Into<String>) -> Self {
        Self {
            value,
            unit: Some(unit.into()),
        }
    }

    /// Create a unitless quantity
    pub fn unitless(value: Decimal) -> Self {
        Self { value, unit: None }
    }

    /// Create a quantity with unit "1" (dimensionless)
    pub fn dimensionless(value: Decimal) -> Self {
        Self {
            value,
            unit: Some("1".into()),
        }
    }

    /// Check if quantities have compatible units
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        match (&self.unit, &other.unit) {
            (None, None) => true,
            (Some(u1), Some(u2)) => u1 == u2, // Simplified - would use UCUM for full check
            _ => false,
        }
    }
}

impl PartialEq for CqlQuantity {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.unit == other.unit
    }
}

impl Eq for CqlQuantity {}

impl PartialOrd for CqlQuantity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.unit == other.unit {
            self.value.partial_cmp(&other.value)
        } else {
            None // Cannot compare quantities with different units
        }
    }
}

impl fmt::Display for CqlQuantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)?;
        if let Some(unit) = &self.unit {
            // Keep space before unit - most tests expect this format
            write!(f, " '{}'", unit)?;
        }
        Ok(())
    }
}

/// CQL Ratio - ratio of two quantities
///
/// Ratios are used for rate measurements like "1 mg per 1 mL".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CqlRatio {
    /// Numerator quantity
    pub numerator: CqlQuantity,
    /// Denominator quantity
    pub denominator: CqlQuantity,
}

impl CqlRatio {
    /// Create a new ratio
    pub fn new(numerator: CqlQuantity, denominator: CqlQuantity) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

impl fmt::Display for CqlRatio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} : {}", self.numerator, self.denominator)
    }
}

/// CQL Code - a code from a code system
///
/// Codes are used to represent clinical concepts from standardized
/// terminologies like SNOMED CT, LOINC, ICD-10, etc.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CqlCode {
    /// Code value
    pub code: String,
    /// Code system URI
    pub system: String,
    /// Code system version (optional)
    pub version: Option<String>,
    /// Display string (optional)
    pub display: Option<String>,
}

impl CqlCode {
    /// Create a new code
    pub fn new(
        code: impl Into<String>,
        system: impl Into<String>,
        version: Option<impl Into<String>>,
        display: Option<impl Into<String>>,
    ) -> Self {
        Self {
            code: code.into(),
            system: system.into(),
            version: version.map(Into::into),
            display: display.map(Into::into),
        }
    }

    /// Check if this code is equivalent to another
    /// (same code and system, version may differ)
    pub fn is_equivalent(&self, other: &Self) -> bool {
        self.code == other.code && self.system == other.system
    }
}

impl fmt::Display for CqlCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Code '{}' from \"{}\"", self.code, self.system)?;
        if let Some(display) = &self.display {
            write!(f, " display '{}'", display)?;
        }
        Ok(())
    }
}

/// CQL Concept - a collection of equivalent codes
///
/// Concepts represent clinical concepts that may be encoded
/// in multiple code systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CqlConcept {
    /// Codes in this concept
    pub codes: SmallVec<[CqlCode; 2]>,
    /// Display string (optional)
    pub display: Option<String>,
}

impl CqlConcept {
    /// Create a new concept
    pub fn new(codes: impl IntoIterator<Item = CqlCode>, display: Option<impl Into<String>>) -> Self {
        Self {
            codes: codes.into_iter().collect(),
            display: display.map(Into::into),
        }
    }

    /// Create a concept from a single code
    pub fn from_code(code: CqlCode) -> Self {
        let display = code.display.clone();
        Self {
            codes: smallvec::smallvec![code],
            display,
        }
    }

    /// Check if this concept contains an equivalent code
    pub fn contains_equivalent(&self, code: &CqlCode) -> bool {
        self.codes.iter().any(|c| c.is_equivalent(code))
    }
}

impl fmt::Display for CqlConcept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Concept {{")?;
        for (i, code) in self.codes.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", code)?;
        }
        write!(f, "}}")?;
        if let Some(display) = &self.display {
            write!(f, " display '{}'", display)?;
        }
        Ok(())
    }
}

// ============================================================================
// Collection Types
// ============================================================================

/// CQL List - ordered collection of values
///
/// Lists are the primary collection type in CQL. They are ordered
/// and may contain duplicates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CqlList {
    /// Element type
    pub element_type: CqlType,
    /// List elements
    pub elements: Vec<CqlValue>,
}

impl CqlList {
    /// Create a new empty list with the specified element type
    pub fn new(element_type: CqlType) -> Self {
        Self {
            element_type,
            elements: Vec::new(),
        }
    }

    /// Create a list from elements, inferring the type
    pub fn from_elements(elements: Vec<CqlValue>) -> Self {
        let element_type = if elements.is_empty() {
            CqlType::Any
        } else {
            // Find common type (simplified - would need full type unification)
            elements[0].get_type()
        };
        Self {
            element_type,
            elements,
        }
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Get an element by index (0-based)
    pub fn get(&self, index: usize) -> Option<&CqlValue> {
        self.elements.get(index)
    }

    /// Get the first element
    pub fn first(&self) -> Option<&CqlValue> {
        self.elements.first()
    }

    /// Get the last element
    pub fn last(&self) -> Option<&CqlValue> {
        self.elements.last()
    }

    /// Iterate over elements
    pub fn iter(&self) -> impl Iterator<Item = &CqlValue> {
        self.elements.iter()
    }
}

impl PartialEq for CqlList {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl Eq for CqlList {}

impl fmt::Display for CqlList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, elem) in self.elements.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;  // Space after comma for spec compliance
            }
            write!(f, "{}", elem)?;
        }
        write!(f, "}}")
    }
}

/// CQL Interval - range between two points
///
/// Intervals represent a range of values. The point type must be
/// ordered (Integer, Decimal, Date, DateTime, Time, Quantity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CqlInterval {
    /// Point type for this interval
    pub point_type: CqlType,
    /// Low bound (None for unbounded)
    pub low: Option<Box<CqlValue>>,
    /// Whether low is closed (inclusive)
    pub low_closed: bool,
    /// High bound (None for unbounded)
    pub high: Option<Box<CqlValue>>,
    /// Whether high is closed (inclusive)
    pub high_closed: bool,
}

impl CqlInterval {
    /// Create a new interval
    pub fn new(
        point_type: CqlType,
        low: Option<CqlValue>,
        low_closed: bool,
        high: Option<CqlValue>,
        high_closed: bool,
    ) -> Self {
        Self {
            point_type,
            low: low.map(Box::new),
            low_closed,
            high: high.map(Box::new),
            high_closed,
        }
    }

    /// Create a closed interval [low, high]
    pub fn closed(point_type: CqlType, low: CqlValue, high: CqlValue) -> Self {
        Self::new(point_type, Some(low), true, Some(high), true)
    }

    /// Create an open interval (low, high)
    pub fn open(point_type: CqlType, low: CqlValue, high: CqlValue) -> Self {
        Self::new(point_type, Some(low), false, Some(high), false)
    }

    /// Create a half-open interval [low, high)
    pub fn closed_open(point_type: CqlType, low: CqlValue, high: CqlValue) -> Self {
        Self::new(point_type, Some(low), true, Some(high), false)
    }

    /// Create a half-open interval (low, high]
    pub fn open_closed(point_type: CqlType, low: CqlValue, high: CqlValue) -> Self {
        Self::new(point_type, Some(low), false, Some(high), true)
    }

    /// Check if this is a point interval (low == high and both closed)
    pub fn is_point(&self) -> bool {
        match (&self.low, &self.high) {
            (Some(l), Some(h)) => self.low_closed && self.high_closed && l == h,
            _ => false,
        }
    }

    /// Get the low bound (None if unbounded or null)
    pub fn low(&self) -> Option<&CqlValue> {
        match self.low.as_deref() {
            Some(CqlValue::Null) => None, // null boundary = unbounded
            other => other,
        }
    }

    /// Get the high bound (None if unbounded or null)
    pub fn high(&self) -> Option<&CqlValue> {
        match self.high.as_deref() {
            Some(CqlValue::Null) => None, // null boundary = unbounded
            other => other,
        }
    }

    /// Get the raw low bound (including null values)
    pub fn low_raw(&self) -> Option<&CqlValue> {
        self.low.as_deref()
    }

    /// Get the raw high bound (including null values)
    pub fn high_raw(&self) -> Option<&CqlValue> {
        self.high.as_deref()
    }
}

impl PartialEq for CqlInterval {
    fn eq(&self, other: &Self) -> bool {
        self.low == other.low
            && self.low_closed == other.low_closed
            && self.high == other.high
            && self.high_closed == other.high_closed
    }
}

impl Eq for CqlInterval {}

impl fmt::Display for CqlInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format as "[ low, high ]" with spaces inside brackets to match CQL test expectations
        // Note: "Interval" prefix is added by the test runner
        if self.low_closed {
            write!(f, "[ ")?;
        } else {
            write!(f, "( ")?;
        }

        match &self.low {
            Some(l) => write!(f, "{}", l)?,
            None => write!(f, "null")?,
        }

        write!(f, ", ")?;

        match &self.high {
            Some(h) => write!(f, "{}", h)?,
            None => write!(f, "null")?,
        }

        if self.high_closed {
            write!(f, " ]")
        } else {
            write!(f, " )")
        }
    }
}

/// CQL Tuple - record with named elements
///
/// Tuples are anonymous record types with named elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CqlTuple {
    /// Named elements (insertion order preserved)
    pub elements: IndexMap<String, CqlValue>,
}

impl CqlTuple {
    /// Create a new empty tuple
    pub fn new() -> Self {
        Self {
            elements: IndexMap::new(),
        }
    }

    /// Create a tuple from an iterator of (name, value) pairs
    pub fn from_elements(elements: impl IntoIterator<Item = (impl Into<String>, CqlValue)>) -> Self {
        Self {
            elements: elements
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        }
    }

    /// Get an element by name
    pub fn get(&self, name: &str) -> Option<&CqlValue> {
        self.elements.get(name)
    }

    /// Set an element
    pub fn set(&mut self, name: impl Into<String>, value: CqlValue) {
        self.elements.insert(name.into(), value);
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the tuple is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Iterate over elements
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CqlValue)> {
        self.elements.iter()
    }
}

impl Default for CqlTuple {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for CqlTuple {
    fn eq(&self, other: &Self) -> bool {
        if self.elements.len() != other.elements.len() {
            return false;
        }
        self.elements
            .iter()
            .all(|(k, v)| other.elements.get(k).map_or(false, |ov| v == ov))
    }
}

impl Eq for CqlTuple {}

impl fmt::Display for CqlTuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tuple {{")?;
        for (i, (name, value)) in self.elements.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", name, value)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cql_date_precision() {
        let full = CqlDate::new(2024, 1, 15);
        assert_eq!(full.precision(), DateTimePrecision::Day);

        let year_month = CqlDate::year_month(2024, 1);
        assert_eq!(year_month.precision(), DateTimePrecision::Month);

        let year_only = CqlDate::year_only(2024);
        assert_eq!(year_only.precision(), DateTimePrecision::Year);
    }

    #[test]
    fn test_cql_date_display() {
        let date = CqlDate::new(2024, 1, 15);
        assert_eq!(date.to_string(), "2024-01-15");

        let year_month = CqlDate::year_month(2024, 1);
        assert_eq!(year_month.to_string(), "2024-01");

        let year_only = CqlDate::year_only(2024);
        assert_eq!(year_only.to_string(), "2024");
    }

    #[test]
    fn test_cql_quantity() {
        let q1 = CqlQuantity::new(Decimal::new(100, 0), "mg");
        assert_eq!(q1.to_string(), "100 'mg'");

        let q2 = CqlQuantity::unitless(Decimal::new(42, 0));
        assert_eq!(q2.to_string(), "42");
    }

    #[test]
    fn test_cql_interval() {
        let interval = CqlInterval::closed(
            CqlType::Integer,
            CqlValue::integer(1),
            CqlValue::integer(10),
        );
        assert_eq!(interval.to_string(), "[ 1, 10 ]");

        let open = CqlInterval::open(
            CqlType::Integer,
            CqlValue::integer(1),
            CqlValue::integer(10),
        );
        assert_eq!(open.to_string(), "( 1, 10 )");
    }

    #[test]
    fn test_cql_list() {
        let list = CqlList::from_elements(vec![
            CqlValue::integer(1),
            CqlValue::integer(2),
            CqlValue::integer(3),
        ]);
        assert_eq!(list.len(), 3);
        assert_eq!(list.to_string(), "{1, 2, 3}");
    }

    #[test]
    fn test_cql_tuple() {
        let tuple = CqlTuple::from_elements([
            ("name", CqlValue::string("John")),
            ("age", CqlValue::integer(30)),
        ]);
        assert_eq!(tuple.len(), 2);
        assert!(tuple.get("name").is_some());
        assert!(tuple.get("age").is_some());
    }

    #[test]
    fn test_cql_code() {
        let code = CqlCode::new(
            "12345",
            "http://snomed.info/sct",
            None::<&str>,
            Some("Test Code"),
        );
        assert!(code.to_string().contains("12345"));
        assert!(code.to_string().contains("snomed"));
    }
}
