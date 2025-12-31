//! Arithmetic Operators for CQL
//!
//! Implements: Add, Subtract, Multiply, Divide, TruncatedDivide, Modulo,
//! Power, Negate, Successor, Predecessor, Abs, Ceiling, Floor, Round,
//! Truncate, Exp, Ln, Log, MinValue, MaxValue, Precision, LowBoundary, HighBoundary

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use chrono::{Datelike, Timelike};
use octofhir_cql_elm::{BinaryExpression, BoundaryExpression, MinMaxValueExpression, RoundExpression, UnaryExpression};
use octofhir_cql_types::{CqlInterval, CqlQuantity, CqlType, CqlValue, DateTimePrecision};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

impl CqlEngine {
    // =========================================================================
    // Binary Arithmetic
    // =========================================================================

    /// Evaluate Add (+) operator
    pub fn eval_add(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        // Null propagation
        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            // Integer + Integer -> Integer
            (CqlValue::Integer(a), CqlValue::Integer(b)) => {
                a.checked_add(*b)
                    .map(CqlValue::Integer)
                    .ok_or_else(|| EvalError::overflow("Add"))
            }
            // Long + Long -> Long
            (CqlValue::Long(a), CqlValue::Long(b)) => {
                a.checked_add(*b)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Add"))
            }
            // Integer + Long -> Long
            (CqlValue::Integer(a), CqlValue::Long(b)) => {
                (*a as i64).checked_add(*b)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Add"))
            }
            (CqlValue::Long(a), CqlValue::Integer(b)) => {
                a.checked_add(*b as i64)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Add"))
            }
            // Decimal + Decimal -> Decimal
            (CqlValue::Decimal(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(a + b))
            }
            // Mixed numeric -> Decimal
            (CqlValue::Integer(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(Decimal::from(*a) + b))
            }
            (CqlValue::Decimal(a), CqlValue::Integer(b)) => {
                Ok(CqlValue::Decimal(a + Decimal::from(*b)))
            }
            (CqlValue::Long(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(Decimal::from(*a) + b))
            }
            (CqlValue::Decimal(a), CqlValue::Long(b)) => {
                Ok(CqlValue::Decimal(a + Decimal::from(*b)))
            }
            // Quantity + Quantity -> Quantity (same units)
            (CqlValue::Quantity(a), CqlValue::Quantity(b)) => {
                if a.unit == b.unit {
                    Ok(CqlValue::Quantity(CqlQuantity {
                        value: a.value + b.value,
                        unit: a.unit.clone(),
                    }))
                } else {
                    Err(EvalError::IncompatibleUnits {
                        unit1: a.unit.clone().unwrap_or_default(),
                        unit2: b.unit.clone().unwrap_or_default(),
                    })
                }
            }
            // Date + Quantity -> Date
            (CqlValue::Date(d), CqlValue::Quantity(q)) => {
                add_duration_to_date(d, q)
            }
            // DateTime + Quantity -> DateTime
            (CqlValue::DateTime(dt), CqlValue::Quantity(q)) => {
                add_duration_to_datetime(dt, q)
            }
            // Time + Quantity -> Time
            (CqlValue::Time(t), CqlValue::Quantity(q)) => {
                add_duration_to_time(t, q)
            }
            // String + String -> String (concatenation)
            (CqlValue::String(a), CqlValue::String(b)) => {
                Ok(CqlValue::String(format!("{}{}", a, b)))
            }
            // Interval + Interval -> Interval (uncertainty propagation)
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                interval_add(a, b)
            }
            _ => Err(EvalError::unsupported_operator(
                "Add",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Subtract (-) operator
    pub fn eval_subtract(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Integer(a), CqlValue::Integer(b)) => {
                a.checked_sub(*b)
                    .map(CqlValue::Integer)
                    .ok_or_else(|| EvalError::overflow("Subtract"))
            }
            (CqlValue::Long(a), CqlValue::Long(b)) => {
                a.checked_sub(*b)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Subtract"))
            }
            (CqlValue::Integer(a), CqlValue::Long(b)) => {
                (*a as i64).checked_sub(*b)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Subtract"))
            }
            (CqlValue::Long(a), CqlValue::Integer(b)) => {
                a.checked_sub(*b as i64)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Subtract"))
            }
            (CqlValue::Decimal(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(a - b))
            }
            (CqlValue::Integer(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(Decimal::from(*a) - b))
            }
            (CqlValue::Decimal(a), CqlValue::Integer(b)) => {
                Ok(CqlValue::Decimal(a - Decimal::from(*b)))
            }
            (CqlValue::Long(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(Decimal::from(*a) - b))
            }
            (CqlValue::Decimal(a), CqlValue::Long(b)) => {
                Ok(CqlValue::Decimal(a - Decimal::from(*b)))
            }
            (CqlValue::Quantity(a), CqlValue::Quantity(b)) => {
                if a.unit == b.unit {
                    Ok(CqlValue::Quantity(CqlQuantity {
                        value: a.value - b.value,
                        unit: a.unit.clone(),
                    }))
                } else {
                    Err(EvalError::IncompatibleUnits {
                        unit1: a.unit.clone().unwrap_or_default(),
                        unit2: b.unit.clone().unwrap_or_default(),
                    })
                }
            }
            // Date - Quantity -> Date
            (CqlValue::Date(d), CqlValue::Quantity(q)) => {
                subtract_duration_from_date(d, q)
            }
            // DateTime - Quantity -> DateTime
            (CqlValue::DateTime(dt), CqlValue::Quantity(q)) => {
                subtract_duration_from_datetime(dt, q)
            }
            // Time - Quantity -> Time
            (CqlValue::Time(t), CqlValue::Quantity(q)) => {
                subtract_duration_from_time(t, q)
            }
            // Interval - Interval -> Interval (uncertainty propagation)
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                interval_subtract(a, b)
            }
            _ => Err(EvalError::unsupported_operator(
                "Subtract",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Multiply (*) operator
    pub fn eval_multiply(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Integer(a), CqlValue::Integer(b)) => {
                a.checked_mul(*b)
                    .map(CqlValue::Integer)
                    .ok_or_else(|| EvalError::overflow("Multiply"))
            }
            (CqlValue::Long(a), CqlValue::Long(b)) => {
                a.checked_mul(*b)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Multiply"))
            }
            (CqlValue::Integer(a), CqlValue::Long(b)) => {
                (*a as i64).checked_mul(*b)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Multiply"))
            }
            (CqlValue::Long(a), CqlValue::Integer(b)) => {
                a.checked_mul(*b as i64)
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Multiply"))
            }
            (CqlValue::Decimal(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(a * b))
            }
            (CqlValue::Integer(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(Decimal::from(*a) * b))
            }
            (CqlValue::Decimal(a), CqlValue::Integer(b)) => {
                Ok(CqlValue::Decimal(a * Decimal::from(*b)))
            }
            (CqlValue::Long(a), CqlValue::Decimal(b)) => {
                Ok(CqlValue::Decimal(Decimal::from(*a) * b))
            }
            (CqlValue::Decimal(a), CqlValue::Long(b)) => {
                Ok(CqlValue::Decimal(a * Decimal::from(*b)))
            }
            // Quantity * numeric
            (CqlValue::Quantity(q), CqlValue::Integer(n)) => {
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: q.value * Decimal::from(*n),
                    unit: q.unit.clone(),
                }))
            }
            (CqlValue::Integer(n), CqlValue::Quantity(q)) => {
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: Decimal::from(*n) * q.value,
                    unit: q.unit.clone(),
                }))
            }
            (CqlValue::Quantity(q), CqlValue::Decimal(n)) => {
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: q.value * n,
                    unit: q.unit.clone(),
                }))
            }
            (CqlValue::Decimal(n), CqlValue::Quantity(q)) => {
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: n * q.value,
                    unit: q.unit.clone(),
                }))
            }
            // Interval * Interval -> Interval (uncertainty propagation)
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                interval_multiply(a, b)
            }
            _ => Err(EvalError::unsupported_operator(
                "Multiply",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Divide (/) operator - always returns Decimal
    pub fn eval_divide(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        // Get decimal values
        let dividend = match &left {
            CqlValue::Integer(i) => Decimal::from(*i),
            CqlValue::Long(l) => Decimal::from(*l),
            CqlValue::Decimal(d) => *d,
            CqlValue::Quantity(q) => q.value,
            _ => return Err(EvalError::unsupported_operator(
                "Divide",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        };

        let divisor = match &right {
            CqlValue::Integer(i) => Decimal::from(*i),
            CqlValue::Long(l) => Decimal::from(*l),
            CqlValue::Decimal(d) => *d,
            CqlValue::Quantity(q) => q.value,
            _ => return Err(EvalError::unsupported_operator(
                "Divide",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        };

        // Division by zero returns null in CQL
        if divisor.is_zero() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            // Quantity / Quantity -> Decimal (units cancel or combine)
            (CqlValue::Quantity(a), CqlValue::Quantity(b)) => {
                if a.unit == b.unit {
                    // Same units cancel out
                    Ok(CqlValue::Decimal(dividend / divisor))
                } else {
                    // Different units - return quantity with combined unit
                    // Simplified: just concatenate units for now
                    let unit = match (&a.unit, &b.unit) {
                        (Some(u1), Some(u2)) => Some(format!("{}/{}", u1, u2)),
                        (Some(u1), None) => Some(u1.clone()),
                        (None, Some(u2)) => Some(format!("1/{}", u2)),
                        (None, None) => None,
                    };
                    Ok(CqlValue::Quantity(CqlQuantity {
                        value: dividend / divisor,
                        unit,
                    }))
                }
            }
            // Quantity / numeric -> Quantity
            (CqlValue::Quantity(q), _) => {
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: dividend / divisor,
                    unit: q.unit.clone(),
                }))
            }
            // Numeric / Numeric -> Decimal
            _ => Ok(CqlValue::Decimal(dividend / divisor)),
        }
    }

    /// Evaluate TruncatedDivide (div) operator - integer division
    pub fn eval_truncated_divide(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Integer(a), CqlValue::Integer(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Integer(a / b))
                }
            }
            (CqlValue::Long(a), CqlValue::Long(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(a / b))
                }
            }
            (CqlValue::Integer(a), CqlValue::Long(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(*a as i64 / b))
                }
            }
            (CqlValue::Long(a), CqlValue::Integer(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(a / *b as i64))
                }
            }
            (CqlValue::Decimal(a), CqlValue::Decimal(b)) => {
                if b.is_zero() {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal((a / b).trunc()))
                }
            }
            // Integer div Decimal -> Decimal
            (CqlValue::Integer(a), CqlValue::Decimal(b)) => {
                if b.is_zero() {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal((Decimal::from(*a) / b).trunc()))
                }
            }
            // Decimal div Integer -> Decimal
            (CqlValue::Decimal(a), CqlValue::Integer(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal((a / Decimal::from(*b)).trunc()))
                }
            }
            // Long div Decimal -> Decimal
            (CqlValue::Long(a), CqlValue::Decimal(b)) => {
                if b.is_zero() {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal((Decimal::from(*a) / b).trunc()))
                }
            }
            // Decimal div Long -> Decimal
            (CqlValue::Decimal(a), CqlValue::Long(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal((a / Decimal::from(*b)).trunc()))
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "TruncatedDivide",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Modulo (mod) operator
    pub fn eval_modulo(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Integer(a), CqlValue::Integer(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Integer(a % b))
                }
            }
            (CqlValue::Long(a), CqlValue::Long(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(a % b))
                }
            }
            (CqlValue::Integer(a), CqlValue::Long(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(*a as i64 % b))
                }
            }
            (CqlValue::Long(a), CqlValue::Integer(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(a % *b as i64))
                }
            }
            (CqlValue::Decimal(a), CqlValue::Decimal(b)) => {
                if b.is_zero() {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal(a % b))
                }
            }
            // Integer % Decimal -> Decimal
            (CqlValue::Integer(a), CqlValue::Decimal(b)) => {
                if b.is_zero() {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal(Decimal::from(*a) % b))
                }
            }
            // Decimal % Integer -> Decimal
            (CqlValue::Decimal(a), CqlValue::Integer(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal(a % Decimal::from(*b)))
                }
            }
            // Long % Decimal -> Decimal
            (CqlValue::Long(a), CqlValue::Decimal(b)) => {
                if b.is_zero() {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal(Decimal::from(*a) % b))
                }
            }
            // Decimal % Long -> Decimal
            (CqlValue::Decimal(a), CqlValue::Long(b)) => {
                if *b == 0 {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Decimal(a % Decimal::from(*b)))
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "Modulo",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Power (^) operator
    pub fn eval_power(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Integer(base), CqlValue::Integer(exp)) => {
                if *exp < 0 {
                    // Negative exponent returns decimal
                    let base_f = *base as f64;
                    let result = base_f.powi(*exp);
                    Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
                } else if let Some(result) = base.checked_pow(*exp as u32) {
                    Ok(CqlValue::Integer(result))
                } else {
                    Err(EvalError::overflow("Power"))
                }
            }
            (CqlValue::Long(base), CqlValue::Integer(exp)) => {
                if *exp < 0 {
                    let base_f = *base as f64;
                    let result = base_f.powi(*exp);
                    Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
                } else if let Some(result) = base.checked_pow(*exp as u32) {
                    Ok(CqlValue::Long(result))
                } else {
                    Err(EvalError::overflow("Power"))
                }
            }
            // Long ^ Long -> Long
            (CqlValue::Long(base), CqlValue::Long(exp)) => {
                if *exp < 0 {
                    let base_f = *base as f64;
                    let result = base_f.powi(*exp as i32);
                    Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
                } else if let Some(result) = base.checked_pow(*exp as u32) {
                    Ok(CqlValue::Long(result))
                } else {
                    Err(EvalError::overflow("Power"))
                }
            }
            (CqlValue::Decimal(base), CqlValue::Integer(exp)) => {
                // Use floating point for decimal power
                if let Some(base_f) = base.to_f64() {
                    let result = base_f.powi(*exp);
                    if result.is_finite() {
                        Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
                    } else {
                        Err(EvalError::overflow("Power"))
                    }
                } else {
                    Err(EvalError::overflow("Power"))
                }
            }
            (CqlValue::Decimal(base), CqlValue::Decimal(exp)) => {
                // Use floating point for decimal power
                if let (Some(base_f), Some(exp_f)) = (base.to_f64(), exp.to_f64()) {
                    let result = base_f.powf(exp_f);
                    if result.is_finite() {
                        Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
                    } else {
                        Err(EvalError::overflow("Power"))
                    }
                } else {
                    Err(EvalError::overflow("Power"))
                }
            }
            // Integer ^ Decimal -> Decimal
            (CqlValue::Integer(base), CqlValue::Decimal(exp)) => {
                if let Some(exp_f) = exp.to_f64() {
                    let base_f = *base as f64;
                    let result = base_f.powf(exp_f);
                    if result.is_finite() {
                        Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
                    } else {
                        Err(EvalError::overflow("Power"))
                    }
                } else {
                    Err(EvalError::overflow("Power"))
                }
            }
            // Long ^ Decimal -> Decimal
            (CqlValue::Long(base), CqlValue::Decimal(exp)) => {
                if let Some(exp_f) = exp.to_f64() {
                    let base_f = *base as f64;
                    let result = base_f.powf(exp_f);
                    if result.is_finite() {
                        Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
                    } else {
                        Err(EvalError::overflow("Power"))
                    }
                } else {
                    Err(EvalError::overflow("Power"))
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "Power",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    // =========================================================================
    // Unary Arithmetic
    // =========================================================================

    /// Evaluate Negate (unary -) operator
    pub fn eval_negate(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Integer(i) => {
                i.checked_neg()
                    .map(CqlValue::Integer)
                    .ok_or_else(|| EvalError::overflow("Negate"))
            }
            CqlValue::Long(l) => {
                l.checked_neg()
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Negate"))
            }
            CqlValue::Decimal(d) => {
                // Avoid -0.0, return 0.0 instead
                if d.is_zero() {
                    Ok(CqlValue::Decimal(Decimal::ZERO))
                } else {
                    Ok(CqlValue::Decimal(-d))
                }
            }
            CqlValue::Quantity(q) => Ok(CqlValue::Quantity(CqlQuantity {
                value: -q.value,
                unit: q.unit.clone(),
            })),
            _ => Err(EvalError::unsupported_operator("Negate", operand.get_type().name())),
        }
    }

    /// Evaluate Abs operator
    pub fn eval_abs(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Integer(i) => {
                i.checked_abs()
                    .map(CqlValue::Integer)
                    .ok_or_else(|| EvalError::overflow("Abs"))
            }
            CqlValue::Long(l) => {
                l.checked_abs()
                    .map(CqlValue::Long)
                    .ok_or_else(|| EvalError::overflow("Abs"))
            }
            CqlValue::Decimal(d) => Ok(CqlValue::Decimal(d.abs())),
            CqlValue::Quantity(q) => Ok(CqlValue::Quantity(CqlQuantity {
                value: q.value.abs(),
                unit: q.unit.clone(),
            })),
            _ => Err(EvalError::unsupported_operator("Abs", operand.get_type().name())),
        }
    }

    /// Evaluate Ceiling operator
    pub fn eval_ceiling(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Integer(i) => Ok(CqlValue::Integer(*i)),
            CqlValue::Long(l) => Ok(CqlValue::Long(*l)),
            CqlValue::Decimal(d) => Ok(CqlValue::Integer(d.ceil().to_i32().unwrap_or(i32::MAX))),
            _ => Err(EvalError::unsupported_operator("Ceiling", operand.get_type().name())),
        }
    }

    /// Evaluate Floor operator
    pub fn eval_floor(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Integer(i) => Ok(CqlValue::Integer(*i)),
            CqlValue::Long(l) => Ok(CqlValue::Long(*l)),
            CqlValue::Decimal(d) => Ok(CqlValue::Integer(d.floor().to_i32().unwrap_or(i32::MIN))),
            _ => Err(EvalError::unsupported_operator("Floor", operand.get_type().name())),
        }
    }

    /// Evaluate Truncate operator
    pub fn eval_truncate(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Integer(i) => Ok(CqlValue::Integer(*i)),
            CqlValue::Long(l) => Ok(CqlValue::Long(*l)),
            CqlValue::Decimal(d) => Ok(CqlValue::Integer(d.trunc().to_i32().unwrap_or(0))),
            _ => Err(EvalError::unsupported_operator("Truncate", operand.get_type().name())),
        }
    }

    /// Evaluate Round operator
    pub fn eval_round(&self, expr: &RoundExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        let precision = if let Some(prec_expr) = &expr.precision {
            match self.evaluate(prec_expr, ctx)? {
                CqlValue::Integer(p) => p as u32,
                CqlValue::Null => return Ok(CqlValue::Null),
                _ => return Err(EvalError::invalid_operand("Round", "precision must be Integer")),
            }
        } else {
            0
        };

        match &operand {
            CqlValue::Integer(i) => {
                // Round returns Decimal even for integer input
                Ok(CqlValue::Decimal(Decimal::from(*i)))
            }
            CqlValue::Long(l) => {
                Ok(CqlValue::Decimal(Decimal::from(*l)))
            }
            CqlValue::Decimal(d) => {
                // CQL uses "round half up" meaning round toward positive infinity at midpoint
                // For positive numbers: 0.5 -> 1, 1.5 -> 2
                // For negative numbers: -0.5 -> 0, -1.5 -> -1
                // This is essentially ceiling at the midpoint
                let scale_factor = Decimal::from_i32(10i32.pow(precision)).unwrap_or(Decimal::ONE);
                let scaled = *d * scale_factor;
                let floor = scaled.floor();
                let frac = scaled - floor;

                // If exactly at midpoint (0.5), round up (toward positive infinity)
                let rounded_scaled = if frac == Decimal::new(5, 1) {
                    floor + Decimal::ONE
                } else if frac > Decimal::new(5, 1) {
                    floor + Decimal::ONE
                } else {
                    floor
                };

                Ok(CqlValue::Decimal(rounded_scaled / scale_factor))
            }
            _ => Err(EvalError::unsupported_operator("Round", operand.get_type().name())),
        }
    }

    /// Evaluate Ln (natural log) operator
    pub fn eval_ln(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        let value = match &operand {
            CqlValue::Integer(i) => *i as f64,
            CqlValue::Long(l) => *l as f64,
            CqlValue::Decimal(d) => d.to_f64().unwrap_or(0.0),
            _ => return Err(EvalError::unsupported_operator("Ln", operand.get_type().name())),
        };

        // ln(0) is undefined and should error, ln(negative) is null
        if value == 0.0 {
            return Err(EvalError::overflow("Ln: log of zero is undefined"));
        }
        if value < 0.0 {
            return Ok(CqlValue::Null);
        }

        let result = value.ln();
        Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
    }

    /// Evaluate Exp (e^x) operator
    pub fn eval_exp(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        let value = match &operand {
            CqlValue::Integer(i) => *i as f64,
            CqlValue::Long(l) => *l as f64,
            CqlValue::Decimal(d) => d.to_f64().unwrap_or(0.0),
            _ => return Err(EvalError::unsupported_operator("Exp", operand.get_type().name())),
        };

        let result = value.exp();
        if result.is_infinite() || result.is_nan() {
            return Err(EvalError::overflow(format!("Exp: result overflow for argument {}", value)));
        }
        Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
    }

    /// Evaluate Log (log base) operator
    pub fn eval_log(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        let value = match &left {
            CqlValue::Integer(i) => *i as f64,
            CqlValue::Long(l) => *l as f64,
            CqlValue::Decimal(d) => d.to_f64().unwrap_or(0.0),
            _ => return Err(EvalError::unsupported_operator("Log", left.get_type().name())),
        };

        let base = match &right {
            CqlValue::Integer(i) => *i as f64,
            CqlValue::Long(l) => *l as f64,
            CqlValue::Decimal(d) => d.to_f64().unwrap_or(0.0),
            _ => return Err(EvalError::unsupported_operator("Log", right.get_type().name())),
        };

        if value <= 0.0 || base <= 0.0 || base == 1.0 {
            return Ok(CqlValue::Null);
        }

        let result = value.log(base);
        Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
    }

    /// Evaluate Successor operator
    pub fn eval_successor(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Integer(i) => {
                if *i == i32::MAX {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Integer(i + 1))
                }
            }
            CqlValue::Long(l) => {
                if *l == i64::MAX {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(l + 1))
                }
            }
            CqlValue::Decimal(d) => {
                // Smallest decimal increment
                let epsilon = Decimal::new(1, 8);
                Ok(CqlValue::Decimal(d + epsilon))
            }
            CqlValue::Date(date) => {
                // Add one day
                if let Some(naive) = date.to_naive_date() {
                    let next = naive + chrono::Duration::days(1);
                    // CQL spec: valid years are 1-9999
                    if next.year() > 9999 {
                        return Err(EvalError::overflow("Successor: Date year exceeds maximum (9999)"));
                    }
                    Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(
                        next.year(),
                        next.month() as u8,
                        next.day() as u8,
                    )))
                } else {
                    Ok(CqlValue::Null)
                }
            }
            CqlValue::Time(time) => {
                // Add one millisecond
                let ms = time.to_milliseconds().unwrap_or(0);
                if ms >= 86_400_000 - 1 {
                    return Err(EvalError::overflow("Successor: Time exceeds maximum (23:59:59.999)"));
                } else {
                    let next_ms = ms + 1;
                    let h = (next_ms / 3_600_000) as u8;
                    let m = ((next_ms % 3_600_000) / 60_000) as u8;
                    let s = ((next_ms % 60_000) / 1_000) as u8;
                    let milli = (next_ms % 1_000) as u16;
                    Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(h, m, s, milli)))
                }
            }
            CqlValue::DateTime(dt) => {
                // Get the precision of the DateTime and add 1 at that precision
                let precision = dt.precision();
                match precision {
                    DateTimePrecision::Year => {
                        if dt.year == 9999 {
                            Ok(CqlValue::Null)
                        } else {
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: dt.year + 1,
                                month: None,
                                day: None,
                                hour: None,
                                minute: None,
                                second: None,
                                millisecond: None,
                                timezone_offset: dt.timezone_offset,
                            }))
                        }
                    }
                    DateTimePrecision::Month => {
                        let month = dt.month.unwrap_or(1);
                        if dt.year == 9999 && month == 12 {
                            Ok(CqlValue::Null)
                        } else if month == 12 {
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: dt.year + 1,
                                month: Some(1),
                                day: None,
                                hour: None,
                                minute: None,
                                second: None,
                                millisecond: None,
                                timezone_offset: dt.timezone_offset,
                            }))
                        } else {
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: dt.year,
                                month: Some(month + 1),
                                day: None,
                                hour: None,
                                minute: None,
                                second: None,
                                millisecond: None,
                                timezone_offset: dt.timezone_offset,
                            }))
                        }
                    }
                    _ => {
                        // For day or more precision, use chrono
                        let naive_date = chrono::NaiveDate::from_ymd_opt(
                            dt.year,
                            dt.month.unwrap_or(1) as u32,
                            dt.day.unwrap_or(1) as u32,
                        );
                        if let Some(date) = naive_date {
                            let naive_time = chrono::NaiveTime::from_hms_milli_opt(
                                dt.hour.unwrap_or(0) as u32,
                                dt.minute.unwrap_or(0) as u32,
                                dt.second.unwrap_or(0) as u32,
                                dt.millisecond.unwrap_or(0) as u32,
                            ).unwrap_or_default();
                            let naive = chrono::NaiveDateTime::new(date, naive_time);

                            // Add at the appropriate precision
                            let next: chrono::NaiveDateTime = match precision {
                                DateTimePrecision::Day =>
                                    naive + chrono::Duration::days(1),
                                DateTimePrecision::Hour =>
                                    naive + chrono::Duration::hours(1),
                                DateTimePrecision::Minute =>
                                    naive + chrono::Duration::minutes(1),
                                DateTimePrecision::Second =>
                                    naive + chrono::Duration::seconds(1),
                                DateTimePrecision::Millisecond =>
                                    naive + chrono::Duration::milliseconds(1),
                                _ => naive,
                            };
                            // CQL spec: valid years are 1-9999
                            if next.year() > 9999 {
                                return Err(EvalError::overflow("Successor: DateTime year exceeds maximum (9999)"));
                            }
                            // Preserve original precision
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: next.year(),
                                month: dt.month.map(|_| next.month() as u8),
                                day: dt.day.map(|_| next.day() as u8),
                                hour: dt.hour.map(|_| next.hour() as u8),
                                minute: dt.minute.map(|_| next.minute() as u8),
                                second: dt.second.map(|_| next.second() as u8),
                                millisecond: dt.millisecond.map(|_| (next.nanosecond() / 1_000_000) as u16),
                                timezone_offset: dt.timezone_offset,
                            }))
                        } else {
                            Ok(CqlValue::Null)
                        }
                    }
                }
            }
            CqlValue::Quantity(q) => {
                let epsilon = Decimal::new(1, 8);
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: q.value + epsilon,
                    unit: q.unit.clone(),
                }))
            }
            _ => Err(EvalError::unsupported_operator("Successor", operand.get_type().name())),
        }
    }

    /// Evaluate Predecessor operator
    pub fn eval_predecessor(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Integer(i) => {
                if *i == i32::MIN {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Integer(i - 1))
                }
            }
            CqlValue::Long(l) => {
                if *l == i64::MIN {
                    Ok(CqlValue::Null)
                } else {
                    Ok(CqlValue::Long(l - 1))
                }
            }
            CqlValue::Decimal(d) => {
                let epsilon = Decimal::new(1, 8);
                Ok(CqlValue::Decimal(d - epsilon))
            }
            CqlValue::Date(date) => {
                if let Some(naive) = date.to_naive_date() {
                    let prev = naive - chrono::Duration::days(1);
                    // CQL spec: valid years are 1-9999
                    if prev.year() < 1 {
                        return Err(EvalError::overflow("Predecessor: Date year below minimum (1)"));
                    }
                    Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(
                        prev.year(),
                        prev.month() as u8,
                        prev.day() as u8,
                    )))
                } else {
                    Ok(CqlValue::Null)
                }
            }
            CqlValue::Time(time) => {
                let ms = time.to_milliseconds().unwrap_or(0);
                if ms == 0 {
                    return Err(EvalError::overflow("Predecessor: Time below minimum (00:00:00.000)"));
                } else {
                    let prev_ms = ms - 1;
                    let h = (prev_ms / 3_600_000) as u8;
                    let m = ((prev_ms % 3_600_000) / 60_000) as u8;
                    let s = ((prev_ms % 60_000) / 1_000) as u8;
                    let milli = (prev_ms % 1_000) as u16;
                    Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(h, m, s, milli)))
                }
            }
            CqlValue::DateTime(dt) => {
                // Get the precision of the DateTime and subtract 1 at that precision
                let precision = dt.precision();
                match precision {
                    DateTimePrecision::Year => {
                        if dt.year == 1 {
                            Ok(CqlValue::Null)
                        } else {
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: dt.year - 1,
                                month: None,
                                day: None,
                                hour: None,
                                minute: None,
                                second: None,
                                millisecond: None,
                                timezone_offset: dt.timezone_offset,
                            }))
                        }
                    }
                    DateTimePrecision::Month => {
                        let month = dt.month.unwrap_or(1);
                        if dt.year == 1 && month == 1 {
                            Ok(CqlValue::Null)
                        } else if month == 1 {
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: dt.year - 1,
                                month: Some(12),
                                day: None,
                                hour: None,
                                minute: None,
                                second: None,
                                millisecond: None,
                                timezone_offset: dt.timezone_offset,
                            }))
                        } else {
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: dt.year,
                                month: Some(month - 1),
                                day: None,
                                hour: None,
                                minute: None,
                                second: None,
                                millisecond: None,
                                timezone_offset: dt.timezone_offset,
                            }))
                        }
                    }
                    _ => {
                        // For day or more precision, use chrono
                        let naive_date = chrono::NaiveDate::from_ymd_opt(
                            dt.year,
                            dt.month.unwrap_or(1) as u32,
                            dt.day.unwrap_or(1) as u32,
                        );
                        if let Some(date) = naive_date {
                            let naive_time = chrono::NaiveTime::from_hms_milli_opt(
                                dt.hour.unwrap_or(0) as u32,
                                dt.minute.unwrap_or(0) as u32,
                                dt.second.unwrap_or(0) as u32,
                                dt.millisecond.unwrap_or(0) as u32,
                            ).unwrap_or_default();
                            let naive = chrono::NaiveDateTime::new(date, naive_time);

                            // Subtract at the appropriate precision
                            let prev: chrono::NaiveDateTime = match precision {
                                DateTimePrecision::Day =>
                                    naive - chrono::Duration::days(1),
                                DateTimePrecision::Hour =>
                                    naive - chrono::Duration::hours(1),
                                DateTimePrecision::Minute =>
                                    naive - chrono::Duration::minutes(1),
                                DateTimePrecision::Second =>
                                    naive - chrono::Duration::seconds(1),
                                DateTimePrecision::Millisecond =>
                                    naive - chrono::Duration::milliseconds(1),
                                _ => naive,
                            };
                            // CQL spec: valid years are 1-9999
                            if prev.year() < 1 {
                                return Err(EvalError::overflow("Predecessor: DateTime year below minimum (1)"));
                            }
                            // Preserve original precision
                            Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
                                year: prev.year(),
                                month: dt.month.map(|_| prev.month() as u8),
                                day: dt.day.map(|_| prev.day() as u8),
                                hour: dt.hour.map(|_| prev.hour() as u8),
                                minute: dt.minute.map(|_| prev.minute() as u8),
                                second: dt.second.map(|_| prev.second() as u8),
                                millisecond: dt.millisecond.map(|_| (prev.nanosecond() / 1_000_000) as u16),
                                timezone_offset: dt.timezone_offset,
                            }))
                        } else {
                            Ok(CqlValue::Null)
                        }
                    }
                }
            }
            CqlValue::Quantity(q) => {
                let epsilon = Decimal::new(1, 8);
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: q.value - epsilon,
                    unit: q.unit.clone(),
                }))
            }
            _ => Err(EvalError::unsupported_operator("Predecessor", operand.get_type().name())),
        }
    }

    /// Evaluate MinValue operator
    pub fn eval_min_value(&self, expr: &MinMaxValueExpression) -> EvalResult<CqlValue> {
        let type_name = expr.value_type.rsplit('}').next().unwrap_or(&expr.value_type);

        match type_name {
            "Integer" => Ok(CqlValue::Integer(i32::MIN)),
            "Long" => Ok(CqlValue::Long(i64::MIN)),
            // CQL spec defines Decimal min as -99999999999999999999.99999999
            "Decimal" => {
                // Use from_str with FromStr trait
                let min_decimal = "-99999999999999999999.99999999".parse::<Decimal>()
                    .unwrap_or(Decimal::MIN);
                Ok(CqlValue::Decimal(min_decimal))
            }
            "Date" => Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(1, 1, 1))),
            "DateTime" => Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime::new(
                1, 1, 1, 0, 0, 0, 0, Some(0),
            ))),
            "Time" => Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(0, 0, 0, 0))),
            _ => Err(EvalError::unsupported_expression(format!("MinValue for {}", type_name))),
        }
    }

    /// Evaluate MaxValue operator
    pub fn eval_max_value(&self, expr: &MinMaxValueExpression) -> EvalResult<CqlValue> {
        let type_name = expr.value_type.rsplit('}').next().unwrap_or(&expr.value_type);

        match type_name {
            "Integer" => Ok(CqlValue::Integer(i32::MAX)),
            "Long" => Ok(CqlValue::Long(i64::MAX)),
            // CQL spec defines Decimal max as 99999999999999999999.99999999
            "Decimal" => {
                // Use from_str with FromStr trait
                let max_decimal = "99999999999999999999.99999999".parse::<Decimal>()
                    .unwrap_or(Decimal::MAX);
                Ok(CqlValue::Decimal(max_decimal))
            }
            "Date" => Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(9999, 12, 31))),
            "DateTime" => Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime::new(
                9999, 12, 31, 23, 59, 59, 999, Some(0),
            ))),
            "Time" => Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(23, 59, 59, 999))),
            _ => Err(EvalError::unsupported_expression(format!("MaxValue for {}", type_name))),
        }
    }

    /// Evaluate Precision operator
    pub fn eval_precision(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        match &operand {
            CqlValue::Decimal(d) => {
                // Count decimal places
                let scale = d.scale();
                Ok(CqlValue::Integer(scale as i32))
            }
            CqlValue::Date(d) => {
                let precision = d.precision();
                Ok(CqlValue::Integer(precision_to_int(&precision)))
            }
            CqlValue::DateTime(dt) => {
                let precision = dt.precision();
                Ok(CqlValue::Integer(precision_to_int(&precision)))
            }
            CqlValue::Time(t) => {
                let precision = t.precision();
                Ok(CqlValue::Integer(time_precision_to_int(&precision)))
            }
            _ => Err(EvalError::unsupported_operator("Precision", operand.get_type().name())),
        }
    }

    /// Evaluate LowBoundary operator
    pub fn eval_low_boundary(&self, expr: &BoundaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        let precision = if let Some(prec_expr) = &expr.precision {
            match self.evaluate(prec_expr, ctx)? {
                CqlValue::Integer(p) => Some(p as u32),
                CqlValue::Null => None,
                _ => return Err(EvalError::invalid_operand("LowBoundary", "precision must be Integer")),
            }
        } else {
            None
        };

        match &operand {
            CqlValue::Decimal(d) => {
                // LowBoundary: extend precision by filling with 0s
                // e.g., 1.587 with precision 8 -> 1.58700000
                let target_scale = precision.unwrap_or(8);
                // Rescale to target precision (this adds trailing zeros if needed)
                let mut result = *d;
                result.rescale(target_scale);
                Ok(CqlValue::Decimal(result))
            }
            CqlValue::Date(date) => {
                // Fill in missing components with minimum values, respecting precision
                // Precision: 4=year, 6=month, 8=day
                let target_precision = precision.unwrap_or(8);
                let month = if target_precision >= 6 {
                    Some(date.month.unwrap_or(1))
                } else {
                    date.month
                };
                let day = if target_precision >= 8 {
                    Some(date.day.or(month.map(|_| 1)).unwrap_or(1))
                } else {
                    date.day
                };
                Ok(CqlValue::Date(octofhir_cql_types::CqlDate {
                    year: date.year,
                    month,
                    day,
                }))
            }
            CqlValue::DateTime(dt) => {
                Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime::new(
                    dt.year,
                    dt.month.unwrap_or(1),
                    dt.day.unwrap_or(1),
                    dt.hour.unwrap_or(0),
                    dt.minute.unwrap_or(0),
                    dt.second.unwrap_or(0),
                    dt.millisecond.unwrap_or(0),
                    dt.timezone_offset,
                )))
            }
            CqlValue::Time(t) => {
                Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(
                    t.hour,
                    t.minute.unwrap_or(0),
                    t.second.unwrap_or(0),
                    t.millisecond.unwrap_or(0),
                )))
            }
            _ => Err(EvalError::unsupported_operator("LowBoundary", operand.get_type().name())),
        }
    }

    /// Evaluate HighBoundary operator
    pub fn eval_high_boundary(&self, expr: &BoundaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        let precision = if let Some(prec_expr) = &expr.precision {
            match self.evaluate(prec_expr, ctx)? {
                CqlValue::Integer(p) => Some(p as u32),
                CqlValue::Null => None,
                _ => return Err(EvalError::invalid_operand("HighBoundary", "precision must be Integer")),
            }
        } else {
            None
        };

        match &operand {
            CqlValue::Decimal(d) => {
                // HighBoundary: extend precision by filling with 9s
                // e.g., 1.587 with precision 8 -> 1.58799999
                let target_scale = precision.unwrap_or(8);
                let current_scale = d.scale();

                if current_scale >= target_scale {
                    // Already at or beyond target precision
                    let mut result = *d;
                    result.rescale(target_scale);
                    Ok(CqlValue::Decimal(result))
                } else {
                    // Add offset to fill with 9s
                    // offset = (10^extra_digits - 1) / 10^target_scale
                    let extra_digits = target_scale - current_scale;
                    let offset_numerator = Decimal::from(10u64.pow(extra_digits)) - Decimal::ONE;
                    let offset_divisor = Decimal::from(10u64.pow(target_scale));
                    let offset = offset_numerator / offset_divisor;
                    let result = *d + offset;
                    let mut final_result = result;
                    final_result.rescale(target_scale);
                    Ok(CqlValue::Decimal(final_result))
                }
            }
            CqlValue::Date(date) => {
                // Fill in missing components with maximum values, respecting precision
                // Precision: 4=year, 6=month, 8=day
                let target_precision = precision.unwrap_or(8);
                let month = if target_precision >= 6 {
                    Some(date.month.unwrap_or(12))
                } else {
                    date.month
                };
                let day = if target_precision >= 8 {
                    let m = month.unwrap_or(12);
                    Some(date.day.unwrap_or_else(|| days_in_month(date.year, m)))
                } else {
                    date.day
                };
                Ok(CqlValue::Date(octofhir_cql_types::CqlDate {
                    year: date.year,
                    month,
                    day,
                }))
            }
            CqlValue::DateTime(dt) => {
                let year = dt.year;
                let month = dt.month.unwrap_or(12);
                let day = dt.day.unwrap_or_else(|| days_in_month(year, month));
                Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime::new(
                    year,
                    month,
                    day,
                    dt.hour.unwrap_or(23),
                    dt.minute.unwrap_or(59),
                    dt.second.unwrap_or(59),
                    dt.millisecond.unwrap_or(999),
                    dt.timezone_offset,
                )))
            }
            CqlValue::Time(t) => {
                Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(
                    t.hour,
                    t.minute.unwrap_or(59),
                    t.second.unwrap_or(59),
                    t.millisecond.unwrap_or(999),
                )))
            }
            _ => Err(EvalError::unsupported_operator("HighBoundary", operand.get_type().name())),
        }
    }

    // =========================================================================
    // Helper methods
    // =========================================================================

    /// Evaluate binary expression operands
    pub(crate) fn eval_binary_operands(
        &self,
        expr: &BinaryExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<(CqlValue, CqlValue)> {
        if expr.operand.len() != 2 {
            return Err(EvalError::internal("Binary expression must have exactly 2 operands"));
        }
        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;
        Ok((left, right))
    }
}

