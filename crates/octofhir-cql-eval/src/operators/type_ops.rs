//! Type Operators for CQL
//!
//! Implements: As, Is, Convert, CanConvert, ToBoolean, ToChars, ToConcept,
//! ToDate, ToDateTime, ToDecimal, ToInteger, ToLong, ToList, ToQuantity,
//! ToRatio, ToString, ToTime, ConvertsToXxx

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use octofhir_cql_elm::{AsExpression, CanConvertExpression, ConvertExpression, IsExpression, UnaryExpression};
use octofhir_cql_types::{
    CqlCode, CqlConcept, CqlDate, CqlDateTime, CqlList, CqlQuantity, CqlRatio, CqlTime, CqlType,
    CqlValue,
};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

impl CqlEngine {
    /// Evaluate As operator (type cast)
    pub fn eval_as(&self, expr: &AsExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        // For now, simplified implementation - just return the operand
        // Full type checking would require type specifier conversion
        if expr.strict.unwrap_or(false) {
            // Strict mode - would need full type checking
            Ok(operand)
        } else {
            // Non-strict mode
            Ok(operand)
        }
    }

    /// Evaluate Is operator (type check)
    pub fn eval_is(&self, expr: &IsExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Boolean(false));
        }

        // Simplified - would need type specifier to fully implement
        Ok(CqlValue::Boolean(true))
    }

    /// Evaluate Convert operator
    pub fn eval_convert(&self, expr: &ConvertExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        // Simplified - return operand as-is for now
        // Full conversion would require type specifier parsing
        Ok(operand)
    }

    /// Evaluate CanConvert operator
    pub fn eval_can_convert(&self, expr: &CanConvertExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Boolean(true)); // Null can convert to any type
        }

        // Simplified - assume conversion is possible
        Ok(CqlValue::Boolean(true))
    }

    /// Evaluate ToBoolean
    pub fn eval_to_boolean(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_boolean(&operand)
    }

    /// Evaluate ToChars - converts string to list of characters
    pub fn eval_to_chars(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::String(s) => {
                let chars: Vec<CqlValue> = s.chars().map(|c| CqlValue::String(c.to_string())).collect();
                Ok(CqlValue::List(CqlList {
                    element_type: CqlType::String,
                    elements: chars,
                }))
            }
            _ => Err(EvalError::conversion_error(operand.get_type().name(), "List<String>")),
        }
    }

    /// Evaluate ToConcept
    pub fn eval_to_concept(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Code(code) => Ok(CqlValue::Concept(CqlConcept::from_code(code.clone()))),
            CqlValue::Concept(c) => Ok(CqlValue::Concept(c.clone())),
            _ => Err(EvalError::conversion_error(operand.get_type().name(), "Concept")),
        }
    }

    /// Evaluate ToDate
    pub fn eval_to_date(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_date(&operand)
    }

    /// Evaluate ToDateTime
    pub fn eval_to_datetime(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_datetime(&operand)
    }

    /// Evaluate ToDecimal
    pub fn eval_to_decimal(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_decimal(&operand)
    }

    /// Evaluate ToInteger
    pub fn eval_to_integer(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_integer(&operand)
    }

    /// Evaluate ToLong
    pub fn eval_to_long(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_long(&operand)
    }

    /// Evaluate ToList
    pub fn eval_to_list(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::List(CqlList::new(CqlType::Any))),
            CqlValue::List(l) => Ok(CqlValue::List(l.clone())),
            _ => Ok(CqlValue::List(CqlList::from_elements(vec![operand]))),
        }
    }

    /// Evaluate ToQuantity
    pub fn eval_to_quantity(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_quantity(&operand)
    }

    /// Evaluate ToRatio
    pub fn eval_to_ratio(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Ratio(r) => Ok(CqlValue::Ratio(r.clone())),
            CqlValue::String(s) => parse_ratio_string(s),
            _ => Err(EvalError::conversion_error(operand.get_type().name(), "Ratio")),
        }
    }

    /// Evaluate ToString
    pub fn eval_to_string(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_string(&operand)
    }

    /// Evaluate ToTime
    pub fn eval_to_time(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        to_time(&operand)
    }

    /// Evaluate ConvertsToBoolean
    pub fn eval_converts_to_boolean(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_boolean(&operand).is_ok()))
    }

    /// Evaluate ConvertsToDate
    pub fn eval_converts_to_date(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_date(&operand).is_ok() && !to_date(&operand)?.is_null()))
    }

    /// Evaluate ConvertsToDateTime
    pub fn eval_converts_to_datetime(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_datetime(&operand).is_ok() && !to_datetime(&operand)?.is_null()))
    }

    /// Evaluate ConvertsToDecimal
    pub fn eval_converts_to_decimal(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_decimal(&operand).is_ok() && !to_decimal(&operand)?.is_null()))
    }

    /// Evaluate ConvertsToInteger
    pub fn eval_converts_to_integer(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_integer(&operand).is_ok() && !to_integer(&operand)?.is_null()))
    }

    /// Evaluate ConvertsToLong
    pub fn eval_converts_to_long(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_long(&operand).is_ok() && !to_long(&operand)?.is_null()))
    }

    /// Evaluate ConvertsToQuantity
    pub fn eval_converts_to_quantity(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_quantity(&operand).is_ok() && !to_quantity(&operand)?.is_null()))
    }

    /// Evaluate ConvertsToRatio
    pub fn eval_converts_to_ratio(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        match &operand {
            CqlValue::Ratio(_) => Ok(CqlValue::Boolean(true)),
            CqlValue::String(s) => Ok(CqlValue::Boolean(parse_ratio_string(s).is_ok())),
            _ => Ok(CqlValue::Boolean(false)),
        }
    }

    /// Evaluate ConvertsToString
    pub fn eval_converts_to_string(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        // Everything can convert to string
        Ok(CqlValue::Boolean(true))
    }

    /// Evaluate ConvertsToTime
    pub fn eval_converts_to_time(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        if operand.is_null() {
            return Ok(CqlValue::Boolean(true));
        }
        Ok(CqlValue::Boolean(to_time(&operand).is_ok() && !to_time(&operand)?.is_null()))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a value is of a given type
fn value_is_type(value: &CqlValue, target_type: &CqlType) -> bool {
    if target_type.is_any() {
        return true;
    }

    let value_type = value.get_type();
    value_type.is_subtype_of(target_type)
}

/// Check if a value can be converted to a target type
fn can_convert(value: &CqlValue, target_type: &CqlType) -> bool {
    match target_type {
        CqlType::Boolean => to_boolean(value).is_ok(),
        CqlType::Integer => to_integer(value).is_ok(),
        CqlType::Long => to_long(value).is_ok(),
        CqlType::Decimal => to_decimal(value).is_ok(),
        CqlType::String => true,
        CqlType::Date => to_date(value).is_ok(),
        CqlType::DateTime => to_datetime(value).is_ok(),
        CqlType::Time => to_time(value).is_ok(),
        CqlType::Quantity => to_quantity(value).is_ok(),
        _ => value_is_type(value, target_type),
    }
}

/// Convert a value to a target type
fn convert_value(value: &CqlValue, target_type: &CqlType) -> EvalResult<CqlValue> {
    match target_type {
        CqlType::Boolean => to_boolean(value),
        CqlType::Integer => to_integer(value),
        CqlType::Long => to_long(value),
        CqlType::Decimal => to_decimal(value),
        CqlType::String => to_string(value),
        CqlType::Date => to_date(value),
        CqlType::DateTime => to_datetime(value),
        CqlType::Time => to_time(value),
        CqlType::Quantity => to_quantity(value),
        _ => {
            if value_is_type(value, target_type) {
                Ok(value.clone())
            } else {
                Err(EvalError::conversion_error(
                    value.get_type().name(),
                    target_type.name(),
                ))
            }
        }
    }
}

/// Convert to Boolean
fn to_boolean(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Boolean(b) => Ok(CqlValue::Boolean(*b)),
        CqlValue::Integer(i) => Ok(CqlValue::Boolean(*i != 0)),
        CqlValue::Long(l) => Ok(CqlValue::Boolean(*l != 0)),
        CqlValue::Decimal(d) => Ok(CqlValue::Boolean(!d.is_zero())),
        CqlValue::String(s) => {
            let lower = s.to_lowercase();
            match lower.as_str() {
                "true" | "t" | "yes" | "y" | "1" => Ok(CqlValue::Boolean(true)),
                "false" | "f" | "no" | "n" | "0" => Ok(CqlValue::Boolean(false)),
                _ => Ok(CqlValue::Null),
            }
        }
        _ => Err(EvalError::conversion_error(value.get_type().name(), "Boolean")),
    }
}

