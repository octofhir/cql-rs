//! Interval Operators for CQL
//!
//! Implements: Interval constructor, Start, End, PointFrom, Width, Size,
//! Contains, In, Includes, IncludedIn, ProperContains, ProperIn,
//! ProperIncludes, ProperIncludedIn, Before, After, Meets, MeetsBefore,
//! MeetsAfter, Overlaps, OverlapsBefore, OverlapsAfter, Starts, Ends,
//! Collapse, Expand, Union, Intersect, Except

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use crate::operators::comparison::{cql_compare, cql_equal};
use octofhir_cql_elm::{BinaryExpression, ExpandExpression, IntervalExpression, UnaryExpression};
use octofhir_cql_types::{CqlInterval, CqlList, CqlType, CqlValue};
use std::cmp::Ordering;

impl CqlEngine {
    /// Evaluate Interval constructor
    pub fn eval_interval_constructor(&self, expr: &IntervalExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let low = if let Some(low_expr) = &expr.low {
            Some(self.evaluate(low_expr, ctx)?)
        } else {
            None
        };

        let high = if let Some(high_expr) = &expr.high {
            Some(self.evaluate(high_expr, ctx)?)
        } else {
            None
        };

        // Determine point type
        let point_type = match (&low, &high) {
            (Some(l), _) if !l.is_null() => l.get_type(),
            (_, Some(h)) if !h.is_null() => h.get_type(),
            _ => CqlType::Any,
        };

        // Check for null boundaries
        let low_value = low.filter(|v| !v.is_null());
        let high_value = high.filter(|v| !v.is_null());

        Ok(CqlValue::Interval(CqlInterval::new(
            point_type,
            low_value,
            expr.low_closed.unwrap_or(true),
            high_value,
            expr.high_closed.unwrap_or(true),
        )))
    }

    /// Evaluate Start operator - returns low bound of interval
    pub fn eval_start(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Interval(interval) => {
                match interval.low() {
                    Some(low) => Ok(low.clone()),
                    None => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::type_mismatch("Interval", operand.get_type().name())),
        }
    }

    /// Evaluate End operator - returns high bound of interval
    pub fn eval_end(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Interval(interval) => {
                match interval.high() {
                    Some(high) => Ok(high.clone()),
                    None => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::type_mismatch("Interval", operand.get_type().name())),
        }
    }