/// Convert DateTimePrecision to integer for Precision operator (DateTime/Date)
fn precision_to_int(precision: &DateTimePrecision) -> i32 {
    match precision {
        DateTimePrecision::Year => 4,
        DateTimePrecision::Month => 6,
        DateTimePrecision::Day => 8,
        DateTimePrecision::Hour => 10,
        DateTimePrecision::Minute => 12,
        DateTimePrecision::Second => 14,
        DateTimePrecision::Millisecond => 17,
    }
}

/// Convert DateTimePrecision to integer for Precision operator (Time only)
/// Time precision counts significant digits: HH=2, MM=4, SS=6, mmm=9
fn time_precision_to_int(precision: &DateTimePrecision) -> i32 {
    match precision {
        DateTimePrecision::Hour => 2,
        DateTimePrecision::Minute => 4,
        DateTimePrecision::Second => 6,
        DateTimePrecision::Millisecond => 9,
        // These shouldn't occur for Time, but provide reasonable defaults
        _ => 0,
    }
}

/// Get number of days in a month
fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

/// Check if year is leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Calendar unit for temporal arithmetic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalendarUnit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

impl CalendarUnit {
    /// Parse calendar unit from a UCUM or CQL duration unit string
    fn from_unit_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "year" | "years" | "a" => Some(Self::Year),
            "month" | "months" | "mo" => Some(Self::Month),
            "week" | "weeks" | "wk" => Some(Self::Week),
            "day" | "days" | "d" => Some(Self::Day),
            "hour" | "hours" | "h" => Some(Self::Hour),
            "minute" | "minutes" | "min" => Some(Self::Minute),
            "second" | "seconds" | "s" => Some(Self::Second),
            "millisecond" | "milliseconds" | "ms" => Some(Self::Millisecond),
            _ => None,
        }
    }
}

