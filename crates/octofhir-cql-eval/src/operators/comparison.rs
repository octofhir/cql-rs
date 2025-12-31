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
use rust_decimal::prelude::ToPrimitive;
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

        match cql_equal(&left, &right)? {
            Some(result) => Ok(CqlValue::Boolean(result)),
            None => Ok(CqlValue::Null),
        }
    }

    /// Evaluate NotEqual (!=) operator
    ///
    /// Equivalent to Not(Equal(left, right))
    pub fn eval_not_equal(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match cql_equal(&left, &right)? {
            Some(result) => Ok(CqlValue::Boolean(!result)),
            None => Ok(CqlValue::Null),
        }
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
/// Returns Some(true) if values are equal, Some(false) if different, None if uncertain (null).
/// Does NOT handle nulls at top level - caller must check for nulls first.
pub fn cql_equal(left: &CqlValue, right: &CqlValue) -> EvalResult<Option<bool>> {
    match (left, right) {
        // Same type comparisons
        (CqlValue::Boolean(a), CqlValue::Boolean(b)) => Ok(Some(a == b)),
        (CqlValue::Integer(a), CqlValue::Integer(b)) => Ok(Some(a == b)),
        (CqlValue::Long(a), CqlValue::Long(b)) => Ok(Some(a == b)),
        (CqlValue::Decimal(a), CqlValue::Decimal(b)) => Ok(Some(a == b)),
        (CqlValue::String(a), CqlValue::String(b)) => Ok(Some(a == b)),

        // Cross-type numeric comparisons
        (CqlValue::Integer(a), CqlValue::Long(b)) => Ok(Some((*a as i64) == *b)),
        (CqlValue::Long(a), CqlValue::Integer(b)) => Ok(Some(*a == (*b as i64))),
        (CqlValue::Integer(a), CqlValue::Decimal(b)) => Ok(Some(Decimal::from(*a) == *b)),
        (CqlValue::Decimal(a), CqlValue::Integer(b)) => Ok(Some(*a == Decimal::from(*b))),
        (CqlValue::Long(a), CqlValue::Decimal(b)) => Ok(Some(Decimal::from(*a) == *b)),
        (CqlValue::Decimal(a), CqlValue::Long(b)) => Ok(Some(*a == Decimal::from(*b))),

        // Date/Time comparisons - handle different precisions as uncertain
        (CqlValue::Date(a), CqlValue::Date(b)) => compare_dates_equal(a, b),
        (CqlValue::DateTime(a), CqlValue::DateTime(b)) => compare_datetimes_equal(a, b),
        (CqlValue::Time(a), CqlValue::Time(b)) => compare_times_equal(a, b),

        // Quantity comparison (same units or UCUM-convertible)
        (CqlValue::Quantity(a), CqlValue::Quantity(b)) => {
            compare_quantities_equal(a, b)
        }

        // Ratio comparison
        (CqlValue::Ratio(a), CqlValue::Ratio(b)) => {
            Ok(Some(a.numerator == b.numerator && a.denominator == b.denominator))
        }

        // Code comparison - must match code, system, and version
        (CqlValue::Code(a), CqlValue::Code(b)) => {
            Ok(Some(a.code == b.code && a.system == b.system && a.version == b.version))
        }

        // Concept comparison - codes must match
        (CqlValue::Concept(a), CqlValue::Concept(b)) => {
            Ok(Some(a.codes == b.codes))
        }

        // List comparison - element-wise
        // Per CQL spec: null elements are considered equal within lists
        (CqlValue::List(a), CqlValue::List(b)) => {
            if a.len() != b.len() {
                return Ok(Some(false));
            }
            let mut has_uncertain = false;
            for (elem_a, elem_b) in a.iter().zip(b.iter()) {
                // Per CQL spec: null elements are considered equal
                if elem_a.is_null() && elem_b.is_null() {
                    continue; // Both null - equal
                }
                // One null, one not - uncertain result
                if elem_a.is_null() || elem_b.is_null() {
                    has_uncertain = true;
                    continue;
                }
                match cql_equal(elem_a, elem_b)? {
                    Some(false) => return Ok(Some(false)), // Definite difference
                    Some(true) => {} // Continue checking
                    None => has_uncertain = true, // Uncertain
                }
            }
            if has_uncertain {
                Ok(None) // At least one uncertain comparison
            } else {
                Ok(Some(true)) // All comparisons were true
            }
        }

        // Interval comparison
        (CqlValue::Interval(a), CqlValue::Interval(b)) => {
            interval_equal(a, b)
        }

        // Tuple comparison - all elements must match
        // Per CQL spec: null elements are considered equal within tuples
        (CqlValue::Tuple(a), CqlValue::Tuple(b)) => {
            if a.len() != b.len() {
                return Ok(Some(false));
            }
            let mut has_uncertain = false;
            for (name, val_a) in a.iter() {
                match b.get(name) {
                    Some(val_b) => {
                        // Per CQL spec: null elements are considered equal
                        if val_a.is_null() && val_b.is_null() {
                            continue; // Both null - equal
                        }
                        // One null, one not - uncertain result
                        if val_a.is_null() || val_b.is_null() {
                            has_uncertain = true;
                            continue;
                        }
                        match cql_equal(val_a, val_b)? {
                            Some(false) => return Ok(Some(false)), // Definite difference
                            Some(true) => {} // Continue checking
                            None => has_uncertain = true, // Uncertain
                        }
                    }
                    None => return Ok(Some(false)),
                }
            }
            if has_uncertain {
                Ok(None) // At least one uncertain comparison
            } else {
                Ok(Some(true)) // All comparisons were true
            }
        }

        // Different types are not equal
        _ => Ok(Some(false)),
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
                // Handle null equivalence for elements
                match (elem_a.is_null(), elem_b.is_null()) {
                    (true, true) => continue, // null ~ null is true
                    (true, false) | (false, true) => return Ok(false), // null ~ non-null is false
                    (false, false) => {
                        if !cql_equivalent(elem_a, elem_b)? {
                            return Ok(false);
                        }
                    }
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
                        // Handle null equivalence for elements
                        match (val_a.is_null(), val_b.is_null()) {
                            (true, true) => continue, // null ~ null is true
                            (true, false) | (false, true) => return Ok(false), // null ~ non-null is false
                            (false, false) => {
                                if !cql_equivalent(val_a, val_b)? {
                                    return Ok(false);
                                }
                            }
                        }
                    }
                    None => return Ok(false),
                }
            }
            Ok(true)
        }

        // Fall back to equality for other types
        _ => cql_equal(left, right).map(|opt| opt.unwrap_or(false)),
    }
}

