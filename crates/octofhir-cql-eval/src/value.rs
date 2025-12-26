//! CQL runtime values

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// CQL runtime value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CqlValue {
    /// Null value
    Null,
    /// Boolean value
    Boolean(bool),
    /// Integer value
    Integer(i32),
    /// Long value
    Long(i64),
    /// Decimal value
    Decimal(Decimal),
    /// String value
    String(String),
    /// Date value
    Date(CqlDate),
    /// DateTime value
    DateTime(CqlDateTime),
    /// Time value
    Time(CqlTime),
    /// Quantity value
    Quantity(CqlQuantity),
    /// Ratio value
    Ratio(CqlRatio),
    /// Code value
    Code(CqlCode),
    /// Concept value
    Concept(CqlConcept),
    /// List value
    List(Vec<CqlValue>),
    /// Tuple value
    Tuple(HashMap<String, CqlValue>),
    /// Interval value
    Interval(CqlInterval),
    /// FHIR resource (JSON)
    Resource(JsonValue),
}

impl CqlValue {
    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if value is truthy (for boolean operations)
    pub fn is_truthy(&self) -> bool {
        matches!(self, Self::Boolean(true))
    }

    /// Convert to boolean if possible
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert to integer if possible
    pub fn as_integer(&self) -> Option<i32> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Convert to string if possible
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

/// CQL Date value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CqlDate {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

/// CQL DateTime value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CqlDateTime {
    pub date: CqlDate,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub millisecond: Option<u16>,
    pub timezone_offset: Option<i16>,
}

/// CQL Time value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CqlTime {
    pub hour: u8,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub millisecond: Option<u16>,
}

/// CQL Quantity value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CqlQuantity {
    pub value: Decimal,
    pub unit: Option<String>,
}

/// CQL Ratio value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CqlRatio {
    pub numerator: CqlQuantity,
    pub denominator: CqlQuantity,
}

/// CQL Code value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CqlCode {
    pub code: String,
    pub system: Option<String>,
    pub version: Option<String>,
    pub display: Option<String>,
}

/// CQL Concept value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CqlConcept {
    pub codes: Vec<CqlCode>,
    pub display: Option<String>,
}

/// CQL Interval value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CqlInterval {
    pub low: Option<Box<CqlValue>>,
    pub low_closed: bool,
    pub high: Option<Box<CqlValue>>,
    pub high_closed: bool,
}