/// Validate that a year is within CQL's valid range (1-9999)
fn validate_year(year: i32, operation: &str) -> EvalResult<()> {
    if year < 1 || year > 9999 {
        Err(EvalError::overflow(format!("{}: year {} out of range (1-9999)", operation, year)))
    } else {
        Ok(())
    }
}

/// Subtract a duration quantity from a date
/// Handles partial precision dates (year-only, year-month, or full date)
fn subtract_duration_from_date(
    date: &octofhir_cql_types::CqlDate,
    quantity: &CqlQuantity,
) -> EvalResult<CqlValue> {
    let unit_str = quantity.unit.as_deref().unwrap_or("day");
    let calendar_unit = CalendarUnit::from_unit_string(unit_str).ok_or_else(|| {
        EvalError::invalid_operand(
            "Date - Quantity",
            format!("Unknown duration unit: {}", unit_str),
        )
    })?;

    // Convert quantity value to i64 (truncating decimals)
    let amount = quantity.value.to_i64().unwrap_or(0);

    match calendar_unit {
        CalendarUnit::Year => {
            // Year operations only need year precision
            let new_year = date.year - amount as i32;
            validate_year(new_year, "Date - years")?;
            Ok(CqlValue::Date(octofhir_cql_types::CqlDate {
                year: new_year,
                month: date.month,
                day: date.day,
            }))
        }
        CalendarUnit::Month => {
            // Month operations need at least year and month
            if let Some(month) = date.month {
                let total_months = date.year as i64 * 12 + month as i64 - 1 - amount;
                let new_year = (total_months / 12) as i32;
                let new_month = (total_months.rem_euclid(12) + 1) as u8;
                validate_year(new_year, "Date - months")?;

                // Handle day overflow if day is present
                let new_day = if let Some(day) = date.day {
                    let max_day = days_in_month(new_year, new_month);
                    Some(day.min(max_day))
                } else {
                    None
                };

                Ok(CqlValue::Date(octofhir_cql_types::CqlDate {
                    year: new_year,
                    month: Some(new_month),
                    day: new_day,
                }))
            } else {
                // If no month precision, convert months to whole years
                // e.g., 24 months = 2 years, 25 months = 2 years (truncated)
                let years_from_months = amount / 12;
                let new_year = date.year - years_from_months as i32;
                validate_year(new_year, "Date - months")?;
                Ok(CqlValue::Date(octofhir_cql_types::CqlDate {
                    year: new_year,
                    month: None,
                    day: None,
                }))
            }
        }
        CalendarUnit::Week | CalendarUnit::Day => {
            // For full-precision dates, use chrono for accurate day/week arithmetic
            if let Some(naive_date) = date.to_naive_date() {
                let result_date = if calendar_unit == CalendarUnit::Week {
                    naive_date
                        .checked_sub_signed(chrono::Duration::weeks(amount))
                        .ok_or_else(|| EvalError::overflow("Date - weeks"))?
                } else {
                    naive_date
                        .checked_sub_signed(chrono::Duration::days(amount))
                        .ok_or_else(|| EvalError::overflow("Date - days"))?
                };
                validate_year(result_date.year(), "Date - days")?;

                Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(
                    result_date.year(),
                    result_date.month() as u8,
                    result_date.day() as u8,
                )))
            } else if date.month.is_some() {
                // Year-month precision: convert days/weeks to months
                let days = if calendar_unit == CalendarUnit::Week { amount * 7 } else { amount };
                // Use 30 days per month for conversion
                let months_from_days = days / 30;

                let total_months = date.year as i64 * 12 + date.month.unwrap() as i64 - 1 - months_from_days;
                let new_year = (total_months / 12) as i32;
                let new_month = (total_months.rem_euclid(12) + 1) as u8;
                validate_year(new_year, "Date - days")?;

                Ok(CqlValue::Date(octofhir_cql_types::CqlDate {
                    year: new_year,
                    month: Some(new_month),
                    day: None,
                }))
            } else {
                // Year-only precision: convert days/weeks to years
                let days = if calendar_unit == CalendarUnit::Week { amount * 7 } else { amount };
                let years_from_days = days / 365;
                let new_year = date.year - years_from_days as i32;
                validate_year(new_year, "Date - days")?;

                Ok(CqlValue::Date(octofhir_cql_types::CqlDate {
                    year: new_year,
                    month: None,
                    day: None,
                }))
            }
        }
        CalendarUnit::Hour => {
            // Hours on a date - convert to days (24 hours = 1 day)
            if let Some(naive_date) = date.to_naive_date() {
                let days = amount / 24;
                let result_date = naive_date
                    .checked_sub_signed(chrono::Duration::days(days))
                    .ok_or_else(|| EvalError::overflow("Date - hours"))?;
                validate_year(result_date.year(), "Date - hours")?;
                Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(
                    result_date.year(),
                    result_date.month() as u8,
                    result_date.day() as u8,
                )))
            } else {
                // If no day precision, hour operations don't change the date
                Ok(CqlValue::Date(date.clone()))
            }
        }
        CalendarUnit::Minute | CalendarUnit::Second | CalendarUnit::Millisecond => {
            // Sub-day units on a date don't change the date
            Ok(CqlValue::Date(date.clone()))
        }
    }
}

