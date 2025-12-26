//! Literal AST nodes for CQL

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A literal value in CQL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    /// Null literal
    Null,
    /// Boolean literal (true/false)
    Boolean(bool),
    /// Integer literal (32-bit signed)
    Integer(i32),
    /// Long literal (64-bit signed, suffix 'L')
    Long(i64),
    /// Decimal literal (arbitrary precision)
    Decimal(Decimal),
    /// String literal
    String(String),
    /// Date literal (@YYYY-MM-DD)
    Date(DateLiteral),
    /// DateTime literal (@YYYY-MM-DDThh:mm:ss.fff(+|-)hh:mm)
    DateTime(DateTimeLiteral),
    /// Time literal (@Thh:mm:ss.fff)
    Time(TimeLiteral),
    /// Quantity literal (number with unit)
    Quantity(QuantityLiteral),
    /// Ratio literal (quantity:quantity)
    Ratio(RatioLiteral),
}

/// Date literal components
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateLiteral {
    /// Year (required)
    pub year: i32,
    /// Month (optional)
    pub month: Option<u8>,
    /// Day (optional)
    pub day: Option<u8>,
}

impl DateLiteral {
    pub fn new(year: i32) -> Self {
        Self {
            year,
            month: None,
            day: None,
        }
    }

    pub fn with_month(mut self, month: u8) -> Self {
        self.month = Some(month);
        self
    }

    pub fn with_day(mut self, day: u8) -> Self {
        self.day = Some(day);
        self
    }

    /// Get the precision level
    pub fn precision(&self) -> DatePrecision {
        if self.day.is_some() {
            DatePrecision::Day
        } else if self.month.is_some() {
            DatePrecision::Month
        } else {
            DatePrecision::Year
        }
    }
}

/// DateTime literal components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateTimeLiteral {
    /// Date portion
    pub date: DateLiteral,
    /// Hour (optional)
    pub hour: Option<u8>,
    /// Minute (optional)
    pub minute: Option<u8>,
    /// Second (optional)
    pub second: Option<u8>,
    /// Millisecond (optional)
    pub millisecond: Option<u16>,
    /// Timezone offset in minutes (optional)
    pub timezone_offset: Option<i16>,
}

impl DateTimeLiteral {
    pub fn new(date: DateLiteral) -> Self {
        Self {
            date,
            hour: None,
            minute: None,
            second: None,
            millisecond: None,
            timezone_offset: None,
        }
    }

    pub fn with_time(mut self, hour: u8, minute: u8) -> Self {
        self.hour = Some(hour);
        self.minute = Some(minute);
        self
    }

    pub fn with_second(mut self, second: u8) -> Self {
        self.second = Some(second);
        self
    }

    pub fn with_millisecond(mut self, millisecond: u16) -> Self {
        self.millisecond = Some(millisecond);
        self
    }

    pub fn with_timezone(mut self, offset_minutes: i16) -> Self {
        self.timezone_offset = Some(offset_minutes);
        self
    }

    /// Get the precision level
    pub fn precision(&self) -> DateTimePrecision {
        if self.millisecond.is_some() {
            DateTimePrecision::Millisecond
        } else if self.second.is_some() {
            DateTimePrecision::Second
        } else if self.minute.is_some() {
            DateTimePrecision::Minute
        } else if self.hour.is_some() {
            DateTimePrecision::Hour
        } else {
            match self.date.precision() {
                DatePrecision::Day => DateTimePrecision::Day,
                DatePrecision::Month => DateTimePrecision::Month,
                DatePrecision::Year => DateTimePrecision::Year,
            }
        }
    }
}

/// Time literal components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeLiteral {
    /// Hour (required)
    pub hour: u8,
    /// Minute (optional)
    pub minute: Option<u8>,
    /// Second (optional)
    pub second: Option<u8>,
    /// Millisecond (optional)
    pub millisecond: Option<u16>,
}

impl TimeLiteral {
    pub fn new(hour: u8) -> Self {
        Self {
            hour,
            minute: None,
            second: None,
            millisecond: None,
        }
    }

    pub fn with_minute(mut self, minute: u8) -> Self {
        self.minute = Some(minute);
        self
    }

    pub fn with_second(mut self, second: u8) -> Self {
        self.second = Some(second);
        self
    }

    pub fn with_millisecond(mut self, millisecond: u16) -> Self {
        self.millisecond = Some(millisecond);
        self
    }

    /// Get the precision level
    pub fn precision(&self) -> TimePrecision {
        if self.millisecond.is_some() {
            TimePrecision::Millisecond
        } else if self.second.is_some() {
            TimePrecision::Second
        } else if self.minute.is_some() {
            TimePrecision::Minute
        } else {
            TimePrecision::Hour
        }
    }
}

/// Quantity literal (value with unit)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantityLiteral {
    /// Numeric value
    pub value: Decimal,
    /// Unit string (UCUM)
    pub unit: Option<String>,
}

impl QuantityLiteral {
    pub fn new(value: Decimal) -> Self {
        Self { value, unit: None }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

/// Ratio literal (two quantities)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioLiteral {
    /// Numerator quantity
    pub numerator: QuantityLiteral,
    /// Denominator quantity
    pub denominator: QuantityLiteral,
}

impl RatioLiteral {
    pub fn new(numerator: QuantityLiteral, denominator: QuantityLiteral) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// Date precision levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatePrecision {
    Year,
    Month,
    Day,
}

/// DateTime precision levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateTimePrecision {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

/// Time precision levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePrecision {
    Hour,
    Minute,
    Second,
    Millisecond,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_precision() {
        assert_eq!(DateLiteral::new(2024).precision(), DatePrecision::Year);
        assert_eq!(
            DateLiteral::new(2024).with_month(1).precision(),
            DatePrecision::Month
        );
        assert_eq!(
            DateLiteral::new(2024).with_month(1).with_day(15).precision(),
            DatePrecision::Day
        );
    }
}
