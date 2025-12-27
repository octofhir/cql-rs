//! Arithmetic Operators for CQL
//!
//! Implements: Add, Subtract, Multiply, Divide, TruncatedDivide, Modulo,
//! Power, Negate, Successor, Predecessor, Abs, Ceiling, Floor, Round,
//! Truncate, Exp, Ln, Log, MinValue, MaxValue, Precision, LowBoundary, HighBoundary

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use chrono::Datelike;
use octofhir_cql_elm::{BinaryExpression, BoundaryExpression, MinMaxValueExpression, RoundExpression, UnaryExpression};
use octofhir_cql_types::{CqlQuantity, CqlValue, DateTimePrecision};
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
            // String concatenation is not handled by Add in CQL (use Concatenate)
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
            CqlValue::Decimal(d) => Ok(CqlValue::Decimal(-d)),
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
            CqlValue::Integer(i) => Ok(CqlValue::Integer(*i)),
            CqlValue::Long(l) => Ok(CqlValue::Long(*l)),
            CqlValue::Decimal(d) => Ok(CqlValue::Decimal(d.round_dp(precision))),
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

        if value <= 0.0 {
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
            Ok(CqlValue::Null)
        } else {
            Ok(CqlValue::Decimal(Decimal::from_f64(result).unwrap_or(Decimal::ZERO)))
        }
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
                    Ok(CqlValue::Null)
                } else {
                    let next_ms = ms + 1;
                    let h = (next_ms / 3_600_000) as u8;
                    let m = ((next_ms % 3_600_000) / 60_000) as u8;
                    let s = ((next_ms % 60_000) / 1_000) as u8;
                    let milli = (next_ms % 1_000) as u16;
                    Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(h, m, s, milli)))
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
                    Ok(CqlValue::Null)
                } else {
                    let prev_ms = ms - 1;
                    let h = (prev_ms / 3_600_000) as u8;
                    let m = ((prev_ms % 3_600_000) / 60_000) as u8;
                    let s = ((prev_ms % 60_000) / 1_000) as u8;
                    let milli = (prev_ms % 1_000) as u16;
                    Ok(CqlValue::Time(octofhir_cql_types::CqlTime::new(h, m, s, milli)))
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
            "Decimal" => Ok(CqlValue::Decimal(Decimal::MIN)),
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
            "Decimal" => Ok(CqlValue::Decimal(Decimal::MAX)),
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
                Ok(CqlValue::Integer(precision_to_int(&precision)))
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
                let scale = precision.unwrap_or(8);
                Ok(CqlValue::Decimal(d.round_dp_with_strategy(scale, RoundingStrategy::RoundDown)))
            }
            CqlValue::Date(date) => {
                // Fill in missing components with minimum values
                Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(
                    date.year,
                    date.month.unwrap_or(1),
                    date.day.unwrap_or(1),
                )))
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
                let scale = precision.unwrap_or(8);
                Ok(CqlValue::Decimal(d.round_dp_with_strategy(scale, RoundingStrategy::RoundUp)))
            }
            CqlValue::Date(date) => {
                // Fill in missing components with maximum values
                let year = date.year;
                let month = date.month.unwrap_or(12);
                let day = date.day.unwrap_or_else(|| days_in_month(year, month));
                Ok(CqlValue::Date(octofhir_cql_types::CqlDate::new(year, month, day)))
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

/// Convert DateTimePrecision to integer for Precision operator
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