/// Compare two CQL values
///
/// Returns Some(Ordering) if comparison is certain, None if uncertain
/// (e.g., partial dates with different precision)
pub fn cql_compare(left: &CqlValue, right: &CqlValue) -> EvalResult<Option<Ordering>> {
    // Handle Interval vs scalar comparison (for uncertainty results like "months between A and B > 5")
    // Per CQL: comparing an interval to a point follows these rules:
    // - interval > point: true if interval.low > point, false if interval.high <= point, null otherwise
    // - interval < point: true if interval.high < point, false if interval.low >= point, null otherwise
    // - etc.
    if let CqlValue::Interval(interval) = left {
        return interval_compare_point(interval, right);
    }
    if let CqlValue::Interval(interval) = right {
        // point compared to interval - reverse the comparison
        return match interval_compare_point(interval, left)? {
            Some(Ordering::Less) => Ok(Some(Ordering::Greater)),
            Some(Ordering::Greater) => Ok(Some(Ordering::Less)),
            Some(Ordering::Equal) => Ok(Some(Ordering::Equal)),
            None => Ok(None),
        };
    }

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

        // Time comparison (precision-aware)
        (CqlValue::Time(a), CqlValue::Time(b)) => compare_times(a, b),

        // Quantity comparison (same units or UCUM-convertible)
        (CqlValue::Quantity(a), CqlValue::Quantity(b)) => {
            compare_quantities(a, b)
        }

        _ => Err(EvalError::unsupported_operator(
            "Compare",
            format!("{}, {}", left.get_type().name(), right.get_type().name()),
        )),
    }
}