/// Subtract a duration quantity from a datetime
fn subtract_duration_from_datetime(
    datetime: &octofhir_cql_types::CqlDateTime,
    quantity: &CqlQuantity,
) -> EvalResult<CqlValue> {
    let unit_str = quantity.unit.as_deref().unwrap_or("day");
    let calendar_unit = CalendarUnit::from_unit_string(unit_str).ok_or_else(|| {
        EvalError::invalid_operand(
            "DateTime - Quantity",
            format!("Unknown duration unit: {}", unit_str),
        )
    })?;

    // Convert quantity value to i64 (truncating decimals)
    let amount = quantity.value.to_i64().unwrap_or(0);

    // Build a chrono DateTime from CqlDateTime
    let naive_date = chrono::NaiveDate::from_ymd_opt(
        datetime.year,
        datetime.month.unwrap_or(1) as u32,
        datetime.day.unwrap_or(1) as u32,
    )
    .ok_or_else(|| EvalError::invalid_operand("DateTime - Quantity", "Invalid date components"))?;

    let naive_time = chrono::NaiveTime::from_hms_milli_opt(
        datetime.hour.unwrap_or(0) as u32,
        datetime.minute.unwrap_or(0) as u32,
        datetime.second.unwrap_or(0) as u32,
        datetime.millisecond.unwrap_or(0) as u32,
    )
    .ok_or_else(|| EvalError::invalid_operand("DateTime - Quantity", "Invalid time components"))?;

    let naive_datetime = chrono::NaiveDateTime::new(naive_date, naive_time);

    let result_datetime = match calendar_unit {
        CalendarUnit::Year => {
            let new_year = naive_datetime.year() - amount as i32;
            let new_date = chrono::NaiveDate::from_ymd_opt(
                new_year,
                naive_datetime.month(),
                naive_datetime.day(),
            )
            .or_else(|| chrono::NaiveDate::from_ymd_opt(new_year, naive_datetime.month(), 28))
            .ok_or_else(|| EvalError::overflow("DateTime - years"))?;
            chrono::NaiveDateTime::new(new_date, naive_datetime.time())
        }
        CalendarUnit::Month => {
            // For year-only precision, convert months to whole years
            if datetime.month.is_none() {
                let years_from_months = amount / 12;
                let new_year = naive_datetime.year() - years_from_months as i32;
                let new_date = chrono::NaiveDate::from_ymd_opt(new_year, 1, 1)
                    .ok_or_else(|| EvalError::overflow("DateTime - months"))?;
                chrono::NaiveDateTime::new(new_date, naive_datetime.time())
            } else {
                let total_months =
                    naive_datetime.year() as i64 * 12 + naive_datetime.month() as i64 - 1 - amount;
                let new_year = (total_months / 12) as i32;
                let new_month = (total_months.rem_euclid(12) + 1) as u32;

                let new_date = chrono::NaiveDate::from_ymd_opt(new_year, new_month, naive_datetime.day())
                    .or_else(|| {
                        let next_month = chrono::NaiveDate::from_ymd_opt(new_year, new_month + 1, 1)
                            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(new_year + 1, 1, 1).unwrap());
                        next_month.pred_opt()
                    })
                    .ok_or_else(|| EvalError::overflow("DateTime - months"))?;
                chrono::NaiveDateTime::new(new_date, naive_datetime.time())
            }
        }
        CalendarUnit::Week => naive_datetime
            .checked_sub_signed(chrono::Duration::weeks(amount))
            .ok_or_else(|| EvalError::overflow("DateTime - weeks"))?,
        CalendarUnit::Day => naive_datetime
            .checked_sub_signed(chrono::Duration::days(amount))
            .ok_or_else(|| EvalError::overflow("DateTime - days"))?,
        CalendarUnit::Hour => naive_datetime
            .checked_sub_signed(chrono::Duration::hours(amount))
            .ok_or_else(|| EvalError::overflow("DateTime - hours"))?,
        CalendarUnit::Minute => naive_datetime
            .checked_sub_signed(chrono::Duration::minutes(amount))
            .ok_or_else(|| EvalError::overflow("DateTime - minutes"))?,
        CalendarUnit::Second => naive_datetime
            .checked_sub_signed(chrono::Duration::seconds(amount))
            .ok_or_else(|| EvalError::overflow("DateTime - seconds"))?,
        CalendarUnit::Millisecond => naive_datetime
            .checked_sub_signed(chrono::Duration::milliseconds(amount))
            .ok_or_else(|| EvalError::overflow("DateTime - milliseconds"))?,
    };

    // CQL spec: valid years are 1-9999
    validate_year(result_datetime.year(), "DateTime arithmetic")?;

    // Preserve the original precision
    Ok(CqlValue::DateTime(octofhir_cql_types::CqlDateTime {
        year: result_datetime.year(),
        month: datetime.month.map(|_| result_datetime.month() as u8),
        day: datetime.day.map(|_| result_datetime.day() as u8),
        hour: datetime.hour.map(|_| result_datetime.hour() as u8),
        minute: datetime.minute.map(|_| result_datetime.minute() as u8),
        second: datetime.second.map(|_| result_datetime.second() as u8),
        millisecond: datetime.millisecond.map(|_| (result_datetime.nanosecond() / 1_000_000) as u16),
        timezone_offset: datetime.timezone_offset,
    }))
}