/// Convert to Integer
fn to_integer(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Integer(i) => Ok(CqlValue::Integer(*i)),
        CqlValue::Long(l) => {
            if *l >= i32::MIN as i64 && *l <= i32::MAX as i64 {
                Ok(CqlValue::Integer(*l as i32))
            } else {
                Ok(CqlValue::Null)
            }
        }
        CqlValue::Decimal(d) => {
            if let Some(i) = d.to_i32() {
                Ok(CqlValue::Integer(i))
            } else {
                Ok(CqlValue::Null)
            }
        }
        CqlValue::Boolean(b) => Ok(CqlValue::Integer(if *b { 1 } else { 0 })),
        CqlValue::String(s) => {
            match s.trim().parse::<i32>() {
                Ok(i) => Ok(CqlValue::Integer(i)),
                Err(_) => Ok(CqlValue::Null),
            }
        }
        _ => Err(EvalError::conversion_error(value.get_type().name(), "Integer")),
    }
}

/// Convert to Long
fn to_long(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Integer(i) => Ok(CqlValue::Long(*i as i64)),
        CqlValue::Long(l) => Ok(CqlValue::Long(*l)),
        CqlValue::Decimal(d) => {
            if let Some(l) = d.to_i64() {
                Ok(CqlValue::Long(l))
            } else {
                Ok(CqlValue::Null)
            }
        }
        CqlValue::Boolean(b) => Ok(CqlValue::Long(if *b { 1 } else { 0 })),
        CqlValue::String(s) => {
            match s.trim().parse::<i64>() {
                Ok(l) => Ok(CqlValue::Long(l)),
                Err(_) => Ok(CqlValue::Null),
            }
        }
        _ => Err(EvalError::conversion_error(value.get_type().name(), "Long")),
    }
}