/// Compare two quantities for equality, with UCUM unit conversion if needed
fn compare_quantities_equal(a: &CqlQuantity, b: &CqlQuantity) -> EvalResult<Option<bool>> {
    // Same unit - direct comparison
    if a.unit == b.unit {
        return Ok(Some(a.value == b.value));
    }

    // Different units - try UCUM conversion
    let unit_a = a.unit.as_deref().unwrap_or("1");
    let unit_b = b.unit.as_deref().unwrap_or("1");

    // Check if units are comparable (same dimension)
    match octofhir_ucum::is_comparable(unit_a, unit_b) {
        Ok(true) => {
            // Convert both to canonical form and compare
            match (octofhir_ucum::get_canonical_units(unit_a), octofhir_ucum::get_canonical_units(unit_b)) {
                (Ok(canon_a), Ok(canon_b)) => {
                    // Convert values to canonical units
                    let val_a = a.value.to_f64().unwrap_or(0.0) * canon_a.factor;
                    let val_b = b.value.to_f64().unwrap_or(0.0) * canon_b.factor;
                    // Compare with small epsilon for floating point
                    let epsilon = 1e-10 * (val_a.abs() + val_b.abs()).max(1.0);
                    Ok(Some((val_a - val_b).abs() < epsilon))
                }
                _ => Err(EvalError::IncompatibleUnits {
                    unit1: unit_a.to_string(),
                    unit2: unit_b.to_string(),
                }),
            }
        }
        Ok(false) => Err(EvalError::IncompatibleUnits {
            unit1: unit_a.to_string(),
            unit2: unit_b.to_string(),
        }),
        Err(_) => Err(EvalError::IncompatibleUnits {
            unit1: unit_a.to_string(),
            unit2: unit_b.to_string(),
        }),
    }
}

/// Compare two quantities for ordering, with UCUM unit conversion if needed
fn compare_quantities(a: &CqlQuantity, b: &CqlQuantity) -> EvalResult<Option<Ordering>> {
    // Same unit - direct comparison
    if a.unit == b.unit {
        return Ok(Some(a.value.cmp(&b.value)));
    }

    // Different units - try UCUM conversion
    let unit_a = a.unit.as_deref().unwrap_or("1");
    let unit_b = b.unit.as_deref().unwrap_or("1");

    // Check if units are comparable (same dimension)
    match octofhir_ucum::is_comparable(unit_a, unit_b) {
        Ok(true) => {
            // Convert both to canonical form and compare
            match (octofhir_ucum::get_canonical_units(unit_a), octofhir_ucum::get_canonical_units(unit_b)) {
                (Ok(canon_a), Ok(canon_b)) => {
                    // Convert values to canonical units
                    let val_a = a.value.to_f64().unwrap_or(0.0) * canon_a.factor;
                    let val_b = b.value.to_f64().unwrap_or(0.0) * canon_b.factor;
                    // Compare with small epsilon for floating point
                    let epsilon = 1e-10 * (val_a.abs() + val_b.abs()).max(1.0);
                    if (val_a - val_b).abs() < epsilon {
                        Ok(Some(Ordering::Equal))
                    } else if val_a < val_b {
                        Ok(Some(Ordering::Less))
                    } else {
                        Ok(Some(Ordering::Greater))
                    }
                }
                _ => Err(EvalError::IncompatibleUnits {
                    unit1: unit_a.to_string(),
                    unit2: unit_b.to_string(),
                }),
            }
        }
        Ok(false) => Err(EvalError::IncompatibleUnits {
            unit1: unit_a.to_string(),
            unit2: unit_b.to_string(),
        }),
        Err(_) => Err(EvalError::IncompatibleUnits {
            unit1: unit_a.to_string(),
            unit2: unit_b.to_string(),
        }),
    }
}