/// Add a duration quantity to a date
fn add_duration_to_date(
    date: &octofhir_cql_types::CqlDate,
    quantity: &CqlQuantity,
) -> EvalResult<CqlValue> {
    // Negate the quantity and use subtract
    let negated = CqlQuantity {
        value: -quantity.value,
        unit: quantity.unit.clone(),
    };
    subtract_duration_from_date(date, &negated)
}

/// Add a duration quantity to a datetime
fn add_duration_to_datetime(
    datetime: &octofhir_cql_types::CqlDateTime,
    quantity: &CqlQuantity,
) -> EvalResult<CqlValue> {
    // Negate the quantity and use subtract
    let negated = CqlQuantity {
        value: -quantity.value,
        unit: quantity.unit.clone(),
    };
    subtract_duration_from_datetime(datetime, &negated)
}

/// Subtract a duration quantity from a time
fn subtract_duration_from_time(
    time: &octofhir_cql_types::CqlTime,
    quantity: &CqlQuantity,
) -> EvalResult<CqlValue> {
    let unit_str = quantity.unit.as_deref().unwrap_or("hour");
    let calendar_unit = CalendarUnit::from_unit_string(unit_str).ok_or_else(|| {
        EvalError::invalid_operand(
            "Time - Quantity",
            format!("Unknown duration unit: {}", unit_str),
        )
    })?;

    // Convert quantity value to i64 (truncating decimals)
    let amount = quantity.value.to_i64().unwrap_or(0);

    // Convert time to total milliseconds
    let hour = time.hour as i64;
    let minute = time.minute.unwrap_or(0) as i64;
    let second = time.second.unwrap_or(0) as i64;
    let ms = time.millisecond.unwrap_or(0) as i64;

    let total_ms = ((hour * 60 + minute) * 60 + second) * 1000 + ms;

    let delta_ms = match calendar_unit {
        CalendarUnit::Hour => amount * 60 * 60 * 1000,
        CalendarUnit::Minute => amount * 60 * 1000,
        CalendarUnit::Second => amount * 1000,
        CalendarUnit::Millisecond => amount,
        _ => {
            // Days, weeks, months, years don't affect time-only values
            return Ok(CqlValue::Time(time.clone()));
        }
    };

    // Subtract the duration
    let result_ms = total_ms - delta_ms;

    // Wrap around to keep within 24 hours (0-86400000 ms)
    let ms_per_day = 24 * 60 * 60 * 1000;
    let wrapped_ms = result_ms.rem_euclid(ms_per_day);

    // Convert back to time components
    let result_hour = (wrapped_ms / (60 * 60 * 1000)) as u8;
    let result_minute = ((wrapped_ms / (60 * 1000)) % 60) as u8;
    let result_second = ((wrapped_ms / 1000) % 60) as u8;
    let result_ms = (wrapped_ms % 1000) as u16;

    Ok(CqlValue::Time(octofhir_cql_types::CqlTime {
        hour: result_hour,
        minute: if time.minute.is_some() { Some(result_minute) } else { None },
        second: if time.second.is_some() { Some(result_second) } else { None },
        millisecond: if time.millisecond.is_some() { Some(result_ms) } else { None },
    }))
}