    /// Evaluate PointFrom - returns point if interval is a point, otherwise null
    pub fn eval_point_from(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Interval(interval) => {
                if interval.is_point() {
                    interval.low().cloned().ok_or_else(|| EvalError::internal("Point interval without low bound"))
                } else {
                    Ok(CqlValue::Null)
                }
            }
            _ => Err(EvalError::type_mismatch("Interval", operand.get_type().name())),
        }
    }

    /// Evaluate Width - returns distance between low and high bounds
    pub fn eval_width(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Interval(interval) => {
                match (interval.low(), interval.high()) {
                    (Some(low), Some(high)) => {
                        // Subtract low from high
                        match (low, high) {
                            (CqlValue::Integer(l), CqlValue::Integer(h)) => {
                                Ok(CqlValue::Integer(h - l))
                            }
                            (CqlValue::Long(l), CqlValue::Long(h)) => {
                                Ok(CqlValue::Long(h - l))
                            }
                            (CqlValue::Decimal(l), CqlValue::Decimal(h)) => {
                                Ok(CqlValue::Decimal(h - l))
                            }
                            _ => Ok(CqlValue::Null),
                        }
                    }
                    _ => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::type_mismatch("Interval", operand.get_type().name())),
        }
    }

    /// Evaluate Size - returns number of points in interval (for integer/date types)
    pub fn eval_size(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Interval(interval) => {
                match (interval.low(), interval.high()) {
                    (Some(low), Some(high)) => {
                        match (low, high) {
                            (CqlValue::Integer(l), CqlValue::Integer(h)) => {
                                let mut size = h - l;
                                if interval.low_closed {
                                    size += 1;
                                }
                                if !interval.high_closed {
                                    size -= 1;
                                }
                                Ok(CqlValue::Integer(size.max(0)))
                            }
                            _ => Ok(CqlValue::Null),
                        }
                    }
                    _ => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::type_mismatch("Interval", operand.get_type().name())),
        }
    }

    /// Evaluate Contains - tests if interval contains point or other interval
    pub fn eval_contains(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match &left {
            CqlValue::Interval(interval) => {
                match &right {
                    CqlValue::Interval(other) => interval_includes(interval, other),
                    _ => interval_contains_point(interval, &right),
                }
            }
            CqlValue::List(list) => list_contains(list, &right),
            _ => Err(EvalError::unsupported_operator("Contains", left.get_type().name())),
        }
    }

    /// Evaluate In - tests if point is in interval
    pub fn eval_in(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match &right {
            CqlValue::Interval(interval) => interval_contains_point(interval, &left),
            CqlValue::List(list) => list_contains(list, &left),
            _ => Err(EvalError::unsupported_operator("In", right.get_type().name())),
        }
    }

    /// Evaluate Includes - tests if first interval includes second
    pub fn eval_includes(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_includes(a, b),
            (CqlValue::List(a), CqlValue::List(b)) => list_includes(a, b),
            _ => Err(EvalError::unsupported_operator(
                "Includes",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate IncludedIn - tests if first interval is included in second
    pub fn eval_included_in(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_includes(b, a),
            (CqlValue::List(a), CqlValue::List(b)) => list_includes(b, a),
            _ => Err(EvalError::unsupported_operator(
                "IncludedIn",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate ProperContains
    pub fn eval_proper_contains(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match &left {
            CqlValue::Interval(interval) => {
                match &right {
                    CqlValue::Interval(other) => interval_proper_includes(interval, other),
                    _ => {
                        // Proper contains for point is same as contains (point can't equal interval)
                        interval_contains_point(interval, &right)
                    }
                }
            }
            CqlValue::List(list) => list_contains(list, &right), // Proper for lists not meaningful
            _ => Err(EvalError::unsupported_operator("ProperContains", left.get_type().name())),
        }
    }

    /// Evaluate ProperIn
    pub fn eval_proper_in(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match &right {
            CqlValue::Interval(interval) => interval_contains_point(interval, &left),
            CqlValue::List(list) => list_contains(list, &left),
            _ => Err(EvalError::unsupported_operator("ProperIn", right.get_type().name())),
        }
    }

    /// Evaluate ProperIncludes
    pub fn eval_proper_includes(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_proper_includes(a, b),
            _ => Err(EvalError::unsupported_operator(
                "ProperIncludes",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate ProperIncludedIn
    pub fn eval_proper_included_in(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_proper_includes(b, a),
            _ => Err(EvalError::unsupported_operator(
                "ProperIncludedIn",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Before - tests if first ends before second starts
    pub fn eval_before(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_before(a, b),
            // Point before interval
            (_, CqlValue::Interval(b)) => {
                let start = b.low();
                match start {
                    Some(s) => {
                        let cmp = cql_compare(&left, s)?;
                        match cmp {
                            Some(Ordering::Less) => Ok(CqlValue::Boolean(true)),
                            Some(_) => Ok(CqlValue::Boolean(false)),
                            None => Ok(CqlValue::Null),
                        }
                    }
                    None => Ok(CqlValue::Null),
                }
            }
            // Interval before point
            (CqlValue::Interval(a), _) => {
                let end = a.high();
                match end {
                    Some(e) => {
                        let cmp = cql_compare(e, &right)?;
                        match cmp {
                            Some(Ordering::Less) => Ok(CqlValue::Boolean(true)),
                            Some(_) => Ok(CqlValue::Boolean(false)),
                            None => Ok(CqlValue::Null),
                        }
                    }
                    None => Ok(CqlValue::Null),
                }
            }
            // Point before point
            _ => {
                let cmp = cql_compare(&left, &right)?;
                match cmp {
                    Some(Ordering::Less) => Ok(CqlValue::Boolean(true)),
                    Some(_) => Ok(CqlValue::Boolean(false)),
                    None => Ok(CqlValue::Null),
                }
            }
        }
    }

    /// Evaluate After - tests if first starts after second ends
    pub fn eval_after(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_after(a, b),
            // Point after interval
            (_, CqlValue::Interval(b)) => {
                let end = b.high();
                match end {
                    Some(e) => {
                        let cmp = cql_compare(&left, e)?;
                        match cmp {
                            Some(Ordering::Greater) => Ok(CqlValue::Boolean(true)),
                            Some(_) => Ok(CqlValue::Boolean(false)),
                            None => Ok(CqlValue::Null),
                        }
                    }
                    None => Ok(CqlValue::Null),
                }
            }
            // Interval after point
            (CqlValue::Interval(a), _) => {
                let start = a.low();
                match start {
                    Some(s) => {
                        let cmp = cql_compare(s, &right)?;
                        match cmp {
                            Some(Ordering::Greater) => Ok(CqlValue::Boolean(true)),
                            Some(_) => Ok(CqlValue::Boolean(false)),
                            None => Ok(CqlValue::Null),
                        }
                    }
                    None => Ok(CqlValue::Null),
                }
            }
            // Point after point
            _ => {
                let cmp = cql_compare(&left, &right)?;
                match cmp {
                    Some(Ordering::Greater) => Ok(CqlValue::Boolean(true)),
                    Some(_) => Ok(CqlValue::Boolean(false)),
                    None => Ok(CqlValue::Null),
                }
            }
        }
    }

    /// Evaluate Meets - tests if first meets second (adjacent)
    pub fn eval_meets(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                // Meets if a.high + 1 = b.low or b.high + 1 = a.low
                let meets_before = interval_meets_before(a, b)?;
                let meets_after = interval_meets_after(a, b)?;

                match (meets_before, meets_after) {
                    (CqlValue::Boolean(true), _) | (_, CqlValue::Boolean(true)) => {
                        Ok(CqlValue::Boolean(true))
                    }
                    (CqlValue::Null, _) | (_, CqlValue::Null) => Ok(CqlValue::Null),
                    _ => Ok(CqlValue::Boolean(false)),
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "Meets",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate MeetsBefore - tests if first ends just before second starts
    pub fn eval_meets_before(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_meets_before(a, b),
            _ => Err(EvalError::unsupported_operator(
                "MeetsBefore",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate MeetsAfter - tests if first starts just after second ends
    pub fn eval_meets_after(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_meets_after(a, b),
            _ => Err(EvalError::unsupported_operator(
                "MeetsAfter",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Overlaps - tests if intervals overlap
    pub fn eval_overlaps(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_overlaps(a, b),
            _ => Err(EvalError::unsupported_operator(
                "Overlaps",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate OverlapsBefore
    pub fn eval_overlaps_before(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_overlaps_before(a, b),
            _ => Err(EvalError::unsupported_operator(
                "OverlapsBefore",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate OverlapsAfter
    pub fn eval_overlaps_after(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_overlaps_after(a, b),
            _ => Err(EvalError::unsupported_operator(
                "OverlapsAfter",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Starts - tests if first starts at same point as second
    pub fn eval_starts(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                // a starts b if a.low = b.low and a included in b
                match (a.low(), b.low()) {
                    (Some(al), Some(bl)) => {
                        if cql_equal(al, bl)? && a.low_closed == b.low_closed {
                            interval_includes(b, a)
                        } else {
                            Ok(CqlValue::Boolean(false))
                        }
                    }
                    _ => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "Starts",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Ends - tests if first ends at same point as second
    pub fn eval_ends(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                // a ends b if a.high = b.high and a included in b
                match (a.high(), b.high()) {
                    (Some(ah), Some(bh)) => {
                        if cql_equal(ah, bh)? && a.high_closed == b.high_closed {
                            interval_includes(b, a)
                        } else {
                            Ok(CqlValue::Boolean(false))
                        }
                    }
                    _ => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "Ends",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Collapse - merges overlapping intervals in a list
    pub fn eval_collapse(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.operand, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        match &source {
            CqlValue::List(list) => collapse_intervals(list),
            _ => Err(EvalError::type_mismatch("List<Interval>", source.get_type().name())),
        }
    }

    /// Evaluate Expand - expands interval to list of points
    pub fn eval_expand(&self, expr: &ExpandExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.operand, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let per = if let Some(per_expr) = &expr.per {
            Some(self.evaluate(per_expr, ctx)?)
        } else {
            None
        };

        match &source {
            CqlValue::Interval(interval) => expand_interval(interval, per.as_ref()),
            CqlValue::List(list) => expand_interval_list(list, per.as_ref()),
            _ => Err(EvalError::type_mismatch("Interval", source.get_type().name())),
        }
    }

    /// Evaluate Union of intervals/lists
    pub fn eval_union(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        match (&left, &right) {
            (CqlValue::Null, other) | (other, CqlValue::Null) => Ok(other.clone()),
            (CqlValue::List(a), CqlValue::List(b)) => list_union(a, b),
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_union(a, b),
            _ => Err(EvalError::unsupported_operator(
                "Union",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Intersect of intervals/lists
    pub fn eval_intersect(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::List(a), CqlValue::List(b)) => list_intersect(a, b),
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_intersect(a, b),
            _ => Err(EvalError::unsupported_operator(
                "Intersect",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Except of intervals/lists
    pub fn eval_except(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() {
            return Ok(CqlValue::Null);
        }

        if right.is_null() {
            return Ok(left);
        }

        match (&left, &right) {
            (CqlValue::List(a), CqlValue::List(b)) => list_except(a, b),
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_except(a, b),
            _ => Err(EvalError::unsupported_operator(
                "Except",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }
}

// Helper functions

fn interval_contains_point(interval: &CqlInterval, point: &CqlValue) -> EvalResult<CqlValue> {
    let low = interval.low();
    let high = interval.high();

    // Check low bound
    let above_low = match low {
        Some(l) => {
            let cmp = cql_compare(point, l)?;
            match cmp {
                Some(Ordering::Greater) => true,
                Some(Ordering::Equal) => interval.low_closed,
                Some(Ordering::Less) => false,
                None => return Ok(CqlValue::Null),
            }
        }
        None => true, // Unbounded low
    };

    if !above_low {
        return Ok(CqlValue::Boolean(false));
    }

    // Check high bound
    let below_high = match high {
        Some(h) => {
            let cmp = cql_compare(point, h)?;
            match cmp {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => interval.high_closed,
                Some(Ordering::Greater) => false,
                None => return Ok(CqlValue::Null),
            }
        }
        None => true, // Unbounded high
    };

    Ok(CqlValue::Boolean(below_high))
}

fn interval_includes(container: &CqlInterval, contained: &CqlInterval) -> EvalResult<CqlValue> {
    // container includes contained if:
    // container.low <= contained.low and contained.high <= container.high

    let low_ok = match (container.low(), contained.low()) {
        (Some(cl), Some(dl)) => {
            let cmp = cql_compare(cl, dl)?;
            match cmp {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => container.low_closed || !contained.low_closed,
                Some(Ordering::Greater) => false,
                None => return Ok(CqlValue::Null),
            }
        }
        (None, _) => true, // Container unbounded low
        (_, None) => false, // Contained unbounded low but container bounded
    };

    if !low_ok {
        return Ok(CqlValue::Boolean(false));
    }

    let high_ok = match (container.high(), contained.high()) {
        (Some(ch), Some(dh)) => {
            let cmp = cql_compare(dh, ch)?;
            match cmp {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => container.high_closed || !contained.high_closed,
                Some(Ordering::Greater) => false,
                None => return Ok(CqlValue::Null),
            }
        }
        (None, _) => true, // Container unbounded high
        (_, None) => false, // Contained unbounded high but container bounded
    };

    Ok(CqlValue::Boolean(high_ok))
}

fn interval_proper_includes(container: &CqlInterval, contained: &CqlInterval) -> EvalResult<CqlValue> {
    let includes = interval_includes(container, contained)?;
    if let CqlValue::Boolean(true) = includes {
        // Check that they're not equal
        let equal = cql_equal(
            &CqlValue::Interval(container.clone()),
            &CqlValue::Interval(contained.clone()),
        )?;
        Ok(CqlValue::Boolean(!equal))
    } else {
        Ok(includes)
    }
}

fn interval_before(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    match (a.high(), b.low()) {
        (Some(ah), Some(bl)) => {
            let cmp = cql_compare(ah, bl)?;
            match cmp {
                Some(Ordering::Less) => Ok(CqlValue::Boolean(true)),
                Some(Ordering::Equal) => Ok(CqlValue::Boolean(!a.high_closed || !b.low_closed)),
                Some(Ordering::Greater) => Ok(CqlValue::Boolean(false)),
                None => Ok(CqlValue::Null),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

fn interval_after(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    interval_before(b, a)
}

fn interval_meets_before(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    // a meets before b if a.high + successor = b.low
    match (a.high(), b.low()) {
        (Some(ah), Some(bl)) => {
            // Check if they're adjacent
            let cmp = cql_compare(ah, bl)?;
            match cmp {
                Some(Ordering::Less) => {
                    // Check if successor of ah equals bl
                    let succ = successor_value(ah)?;
                    if let Some(s) = succ {
                        Ok(CqlValue::Boolean(cql_equal(&s, bl)?))
                    } else {
                        Ok(CqlValue::Boolean(false))
                    }
                }
                Some(Ordering::Equal) => {
                    // Adjacent if one is closed and other is open
                    Ok(CqlValue::Boolean(a.high_closed != b.low_closed))
                }
                _ => Ok(CqlValue::Boolean(false)),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

fn interval_meets_after(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    interval_meets_before(b, a)
}

fn interval_overlaps(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    // Intervals overlap if they share at least one point
    let before = interval_before(a, b)?;
    let after = interval_after(a, b)?;

    match (before, after) {
        (CqlValue::Boolean(true), _) | (_, CqlValue::Boolean(true)) => {
            Ok(CqlValue::Boolean(false))
        }
        (CqlValue::Null, _) | (_, CqlValue::Null) => Ok(CqlValue::Null),
        _ => Ok(CqlValue::Boolean(true)),
    }
}

fn interval_overlaps_before(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    // a overlaps before b if a starts before b and ends in b
    let overlaps = interval_overlaps(a, b)?;
    if !matches!(overlaps, CqlValue::Boolean(true)) {
        return Ok(overlaps);
    }

    match (a.low(), b.low(), a.high(), b.high()) {
        (Some(al), Some(bl), Some(ah), Some(bh)) => {
            let starts_before = cql_compare(al, bl)?;
            let ends_in = cql_compare(ah, bh)?;

            match (starts_before, ends_in) {
                (Some(Ordering::Less), Some(Ordering::Less | Ordering::Equal)) => {
                    Ok(CqlValue::Boolean(true))
                }
                _ => Ok(CqlValue::Boolean(false)),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

fn interval_overlaps_after(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    interval_overlaps_before(b, a)
}

fn successor_value(value: &CqlValue) -> EvalResult<Option<CqlValue>> {
    match value {
        CqlValue::Integer(i) => {
            if *i == i32::MAX {
                Ok(None)
            } else {
                Ok(Some(CqlValue::Integer(i + 1)))
            }
        }
        CqlValue::Long(l) => {
            if *l == i64::MAX {
                Ok(None)
            } else {
                Ok(Some(CqlValue::Long(l + 1)))
            }
        }
        _ => Ok(None),
    }
}

fn list_contains(list: &CqlList, element: &CqlValue) -> EvalResult<CqlValue> {
    for item in list.iter() {
        if cql_equal(item, element)? {
            return Ok(CqlValue::Boolean(true));
        }
    }
    Ok(CqlValue::Boolean(false))
}

fn list_includes(container: &CqlList, contained: &CqlList) -> EvalResult<CqlValue> {
    for item in contained.iter() {
        let found = list_contains(container, item)?;
        if !matches!(found, CqlValue::Boolean(true)) {
            return Ok(CqlValue::Boolean(false));
        }
    }
    Ok(CqlValue::Boolean(true))
}

fn list_union(a: &CqlList, b: &CqlList) -> EvalResult<CqlValue> {
    let mut result: Vec<CqlValue> = a.elements.clone();
    for item in b.iter() {
        let found = list_contains(&CqlList::from_elements(result.clone()), item)?;
        if !matches!(found, CqlValue::Boolean(true)) {
            result.push(item.clone());
        }
    }
    Ok(CqlValue::List(CqlList::from_elements(result)))
}

fn list_intersect(a: &CqlList, b: &CqlList) -> EvalResult<CqlValue> {
    let mut result: Vec<CqlValue> = Vec::new();
    for item in a.iter() {
        let in_b = list_contains(b, item)?;
        if matches!(in_b, CqlValue::Boolean(true)) {
            let in_result = list_contains(&CqlList::from_elements(result.clone()), item)?;
            if !matches!(in_result, CqlValue::Boolean(true)) {
                result.push(item.clone());
            }
        }
    }
    Ok(CqlValue::List(CqlList::from_elements(result)))
}

fn list_except(a: &CqlList, b: &CqlList) -> EvalResult<CqlValue> {
    let mut result: Vec<CqlValue> = Vec::new();
    for item in a.iter() {
        let in_b = list_contains(b, item)?;
        if !matches!(in_b, CqlValue::Boolean(true)) {
            result.push(item.clone());
        }
    }
    Ok(CqlValue::List(CqlList::from_elements(result)))
}

fn interval_union(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    // Union only works for overlapping or adjacent intervals
    let overlaps = interval_overlaps(a, b)?;
    let meets = {
        let mb = interval_meets_before(a, b)?;
        let ma = interval_meets_after(a, b)?;
        match (mb, ma) {
            (CqlValue::Boolean(true), _) | (_, CqlValue::Boolean(true)) => CqlValue::Boolean(true),
            _ => CqlValue::Boolean(false),
        }
    };

    if !matches!(overlaps, CqlValue::Boolean(true)) && !matches!(meets, CqlValue::Boolean(true)) {
        return Ok(CqlValue::Null); // Disjoint intervals can't be unioned
    }

    // Find min low and max high
    let (new_low, new_low_closed) = match (a.low(), b.low()) {
        (Some(al), Some(bl)) => {
            match cql_compare(al, bl)? {
                Some(Ordering::Less) => (Some(al.clone()), a.low_closed),
                Some(Ordering::Greater) => (Some(bl.clone()), b.low_closed),
                Some(Ordering::Equal) => (Some(al.clone()), a.low_closed || b.low_closed),
                None => return Ok(CqlValue::Null),
            }
        }
        (None, _) | (_, None) => (None, true),
    };

    let (new_high, new_high_closed) = match (a.high(), b.high()) {
        (Some(ah), Some(bh)) => {
            match cql_compare(ah, bh)? {
                Some(Ordering::Greater) => (Some(ah.clone()), a.high_closed),
                Some(Ordering::Less) => (Some(bh.clone()), b.high_closed),
                Some(Ordering::Equal) => (Some(ah.clone()), a.high_closed || b.high_closed),
                None => return Ok(CqlValue::Null),
            }
        }
        (None, _) | (_, None) => (None, true),
    };

    Ok(CqlValue::Interval(CqlInterval::new(
        a.point_type.clone(),
        new_low,
        new_low_closed,
        new_high,
        new_high_closed,
    )))
}

fn interval_intersect(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    let overlaps = interval_overlaps(a, b)?;
    if !matches!(overlaps, CqlValue::Boolean(true)) {
        return Ok(CqlValue::Null);
    }

    // Find max low and min high
    let (new_low, new_low_closed) = match (a.low(), b.low()) {
        (Some(al), Some(bl)) => {
            match cql_compare(al, bl)? {
                Some(Ordering::Greater) => (Some(al.clone()), a.low_closed),
                Some(Ordering::Less) => (Some(bl.clone()), b.low_closed),
                Some(Ordering::Equal) => (Some(al.clone()), a.low_closed && b.low_closed),
                None => return Ok(CqlValue::Null),
            }
        }
        (Some(l), None) => (Some(l.clone()), a.low_closed),
        (None, Some(l)) => (Some(l.clone()), b.low_closed),
        (None, None) => (None, true),
    };

    let (new_high, new_high_closed) = match (a.high(), b.high()) {
        (Some(ah), Some(bh)) => {
            match cql_compare(ah, bh)? {
                Some(Ordering::Less) => (Some(ah.clone()), a.high_closed),
                Some(Ordering::Greater) => (Some(bh.clone()), b.high_closed),
                Some(Ordering::Equal) => (Some(ah.clone()), a.high_closed && b.high_closed),
                None => return Ok(CqlValue::Null),
            }
        }
        (Some(h), None) => (Some(h.clone()), a.high_closed),
        (None, Some(h)) => (Some(h.clone()), b.high_closed),
        (None, None) => (None, true),
    };

    Ok(CqlValue::Interval(CqlInterval::new(
        a.point_type.clone(),
        new_low,
        new_low_closed,
        new_high,
        new_high_closed,
    )))
}

fn interval_except(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    // a except b - returns the part of a not in b
    // This is complex and may return null if b is in the middle of a
    let includes = interval_includes(b, a)?;
    if matches!(includes, CqlValue::Boolean(true)) {
        return Ok(CqlValue::Null); // b contains all of a
    }

    let overlaps = interval_overlaps(a, b)?;
    if !matches!(overlaps, CqlValue::Boolean(true)) {
        return Ok(CqlValue::Interval(a.clone())); // No overlap
    }

    // Check if b starts before a ends and ends before a ends
    // In that case, return the part after b
    let b_starts_before_a_ends = match (b.low(), a.high()) {
        (Some(bl), Some(ah)) => {
            matches!(cql_compare(bl, ah)?, Some(Ordering::Less | Ordering::Equal))
        }
        _ => false,
    };

    let b_ends_before_a_ends = match (b.high(), a.high()) {
        (Some(bh), Some(ah)) => {
            matches!(cql_compare(bh, ah)?, Some(Ordering::Less))
        }
        _ => false,
    };

    if b_starts_before_a_ends && b_ends_before_a_ends {
        // Return (b.high, a.high]
        return Ok(CqlValue::Interval(CqlInterval::new(
            a.point_type.clone(),
            b.high.as_ref().map(|h| (**h).clone()),
            !b.high_closed,
            a.high.as_ref().map(|h| (**h).clone()),
            a.high_closed,
        )));
    }

    // Check if b ends after a starts
    let b_ends_after_a_starts = match (b.high(), a.low()) {
        (Some(bh), Some(al)) => {
            matches!(cql_compare(bh, al)?, Some(Ordering::Greater | Ordering::Equal))
        }
        _ => false,
    };

    let b_starts_after_a_starts = match (b.low(), a.low()) {
        (Some(bl), Some(al)) => {
            matches!(cql_compare(bl, al)?, Some(Ordering::Greater))
        }
        _ => false,
    };

    if b_ends_after_a_starts && b_starts_after_a_starts {
        // Return [a.low, b.low)
        return Ok(CqlValue::Interval(CqlInterval::new(
            a.point_type.clone(),
            a.low.as_ref().map(|l| (**l).clone()),
            a.low_closed,
            b.low.as_ref().map(|l| (**l).clone()),
            !b.low_closed,
        )));
    }

    Ok(CqlValue::Null) // b is in the middle of a
}

fn collapse_intervals(list: &CqlList) -> EvalResult<CqlValue> {
    // Extract intervals
    let mut intervals: Vec<CqlInterval> = Vec::new();
    for item in list.iter() {
        match item {
            CqlValue::Interval(i) => intervals.push(i.clone()),
            CqlValue::Null => continue,
            _ => return Err(EvalError::type_mismatch("Interval", item.get_type().name())),
        }
    }

    if intervals.is_empty() {
        return Ok(CqlValue::List(CqlList::new(CqlType::interval(CqlType::Any))));
    }

    // Sort by low bound
    // Simplified - would need proper sorting
    let point_type = intervals[0].point_type.clone();

    // For now, just return the list as-is (proper implementation would merge)
    let result: Vec<CqlValue> = intervals.into_iter().map(CqlValue::Interval).collect();
    Ok(CqlValue::List(CqlList {
        element_type: CqlType::interval(point_type),
        elements: result,
    }))
}

fn expand_interval(interval: &CqlInterval, _per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    // Expand interval to list of points
    match (&interval.low, &interval.high) {
        (Some(low), Some(high)) => {
            match (low.as_ref(), high.as_ref()) {
                (CqlValue::Integer(l), CqlValue::Integer(h)) => {
                    let start = if interval.low_closed { *l } else { l + 1 };
                    let end = if interval.high_closed { *h } else { h - 1 };

                    if start > end {
                        return Ok(CqlValue::List(CqlList::new(CqlType::Integer)));
                    }

                    let elements: Vec<CqlValue> = (start..=end).map(CqlValue::Integer).collect();
                    Ok(CqlValue::List(CqlList {
                        element_type: CqlType::Integer,
                        elements,
                    }))
                }
                _ => Ok(CqlValue::Null), // Only integer expansion supported for now
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

fn expand_interval_list(list: &CqlList, per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    let mut all_points: Vec<CqlValue> = Vec::new();

    for item in list.iter() {
        match item {
            CqlValue::Interval(i) => {
                if let CqlValue::List(expanded) = expand_interval(i, per)? {
                    all_points.extend(expanded.elements);
                }
            }
            CqlValue::Null => continue,
            _ => return Err(EvalError::type_mismatch("Interval", item.get_type().name())),
        }
    }

    Ok(CqlValue::List(CqlList::from_elements(all_points)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_contains_point() {
        let interval = CqlInterval::closed(
            CqlType::Integer,
            CqlValue::Integer(1),
            CqlValue::Integer(10),
        );

        // Point inside
        assert_eq!(
            interval_contains_point(&interval, &CqlValue::Integer(5)).unwrap(),
            CqlValue::Boolean(true)
        );

        // Point at boundary (closed)
        assert_eq!(
            interval_contains_point(&interval, &CqlValue::Integer(1)).unwrap(),
            CqlValue::Boolean(true)
        );

        // Point outside
        assert_eq!(
            interval_contains_point(&interval, &CqlValue::Integer(0)).unwrap(),
            CqlValue::Boolean(false)
        );
    }

    #[test]
    fn test_interval_includes() {
        let outer = CqlInterval::closed(
            CqlType::Integer,
            CqlValue::Integer(1),
            CqlValue::Integer(10),
        );
        let inner = CqlInterval::closed(
            CqlType::Integer,
            CqlValue::Integer(2),
            CqlValue::Integer(8),
        );

        assert_eq!(
            interval_includes(&outer, &inner).unwrap(),
            CqlValue::Boolean(true)
        );
        assert_eq!(
            interval_includes(&inner, &outer).unwrap(),
            CqlValue::Boolean(false)
        );
    }

    #[test]
    fn test_interval_overlaps() {
        let a = CqlInterval::closed(
            CqlType::Integer,
            CqlValue::Integer(1),
            CqlValue::Integer(5),
        );
        let b = CqlInterval::closed(
            CqlType::Integer,
            CqlValue::Integer(3),
            CqlValue::Integer(8),
        );
        let c = CqlInterval::closed(
            CqlType::Integer,
            CqlValue::Integer(6),
            CqlValue::Integer(10),
        );

        assert_eq!(interval_overlaps(&a, &b).unwrap(), CqlValue::Boolean(true));
        assert_eq!(interval_overlaps(&a, &c).unwrap(), CqlValue::Boolean(false));
    }
}