/// Compare two intervals for equality
fn interval_equal(a: &CqlInterval, b: &CqlInterval) -> EvalResult<Option<bool>> {
    // Compare bounds and closed flags
    // If one bound is null (unknown) and the other is a value, result is uncertain
    let low_equal = match (&a.low, &b.low) {
        (Some(al), Some(bl)) => cql_equal(al, bl)?,
        (None, None) => Some(true),
        // One is null (unknown), can't determine equality
        _ => None,
    };

    let high_equal = match (&a.high, &b.high) {
        (Some(ah), Some(bh)) => cql_equal(ah, bh)?,
        (None, None) => Some(true),
        // One is null (unknown), can't determine equality
        _ => None,
    };

    match (low_equal, high_equal) {
        (Some(le), Some(he)) => Ok(Some(le && he && a.low_closed == b.low_closed && a.high_closed == b.high_closed)),
        // If either bound comparison is uncertain, result is uncertain
        _ => Ok(None),
    }
}

/// Compare two dates for equality with precision handling
fn compare_dates_equal(a: &octofhir_cql_types::CqlDate, b: &octofhir_cql_types::CqlDate) -> EvalResult<Option<bool>> {
    // Compare year (always present)
    if a.year != b.year {
        return Ok(Some(false));
    }

    // Compare month if both have it
    match (a.month, b.month) {
        (Some(am), Some(bm)) => {
            if am != bm {
                return Ok(Some(false));
            }
        }
        (None, None) => return Ok(Some(true)), // Both have same precision
        _ => return Ok(None), // Different precision - uncertain
    }

    // Compare day if both have it
    match (a.day, b.day) {
        (Some(ad), Some(bd)) => Ok(Some(ad == bd)),
        (None, None) => Ok(Some(true)), // Both have same precision
        _ => Ok(None), // Different precision - uncertain
    }
}

/// Compare two datetimes for equality with precision handling
fn compare_datetimes_equal(a: &octofhir_cql_types::CqlDateTime, b: &octofhir_cql_types::CqlDateTime) -> EvalResult<Option<bool>> {
    // Compare required fields first
    if a.year != b.year {
        return Ok(Some(false));
    }
    if a.month != b.month {
        return Ok(Some(false));
    }
    if a.day != b.day {
        return Ok(Some(false));
    }

    // Compare optional fields - if one has it and other doesn't, uncertain
    match (a.hour, b.hour) {
        (Some(ah), Some(bh)) => {
            if ah != bh {
                return Ok(Some(false));
            }
        }
        (None, None) => return Ok(Some(true)), // Both have same precision
        _ => return Ok(None), // Different precision - uncertain
    }

    match (a.minute, b.minute) {
        (Some(am), Some(bm)) => {
            if am != bm {
                return Ok(Some(false));
            }
        }
        (None, None) => return Ok(Some(true)),
        _ => return Ok(None),
    }

    match (a.second, b.second) {
        (Some(as_), Some(bs)) => {
            if as_ != bs {
                return Ok(Some(false));
            }
        }
        (None, None) => return Ok(Some(true)),
        _ => return Ok(None),
    }

    match (a.millisecond, b.millisecond) {
        (Some(am), Some(bm)) => Ok(Some(am == bm)),
        (None, None) => Ok(Some(true)),
        _ => Ok(None),
    }
}