/// Add a duration quantity to a time
fn add_duration_to_time(
    time: &octofhir_cql_types::CqlTime,
    quantity: &CqlQuantity,
) -> EvalResult<CqlValue> {
    // Negate the quantity and use subtract
    let negated = CqlQuantity {
        value: -quantity.value,
        unit: quantity.unit.clone(),
    };
    subtract_duration_from_time(time, &negated)
}

/// Add two intervals for uncertainty propagation
/// result.low = left.low + right.low
/// result.high = left.high + right.high
fn interval_add(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    let (al, ah) = match (a.low(), a.high()) {
        (Some(l), Some(h)) => (l, h),
        _ => return Ok(CqlValue::Null),
    };
    let (bl, bh) = match (b.low(), b.high()) {
        (Some(l), Some(h)) => (l, h),
        _ => return Ok(CqlValue::Null),
    };

    // Add lows
    let new_low = match (al, bl) {
        (CqlValue::Integer(x), CqlValue::Integer(y)) => {
            x.checked_add(*y).map(CqlValue::Integer)
        }
        _ => None,
    };

    // Add highs
    let new_high = match (ah, bh) {
        (CqlValue::Integer(x), CqlValue::Integer(y)) => {
            x.checked_add(*y).map(CqlValue::Integer)
        }
        _ => None,
    };

    match (new_low, new_high) {
        (Some(low), Some(high)) => Ok(CqlValue::Interval(CqlInterval::new(
            CqlType::Integer,
            Some(low),
            true,
            Some(high),
            true,
        ))),
        _ => Ok(CqlValue::Null),
    }
}

