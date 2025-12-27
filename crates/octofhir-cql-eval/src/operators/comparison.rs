//! Comparison Operators for CQL
//!
//! Implements: Equal, NotEqual, Equivalent, Less, Greater, LessOrEqual, GreaterOrEqual
//! All comparison operators implement three-valued logic (true/false/null)

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use octofhir_cql_elm::BinaryExpression;
use octofhir_cql_types::{CqlCode, CqlConcept, CqlInterval, CqlList, CqlQuantity, CqlTuple, CqlType, CqlValue};
use rust_decimal::Decimal;
use std::cmp::Ordering;

impl CqlEngine {
    /// Evaluate Equal (=) operator with three-valued logic
    ///
    /// Returns null if either operand is null (unless both are null, then true)
    /// For structured types, compares all elements
    pub fn eval_equal(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        // Both null -> null (not true per CQL spec for Equal)
        if left.is_null() && right.is_null() {
            return Ok(CqlValue::Null);
        }

        // One null -> null
        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        Ok(CqlValue::Boolean(cql_equal(&left, &right)?))
    }

    /// Evaluate NotEqual (!=) operator
    ///
    /// Equivalent to Not(Equal(left, right))
    pub fn eval_not_equal(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        Ok(CqlValue::Boolean(!cql_equal(&left, &right)?))
    }

    /// Evaluate Equivalent (~) operator
    ///
    /// Unlike Equal, Equivalent handles nulls:
    /// - null ~ null -> true
    /// - null ~ non-null -> false
    /// For codes, compares code and system only
    pub fn eval_equivalent(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        // Both null -> true
        if left.is_null() && right.is_null() {
            return Ok(CqlValue::Boolean(true));
        }

        // One null -> false
        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Boolean(false));
        }

        Ok(CqlValue::Boolean(cql_equivalent(&left, &right)?))
    }

    /// Evaluate Less (<) operator
    pub fn eval_less(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match cql_compare(&left, &right)? {
            Some(Ordering::Less) => Ok(CqlValue::Boolean(true)),
            Some(_) => Ok(CqlValue::Boolean(false)),
            None => Ok(CqlValue::Null), // Uncertain comparison
        }
    }

    /// Evaluate Greater (>) operator
    pub fn eval_greater(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match cql_compare(&left, &right)? {
            Some(Ordering::Greater) => Ok(CqlValue::Boolean(true)),
            Some(_) => Ok(CqlValue::Boolean(false)),
            None => Ok(CqlValue::Null),
        }
    }

    /// Evaluate LessOrEqual (<=) operator
    pub fn eval_less_or_equal(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match cql_compare(&left, &right)? {
            Some(Ordering::Less | Ordering::Equal) => Ok(CqlValue::Boolean(true)),
            Some(Ordering::Greater) => Ok(CqlValue::Boolean(false)),
            None => Ok(CqlValue::Null),
        }
    }

    /// Evaluate GreaterOrEqual (>=) operator
    pub fn eval_greater_or_equal(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match cql_compare(&left, &right)? {
            Some(Ordering::Greater | Ordering::Equal) => Ok(CqlValue::Boolean(true)),
            Some(Ordering::Less) => Ok(CqlValue::Boolean(false)),
            None => Ok(CqlValue::Null),
        }
    }
}