/// Compare two times for ordering with precision handling
/// Returns None when the comparison is uncertain due to precision differences
fn compare_times(a: &octofhir_cql_types::CqlTime, b: &octofhir_cql_types::CqlTime) -> EvalResult<Option<Ordering>> {
    // Compare hour (always present)
    match a.hour.cmp(&b.hour) {
        Ordering::Equal => {}
        ord => return Ok(Some(ord)),
    }

    // Compare minute if both have it
    match (a.minute, b.minute) {
        (Some(am), Some(bm)) => match am.cmp(&bm) {
            Ordering::Equal => {}
            ord => return Ok(Some(ord)),
        },
        (None, None) => return Ok(Some(Ordering::Equal)),
        // One has minute precision, other doesn't - uncertain
        // e.g., @T12 vs @T12:30 - could be equal or either could be greater
        _ => return Ok(None),
    }

    // Compare second if both have it
    match (a.second, b.second) {
        (Some(as_), Some(bs)) => match as_.cmp(&bs) {
            Ordering::Equal => {}
            ord => return Ok(Some(ord)),
        },
        (None, None) => return Ok(Some(Ordering::Equal)),
        _ => return Ok(None),
    }

    // Compare millisecond if both have it
    match (a.millisecond, b.millisecond) {
        (Some(am), Some(bm)) => Ok(Some(am.cmp(&bm))),
        (None, None) => Ok(Some(Ordering::Equal)),
        // e.g., @T12:00:00 vs @T12:00:00.001 - uncertain
        _ => Ok(None),
    }
}

/// Compare two times for equality with precision handling
fn compare_times_equal(a: &octofhir_cql_types::CqlTime, b: &octofhir_cql_types::CqlTime) -> EvalResult<Option<bool>> {
    // Compare hour (always present)
    if a.hour != b.hour {
        return Ok(Some(false));
    }

    // Compare minute if both have it
    match (a.minute, b.minute) {
        (Some(am), Some(bm)) => {
            if am != bm {
                return Ok(Some(false));
            }
        }
        (None, None) => return Ok(Some(true)), // Both have same precision
        _ => return Ok(None), // Different precision - uncertain
    }

    // Compare second if both have it
    match (a.second, b.second) {
        (Some(as_), Some(bs)) => {
            if as_ != bs {
                return Ok(Some(false));
            }
        }
        (None, None) => return Ok(Some(true)),
        _ => return Ok(None),
    }

    // Compare millisecond if both have it
    match (a.millisecond, b.millisecond) {
        (Some(am), Some(bm)) => Ok(Some(am == bm)),
        (None, None) => Ok(Some(true)),
        _ => Ok(None), // Different precision - uncertain
    }
}

/// Compare an interval to a point value
///
/// This implements CQL uncertainty semantics for comparing an interval (representing
/// an uncertain value) to a point:
/// - Returns Some(Greater) if interval.low > point (all possible values are greater)
/// - Returns Some(Less) if interval.high < point (all possible values are less)
/// - Returns Some(Equal) if interval.low == interval.high == point (single value equals point)
/// - Returns None otherwise (uncertain - some values might be greater, some less)
fn interval_compare_point(interval: &CqlInterval, point: &CqlValue) -> EvalResult<Option<Ordering>> {
    let low = match &interval.low {
        Some(v) => v,
        None => return Ok(None), // Unbounded low means uncertain
    };
    let high = match &interval.high {
        Some(v) => v,
        None => return Ok(None), // Unbounded high means uncertain
    };

    // Compare low bound to point
    let low_cmp = cql_compare_values(low, point)?;
    // Compare high bound to point
    let high_cmp = cql_compare_values(high, point)?;

    match (low_cmp, high_cmp) {
        // If low > point, then all values in interval > point
        (Some(Ordering::Greater), _) => Ok(Some(Ordering::Greater)),
        // If high < point, then all values in interval < point
        (_, Some(Ordering::Less)) => Ok(Some(Ordering::Less)),
        // If low == high == point, then interval represents exactly point
        (Some(Ordering::Equal), Some(Ordering::Equal)) => Ok(Some(Ordering::Equal)),
        // Otherwise uncertain - interval spans across point
        _ => Ok(None),
    }
}