/// Subtract two intervals for uncertainty propagation
/// result.low = left.low - right.high
/// result.high = left.high - right.low
fn interval_subtract(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    let (al, ah) = match (a.low(), a.high()) {
        (Some(l), Some(h)) => (l, h),
        _ => return Ok(CqlValue::Null),
    };
    let (bl, bh) = match (b.low(), b.high()) {
        (Some(l), Some(h)) => (l, h),
        _ => return Ok(CqlValue::Null),
    };

    // result_low = a.low - b.high (smallest result)
    let new_low = match (al, bh) {
        (CqlValue::Integer(x), CqlValue::Integer(y)) => {
            x.checked_sub(*y).map(CqlValue::Integer)
        }
        _ => None,
    };

    // result_high = a.high - b.low (largest result)
    let new_high = match (ah, bl) {
        (CqlValue::Integer(x), CqlValue::Integer(y)) => {
            x.checked_sub(*y).map(CqlValue::Integer)
        }
        _ => None,
    };

    match (new_low, new_high) {
        (Some(low), Some(high)) => Ok(CqlValue::Interval(CqlInterval::new(
            CqlType::Integer,
            Some(low),
            true,
            Some(high),
            true,
        ))),
        _ => Ok(CqlValue::Null),
    }
}

/// Multiply two intervals for uncertainty propagation
/// result.low = min of all products
/// result.high = max of all products
fn interval_multiply(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    let (al, ah) = match (a.low(), a.high()) {
        (Some(l), Some(h)) => (l, h),
        _ => return Ok(CqlValue::Null),
    };
    let (bl, bh) = match (b.low(), b.high()) {
        (Some(l), Some(h)) => (l, h),
        _ => return Ok(CqlValue::Null),
    };

    // Get all integer values
    let vals = match (al, ah, bl, bh) {
        (CqlValue::Integer(al), CqlValue::Integer(ah), CqlValue::Integer(bl), CqlValue::Integer(bh)) => {
            // Calculate all products
            let products = [
                al.checked_mul(*bl),
                al.checked_mul(*bh),
                ah.checked_mul(*bl),
                ah.checked_mul(*bh),
            ];

            // Find min and max
            let mut min: Option<i32> = None;
            let mut max: Option<i32> = None;
            for p in products.iter().flatten() {
                min = Some(min.map_or(*p, |m| m.min(*p)));
                max = Some(max.map_or(*p, |m| m.max(*p)));
            }

            match (min, max) {
                (Some(l), Some(h)) => Some((l, h)),
                _ => None,
            }
        }
        _ => None,
    };

    match vals {
        Some((low, high)) => Ok(CqlValue::Interval(CqlInterval::new(
            CqlType::Integer,
            Some(CqlValue::Integer(low)),
            true,
            Some(CqlValue::Integer(high)),
            true,
        ))),
        None => Ok(CqlValue::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> CqlEngine {
        CqlEngine::new()
    }

    fn ctx() -> EvaluationContext {
        EvaluationContext::new()
    }

    #[test]
    fn test_add_integers() {
        let e = engine();
        let mut c = ctx();

        let result = e.eval_add(
            &make_binary_expr(CqlValue::Integer(2), CqlValue::Integer(3)),
            &mut c,
        ).unwrap();

        assert_eq!(result, CqlValue::Integer(5));
    }

    #[test]
    fn test_add_null_propagation() {
        let e = engine();
        let mut c = ctx();

        let result = e.eval_add(
            &make_binary_expr(CqlValue::Integer(2), CqlValue::Null),
            &mut c,
        ).unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn test_divide_by_zero() {
        let e = engine();
        let mut c = ctx();

        let result = e.eval_divide(
            &make_binary_expr(CqlValue::Integer(10), CqlValue::Integer(0)),
            &mut c,
        ).unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn test_negate() {
        let e = engine();
        let mut c = ctx();

        let result = e.eval_negate(
            &make_unary_expr(CqlValue::Integer(5)),
            &mut c,
        ).unwrap();

        assert_eq!(result, CqlValue::Integer(-5));
    }

    #[test]
    fn test_abs() {
        let e = engine();
        let mut c = ctx();

        let result = e.eval_abs(
            &make_unary_expr(CqlValue::Integer(-5)),
            &mut c,
        ).unwrap();

        assert_eq!(result, CqlValue::Integer(5));
    }

    // Helper to create binary expression for testing
    fn make_binary_expr(left: CqlValue, right: CqlValue) -> BinaryExpression {
        use octofhir_cql_elm::{Element, Literal, Expression};

        BinaryExpression {
            element: Element::default(),
            operand: vec![
                Box::new(value_to_expr(left)),
                Box::new(value_to_expr(right)),
            ],
        }
    }

    // Helper to create unary expression for testing
    fn make_unary_expr(operand: CqlValue) -> UnaryExpression {
        use octofhir_cql_elm::Element;

        UnaryExpression {
            element: Element::default(),
            operand: Box::new(value_to_expr(operand)),
        }
    }

    fn value_to_expr(value: CqlValue) -> octofhir_cql_elm::Expression {
        use octofhir_cql_elm::{Element, Literal, NullLiteral, Expression};

        match value {
            CqlValue::Null => Expression::Null(NullLiteral { element: Element::default() }),
            CqlValue::Integer(i) => Expression::Literal(Literal {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                value: Some(i.to_string()),
            }),
            CqlValue::Decimal(d) => Expression::Literal(Literal {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Decimal".to_string(),
                value: Some(d.to_string()),
            }),
            _ => Expression::Null(NullLiteral { element: Element::default() }),
        }
    }
}