/// Convert to Decimal
fn to_decimal(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Integer(i) => Ok(CqlValue::Decimal(Decimal::from(*i))),
        CqlValue::Long(l) => Ok(CqlValue::Decimal(Decimal::from(*l))),
        CqlValue::Decimal(d) => Ok(CqlValue::Decimal(*d)),
        CqlValue::Boolean(b) => Ok(CqlValue::Decimal(if *b { Decimal::ONE } else { Decimal::ZERO })),
        CqlValue::String(s) => {
            match Decimal::from_str(s.trim()) {
                Ok(d) => Ok(CqlValue::Decimal(d)),
                Err(_) => Ok(CqlValue::Null),
            }
        }
        CqlValue::Quantity(q) => Ok(CqlValue::Decimal(q.value)),
        _ => Err(EvalError::conversion_error(value.get_type().name(), "Decimal")),
    }
}

/// Convert to String
fn to_string(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Boolean(b) => Ok(CqlValue::String(b.to_string())),
        CqlValue::Integer(i) => Ok(CqlValue::String(i.to_string())),
        CqlValue::Long(l) => Ok(CqlValue::String(l.to_string())),
        CqlValue::Decimal(d) => Ok(CqlValue::String(d.to_string())),
        CqlValue::String(s) => Ok(CqlValue::String(s.clone())),
        CqlValue::Date(d) => Ok(CqlValue::String(d.to_string())),
        CqlValue::DateTime(dt) => Ok(CqlValue::String(dt.to_string())),
        CqlValue::Time(t) => Ok(CqlValue::String(t.to_string())),
        CqlValue::Quantity(q) => Ok(CqlValue::String(q.to_string())),
        CqlValue::Ratio(r) => Ok(CqlValue::String(r.to_string())),
        CqlValue::Code(c) => Ok(CqlValue::String(c.to_string())),
        CqlValue::Concept(c) => Ok(CqlValue::String(c.to_string())),
        _ => Ok(CqlValue::String(format!("{}", value))),
    }
}