/// CQL equality comparison
///
/// Returns true if values are equal, false otherwise.
/// Does NOT handle nulls - caller must check for nulls first.
pub fn cql_equal(left: &CqlValue, right: &CqlValue) -> EvalResult<bool> {
    match (left, right) {
        // Same type comparisons
        (CqlValue::Boolean(a), CqlValue::Boolean(b)) => Ok(a == b),
        (CqlValue::Integer(a), CqlValue::Integer(b)) => Ok(a == b),
        (CqlValue::Long(a), CqlValue::Long(b)) => Ok(a == b),
        (CqlValue::Decimal(a), CqlValue::Decimal(b)) => Ok(a == b),
        (CqlValue::String(a), CqlValue::String(b)) => Ok(a == b),

        // Cross-type numeric comparisons
        (CqlValue::Integer(a), CqlValue::Long(b)) => Ok((*a as i64) == *b),
        (CqlValue::Long(a), CqlValue::Integer(b)) => Ok(*a == (*b as i64)),
        (CqlValue::Integer(a), CqlValue::Decimal(b)) => Ok(Decimal::from(*a) == *b),
        (CqlValue::Decimal(a), CqlValue::Integer(b)) => Ok(*a == Decimal::from(*b)),
        (CqlValue::Long(a), CqlValue::Decimal(b)) => Ok(Decimal::from(*a) == *b),
        (CqlValue::Decimal(a), CqlValue::Long(b)) => Ok(*a == Decimal::from(*b)),

        // Date/Time comparisons
        (CqlValue::Date(a), CqlValue::Date(b)) => Ok(a == b),
        (CqlValue::DateTime(a), CqlValue::DateTime(b)) => Ok(a == b),
        (CqlValue::Time(a), CqlValue::Time(b)) => Ok(a == b),

        // Quantity comparison (same units)
        (CqlValue::Quantity(a), CqlValue::Quantity(b)) => {
            if a.unit == b.unit {
                Ok(a.value == b.value)
            } else {
                // Different units - would need UCUM conversion
                Err(EvalError::IncompatibleUnits {
                    unit1: a.unit.clone().unwrap_or_default(),
                    unit2: b.unit.clone().unwrap_or_default(),
                })
            }
        }

        // Ratio comparison
        (CqlValue::Ratio(a), CqlValue::Ratio(b)) => {
            Ok(a.numerator == b.numerator && a.denominator == b.denominator)
        }

        // Code comparison - must match code, system, and version
        (CqlValue::Code(a), CqlValue::Code(b)) => {
            Ok(a.code == b.code && a.system == b.system && a.version == b.version)
        }

        // Concept comparison - codes must match
        (CqlValue::Concept(a), CqlValue::Concept(b)) => {
            Ok(a.codes == b.codes)
        }

        // List comparison - element-wise
        (CqlValue::List(a), CqlValue::List(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (elem_a, elem_b) in a.iter().zip(b.iter()) {
                if !cql_equal(elem_a, elem_b)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }

        // Interval comparison
        (CqlValue::Interval(a), CqlValue::Interval(b)) => {
            interval_equal(a, b)
        }

        // Tuple comparison - all elements must match
        (CqlValue::Tuple(a), CqlValue::Tuple(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (name, val_a) in a.iter() {
                match b.get(name) {
                    Some(val_b) => {
                        if !cql_equal(val_a, val_b)? {
                            return Ok(false);
                        }
                    }
                    None => return Ok(false),
                }
            }
            Ok(true)
        }

        // Different types are not equal
        _ => Ok(false),
    }
}

/// CQL equivalence comparison
///
/// More lenient than equality:
/// - Codes compare only code and system (not version or display)
/// - Concepts are equivalent if any code is equivalent
pub fn cql_equivalent(left: &CqlValue, right: &CqlValue) -> EvalResult<bool> {
    match (left, right) {
        // Code equivalence - ignores version and display
        (CqlValue::Code(a), CqlValue::Code(b)) => {
            Ok(a.code == b.code && a.system == b.system)
        }

        // Concept equivalence - any equivalent code
        (CqlValue::Concept(a), CqlValue::Concept(b)) => {
            for code_a in &a.codes {
                for code_b in &b.codes {
                    if code_a.code == code_b.code && code_a.system == code_b.system {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }

        // Code to Concept
        (CqlValue::Code(a), CqlValue::Concept(b)) | (CqlValue::Concept(b), CqlValue::Code(a)) => {
            Ok(b.codes.iter().any(|c| c.code == a.code && c.system == a.system))
        }

        // String equivalence is case-insensitive
        (CqlValue::String(a), CqlValue::String(b)) => {
            Ok(a.to_lowercase() == b.to_lowercase())
        }

        // List equivalence - order matters, element equivalence
        (CqlValue::List(a), CqlValue::List(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (elem_a, elem_b) in a.iter().zip(b.iter()) {
                if !cql_equivalent(elem_a, elem_b)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }

        // Tuple equivalence
        (CqlValue::Tuple(a), CqlValue::Tuple(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (name, val_a) in a.iter() {
                match b.get(name) {
                    Some(val_b) => {
                        if !cql_equivalent(val_a, val_b)? {
                            return Ok(false);
                        }
                    }
                    None => return Ok(false),
                }
            }
            Ok(true)
        }

        // Fall back to equality for other types
        _ => cql_equal(left, right),
    }
}

/// Compare two CQL values
///
/// Returns Some(Ordering) if comparison is certain, None if uncertain
/// (e.g., partial dates with different precision)
pub fn cql_compare(left: &CqlValue, right: &CqlValue) -> EvalResult<Option<Ordering>> {
    match (left, right) {
        // Integer comparisons
        (CqlValue::Integer(a), CqlValue::Integer(b)) => Ok(Some(a.cmp(b))),
        (CqlValue::Long(a), CqlValue::Long(b)) => Ok(Some(a.cmp(b))),
        (CqlValue::Integer(a), CqlValue::Long(b)) => Ok(Some((*a as i64).cmp(b))),
        (CqlValue::Long(a), CqlValue::Integer(b)) => Ok(Some(a.cmp(&(*b as i64)))),

        // Decimal comparisons
        (CqlValue::Decimal(a), CqlValue::Decimal(b)) => Ok(Some(a.cmp(b))),
        (CqlValue::Integer(a), CqlValue::Decimal(b)) => Ok(Some(Decimal::from(*a).cmp(b))),
        (CqlValue::Decimal(a), CqlValue::Integer(b)) => Ok(Some(a.cmp(&Decimal::from(*b)))),
        (CqlValue::Long(a), CqlValue::Decimal(b)) => Ok(Some(Decimal::from(*a).cmp(b))),
        (CqlValue::Decimal(a), CqlValue::Long(b)) => Ok(Some(a.cmp(&Decimal::from(*b)))),

        // String comparison (lexicographic)
        (CqlValue::String(a), CqlValue::String(b)) => Ok(Some(a.cmp(b))),

        // Date comparison with precision
        (CqlValue::Date(a), CqlValue::Date(b)) => Ok(a.partial_cmp(b)),

        // DateTime comparison
        (CqlValue::DateTime(a), CqlValue::DateTime(b)) => {
            // Compare components in order, returning None if precision differs
            let year_cmp = a.year.cmp(&b.year);
            if year_cmp != Ordering::Equal {
                return Ok(Some(year_cmp));
            }

            match (a.month, b.month) {
                (None, None) => return Ok(Some(Ordering::Equal)),
                (None, Some(_)) | (Some(_), None) => return Ok(None),
                (Some(am), Some(bm)) => {
                    let month_cmp = am.cmp(&bm);
                    if month_cmp != Ordering::Equal {
                        return Ok(Some(month_cmp));
                    }
                }
            }

            match (a.day, b.day) {
                (None, None) => return Ok(Some(Ordering::Equal)),
                (None, Some(_)) | (Some(_), None) => return Ok(None),
                (Some(ad), Some(bd)) => {
                    let day_cmp = ad.cmp(&bd);
                    if day_cmp != Ordering::Equal {
                        return Ok(Some(day_cmp));
                    }
                }
            }

            match (a.hour, b.hour) {
                (None, None) => return Ok(Some(Ordering::Equal)),
                (None, Some(_)) | (Some(_), None) => return Ok(None),
                (Some(ah), Some(bh)) => {
                    let hour_cmp = ah.cmp(&bh);
                    if hour_cmp != Ordering::Equal {
                        return Ok(Some(hour_cmp));
                    }
                }
            }

            match (a.minute, b.minute) {
                (None, None) => return Ok(Some(Ordering::Equal)),
                (None, Some(_)) | (Some(_), None) => return Ok(None),
                (Some(am), Some(bm)) => {
                    let min_cmp = am.cmp(&bm);
                    if min_cmp != Ordering::Equal {
                        return Ok(Some(min_cmp));
                    }
                }
            }

            match (a.second, b.second) {
                (None, None) => return Ok(Some(Ordering::Equal)),
                (None, Some(_)) | (Some(_), None) => return Ok(None),
                (Some(as_), Some(bs)) => {
                    let sec_cmp = as_.cmp(&bs);
                    if sec_cmp != Ordering::Equal {
                        return Ok(Some(sec_cmp));
                    }
                }
            }

            match (a.millisecond, b.millisecond) {
                (None, None) => Ok(Some(Ordering::Equal)),
                (None, Some(_)) | (Some(_), None) => Ok(None),
                (Some(ams), Some(bms)) => Ok(Some(ams.cmp(&bms))),
            }
        }

        // Time comparison
        (CqlValue::Time(a), CqlValue::Time(b)) => Ok(a.partial_cmp(b)),

        // Quantity comparison (same units)
        (CqlValue::Quantity(a), CqlValue::Quantity(b)) => {
            if a.unit == b.unit {
                Ok(Some(a.value.cmp(&b.value)))
            } else {
                Err(EvalError::IncompatibleUnits {
                    unit1: a.unit.clone().unwrap_or_default(),
                    unit2: b.unit.clone().unwrap_or_default(),
                })
            }
        }

        _ => Err(EvalError::unsupported_operator(
            "Compare",
            format!("{}, {}", left.get_type().name(), right.get_type().name()),
        )),
    }
}

/// Compare two intervals for equality
fn interval_equal(a: &CqlInterval, b: &CqlInterval) -> EvalResult<bool> {
    // Compare bounds and closed flags
    let low_equal = match (&a.low, &b.low) {
        (Some(al), Some(bl)) => cql_equal(al, bl)?,
        (None, None) => true,
        _ => false,
    };

    let high_equal = match (&a.high, &b.high) {
        (Some(ah), Some(bh)) => cql_equal(ah, bh)?,
        (None, None) => true,
        _ => false,
    };

    Ok(low_equal && high_equal && a.low_closed == b.low_closed && a.high_closed == b.high_closed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_equality() {
        assert!(cql_equal(&CqlValue::Integer(5), &CqlValue::Integer(5)).unwrap());
        assert!(!cql_equal(&CqlValue::Integer(5), &CqlValue::Integer(6)).unwrap());
    }

    #[test]
    fn test_cross_type_numeric_equality() {
        assert!(cql_equal(&CqlValue::Integer(5), &CqlValue::Long(5)).unwrap());
        assert!(cql_equal(
            &CqlValue::Integer(5),
            &CqlValue::Decimal(Decimal::from(5))
        ).unwrap());
    }

    #[test]
    fn test_string_equality() {
        assert!(cql_equal(
            &CqlValue::String("hello".to_string()),
            &CqlValue::String("hello".to_string())
        ).unwrap());
        assert!(!cql_equal(
            &CqlValue::String("hello".to_string()),
            &CqlValue::String("Hello".to_string())
        ).unwrap());
    }

    #[test]
    fn test_string_equivalence() {
        // Equivalence is case-insensitive for strings
        assert!(cql_equivalent(
            &CqlValue::String("hello".to_string()),
            &CqlValue::String("HELLO".to_string())
        ).unwrap());
    }

    #[test]
    fn test_code_equivalence() {
        let code1 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", Some("1.0"), Some("Test")));
        let code2 = CqlValue::Code(CqlCode::new("123", "http://snomed.info/sct", Some("2.0"), Some("Different")));

        // Equal fails because version differs
        assert!(!cql_equal(&code1, &code2).unwrap());
        // Equivalent succeeds because only code and system matter
        assert!(cql_equivalent(&code1, &code2).unwrap());
    }

    #[test]
    fn test_comparison() {
        assert_eq!(
            cql_compare(&CqlValue::Integer(5), &CqlValue::Integer(3)).unwrap(),
            Some(Ordering::Greater)
        );
        assert_eq!(
            cql_compare(&CqlValue::Integer(3), &CqlValue::Integer(5)).unwrap(),
            Some(Ordering::Less)
        );
        assert_eq!(
            cql_compare(&CqlValue::Integer(5), &CqlValue::Integer(5)).unwrap(),
            Some(Ordering::Equal)
        );
    }
}