/// Compare two scalar values (helper for interval_compare_point)
fn cql_compare_values(left: &CqlValue, right: &CqlValue) -> EvalResult<Option<Ordering>> {
    match (left, right) {
        (CqlValue::Integer(a), CqlValue::Integer(b)) => Ok(Some(a.cmp(b))),
        (CqlValue::Long(a), CqlValue::Long(b)) => Ok(Some(a.cmp(b))),
        (CqlValue::Integer(a), CqlValue::Long(b)) => Ok(Some((*a as i64).cmp(b))),
        (CqlValue::Long(a), CqlValue::Integer(b)) => Ok(Some(a.cmp(&(*b as i64)))),
        (CqlValue::Decimal(a), CqlValue::Decimal(b)) => Ok(Some(a.cmp(b))),
        (CqlValue::Integer(a), CqlValue::Decimal(b)) => Ok(Some(Decimal::from(*a).cmp(b))),
        (CqlValue::Decimal(a), CqlValue::Integer(b)) => Ok(Some(a.cmp(&Decimal::from(*b)))),
        (CqlValue::Long(a), CqlValue::Decimal(b)) => Ok(Some(Decimal::from(*a).cmp(b))),
        (CqlValue::Decimal(a), CqlValue::Long(b)) => Ok(Some(a.cmp(&Decimal::from(*b)))),
        (CqlValue::String(a), CqlValue::String(b)) => Ok(Some(a.cmp(b))),
        (CqlValue::Date(a), CqlValue::Date(b)) => Ok(a.partial_cmp(b)),
        (CqlValue::Time(a), CqlValue::Time(b)) => compare_times(a, b),
        _ => Ok(None), // Incomparable types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_equality() {
        assert_eq!(cql_equal(&CqlValue::Integer(5), &CqlValue::Integer(5)).unwrap(), Some(true));
        assert_eq!(cql_equal(&CqlValue::Integer(5), &CqlValue::Integer(6)).unwrap(), Some(false));
    }

    #[test]
    fn test_cross_type_numeric_equality() {
        assert_eq!(cql_equal(&CqlValue::Integer(5), &CqlValue::Long(5)).unwrap(), Some(true));
        assert_eq!(cql_equal(
            &CqlValue::Integer(5),
            &CqlValue::Decimal(Decimal::from(5))
        ).unwrap(), Some(true));
    }

    #[test]
    fn test_string_equality() {
        assert_eq!(cql_equal(
            &CqlValue::String("hello".to_string()),
            &CqlValue::String("hello".to_string())
        ).unwrap(), Some(true));
        assert_eq!(cql_equal(
            &CqlValue::String("hello".to_string()),
            &CqlValue::String("Hello".to_string())
        ).unwrap(), Some(false));
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
        assert_eq!(cql_equal(&code1, &code2).unwrap(), Some(false));
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

    #[test]
    fn test_tuple_equality_with_nulls() {
        use octofhir_cql_types::CqlTuple;

        // Two tuples with null values at the same position
        let mut t1 = CqlTuple::new();
        t1.set("Id".to_string(), CqlValue::Integer(1));
        t1.set("Name".to_string(), CqlValue::Null);

        let mut t2 = CqlTuple::new();
        t2.set("Id".to_string(), CqlValue::Integer(1));
        t2.set("Name".to_string(), CqlValue::Null);

        // For equivalence, null ~ null is true
        assert!(cql_equivalent(&CqlValue::Tuple(t1.clone()), &CqlValue::Tuple(t2.clone())).unwrap());

        // Per CQL spec: null elements are considered equal within tuples
        assert_eq!(cql_equal(&CqlValue::Tuple(t1), &CqlValue::Tuple(t2)).unwrap(), Some(true));
    }

    #[test]
    fn test_tuple_equality_with_one_null() {
        use octofhir_cql_types::CqlTuple;

        // One tuple has null, other has value - should be uncertain
        let mut t1 = CqlTuple::new();
        t1.set("Id".to_string(), CqlValue::Integer(1));
        t1.set("Name".to_string(), CqlValue::Null);

        let mut t2 = CqlTuple::new();
        t2.set("Id".to_string(), CqlValue::Integer(1));
        t2.set("Name".to_string(), CqlValue::String("John".to_string()));

        // One null, one value - uncertain
        assert_eq!(cql_equal(&CqlValue::Tuple(t1), &CqlValue::Tuple(t2)).unwrap(), None);
    }
}