/// Convert to Date
fn to_date(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Date(d) => Ok(CqlValue::Date(d.clone())),
        CqlValue::DateTime(dt) => Ok(CqlValue::Date(dt.date())),
        CqlValue::String(s) => {
            match CqlDate::parse(s.trim()) {
                Some(d) => Ok(CqlValue::Date(d)),
                None => Ok(CqlValue::Null),
            }
        }
        _ => Err(EvalError::conversion_error(value.get_type().name(), "Date")),
    }
}

/// Convert to DateTime
fn to_datetime(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Date(d) => Ok(CqlValue::DateTime(CqlDateTime::from_date(d.clone()))),
        CqlValue::DateTime(dt) => Ok(CqlValue::DateTime(dt.clone())),
        CqlValue::String(s) => {
            // Try to parse ISO 8601 datetime
            let trimmed = s.trim();
            // Simplified parsing - just try date first
            if let Some(d) = CqlDate::parse(trimmed) {
                return Ok(CqlValue::DateTime(CqlDateTime::from_date(d)));
            }
            Ok(CqlValue::Null)
        }
        _ => Err(EvalError::conversion_error(value.get_type().name(), "DateTime")),
    }
}

/// Convert to Time
fn to_time(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Time(t) => Ok(CqlValue::Time(t.clone())),
        CqlValue::DateTime(dt) => {
            match dt.time() {
                Some(t) => Ok(CqlValue::Time(t)),
                None => Ok(CqlValue::Null),
            }
        }
        CqlValue::String(s) => {
            // Try to parse time string HH:MM:SS.mmm
            let parts: Vec<&str> = s.trim().split(':').collect();
            if parts.is_empty() {
                return Ok(CqlValue::Null);
            }

            let hour: u8 = match parts[0].parse() {
                Ok(h) if h < 24 => h,
                _ => return Ok(CqlValue::Null),
            };

            let minute = if parts.len() > 1 {
                match parts[1].parse::<u8>() {
                    Ok(m) if m < 60 => Some(m),
                    _ => return Ok(CqlValue::Null),
                }
            } else {
                None
            };

            let (second, millisecond) = if parts.len() > 2 {
                let sec_parts: Vec<&str> = parts[2].split('.').collect();
                let sec: u8 = match sec_parts[0].parse() {
                    Ok(s) if s < 60 => s,
                    _ => return Ok(CqlValue::Null),
                };
                let ms: Option<u16> = if sec_parts.len() > 1 {
                    sec_parts[1].parse().ok()
                } else {
                    None
                };
                (Some(sec), ms)
            } else {
                (None, None)
            };

            Ok(CqlValue::Time(CqlTime {
                hour,
                minute,
                second,
                millisecond,
            }))
        }
        _ => Err(EvalError::conversion_error(value.get_type().name(), "Time")),
    }
}

/// Convert to Quantity
fn to_quantity(value: &CqlValue) -> EvalResult<CqlValue> {
    match value {
        CqlValue::Null => Ok(CqlValue::Null),
        CqlValue::Quantity(q) => Ok(CqlValue::Quantity(q.clone())),
        CqlValue::Integer(i) => Ok(CqlValue::Quantity(CqlQuantity::unitless(Decimal::from(*i)))),
        CqlValue::Long(l) => Ok(CqlValue::Quantity(CqlQuantity::unitless(Decimal::from(*l)))),
        CqlValue::Decimal(d) => Ok(CqlValue::Quantity(CqlQuantity::unitless(*d))),
        CqlValue::String(s) => parse_quantity_string(s),
        CqlValue::Ratio(r) => {
            // Convert ratio to decimal quantity
            if r.denominator.value.is_zero() {
                Ok(CqlValue::Null)
            } else {
                let value = r.numerator.value / r.denominator.value;
                // Combine units
                let unit = match (&r.numerator.unit, &r.denominator.unit) {
                    (Some(n), Some(d)) => Some(format!("{}/{}", n, d)),
                    (Some(n), None) => Some(n.clone()),
                    (None, Some(d)) => Some(format!("1/{}", d)),
                    (None, None) => None,
                };
                Ok(CqlValue::Quantity(CqlQuantity { value, unit }))
            }
        }
        _ => Err(EvalError::conversion_error(value.get_type().name(), "Quantity")),
    }
}

/// Parse a quantity from a string like "10 'mg'"
fn parse_quantity_string(s: &str) -> EvalResult<CqlValue> {
    let trimmed = s.trim();

    // Try to find where the number ends
    let mut num_end = 0;
    let chars: Vec<char> = trimmed.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+' {
            num_end = i + 1;
        } else if !c.is_whitespace() {
            break;
        }
    }

    if num_end == 0 {
        return Ok(CqlValue::Null);
    }

    let num_str = &trimmed[..num_end];
    let unit_str = trimmed[num_end..].trim();

    let value = match Decimal::from_str(num_str) {
        Ok(d) => d,
        Err(_) => return Ok(CqlValue::Null),
    };

    // Parse unit (may be in single quotes)
    let unit = if unit_str.is_empty() {
        None
    } else if unit_str.starts_with('\'') && unit_str.ends_with('\'') && unit_str.len() > 2 {
        Some(unit_str[1..unit_str.len()-1].to_string())
    } else {
        Some(unit_str.to_string())
    };

    Ok(CqlValue::Quantity(CqlQuantity { value, unit }))
}

/// Parse a ratio from a string like "1 'mg' : 2 'mL'"
fn parse_ratio_string(s: &str) -> EvalResult<CqlValue> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(EvalError::conversion_error("String", "Ratio"));
    }

    let numerator = match parse_quantity_string(parts[0])? {
        CqlValue::Quantity(q) => q,
        _ => return Err(EvalError::conversion_error("String", "Ratio")),
    };

    let denominator = match parse_quantity_string(parts[1])? {
        CqlValue::Quantity(q) => q,
        _ => return Err(EvalError::conversion_error("String", "Ratio")),
    };

    Ok(CqlValue::Ratio(CqlRatio::new(numerator, denominator)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_boolean() {
        assert_eq!(to_boolean(&CqlValue::Boolean(true)).unwrap(), CqlValue::Boolean(true));
        assert_eq!(to_boolean(&CqlValue::Integer(1)).unwrap(), CqlValue::Boolean(true));
        assert_eq!(to_boolean(&CqlValue::Integer(0)).unwrap(), CqlValue::Boolean(false));
        assert_eq!(to_boolean(&CqlValue::String("true".to_string())).unwrap(), CqlValue::Boolean(true));
        assert_eq!(to_boolean(&CqlValue::String("false".to_string())).unwrap(), CqlValue::Boolean(false));
    }

    #[test]
    fn test_to_integer() {
        assert_eq!(to_integer(&CqlValue::Integer(42)).unwrap(), CqlValue::Integer(42));
        assert_eq!(to_integer(&CqlValue::Long(42)).unwrap(), CqlValue::Integer(42));
        assert_eq!(to_integer(&CqlValue::String("42".to_string())).unwrap(), CqlValue::Integer(42));
        assert!(to_integer(&CqlValue::String("not a number".to_string())).unwrap().is_null());
    }

    #[test]
    fn test_to_decimal() {
        assert_eq!(
            to_decimal(&CqlValue::Integer(42)).unwrap(),
            CqlValue::Decimal(Decimal::from(42))
        );
        assert_eq!(
            to_decimal(&CqlValue::String("3.14".to_string())).unwrap(),
            CqlValue::Decimal(Decimal::from_str("3.14").unwrap())
        );
    }

    #[test]
    fn test_to_string() {
        assert_eq!(to_string(&CqlValue::Integer(42)).unwrap(), CqlValue::String("42".to_string()));
        assert_eq!(to_string(&CqlValue::Boolean(true)).unwrap(), CqlValue::String("true".to_string()));
    }

    #[test]
    fn test_to_date() {
        let result = to_date(&CqlValue::String("2024-01-15".to_string())).unwrap();
        assert!(matches!(result, CqlValue::Date(_)));
    }

    #[test]
    fn test_parse_quantity() {
        let result = parse_quantity_string("10 'mg'").unwrap();
        if let CqlValue::Quantity(q) = result {
            assert_eq!(q.value, Decimal::from(10));
            assert_eq!(q.unit, Some("mg".to_string()));
        } else {
            panic!("Expected Quantity");
        }
    }
}
