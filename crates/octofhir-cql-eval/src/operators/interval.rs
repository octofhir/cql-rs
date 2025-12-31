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
use octofhir_cql_elm::{AfterExpression, BeforeExpression, BinaryExpression, ExpandExpression, Expression, IncludedInExpression, IntervalExpression, ProperIncludedInExpression, ProperIncludesExpression, TypeSpecifier, UnaryExpression};
use octofhir_cql_types::{CqlDate, CqlDateTime, CqlInterval, CqlList, CqlTime, CqlType, CqlValue, DateTimePrecision};
use std::cmp::Ordering;

/// Extract the result type specifier from an expression
fn get_result_type_specifier(expr: &Expression) -> Option<TypeSpecifier> {
    // Check common expression types for their type information
    match expr {
        Expression::As(e) => {
            // For As expression, check as_type_specifier first, then element's result_type_specifier
            e.as_type_specifier.clone()
                .or_else(|| e.element.result_type_specifier.clone())
                .or_else(|| {
                    // Try to parse as_type string like "{urn:hl7-org:elm-types:r1}Integer"
                    e.as_type.as_ref().map(|t| {
                        TypeSpecifier::Named(octofhir_cql_elm::NamedTypeSpecifier {
                            namespace: None,
                            name: t.clone(),
                        })
                    })
                })
        }
        Expression::Null(e) => e.element.result_type_specifier.clone(),
        _ => None,
    }
}

/// Convert a TypeSpecifier to CqlType
fn type_specifier_to_cql_type(ts: &TypeSpecifier) -> CqlType {
    match ts {
        TypeSpecifier::Named(n) => {
            let name = &n.name;
            match name.as_str() {
                "Integer" | "{urn:hl7-org:elm-types:r1}Integer" => CqlType::Integer,
                "Decimal" | "{urn:hl7-org:elm-types:r1}Decimal" => CqlType::Decimal,
                "String" | "{urn:hl7-org:elm-types:r1}String" => CqlType::String,
                "Boolean" | "{urn:hl7-org:elm-types:r1}Boolean" => CqlType::Boolean,
                "Date" | "{urn:hl7-org:elm-types:r1}Date" => CqlType::Date,
                "DateTime" | "{urn:hl7-org:elm-types:r1}DateTime" => CqlType::DateTime,
                "Time" | "{urn:hl7-org:elm-types:r1}Time" => CqlType::Time,
                "Quantity" | "{urn:hl7-org:elm-types:r1}Quantity" => CqlType::Quantity,
                _ => CqlType::Any,
            }
        }
        _ => CqlType::Any,
    }
}

/// Convert ELM DateTimePrecision to CqlType DateTimePrecision
fn convert_precision(precision: &octofhir_cql_elm::DateTimePrecision) -> DateTimePrecision {
    match precision {
        octofhir_cql_elm::DateTimePrecision::Year => DateTimePrecision::Year,
        octofhir_cql_elm::DateTimePrecision::Month => DateTimePrecision::Month,
        octofhir_cql_elm::DateTimePrecision::Week => DateTimePrecision::Day, // Week maps to Day for truncation
        octofhir_cql_elm::DateTimePrecision::Day => DateTimePrecision::Day,
        octofhir_cql_elm::DateTimePrecision::Hour => DateTimePrecision::Hour,
        octofhir_cql_elm::DateTimePrecision::Minute => DateTimePrecision::Minute,
        octofhir_cql_elm::DateTimePrecision::Second => DateTimePrecision::Second,
        octofhir_cql_elm::DateTimePrecision::Millisecond => DateTimePrecision::Millisecond,
    }
}

/// Truncate a CqlValue to the specified precision (for DateTime/Time types)
fn truncate_value(value: &CqlValue, precision: DateTimePrecision) -> CqlValue {
    match value {
        CqlValue::DateTime(dt) => CqlValue::DateTime(dt.truncate_to_precision(precision)),
        CqlValue::Time(t) => CqlValue::Time(t.truncate_to_precision(precision)),
        // For non-temporal types, return as-is
        other => other.clone(),
    }
}

/// Truncate an interval's bounds to the specified precision
fn truncate_interval(interval: &CqlInterval, precision: DateTimePrecision) -> CqlInterval {
    CqlInterval {
        low: interval.low.as_ref().map(|v| Box::new(truncate_value(v, precision))),
        high: interval.high.as_ref().map(|v| Box::new(truncate_value(v, precision))),
        low_closed: interval.low_closed,
        high_closed: interval.high_closed,
        point_type: interval.point_type.clone(),
    }
}

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

        // Filter out null values from bounds - null bounds represent unknown boundaries
        let low_value = low.as_ref().filter(|v| !v.is_null());
        let high_value = high.as_ref().filter(|v| !v.is_null());

        // Determine point type from non-null values or from expression type specifiers
        let point_type = match (&low, &high) {
            (Some(l), _) if !l.is_null() => l.get_type(),
            (_, Some(h)) if !h.is_null() => h.get_type(),
            _ => {
                // Try to get type from low expression's result type
                if let Some(low_expr) = &expr.low {
                    if let Some(ts) = get_result_type_specifier(low_expr) {
                        type_specifier_to_cql_type(&ts)
                    } else {
                        CqlType::Any
                    }
                } else if let Some(high_expr) = &expr.high {
                    if let Some(ts) = get_result_type_specifier(high_expr) {
                        type_specifier_to_cql_type(&ts)
                    } else {
                        CqlType::Any
                    }
                } else {
                    CqlType::Any
                }
            }
        };

        // If both bounds are null (evaluated to CqlValue::Null), the interval handling depends on type:
        // - If we have type info (point_type is NOT Any), the interval is a valid unbounded interval
        // - If we have no type info (point_type IS Any), the interval is null per CQL spec
        if low_value.is_none() && high_value.is_none() {
            let low_was_null = low.as_ref().map(|v| v.is_null()).unwrap_or(false);
            let high_was_null = high.as_ref().map(|v| v.is_null()).unwrap_or(false);
            if low_was_null && high_was_null && matches!(point_type, CqlType::Any) {
                return Ok(CqlValue::Null);
            }
        }

        let low_closed = expr.low_closed.unwrap_or(true);
        let high_closed = expr.high_closed.unwrap_or(true);

        // Validate interval bounds - error if low > high (invalid interval)
        if let (Some(low_val), Some(high_val)) = (low_value, high_value) {
            if let Some(ordering) = cql_compare(low_val, high_val)? {
                match ordering {
                    Ordering::Greater => {
                        // low > high is always invalid
                        return Err(EvalError::InvalidInterval);
                    }
                    Ordering::Equal => {
                        // low == high: valid only if both ends are closed [a, a]
                        // For [a, a), (a, a], or (a, a) the interval is empty/invalid
                        if !low_closed || !high_closed {
                            return Err(EvalError::InvalidInterval);
                        }
                    }
                    Ordering::Less => {
                        // low < high is always valid
                    }
                }
            }
        }

        Ok(CqlValue::Interval(CqlInterval::new(
            point_type,
            low_value.cloned(),
            low_closed,
            high_value.cloned(),
            high_closed,
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
                // Width is only defined for ordinal types (Integer, Decimal, Quantity)
                // For DateTime/Time, width is undefined and should return an error
                match &interval.point_type {
                    CqlType::DateTime | CqlType::Time | CqlType::Date => {
                        return Err(EvalError::unsupported_operator(
                            "Width",
                            interval.point_type.name(),
                        ));
                    }
                    _ => {}
                }

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
                            (CqlValue::Quantity(l), CqlValue::Quantity(h)) => {
                                // Quantities must have compatible units for subtraction
                                if l.unit == h.unit {
                                    let width = h.value - l.value;
                                    Ok(CqlValue::Quantity(octofhir_cql_types::CqlQuantity {
                                        value: width,
                                        unit: l.unit.clone(),
                                    }))
                                } else {
                                    Err(EvalError::IncompatibleUnits {
                                        unit1: l.unit.clone().unwrap_or_default(),
                                        unit2: h.unit.clone().unwrap_or_default(),
                                    })
                                }
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

        // If the container is null, return false (can't contain anything)
        if left.is_null() {
            return Ok(CqlValue::Boolean(false));
        }

        // For lists, we can check if a null element is contained
        // For intervals, null element operand returns null
        match &left {
            CqlValue::List(list) => list_contains(list, &right),
            CqlValue::Interval(interval) => {
                if right.is_null() {
                    return Ok(CqlValue::Null);
                }
                match &right {
                    CqlValue::Interval(other) => interval_includes(interval, other),
                    _ => interval_contains_point(interval, &right),
                }
            }
            _ => Err(EvalError::unsupported_operator("Contains", left.get_type().name())),
        }
    }

    /// Evaluate In - tests if point is in interval or list
    pub fn eval_in(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        // If the container is null, return false (can't be in a null container)
        if right.is_null() {
            return Ok(CqlValue::Boolean(false));
        }

        // For lists, we can check if a null element is in the list
        // For intervals, null element operand returns null
        match &right {
            CqlValue::List(list) => list_contains(list, &left),
            CqlValue::Interval(interval) => {
                if left.is_null() {
                    return Ok(CqlValue::Null);
                }
                interval_contains_point(interval, &left)
            }
            _ => Err(EvalError::unsupported_operator("In", right.get_type().name())),
        }
    }

    /// Evaluate Includes - tests if first interval/list includes second
    pub fn eval_includes(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        // Standard CQL null propagation - if either operand is null, result is null
        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => interval_includes(a, b),
            (CqlValue::List(a), CqlValue::List(b)) => list_includes(a, b),
            // List includes element - check if element is in the list
            (CqlValue::List(list), element) if !matches!(element, CqlValue::List(_)) => {
                list_contains(list, element)
            }
            _ => Err(EvalError::unsupported_operator(
                "Includes",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate IncludedIn - tests if first interval/element is included in second
    pub fn eval_included_in(&self, expr: &IncludedInExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::invalid_operand("IncludedIn", "requires exactly 2 operands"));
        }
        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        // Standard CQL null propagation - if either operand is null, result is null
        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        // Apply precision if specified
        let precision = expr.precision.as_ref().map(convert_precision);

        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                if let Some(prec) = precision {
                    let a_truncated = truncate_interval(a, prec);
                    let b_truncated = truncate_interval(b, prec);
                    interval_includes(&b_truncated, &a_truncated)
                } else {
                    interval_includes(b, a)
                }
            }
            (CqlValue::List(a), CqlValue::List(b)) => list_includes(b, a),
            // Element included in list - check if element is in the list
            (element, CqlValue::List(list)) if !matches!(element, CqlValue::List(_)) => {
                list_contains(list, element)
            }
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
    pub fn eval_proper_includes(&self, expr: &ProperIncludesExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::invalid_operand("ProperIncludes", "requires exactly 2 operands"));
        }
        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        // Apply precision if specified
        let precision = expr.precision.as_ref().map(convert_precision);

        // For intervals, both must be non-null
        // For lists, container must be non-null but element can be null (checking if list contains null)
        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                if left.is_null() || right.is_null() {
                    return Ok(CqlValue::Null);
                }
                if let Some(prec) = precision {
                    let a_truncated = truncate_interval(a, prec);
                    let b_truncated = truncate_interval(b, prec);
                    interval_proper_includes(&a_truncated, &b_truncated)
                } else {
                    interval_proper_includes(a, b)
                }
            }
            (CqlValue::List(a), CqlValue::List(b)) => {
                if left.is_null() || right.is_null() {
                    return Ok(CqlValue::Null);
                }
                list_proper_includes(a, b)
            }
            // Interval proper contains point: point is strictly inside the interval
            (CqlValue::Interval(interval), point) if !matches!(point, CqlValue::Interval(_) | CqlValue::List(_)) => {
                if left.is_null() || right.is_null() {
                    return Ok(CqlValue::Null);
                }
                if let Some(prec) = precision {
                    let interval_truncated = truncate_interval(interval, prec);
                    let point_truncated = truncate_value(point, prec);
                    interval_proper_contains_point(&interval_truncated, &point_truncated)
                } else {
                    interval_proper_contains_point(interval, point)
                }
            }
            // List proper contains element - element can be null (checking if list contains null)
            (CqlValue::List(list), element) if !matches!(element, CqlValue::List(_)) => {
                // Only return null if list is null, not if element is null
                list_proper_contains(list, element)
            }
            // If left is null and we don't know what type it should be
            (CqlValue::Null, _) => Ok(CqlValue::Null),
            _ => Err(EvalError::unsupported_operator(
                "ProperIncludes",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate ProperIncludedIn
    pub fn eval_proper_included_in(&self, expr: &ProperIncludedInExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::invalid_operand("ProperIncludedIn", "requires exactly 2 operands"));
        }
        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        // Apply precision if specified
        let precision = expr.precision.as_ref().map(convert_precision);

        // For intervals, both must be non-null
        // For lists, container must be non-null but element can be null (checking if null is in list)
        match (&left, &right) {
            (CqlValue::Interval(a), CqlValue::Interval(b)) => {
                if left.is_null() || right.is_null() {
                    return Ok(CqlValue::Null);
                }
                if let Some(prec) = precision {
                    let a_truncated = truncate_interval(a, prec);
                    let b_truncated = truncate_interval(b, prec);
                    interval_proper_includes(&b_truncated, &a_truncated)
                } else {
                    interval_proper_includes(b, a)
                }
            }
            (CqlValue::List(a), CqlValue::List(b)) => {
                if left.is_null() || right.is_null() {
                    return Ok(CqlValue::Null);
                }
                list_proper_includes(b, a)
            }
            // Point proper in interval: point is strictly inside the interval
            (point, CqlValue::Interval(interval)) if !matches!(point, CqlValue::Interval(_) | CqlValue::List(_)) => {
                if left.is_null() || right.is_null() {
                    return Ok(CqlValue::Null);
                }
                if let Some(prec) = precision {
                    let interval_truncated = truncate_interval(interval, prec);
                    let point_truncated = truncate_value(point, prec);
                    interval_proper_contains_point(&interval_truncated, &point_truncated)
                } else {
                    interval_proper_contains_point(interval, point)
                }
            }
            // Element proper in list - element can be null (checking if null is in list)
            (element, CqlValue::List(list)) if !matches!(element, CqlValue::List(_)) => {
                // Only return null if list is null, not if element is null
                list_proper_contains(list, element)
            }
            // Special case: Interval properly included in null (unbounded interval)
            // Per CQL spec: any finite interval is properly included in an unbounded interval
            (CqlValue::Interval(a), CqlValue::Null) => {
                // Only if the interval has finite bounds (not null bounds)
                if a.low.is_some() || a.high.is_some() {
                    Ok(CqlValue::Boolean(true))
                } else {
                    Ok(CqlValue::Null)
                }
            }
            // If right is null and we don't know what type it should be
            (_, CqlValue::Null) => Ok(CqlValue::Null),
            _ => Err(EvalError::unsupported_operator(
                "ProperIncludedIn",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate Before - tests if first ends before second starts
    pub fn eval_before(&self, expr: &BeforeExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::invalid_operand("Before", "requires exactly 2 operands"));
        }
        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        // If precision is specified, use precision-aware comparison for temporal types
        if let Some(precision) = &expr.precision {
            return self.temporal_before_with_precision(&left, &right, precision);
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
    pub fn eval_after(&self, expr: &AfterExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::invalid_operand("After", "requires exactly 2 operands"));
        }
        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        // If precision is specified, use precision-aware comparison for temporal types
        if let Some(precision) = &expr.precision {
            return self.temporal_after_with_precision(&left, &right, precision);
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
                        if cql_equal(al, bl)?.unwrap_or(false) && a.low_closed == b.low_closed {
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
                        if cql_equal(ah, bh)?.unwrap_or(false) && a.high_closed == b.high_closed {
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
            // For lists: null is treated as empty list
            (CqlValue::Null, CqlValue::List(b)) => Ok(CqlValue::List(b.clone())),
            (CqlValue::List(a), CqlValue::Null) => Ok(CqlValue::List(a.clone())),
            // For intervals: null operand means null result
            (CqlValue::Null, CqlValue::Interval(_)) | (CqlValue::Interval(_), CqlValue::Null) => Ok(CqlValue::Null),
            // Both null
            (CqlValue::Null, CqlValue::Null) => Ok(CqlValue::Null),
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
        )?.unwrap_or(false);
        Ok(CqlValue::Boolean(!equal))
    } else {
        Ok(includes)
    }
}

/// Check if interval properly contains a point (point is strictly inside)
fn interval_proper_contains_point(interval: &CqlInterval, point: &CqlValue) -> EvalResult<CqlValue> {
    // Point must be strictly inside: low < point < high
    match (interval.low(), interval.high()) {
        (Some(low), Some(high)) => {
            // Check low < point
            let cmp_low = cql_compare(low, point)?;
            let after_low = match cmp_low {
                Some(Ordering::Less) => Some(true),
                Some(Ordering::Equal) => Some(!interval.low_closed), // If low is open, equal is inside
                Some(Ordering::Greater) => Some(false),
                None => None, // Uncertain comparison
            };

            match after_low {
                Some(false) => return Ok(CqlValue::Boolean(false)),
                None => return Ok(CqlValue::Null), // Uncertain
                Some(true) => {}
            }

            // Check point < high
            let cmp_high = cql_compare(point, high)?;
            let before_high = match cmp_high {
                Some(Ordering::Less) => Some(true),
                Some(Ordering::Equal) => Some(!interval.high_closed), // If high is open, equal is inside
                Some(Ordering::Greater) => Some(false),
                None => None, // Uncertain comparison
            };

            match before_high {
                Some(b) => Ok(CqlValue::Boolean(b)),
                None => Ok(CqlValue::Null),
            }
        }
        (Some(low), None) => {
            // Unbounded high - just check point > low
            let cmp = cql_compare(low, point)?;
            match cmp {
                Some(Ordering::Less) => Ok(CqlValue::Boolean(true)),
                Some(Ordering::Equal) => Ok(CqlValue::Boolean(!interval.low_closed)),
                Some(Ordering::Greater) => Ok(CqlValue::Boolean(false)),
                None => Ok(CqlValue::Null),
            }
        }
        (None, Some(high)) => {
            // Unbounded low - just check point < high
            let cmp = cql_compare(point, high)?;
            match cmp {
                Some(Ordering::Less) => Ok(CqlValue::Boolean(true)),
                Some(Ordering::Equal) => Ok(CqlValue::Boolean(!interval.high_closed)),
                Some(Ordering::Greater) => Ok(CqlValue::Boolean(false)),
                None => Ok(CqlValue::Null),
            }
        }
        (None, None) => {
            // Unbounded on both - any point is properly contained
            Ok(CqlValue::Boolean(true))
        }
    }
}

/// Check if list properly contains an element (element is in list and list has other elements)
fn list_proper_contains(list: &CqlList, element: &CqlValue) -> EvalResult<CqlValue> {
    // Element must be in list AND list must have more than just this element
    let contains = list_contains(list, element)?;
    match &contains {
        CqlValue::Boolean(true) => {
            // Check if list has more than just this element
            let count = list.iter().filter(|item| {
                if element.is_null() && item.is_null() {
                    true
                } else {
                    cql_equal(item, element).unwrap_or(None).unwrap_or(false)
                }
            }).count();

            // Properly contains if list has more elements than just copies of this element
            Ok(CqlValue::Boolean(list.len() > count))
        }
        CqlValue::Null => Ok(CqlValue::Null), // Uncertain contains => uncertain proper contains
        _ => Ok(contains), // Boolean(false) or other
    }
}

fn interval_before(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    match (a.high(), b.low()) {
        (Some(ah), Some(bl)) => {
            let cmp = cql_compare(ah, bl)?;
            match cmp {
                Some(Ordering::Less) => {
                    // a.high < b.low
                    // For discrete types (Integer): if a.high is open, effective is ah-1
                    // if b.low is open, effective is bl+1
                    // If both are open and ah == bl, then effective_high < effective_low
                    Ok(CqlValue::Boolean(true))
                }
                Some(Ordering::Equal) => {
                    // a.high == b.low at the boundary
                    // a is before b if at least one boundary is open
                    Ok(CqlValue::Boolean(!a.high_closed || !b.low_closed))
                }
                Some(Ordering::Greater) => {
                    // a.high > b.low - but we need to check for discrete type adjacent
                    // For discrete types: if a.high is open and b.low is open,
                    // and successor(b.low) == a.high, then a is before b
                    // Example: [4, 10) vs (9, 20] - a.high=10, b.low=9
                    // effective_a_high = 9, effective_b_low = 10, so a is before b
                    if !a.high_closed && !b.low_closed {
                        // Check if they're adjacent (successor of b.low equals a.high)
                        if let Some(succ_bl) = successor_value(bl)? {
                            if cql_equal(ah, &succ_bl)?.unwrap_or(false) {
                                return Ok(CqlValue::Boolean(true)); // a is before b
                            }
                        }
                    }
                    Ok(CqlValue::Boolean(false))
                }
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
    // First check: if we can determine the intervals are clearly separated, return false
    // If a ends before b starts (a.high < b.low), they can't meet
    // If a starts after b ends (a.low > b.high), they also can't meet (but this is checked via b meets a)

    // Check if we can determine they can't possibly meet using any available bounds
    if let (Some(ah), Some(bl)) = (a.high(), b.low()) {
        // If a.high is clearly before b.low (not adjacent), they can't meet
        let cmp = cql_compare(ah, bl)?;
        if matches!(cmp, Some(Ordering::Greater) | Some(Ordering::Equal)) {
            // a.high >= b.low means a doesn't end before b starts
            // Check for exact adjacency
            match cmp {
                Some(Ordering::Equal) => {
                    // Adjacent if one is closed and other is open
                    return Ok(CqlValue::Boolean(a.high_closed != b.low_closed));
                }
                Some(Ordering::Greater) => {
                    // a.high > b.low means they overlap or a is after, not meeting
                    return Ok(CqlValue::Boolean(false));
                }
                _ => {}
            }
        }
        // a.high < b.low - check if they're exactly adjacent
        let succ = successor_value(ah)?;
        if let Some(s) = succ {
            return Ok(CqlValue::Boolean(cql_equal(&s, bl)?.unwrap_or(false)));
        } else {
            return Ok(CqlValue::Boolean(false));
        }
    }

    // Some bounds are null - check if we can still determine false
    // If a's known high < b's known low, they can't meet
    if let (Some(ah), Some(bl)) = (a.high(), b.low()) {
        // Already handled above
    } else if let (Some(ah), None) = (a.high(), b.low()) {
        // b.low is unknown, check if b.high < a.high (b ends before a ends)
        if let Some(bh) = b.high() {
            let cmp = cql_compare(bh, ah)?;
            if matches!(cmp, Some(Ordering::Less)) {
                return Ok(CqlValue::Boolean(false));
            }
        }
    } else if let (None, Some(bl)) = (a.high(), b.low()) {
        // a.high is unknown, check if a.low > b.low (a starts after b starts)
        if let Some(al) = a.low() {
            let cmp = cql_compare(al, bl)?;
            if matches!(cmp, Some(Ordering::Greater)) {
                return Ok(CqlValue::Boolean(false));
            }
        }
    }

    // Also check: if a.high < b.low using both directions
    // For meets_after(a, b) = meets_before(b, a): if a ends before b starts, a can't meet after b
    // So check if a.high and b.low are available
    if let (Some(al), Some(bh)) = (a.low(), b.high()) {
        // If a.low > b.high, then a is entirely after b, so a can't meet before b
        let cmp = cql_compare(al, bh)?;
        if matches!(cmp, Some(Ordering::Greater)) {
            return Ok(CqlValue::Boolean(false));
        }
    }

    Ok(CqlValue::Null)
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
    // a overlaps before b if:
    // 1. a and b overlap (share at least one point)
    // 2. a starts before b (considering open/closed boundaries)
    // 3. a ends within b (not after b ends)

    let overlaps = interval_overlaps(a, b)?;
    if !matches!(overlaps, CqlValue::Boolean(true)) {
        return Ok(overlaps);
    }

    match (a.low(), b.low(), a.high(), b.high()) {
        (Some(al), Some(bl), Some(ah), Some(bh)) => {
            // Check if a starts before b, considering open/closed boundaries
            // For discrete types:
            // - [4, _) starts at 4
            // - (4, _) starts at successor(4) = 5
            let starts_before = compare_interval_starts(al, a.low_closed, bl, b.low_closed)?;

            // Check if a ends within or at b's end
            let ends_in = cql_compare(ah, bh)?;
            let ends_within = match ends_in {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => true, // ends at same point
                _ => false,
            };

            Ok(CqlValue::Boolean(starts_before && ends_within))
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Compare interval start points, accounting for open/closed boundaries
/// Returns true if a_start is strictly before b_start
fn compare_interval_starts(a_low: &CqlValue, a_low_closed: bool, b_low: &CqlValue, b_low_closed: bool) -> EvalResult<bool> {
    // For discrete types (Integer), compute effective starts
    // Open boundary (3, ...] has effective start at successor(3) = 4
    // Closed boundary [4, ...] has effective start at 4
    let effective_a = if !a_low_closed && is_discrete_type(a_low) {
        successor(a_low)
    } else {
        a_low.clone()
    };

    let effective_b = if !b_low_closed && is_discrete_type(b_low) {
        successor(b_low)
    } else {
        b_low.clone()
    };

    let cmp = cql_compare(&effective_a, &effective_b)?;
    match cmp {
        Some(Ordering::Less) => Ok(true),
        Some(Ordering::Equal) => {
            // Effective values are equal - consider boundaries for non-discrete types
            if is_discrete_type(a_low) {
                // For discrete types, effective values are already adjusted
                Ok(false)
            } else {
                // For continuous types, open boundary means "just after" the value
                Ok(a_low_closed && !b_low_closed)
            }
        }
        Some(Ordering::Greater) => Ok(false),
        None => Ok(false),
    }
}

/// Check if value is a discrete type (Integer)
fn is_discrete_type(value: &CqlValue) -> bool {
    matches!(value, CqlValue::Integer(_))
}

fn interval_overlaps_after(a: &CqlInterval, b: &CqlInterval) -> EvalResult<CqlValue> {
    // a overlaps after b if:
    // 1. a and b overlap (share at least one point)
    // 2. a starts within b (not before b starts)
    // 3. a ends after b (a.high > b.high, considering boundaries)

    let overlaps = interval_overlaps(a, b)?;
    if !matches!(overlaps, CqlValue::Boolean(true)) {
        return Ok(overlaps);
    }

    match (a.low(), b.low(), a.high(), b.high()) {
        (Some(al), Some(bl), Some(ah), Some(bh)) => {
            // Check if a starts in b (not before b starts)
            // This means a.start >= b.start (using effective starts)
            let starts_in_b = !compare_interval_starts(al, a.low_closed, bl, b.low_closed)?;
            if !starts_in_b {
                return Ok(CqlValue::Boolean(false));
            }

            // Also check that a starts before b ends (within b)
            // a.low must be < b.high (considering boundaries)
            let a_starts_in_range = compare_starts_before_ends(al, a.low_closed, bh, b.high_closed)?;
            if !a_starts_in_range {
                return Ok(CqlValue::Boolean(false));
            }

            // Check if a ends after b (a.high > b.high, considering boundaries)
            let ends_after = compare_interval_ends(ah, a.high_closed, bh, b.high_closed)?;

            Ok(CqlValue::Boolean(ends_after))
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Compare interval end points, accounting for open/closed boundaries
/// Returns true if a_end is strictly after b_end
fn compare_interval_ends(a_high: &CqlValue, a_high_closed: bool, b_high: &CqlValue, b_high_closed: bool) -> EvalResult<bool> {
    // For discrete types (Integer), compute effective ends
    // Open boundary [..., 11) has effective end at predecessor(11) = 10
    // Closed boundary [..., 10] has effective end at 10
    let effective_a = if !a_high_closed && is_discrete_type(a_high) {
        predecessor(a_high)
    } else {
        a_high.clone()
    };

    let effective_b = if !b_high_closed && is_discrete_type(b_high) {
        predecessor(b_high)
    } else {
        b_high.clone()
    };

    let cmp = cql_compare(&effective_a, &effective_b)?;
    match cmp {
        Some(Ordering::Greater) => Ok(true),
        Some(Ordering::Equal) => {
            // Effective values are equal - consider boundaries for non-discrete types
            if is_discrete_type(a_high) {
                // For discrete types, effective values are already adjusted
                Ok(false)
            } else {
                // For continuous types, open boundary means "just before" the value
                Ok(a_high_closed && !b_high_closed)
            }
        }
        Some(Ordering::Less) => Ok(false),
        None => Ok(false),
    }
}

/// Check if a start point is before an end point (for checking if a starts within b)
fn compare_starts_before_ends(a_low: &CqlValue, a_low_closed: bool, b_high: &CqlValue, b_high_closed: bool) -> EvalResult<bool> {
    let cmp = cql_compare(a_low, b_high)?;
    match cmp {
        Some(Ordering::Less) => Ok(true),
        Some(Ordering::Equal) => {
            // a starts at b's end - only valid if both are closed
            Ok(a_low_closed && b_high_closed)
        }
        Some(Ordering::Greater) => Ok(false),
        None => Ok(false),
    }
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
        CqlValue::Decimal(d) => {
            // CQL Decimal has 8 decimal places precision, so step is 0.00000001
            use rust_decimal::Decimal;
            let step = Decimal::new(1, 8); // 0.00000001
            Ok(Some(CqlValue::Decimal(*d + step)))
        }
        CqlValue::Quantity(q) => {
            // Same as Decimal, step is 0.00000001
            use rust_decimal::Decimal;
            let step = Decimal::new(1, 8);
            let mut new_q = q.clone();
            new_q.value = q.value + step;
            Ok(Some(CqlValue::Quantity(new_q)))
        }
        CqlValue::Date(d) => {
            // Successor of a date is the next day at the same precision
            let mut new_d = d.clone();
            match (d.month, d.day) {
                (Some(m), Some(day)) => {
                    // Full date - add 1 day
                    let dim = interval_days_in_month(d.year, m);
                    if day < dim {
                        new_d.day = Some(day + 1);
                    } else if m < 12 {
                        new_d.month = Some(m + 1);
                        new_d.day = Some(1);
                    } else {
                        new_d.year = d.year + 1;
                        new_d.month = Some(1);
                        new_d.day = Some(1);
                    }
                }
                (Some(m), None) => {
                    // Month precision - successor is next month
                    if m < 12 {
                        new_d.month = Some(m + 1);
                    } else {
                        new_d.year = d.year + 1;
                        new_d.month = Some(1);
                    }
                }
                _ => {
                    // Year precision only
                    new_d.year = d.year + 1;
                }
            }
            Ok(Some(CqlValue::Date(new_d)))
        }
        CqlValue::DateTime(dt) => {
            // Successor depends on precision - find the finest precision and add 1
            let mut new_dt = dt.clone();
            if let Some(ms) = dt.millisecond {
                if ms < 999 {
                    new_dt.millisecond = Some(ms + 1);
                } else if let Some(s) = dt.second {
                    if s < 59 {
                        new_dt.second = Some(s + 1);
                        new_dt.millisecond = Some(0);
                    } else if let Some(min) = dt.minute {
                        if min < 59 {
                            new_dt.minute = Some(min + 1);
                            new_dt.second = Some(0);
                            new_dt.millisecond = Some(0);
                        } else if let Some(h) = dt.hour {
                            if h < 23 {
                                new_dt.hour = Some(h + 1);
                                new_dt.minute = Some(0);
                                new_dt.second = Some(0);
                                new_dt.millisecond = Some(0);
                            } else {
                                // Roll over to next day
                                if let Some(d) = dt.day {
                                    let month = dt.month.unwrap_or(1);
                                    let dim = interval_days_in_month(dt.year, month);
                                    if d < dim {
                                        new_dt.day = Some(d + 1);
                                    } else if month < 12 {
                                        new_dt.month = Some(month + 1);
                                        new_dt.day = Some(1);
                                    } else {
                                        new_dt.year = dt.year + 1;
                                        new_dt.month = Some(1);
                                        new_dt.day = Some(1);
                                    }
                                }
                                new_dt.hour = Some(0);
                                new_dt.minute = Some(0);
                                new_dt.second = Some(0);
                                new_dt.millisecond = Some(0);
                            }
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }
            } else if let Some(s) = dt.second {
                // Second precision
                if s < 59 {
                    new_dt.second = Some(s + 1);
                } else {
                    new_dt.minute = dt.minute.map(|m| m + 1);
                    new_dt.second = Some(0);
                }
            } else if let Some(min) = dt.minute {
                // Minute precision
                if min < 59 {
                    new_dt.minute = Some(min + 1);
                } else {
                    new_dt.hour = dt.hour.map(|h| h + 1);
                    new_dt.minute = Some(0);
                }
            } else if let Some(h) = dt.hour {
                // Hour precision
                if h < 23 {
                    new_dt.hour = Some(h + 1);
                } else {
                    if let Some(d) = dt.day {
                        let month = dt.month.unwrap_or(1);
                        let dim = interval_days_in_month(dt.year, month);
                        if d < dim {
                            new_dt.day = Some(d + 1);
                        } else if month < 12 {
                            new_dt.month = Some(month + 1);
                            new_dt.day = Some(1);
                        } else {
                            new_dt.year = dt.year + 1;
                            new_dt.month = Some(1);
                            new_dt.day = Some(1);
                        }
                    }
                    new_dt.hour = Some(0);
                }
            } else if let Some(d) = dt.day {
                // Day precision - successor is next day
                let month = dt.month.unwrap_or(1);
                let dim = interval_days_in_month(dt.year, month);
                if d < dim {
                    new_dt.day = Some(d + 1);
                } else if month < 12 {
                    new_dt.month = Some(month + 1);
                    new_dt.day = Some(1);
                } else {
                    new_dt.year = dt.year + 1;
                    new_dt.month = Some(1);
                    new_dt.day = Some(1);
                }
            } else if let Some(m) = dt.month {
                // Month precision
                if m < 12 {
                    new_dt.month = Some(m + 1);
                } else {
                    new_dt.year = dt.year + 1;
                    new_dt.month = Some(1);
                }
            } else {
                // Year precision
                new_dt.year = dt.year + 1;
            }
            Ok(Some(CqlValue::DateTime(new_dt)))
        }
        CqlValue::Time(t) => {
            // Successor depends on precision
            let mut new_t = t.clone();
            if let Some(ms) = t.millisecond {
                if ms < 999 {
                    new_t.millisecond = Some(ms + 1);
                } else if let Some(s) = t.second {
                    if s < 59 {
                        new_t.second = Some(s + 1);
                        new_t.millisecond = Some(0);
                    } else if let Some(min) = t.minute {
                        if min < 59 {
                            new_t.minute = Some(min + 1);
                            new_t.second = Some(0);
                            new_t.millisecond = Some(0);
                        } else if t.hour < 23 {
                            new_t.hour = t.hour + 1;
                            new_t.minute = Some(0);
                            new_t.second = Some(0);
                            new_t.millisecond = Some(0);
                        } else {
                            // Time wraps at 24:00
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }
            } else if let Some(s) = t.second {
                if s < 59 {
                    new_t.second = Some(s + 1);
                } else {
                    new_t.minute = t.minute.map(|m| m + 1);
                    new_t.second = Some(0);
                }
            } else if let Some(min) = t.minute {
                if min < 59 {
                    new_t.minute = Some(min + 1);
                } else {
                    new_t.hour = t.hour + 1;
                    new_t.minute = Some(0);
                }
            } else {
                if t.hour < 23 {
                    new_t.hour = t.hour + 1;
                } else {
                    return Ok(None);
                }
            }
            Ok(Some(CqlValue::Time(new_t)))
        }
        _ => Ok(None),
    }
}

/// Days in a given month (local helper to avoid name collision)
fn interval_days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn list_contains(list: &CqlList, element: &CqlValue) -> EvalResult<CqlValue> {
    // If element is null, check if list contains null
    if element.is_null() {
        for item in list.iter() {
            if item.is_null() {
                return Ok(CqlValue::Boolean(true));
            }
        }
        // List doesn't contain null, so definitely doesn't contain the null element
        return Ok(CqlValue::Boolean(false));
    }

    // Element is not null - check against each list item
    let mut has_uncertain = false;
    for item in list.iter() {
        // Null items in list definitely don't match a non-null search element
        if item.is_null() {
            continue; // Skip null items when searching for non-null
        }
        match cql_equal(item, element)? {
            Some(true) => return Ok(CqlValue::Boolean(true)),
            Some(false) => {}
            None => has_uncertain = true, // Track uncertainty (e.g., different precision)
        }
    }
    // If we had any uncertain comparisons with non-null items, result is uncertain
    if has_uncertain {
        Ok(CqlValue::Null)
    } else {
        Ok(CqlValue::Boolean(false))
    }
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

/// Proper includes: container includes contained AND container has at least one element not in contained
fn list_proper_includes(container: &CqlList, contained: &CqlList) -> EvalResult<CqlValue> {
    // First check that container includes contained
    for item in contained.iter() {
        let found = list_contains(container, item)?;
        if !matches!(found, CqlValue::Boolean(true)) {
            return Ok(CqlValue::Boolean(false));
        }
    }

    // Check that container has at least one element not in contained (proper subset)
    for item in container.iter() {
        let in_contained = list_contains(contained, item)?;
        if !matches!(in_contained, CqlValue::Boolean(true)) {
            // Found an element in container that's not in contained
            return Ok(CqlValue::Boolean(true));
        }
    }

    // All elements of container are in contained, so container equals contained
    Ok(CqlValue::Boolean(false))
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
    // Find max low and min high
    // For null boundaries: null represents "unknown", so we keep it when we can't determine
    let (new_low, new_low_closed) = match (a.low(), b.low()) {
        (Some(al), Some(bl)) => {
            match cql_compare(al, bl)? {
                Some(Ordering::Greater) => (Some(al.clone()), a.low_closed),
                Some(Ordering::Less) => (Some(bl.clone()), b.low_closed),
                Some(Ordering::Equal) => (Some(al.clone()), a.low_closed && b.low_closed),
                None => return Ok(CqlValue::Null),
            }
        }
        // If one low is null, use the other (higher is more restrictive for intersection)
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
        // If one high is null (unknown), keep null since we don't know the bound
        (Some(_), None) => (None, b.high_closed),
        (None, Some(_)) => (None, a.high_closed),
        (None, None) => (None, true),
    };

    // Check if the interval is definitely empty (low > high)
    if let (Some(low), Some(high)) = (&new_low, &new_high) {
        match cql_compare(low, high)? {
            Some(Ordering::Greater) => return Ok(CqlValue::Null),
            Some(Ordering::Equal) => {
                // Point interval is valid only if both bounds are closed
                if !new_low_closed || !new_high_closed {
                    return Ok(CqlValue::Null);
                }
            }
            _ => {}
        }
    }

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

    // Check for null bounds in a - if so, return null
    if a.low.is_none() && a.high.is_none() {
        return Ok(CqlValue::Null);
    }

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

    // Check if b ends after a starts and starts after a starts
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

    // Check if b is in the middle of a - both start and end are strictly inside
    // In this case, we can't represent the result as a single interval
    if b_starts_after_a_starts && b_ends_before_a_ends {
        return Ok(CqlValue::Null); // b is in the middle of a
    }

    // b overlaps with the end of a - return the beginning of a
    if b_ends_after_a_starts && b_starts_after_a_starts {
        // Return [a.low, b.low) - normalize to closed interval using predecessor
        let new_high = if b.low_closed {
            b.low.as_ref().map(|l| predecessor(l))
        } else {
            b.low.as_ref().map(|l| (**l).clone())
        };
        return Ok(CqlValue::Interval(CqlInterval::new(
            a.point_type.clone(),
            a.low.as_ref().map(|l| (**l).clone()),
            a.low_closed,
            new_high,
            true, // closed after normalization
        )));
    }

    // b overlaps with the start of a - return the end of a
    if b_starts_before_a_ends && b_ends_before_a_ends {
        // Return (b.high, a.high] - normalize to closed interval using successor
        let new_low = if b.high_closed {
            b.high.as_ref().map(|h| successor(h))
        } else {
            b.high.as_ref().map(|h| (**h).clone())
        };
        return Ok(CqlValue::Interval(CqlInterval::new(
            a.point_type.clone(),
            new_low,
            true, // closed after normalization
            a.high.as_ref().map(|h| (**h).clone()),
            a.high_closed,
        )));
    }

    Ok(CqlValue::Null)
}

/// Compute predecessor of a value for interval normalization
fn predecessor(value: &CqlValue) -> CqlValue {
    match value {
        CqlValue::Integer(n) => CqlValue::Integer(n - 1),
        CqlValue::Decimal(d) => CqlValue::Decimal(*d - rust_decimal::Decimal::new(1, 8)), // 0.00000001
        CqlValue::Date(dt) => {
            use chrono::{Duration, Datelike};
            if let Some(naive) = dt.to_naive_date() {
                let pred = naive - Duration::days(1);
                CqlValue::Date(CqlDate::new(pred.year(), pred.month() as u8, pred.day() as u8))
            } else {
                value.clone()
            }
        }
        CqlValue::DateTime(dt) => {
            // Subtract based on the lowest precision present
            use chrono::{Duration, Datelike, Timelike, NaiveDateTime, NaiveDate, NaiveTime};
            // Need month and day to construct a proper date
            let (Some(month), Some(day)) = (dt.month, dt.day) else {
                return value.clone();
            };
            let date = NaiveDate::from_ymd_opt(dt.year, month as u32, day as u32);
            let time = NaiveTime::from_hms_milli_opt(
                dt.hour.unwrap_or(0) as u32,
                dt.minute.unwrap_or(0) as u32,
                dt.second.unwrap_or(0) as u32,
                dt.millisecond.unwrap_or(0) as u32,
            );
            if let (Some(d), Some(t)) = (date, time) {
                let naive = NaiveDateTime::new(d, t);
                let pred = if dt.millisecond.is_some() {
                    naive - Duration::milliseconds(1)
                } else if dt.second.is_some() {
                    naive - Duration::seconds(1)
                } else if dt.minute.is_some() {
                    naive - Duration::minutes(1)
                } else if dt.hour.is_some() {
                    naive - Duration::hours(1)
                } else {
                    naive - Duration::days(1)
                };
                CqlValue::DateTime(CqlDateTime {
                    year: pred.year(),
                    month: Some(pred.month() as u8),
                    day: Some(pred.day() as u8),
                    hour: if dt.hour.is_some() { Some(pred.hour() as u8) } else { None },
                    minute: if dt.minute.is_some() { Some(pred.minute() as u8) } else { None },
                    second: if dt.second.is_some() { Some(pred.second() as u8) } else { None },
                    millisecond: if dt.millisecond.is_some() {
                        Some((pred.nanosecond() / 1_000_000) as u16)
                    } else {
                        None
                    },
                    timezone_offset: dt.timezone_offset,
                })
            } else {
                value.clone()
            }
        }
        CqlValue::Time(t) => {
            // Subtract 1 millisecond
            CqlValue::Time(octofhir_cql_types::CqlTime {
                hour: t.hour,
                minute: t.minute,
                second: t.second,
                millisecond: t.millisecond.map(|ms| if ms > 0 { ms - 1 } else { 999 }),
            })
        }
        CqlValue::Quantity(q) => {
            CqlValue::Quantity(octofhir_cql_types::CqlQuantity {
                value: q.value - rust_decimal::Decimal::new(1, 8),
                unit: q.unit.clone(),
            })
        }
        _ => value.clone(),
    }
}

/// Compute successor of a value for interval normalization
fn successor(value: &CqlValue) -> CqlValue {
    match value {
        CqlValue::Integer(n) => CqlValue::Integer(n + 1),
        CqlValue::Decimal(d) => CqlValue::Decimal(*d + rust_decimal::Decimal::new(1, 8)),
        CqlValue::Date(dt) => {
            use chrono::{Duration, Datelike};
            if let Some(naive) = dt.to_naive_date() {
                let succ = naive + Duration::days(1);
                CqlValue::Date(CqlDate::new(succ.year(), succ.month() as u8, succ.day() as u8))
            } else {
                value.clone()
            }
        }
        CqlValue::DateTime(dt) => {
            // Add based on the lowest precision present
            use chrono::{Duration, Datelike, Timelike, NaiveDateTime, NaiveDate, NaiveTime};
            // Need month and day to construct a proper date
            let (Some(month), Some(day)) = (dt.month, dt.day) else {
                return value.clone();
            };
            let date = NaiveDate::from_ymd_opt(dt.year, month as u32, day as u32);
            let time = NaiveTime::from_hms_milli_opt(
                dt.hour.unwrap_or(0) as u32,
                dt.minute.unwrap_or(0) as u32,
                dt.second.unwrap_or(0) as u32,
                dt.millisecond.unwrap_or(0) as u32,
            );
            if let (Some(d), Some(t)) = (date, time) {
                let naive = NaiveDateTime::new(d, t);
                let succ = if dt.millisecond.is_some() {
                    naive + Duration::milliseconds(1)
                } else if dt.second.is_some() {
                    naive + Duration::seconds(1)
                } else if dt.minute.is_some() {
                    naive + Duration::minutes(1)
                } else if dt.hour.is_some() {
                    naive + Duration::hours(1)
                } else {
                    naive + Duration::days(1)
                };
                CqlValue::DateTime(CqlDateTime {
                    year: succ.year(),
                    month: Some(succ.month() as u8),
                    day: Some(succ.day() as u8),
                    hour: if dt.hour.is_some() { Some(succ.hour() as u8) } else { None },
                    minute: if dt.minute.is_some() { Some(succ.minute() as u8) } else { None },
                    second: if dt.second.is_some() { Some(succ.second() as u8) } else { None },
                    millisecond: if dt.millisecond.is_some() {
                        Some((succ.nanosecond() / 1_000_000) as u16)
                    } else {
                        None
                    },
                    timezone_offset: dt.timezone_offset,
                })
            } else {
                value.clone()
            }
        }
        CqlValue::Time(t) => {
            // Add based on the lowest precision present
            use chrono::{Duration, Timelike, NaiveTime};
            let naive = NaiveTime::from_hms_milli_opt(
                t.hour as u32,
                t.minute.unwrap_or(0) as u32,
                t.second.unwrap_or(0) as u32,
                t.millisecond.unwrap_or(0) as u32,
            );
            if let Some(naive) = naive {
                let succ = if t.millisecond.is_some() {
                    naive + Duration::milliseconds(1)
                } else if t.second.is_some() {
                    naive + Duration::seconds(1)
                } else if t.minute.is_some() {
                    naive + Duration::minutes(1)
                } else {
                    naive + Duration::hours(1)
                };
                CqlValue::Time(octofhir_cql_types::CqlTime {
                    hour: succ.hour() as u8,
                    minute: if t.minute.is_some() { Some(succ.minute() as u8) } else { None },
                    second: if t.second.is_some() { Some(succ.second() as u8) } else { None },
                    millisecond: if t.millisecond.is_some() {
                        Some((succ.nanosecond() / 1_000_000) as u16)
                    } else {
                        None
                    },
                })
            } else {
                value.clone()
            }
        }
        CqlValue::Quantity(q) => {
            CqlValue::Quantity(octofhir_cql_types::CqlQuantity {
                value: q.value + rust_decimal::Decimal::new(1, 8),
                unit: q.unit.clone(),
            })
        }
        _ => value.clone(),
    }
}

fn collapse_intervals(list: &CqlList) -> EvalResult<CqlValue> {
    // Extract intervals, skipping nulls
    let mut intervals: Vec<CqlInterval> = Vec::new();
    for item in list.iter() {
        match item {
            CqlValue::Interval(i) => {
                // Skip null intervals (both bounds null)
                if i.low.is_none() && i.high.is_none() {
                    continue;
                }
                intervals.push(i.clone());
            }
            CqlValue::Null => continue,
            _ => return Err(EvalError::type_mismatch("Interval", item.get_type().name())),
        }
    }

    if intervals.is_empty() {
        return Ok(CqlValue::List(CqlList::new(CqlType::interval(CqlType::Any))));
    }

    let point_type = intervals[0].point_type.clone();

    // Sort intervals by low bound
    intervals.sort_by(|a, b| {
        match (&a.low, &b.low) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less, // Null low = -infinity
            (Some(_), None) => Ordering::Greater,
            (Some(al), Some(bl)) => {
                cql_compare(al, bl).unwrap_or(None).unwrap_or(Ordering::Equal)
            }
        }
    });

    // Merge overlapping/adjacent intervals
    let mut result: Vec<CqlInterval> = Vec::new();
    for interval in intervals {
        if result.is_empty() {
            result.push(interval);
            continue;
        }

        let last = result.last_mut().unwrap();

        // Check if intervals overlap or are adjacent (can be merged)
        let can_merge = match (&last.high, &interval.low) {
            (None, _) => true, // last.high = +infinity, always overlaps
            (_, None) => true, // interval.low = -infinity, always overlaps
            (Some(last_high), Some(int_low)) => {
                // Merge if last_high >= int_low - 1 (for integers) or last_high >= int_low
                match cql_compare(last_high, int_low).unwrap_or(None) {
                    Some(Ordering::Less) => {
                        // Check if they are adjacent (for integer types)
                        is_adjacent(last_high, int_low, last.high_closed, interval.low_closed)
                    }
                    Some(Ordering::Equal) => last.high_closed || interval.low_closed,
                    Some(Ordering::Greater) => true,
                    None => false,
                }
            }
        };

        if can_merge {
            // Extend the current interval's high bound if needed
            match (&last.high, &interval.high) {
                (None, _) => {} // already +infinity
                (_, None) => {
                    last.high = None;
                    last.high_closed = interval.high_closed;
                }
                (Some(lh), Some(ih)) => {
                    match cql_compare(lh, ih).unwrap_or(None) {
                        Some(Ordering::Less) => {
                            last.high = interval.high.clone();
                            last.high_closed = interval.high_closed;
                        }
                        Some(Ordering::Equal) => {
                            last.high_closed = last.high_closed || interval.high_closed;
                        }
                        _ => {} // keep current high
                    }
                }
            }
        } else {
            result.push(interval);
        }
    }

    let elements: Vec<CqlValue> = result.into_iter().map(CqlValue::Interval).collect();
    Ok(CqlValue::List(CqlList {
        element_type: CqlType::interval(point_type),
        elements,
    }))
}

/// Convert a CqlTime to total milliseconds from midnight
fn time_to_milliseconds(t: &octofhir_cql_types::CqlTime) -> i64 {
    let hours = t.hour as i64;
    let minutes = t.minute.unwrap_or(0) as i64;
    let seconds = t.second.unwrap_or(0) as i64;
    let millis = t.millisecond.unwrap_or(0) as i64;
    hours * 3600000 + minutes * 60000 + seconds * 1000 + millis
}

/// Check if two values are adjacent (for merge purposes)
fn is_adjacent(high: &CqlValue, low: &CqlValue, high_closed: bool, low_closed: bool) -> bool {
    // If one boundary is open and the other is closed, they're adjacent if values differ by 1
    match (high, low) {
        (CqlValue::Integer(h), CqlValue::Integer(l)) => {
            if high_closed && low_closed {
                *h + 1 >= *l
            } else if high_closed {
                *h + 1 >= *l
            } else if low_closed {
                *h >= *l - 1
            } else {
                *h + 1 >= *l - 1
            }
        }
        (CqlValue::Decimal(h), CqlValue::Decimal(l)) => {
            // For decimals, consider adjacent if difference is very small
            let diff = (*l - *h).abs();
            diff < rust_decimal::Decimal::new(1, 7) // 0.0000001
        }
        (CqlValue::DateTime(h), CqlValue::DateTime(l)) => {
            // Check if DateTime values are adjacent (within 1 day or 1 millisecond)
            // Use the lowest precision to determine adjacency
            if let (Some(h_month), Some(l_month)) = (h.month, l.month) {
                if let (Some(h_day), Some(l_day)) = (h.day, l.day) {
                    // Full date precision - check if consecutive days
                    if h.year == l.year && h_month == l_month {
                        high_closed && low_closed && h_day + 1 >= l_day
                    } else if h.year == l.year && h_month + 1 == l_month && l_day == 1 {
                        // Adjacent months
                        true
                    } else {
                        false
                    }
                } else {
                    // Month precision - check if consecutive months
                    if h.year == l.year {
                        high_closed && low_closed && h_month + 1 >= l_month
                    } else {
                        false
                    }
                }
            } else {
                // Year precision only
                high_closed && low_closed && h.year + 1 >= l.year
            }
        }
        (CqlValue::Date(h), CqlValue::Date(l)) => {
            if let (Some(h_month), Some(l_month)) = (h.month, l.month) {
                if let (Some(h_day), Some(l_day)) = (h.day, l.day) {
                    if h.year == l.year && h_month == l_month {
                        high_closed && low_closed && h_day + 1 >= l_day
                    } else if h.year == l.year && h_month + 1 == l_month && l_day == 1 {
                        true
                    } else {
                        false
                    }
                } else {
                    if h.year == l.year {
                        high_closed && low_closed && h_month + 1 >= l_month
                    } else {
                        false
                    }
                }
            } else {
                high_closed && low_closed && h.year + 1 >= l.year
            }
        }
        (CqlValue::Time(h), CqlValue::Time(l)) => {
            // Check if times are adjacent (successor/predecessor relationship)
            // For two closed intervals [a, b] and [c, d], they're adjacent if successor(b) = c
            if !high_closed || !low_closed {
                return false; // Only handle both-closed case for simplicity
            }

            // Convert both times to total milliseconds for easier comparison
            let h_ms_total = time_to_milliseconds(h);
            let l_ms_total = time_to_milliseconds(l);

            // Adjacent if high + 1ms = low (for millisecond precision)
            // Or if they have lower precision, check adjacency at that level
            if h.millisecond.is_some() && l.millisecond.is_some() {
                // Millisecond precision - adjacent if differ by 1ms
                h_ms_total + 1 == l_ms_total
            } else if h.second.is_some() && l.second.is_some() {
                // Second precision - adjacent if differ by 1 second
                h_ms_total + 1000 == l_ms_total || h_ms_total / 1000 + 1 == l_ms_total / 1000
            } else if h.minute.is_some() && l.minute.is_some() {
                // Minute precision - adjacent if differ by 1 minute
                h_ms_total / 60000 + 1 == l_ms_total / 60000
            } else {
                // Hour precision - adjacent if differ by 1 hour
                h.hour + 1 == l.hour
            }
        }
        _ => false, // For other types, only overlapping intervals merge
    }
}

/// Expand a single interval to a list of raw values (the "interval overload")
/// expand Interval[1, 10] => { 1, 2, 3, ..., 10 }
fn expand_interval(interval: &CqlInterval, per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    // Get the step size from `per` parameter
    let step = match per {
        Some(CqlValue::Integer(n)) => *n,
        Some(CqlValue::Decimal(d)) => {
            // For decimal step, handle separately
            return expand_interval_decimal(interval, *d);
        }
        Some(CqlValue::Quantity(q)) => {
            // For quantity per, extract value and unit
            return expand_interval_with_quantity(interval, q);
        }
        None => 1, // default step is 1
        _ => 1,
    };

    match (&interval.low, &interval.high) {
        (Some(low), Some(high)) => {
            match (low.as_ref(), high.as_ref()) {
                (CqlValue::Integer(l), CqlValue::Integer(h)) => {
                    let start = if interval.low_closed { *l } else { l + 1 };
                    let end = if interval.high_closed { *h } else { h - 1 };

                    if start > end {
                        return Ok(CqlValue::List(CqlList::new(CqlType::Integer)));
                    }

                    // Create list of raw values with the given step
                    // Only include values where the entire unit [value, value+step-1] fits within the interval
                    let mut elements = Vec::new();
                    let mut current = start;
                    while current + step - 1 <= end {
                        elements.push(CqlValue::Integer(current));
                        current += step;
                    }
                    Ok(CqlValue::List(CqlList {
                        element_type: CqlType::Integer,
                        elements,
                    }))
                }
                (CqlValue::Date(l), CqlValue::Date(h)) => {
                    // For dates, expand to raw date values
                    expand_date_interval_values(l, h, interval.low_closed, interval.high_closed, per)
                }
                (CqlValue::DateTime(l), CqlValue::DateTime(h)) => {
                    // For datetimes, expand to raw datetime values
                    expand_datetime_interval_values(l, h, interval.low_closed, interval.high_closed, per)
                }
                (CqlValue::Time(l), CqlValue::Time(h)) => {
                    // For times, expand to raw time values
                    expand_time_interval_values(l, h, interval.low_closed, interval.high_closed, per)
                }
                // Handle Decimal bounds with Integer step - return Integer values
                (CqlValue::Decimal(l), CqlValue::Decimal(h)) => {
                    use rust_decimal::prelude::ToPrimitive;
                    // For Decimal intervals with Integer step, return Integer values
                    let start_int = if interval.low_closed {
                        l.ceil().to_i32().unwrap_or(0) // Include the ceiling if closed
                    } else {
                        l.floor().to_i32().unwrap_or(0) + 1 // Exclude the value, so floor + 1
                    };
                    let end_int = if interval.high_closed {
                        h.floor().to_i32().unwrap_or(0) // Include the floor if closed
                    } else {
                        // For open bound: if floor(h) < h, floor is valid; if floor(h) == h, exclude it
                        let floored = h.floor();
                        if floored < *h {
                            floored.to_i32().unwrap_or(0)
                        } else {
                            floored.to_i32().unwrap_or(0) - 1
                        }
                    };

                    if start_int > end_int {
                        return Ok(CqlValue::List(CqlList::new(CqlType::Integer)));
                    }

                    let mut elements = Vec::new();
                    let mut current = start_int;
                    while current <= end_int {
                        elements.push(CqlValue::Integer(current));
                        current += step;
                    }
                    Ok(CqlValue::List(CqlList {
                        element_type: CqlType::Integer,
                        elements,
                    }))
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Expand a single interval to a list of unit intervals (for list input)
/// expand { Interval[1, 10] } => { Interval[1,1], Interval[2,2], ..., Interval[10,10] }
fn expand_interval_to_unit_intervals(interval: &CqlInterval, per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    // Handle Decimal per parameter specially - produce Decimal intervals
    if let Some(CqlValue::Decimal(step_decimal)) = per {
        return expand_interval_to_decimal_unit_intervals(interval, *step_decimal);
    }

    // Get the step size from `per` parameter
    let step: i64 = match per {
        Some(CqlValue::Integer(n)) => *n as i64,
        Some(CqlValue::Quantity(q)) => {
            // For quantity, extract value
            q.value.to_string().parse().unwrap_or(1)
        }
        None => 1,
        _ => 1,
    };

    match (&interval.low, &interval.high) {
        (Some(low), Some(high)) => {
            match (low.as_ref(), high.as_ref()) {
                (CqlValue::Integer(l), CqlValue::Integer(h)) => {
                    let step_i32 = step as i32;
                    let start = if interval.low_closed { *l } else { l + 1 };
                    let end = if interval.high_closed { *h } else { h - 1 };

                    if start > end {
                        return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Integer)))));
                    }

                    // Create list of intervals with width = step
                    // Only include full-width intervals (not partial ones at the end)
                    let mut elements = Vec::new();
                    let mut current = start;
                    while current + step_i32 - 1 <= end {
                        let interval_end = current + step_i32 - 1;
                        elements.push(CqlValue::Interval(CqlInterval::closed(
                            CqlType::Integer,
                            CqlValue::Integer(current),
                            CqlValue::Integer(interval_end),
                        )));
                        current += step_i32;
                    }
                    Ok(CqlValue::List(CqlList {
                        element_type: CqlType::Interval(Box::new(CqlType::Integer)),
                        elements,
                    }))
                }
                // Handle Decimal intervals with Integer step - expand as Integer intervals
                (CqlValue::Decimal(l), CqlValue::Decimal(h)) => {
                    use rust_decimal::prelude::ToPrimitive;
                    // For Decimal intervals, produce Integer unit intervals
                    let start_int = if interval.low_closed {
                        l.ceil().to_i64().unwrap_or(0) // Include the ceiling if closed
                    } else {
                        l.floor().to_i64().unwrap_or(0) + 1 // Exclude the value
                    };
                    let end_int = if interval.high_closed {
                        h.floor().to_i64().unwrap_or(0) // Include the floor if closed
                    } else {
                        // For open bound: if floor(h) < h, floor is valid; if floor(h) == h, exclude it
                        let floored = h.floor();
                        if floored < *h {
                            floored.to_i64().unwrap_or(0)
                        } else {
                            floored.to_i64().unwrap_or(0) - 1
                        }
                    };

                    if start_int > end_int {
                        return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Integer)))));
                    }

                    let mut elements = Vec::new();
                    let mut current = start_int;
                    while current <= end_int {
                        let interval_end = (current + step - 1).min(end_int);
                        elements.push(CqlValue::Interval(CqlInterval::closed(
                            CqlType::Integer,
                            CqlValue::Integer(current as i32),
                            CqlValue::Integer(interval_end as i32),
                        )));
                        current += step;
                    }
                    Ok(CqlValue::List(CqlList {
                        element_type: CqlType::Interval(Box::new(CqlType::Integer)),
                        elements,
                    }))
                }
                (CqlValue::Date(l), CqlValue::Date(h)) => {
                    expand_date_interval_unit_intervals_with_step(l, h, interval.low_closed, interval.high_closed, step, per)
                }
                (CqlValue::DateTime(l), CqlValue::DateTime(h)) => {
                    expand_datetime_interval_unit_intervals(l, h, interval.low_closed, interval.high_closed, per)
                }
                (CqlValue::Time(l), CqlValue::Time(h)) => {
                    expand_time_interval_unit_intervals(l, h, interval.low_closed, interval.high_closed, per)
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Expand interval with decimal step
fn expand_interval_decimal(interval: &CqlInterval, step: rust_decimal::Decimal) -> EvalResult<CqlValue> {
    use rust_decimal::Decimal;

    match (&interval.low, &interval.high) {
        (Some(low), Some(high)) => {
            // For Integer intervals, Integer N expanded with Decimal step covers [N.0, N+1-step]
            let (start_dec, end_dec) = match (low.as_ref(), high.as_ref()) {
                (CqlValue::Integer(l), CqlValue::Integer(h)) => {
                    let start = if interval.low_closed {
                        Decimal::from(*l)
                    } else {
                        Decimal::from(*l) + Decimal::ONE
                    };
                    // For Integer high, the effective end for Decimal expansion is h + 1 - step
                    let end = if interval.high_closed {
                        Decimal::from(*h) + Decimal::ONE - step
                    } else {
                        Decimal::from(*h) - step
                    };
                    (start, end)
                }
                (CqlValue::Decimal(l), CqlValue::Decimal(h)) => {
                    let start = if interval.low_closed { *l } else { *l + step };
                    let end = if interval.high_closed { *h } else { *h - step };
                    (start, end)
                }
                _ => return Ok(CqlValue::Null),
            };

            if start_dec > end_dec {
                return Ok(CqlValue::List(CqlList::new(CqlType::Decimal)));
            }

            let mut elements = Vec::new();
            let mut current = start_dec;
            while current <= end_dec {
                elements.push(CqlValue::Decimal(current));
                current += step;
                // Safety check to prevent infinite loop
                if elements.len() > 10000 {
                    break;
                }
            }
            Ok(CqlValue::List(CqlList {
                element_type: CqlType::Decimal,
                elements,
            }))
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Expand interval to decimal unit intervals (for list input with decimal step)
/// e.g., expand { Interval[10, 10] } per 0.1 => { Interval[10.0,10.0], Interval[10.1,10.1], ... }
fn expand_interval_to_decimal_unit_intervals(interval: &CqlInterval, step: rust_decimal::Decimal) -> EvalResult<CqlValue> {
    use rust_decimal::Decimal;

    match (&interval.low, &interval.high) {
        (Some(low), Some(high)) => {
            // Convert bounds to Decimal
            // For Integer intervals, Integer N expanded with Decimal step covers [N.0, N+1-step]
            // e.g., Interval[10, 10] per 0.1 covers 10.0 to 10.9
            let (start_dec, end_dec) = match (low.as_ref(), high.as_ref()) {
                (CqlValue::Integer(l), CqlValue::Integer(h)) => {
                    let start = if interval.low_closed {
                        Decimal::from(*l)
                    } else {
                        Decimal::from(*l) + Decimal::ONE
                    };
                    // For Integer high, the effective end for Decimal expansion is h + 1 - step
                    let end = if interval.high_closed {
                        Decimal::from(*h) + Decimal::ONE - step
                    } else {
                        Decimal::from(*h) - step
                    };
                    (start, end)
                }
                (CqlValue::Decimal(l), CqlValue::Decimal(h)) => {
                    let start = if interval.low_closed { *l } else { *l + step };
                    let end = if interval.high_closed { *h } else { *h - step };
                    (start, end)
                }
                _ => return Ok(CqlValue::Null),
            };

            if start_dec > end_dec {
                return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Decimal)))));
            }

            // Create list of unit intervals
            let mut elements = Vec::new();
            let mut current = start_dec;
            while current <= end_dec {
                elements.push(CqlValue::Interval(CqlInterval::closed(
                    CqlType::Decimal,
                    CqlValue::Decimal(current),
                    CqlValue::Decimal(current), // Unit interval: start = end
                )));
                current += step;
                // Safety check to prevent infinite loop
                if elements.len() > 10000 {
                    break;
                }
            }
            Ok(CqlValue::List(CqlList {
                element_type: CqlType::Interval(Box::new(CqlType::Decimal)),
                elements,
            }))
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Expand interval with quantity step (e.g., "2 days")
fn expand_interval_with_quantity(interval: &CqlInterval, quantity: &octofhir_cql_types::CqlQuantity) -> EvalResult<CqlValue> {
    // Extract unit to determine what kind of expansion
    let unit = quantity.unit.as_deref().unwrap_or("");
    let value: i64 = quantity.value.to_string().parse().unwrap_or(1);

    match (&interval.low, &interval.high) {
        (Some(low), Some(high)) => {
            match (low.as_ref(), high.as_ref()) {
                (CqlValue::Date(l), CqlValue::Date(h)) => {
                    expand_date_interval_values_with_step(l, h, interval.low_closed, interval.high_closed, value, unit)
                }
                (CqlValue::DateTime(l), CqlValue::DateTime(h)) => {
                    expand_datetime_interval_values_with_step(l, h, interval.low_closed, interval.high_closed, value, unit)
                }
                (CqlValue::Time(l), CqlValue::Time(h)) => {
                    expand_time_interval_values_with_step(l, h, interval.low_closed, interval.high_closed, value, unit)
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Expand a date interval to a list of unit intervals
fn expand_date_interval(low: &CqlDate, high: &CqlDate, low_closed: bool, high_closed: bool) -> EvalResult<CqlValue> {
    use chrono::{NaiveDate, Datelike, Duration};

    let start_date = low.to_naive_date();
    let end_date = high.to_naive_date();

    let (start, end): (NaiveDate, NaiveDate) = match (start_date, end_date) {
        (Some(s), Some(e)) => {
            let actual_start: NaiveDate = if low_closed { s } else { s + Duration::days(1) };
            let actual_end: NaiveDate = if high_closed { e } else { e - Duration::days(1) };
            (actual_start, actual_end)
        }
        _ => return Ok(CqlValue::Null),
    };

    if start > end {
        return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Date)))));
    }

    let mut elements = Vec::new();
    let mut current = start;
    while current <= end {
        let date = CqlDate::new(current.year(), current.month() as u8, current.day() as u8);
        elements.push(CqlValue::Interval(CqlInterval::closed(
            CqlType::Date,
            CqlValue::Date(date.clone()),
            CqlValue::Date(date),
        )));
        current = current + Duration::days(1);
    }

    Ok(CqlValue::List(CqlList {
        element_type: CqlType::Interval(Box::new(CqlType::Date)),
        elements,
    }))
}

/// Expand a datetime interval to a list of unit intervals
fn expand_datetime_interval(low: &CqlDateTime, high: &CqlDateTime, low_closed: bool, high_closed: bool) -> EvalResult<CqlValue> {
    // For datetimes, expand by the precision of the endpoints (typically day)
    // This is a simplified implementation that expands by day

    let start_date = low.date();
    let end_date = high.date();

    let result = expand_date_interval(&start_date, &end_date, low_closed, high_closed)?;

    // Convert Date intervals to DateTime intervals
    match result {
        CqlValue::List(list) => {
            let elements: Vec<CqlValue> = list.elements.into_iter().map(|elem| {
                match &elem {
                    CqlValue::Interval(i) => {
                        match (&i.low, &i.high) {
                            (Some(low), Some(high)) => {
                                match (low.as_ref(), high.as_ref()) {
                                    (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                                        CqlValue::Interval(CqlInterval::closed(
                                            CqlType::DateTime,
                                            CqlValue::DateTime(CqlDateTime::from_date(d1.clone())),
                                            CqlValue::DateTime(CqlDateTime::from_date(d2.clone())),
                                        ))
                                    }
                                    _ => elem,
                                }
                            }
                            _ => elem,
                        }
                    }
                    _ => elem,
                }
            }).collect();
            Ok(CqlValue::List(CqlList {
                element_type: CqlType::Interval(Box::new(CqlType::DateTime)),
                elements,
            }))
        }
        other => Ok(other),
    }
}

/// Expand date interval to raw date values (for single interval input)
fn expand_date_interval_values(low: &CqlDate, high: &CqlDate, low_closed: bool, high_closed: bool, _per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    use chrono::{NaiveDate, Datelike, Duration};

    let start_date = low.to_naive_date();
    let end_date = high.to_naive_date();

    let (start, end): (NaiveDate, NaiveDate) = match (start_date, end_date) {
        (Some(s), Some(e)) => {
            let actual_start: NaiveDate = if low_closed { s } else { s + Duration::days(1) };
            let actual_end: NaiveDate = if high_closed { e } else { e - Duration::days(1) };
            (actual_start, actual_end)
        }
        _ => return Ok(CqlValue::Null),
    };

    if start > end {
        return Ok(CqlValue::List(CqlList::new(CqlType::Date)));
    }

    let mut elements = Vec::new();
    let mut current = start;
    while current <= end {
        let date = CqlDate::new(current.year(), current.month() as u8, current.day() as u8);
        elements.push(CqlValue::Date(date));
        current = current + Duration::days(1);
    }

    Ok(CqlValue::List(CqlList {
        element_type: CqlType::Date,
        elements,
    }))
}

/// Expand datetime interval to raw datetime values (for single interval input)
fn expand_datetime_interval_values(low: &CqlDateTime, high: &CqlDateTime, low_closed: bool, high_closed: bool, per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    // Expand by day and convert to datetimes
    let result = expand_date_interval_values(&low.date(), &high.date(), low_closed, high_closed, per)?;

    match result {
        CqlValue::List(list) => {
            let elements: Vec<CqlValue> = list.elements.into_iter().map(|elem| {
                match elem {
                    CqlValue::Date(d) => CqlValue::DateTime(CqlDateTime::from_date(d)),
                    other => other,
                }
            }).collect();
            Ok(CqlValue::List(CqlList {
                element_type: CqlType::DateTime,
                elements,
            }))
        }
        other => Ok(other),
    }
}

/// Expand time interval to raw time values (for single interval input)
fn expand_time_interval_values(low: &octofhir_cql_types::CqlTime, high: &octofhir_cql_types::CqlTime, low_closed: bool, high_closed: bool, _per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    // For low bound: if closed, include low.hour; if open, start at low.hour + 1
    let start_hour = if low_closed { low.hour } else { low.hour + 1 };

    // For high bound: if closed, include high.hour
    // If open: include high.hour if there are subcomponents > 0 (e.g., @T12:30 means 12:00 < 12:30)
    let end_hour = if high_closed {
        high.hour
    } else {
        let has_subcomponents = high.minute.unwrap_or(0) > 0
            || high.second.unwrap_or(0) > 0
            || high.millisecond.unwrap_or(0) > 0;
        if has_subcomponents {
            high.hour
        } else {
            high.hour.saturating_sub(1)
        }
    };

    if start_hour > end_hour {
        return Ok(CqlValue::List(CqlList::new(CqlType::Time)));
    }

    let mut elements = Vec::new();
    for hour in start_hour..=end_hour {
        // Use hour_only precision for the output
        elements.push(CqlValue::Time(octofhir_cql_types::CqlTime::hour_only(hour)));
    }

    Ok(CqlValue::List(CqlList {
        element_type: CqlType::Time,
        elements,
    }))
}

/// Expand date interval to unit intervals (for list input)
fn expand_date_interval_unit_intervals(low: &CqlDate, high: &CqlDate, low_closed: bool, high_closed: bool, _per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    expand_date_interval(low, high, low_closed, high_closed)
}

/// Expand date interval to unit intervals with custom step (for `per N days`)
fn expand_date_interval_unit_intervals_with_step(low: &CqlDate, high: &CqlDate, low_closed: bool, high_closed: bool, step: i64, per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    use chrono::{NaiveDate, Datelike, Duration};

    // Get the unit from per if it's a Quantity
    let unit = match per {
        Some(CqlValue::Quantity(q)) => q.unit.as_deref().unwrap_or("day"),
        _ => "day",
    };

    let duration = match unit {
        "day" | "days" | "d" => Duration::days(step),
        "week" | "weeks" | "wk" => Duration::weeks(step),
        "month" | "months" | "mo" => Duration::days(step * 30), // approximate
        "year" | "years" | "a" => Duration::days(step * 365), // approximate
        _ => Duration::days(step),
    };

    let start_date = low.to_naive_date();
    let end_date = high.to_naive_date();

    let (start, end): (NaiveDate, NaiveDate) = match (start_date, end_date) {
        (Some(s), Some(e)) => {
            let actual_start: NaiveDate = if low_closed { s } else { s + Duration::days(1) };
            let actual_end: NaiveDate = if high_closed { e } else { e - Duration::days(1) };
            (actual_start, actual_end)
        }
        _ => return Ok(CqlValue::Null),
    };

    if start > end {
        return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Date)))));
    }

    let mut elements = Vec::new();
    let mut interval_start = start;
    while interval_start <= end {
        let interval_end_candidate = interval_start + duration - Duration::days(1);
        let interval_end = if interval_end_candidate > end { end } else { interval_end_candidate };

        let d1 = CqlDate::new(interval_start.year(), interval_start.month() as u8, interval_start.day() as u8);
        let d2 = CqlDate::new(interval_end.year(), interval_end.month() as u8, interval_end.day() as u8);

        elements.push(CqlValue::Interval(CqlInterval::closed(
            CqlType::Date,
            CqlValue::Date(d1),
            CqlValue::Date(d2),
        )));

        interval_start = interval_start + duration;
    }

    Ok(CqlValue::List(CqlList {
        element_type: CqlType::Interval(Box::new(CqlType::Date)),
        elements,
    }))
}

/// Expand datetime interval to unit intervals (for list input)
fn expand_datetime_interval_unit_intervals(low: &CqlDateTime, high: &CqlDateTime, low_closed: bool, high_closed: bool, _per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    expand_datetime_interval(low, high, low_closed, high_closed)
}

/// Expand time interval to unit intervals (for list input)
fn expand_time_interval_unit_intervals(low: &octofhir_cql_types::CqlTime, high: &octofhir_cql_types::CqlTime, low_closed: bool, high_closed: bool, per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    // Check if the requested expansion unit is more precise than the interval's precision
    // If expanding per minute/second/ms but interval only has hour precision, return empty
    if let Some(CqlValue::Quantity(q)) = per {
        let unit = q.unit.as_deref().unwrap_or("");
        match unit {
            "minute" | "minutes" | "min" => {
                // If interval doesn't have minute precision, return empty
                if low.minute.is_none() || high.minute.is_none() {
                    return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Time)))));
                }
            }
            "second" | "seconds" | "s" => {
                // If interval doesn't have second precision, return empty
                if low.second.is_none() || high.second.is_none() {
                    return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Time)))));
                }
            }
            "millisecond" | "milliseconds" | "ms" => {
                // If interval doesn't have millisecond precision, return empty
                if low.millisecond.is_none() || high.millisecond.is_none() {
                    return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Time)))));
                }
            }
            _ => {} // hour or other - proceed normally
        }
    }

    // For low bound: if closed, include low.hour; if open, start at low.hour + 1
    let start_hour = if low_closed { low.hour } else { low.hour + 1 };

    // For high bound: if closed, include high.hour
    // If open: include high.hour if there are subcomponents > 0 (e.g., @T12:30 means 12:00 < 12:30)
    let end_hour = if high_closed {
        high.hour
    } else {
        // Check if high has subcomponents - if so, the hour is still valid
        let has_subcomponents = high.minute.unwrap_or(0) > 0
            || high.second.unwrap_or(0) > 0
            || high.millisecond.unwrap_or(0) > 0;
        if has_subcomponents {
            high.hour // Include this hour since @T{hour}:00 < high
        } else {
            high.hour.saturating_sub(1) // Exclude the exact hour boundary
        }
    };

    if start_hour > end_hour {
        return Ok(CqlValue::List(CqlList::new(CqlType::Interval(Box::new(CqlType::Time)))));
    }

    let mut elements = Vec::new();
    for hour in start_hour..=end_hour {
        // Use hour_only precision for the output
        let t = octofhir_cql_types::CqlTime::hour_only(hour);
        elements.push(CqlValue::Interval(CqlInterval::closed(
            CqlType::Time,
            CqlValue::Time(t.clone()),
            CqlValue::Time(t),
        )));
    }

    Ok(CqlValue::List(CqlList {
        element_type: CqlType::Interval(Box::new(CqlType::Time)),
        elements,
    }))
}

/// Expand date interval with custom step (for quantity-based expansion)
fn expand_date_interval_values_with_step(low: &CqlDate, high: &CqlDate, low_closed: bool, high_closed: bool, step: i64, unit: &str) -> EvalResult<CqlValue> {
    use chrono::{NaiveDate, Datelike, Duration};

    let start_date = low.to_naive_date();
    let end_date = high.to_naive_date();

    let (start, end): (NaiveDate, NaiveDate) = match (start_date, end_date) {
        (Some(s), Some(e)) => {
            let actual_start: NaiveDate = if low_closed { s } else { s + Duration::days(1) };
            let actual_end: NaiveDate = if high_closed { e } else { e - Duration::days(1) };
            (actual_start, actual_end)
        }
        _ => return Ok(CqlValue::Null),
    };

    if start > end {
        return Ok(CqlValue::List(CqlList::new(CqlType::Date)));
    }

    let duration = match unit {
        "day" | "days" | "d" => Duration::days(step),
        "week" | "weeks" | "wk" => Duration::weeks(step),
        "month" | "months" | "mo" => Duration::days(step * 30), // approximate
        "year" | "years" | "a" => Duration::days(step * 365), // approximate
        _ => Duration::days(step),
    };

    let mut elements = Vec::new();
    let mut current = start;
    while current <= end {
        let date = CqlDate::new(current.year(), current.month() as u8, current.day() as u8);
        elements.push(CqlValue::Date(date));
        current = current + duration;
    }

    Ok(CqlValue::List(CqlList {
        element_type: CqlType::Date,
        elements,
    }))
}

/// Expand datetime interval with custom step
fn expand_datetime_interval_values_with_step(low: &CqlDateTime, high: &CqlDateTime, low_closed: bool, high_closed: bool, step: i64, unit: &str) -> EvalResult<CqlValue> {
    let result = expand_date_interval_values_with_step(&low.date(), &high.date(), low_closed, high_closed, step, unit)?;

    match result {
        CqlValue::List(list) => {
            let elements: Vec<CqlValue> = list.elements.into_iter().map(|elem| {
                match elem {
                    CqlValue::Date(d) => CqlValue::DateTime(CqlDateTime::from_date(d)),
                    other => other,
                }
            }).collect();
            Ok(CqlValue::List(CqlList {
                element_type: CqlType::DateTime,
                elements,
            }))
        }
        other => Ok(other),
    }
}

/// Expand time interval with custom step
fn expand_time_interval_values_with_step(low: &octofhir_cql_types::CqlTime, high: &octofhir_cql_types::CqlTime, low_closed: bool, high_closed: bool, step: i64, unit: &str) -> EvalResult<CqlValue> {
    let step_hours: u8 = match unit {
        "hour" | "hours" | "h" => step as u8,
        "minute" | "minutes" | "min" => 0, // not supported in hour precision
        _ => step as u8,
    };

    if step_hours == 0 {
        return Ok(CqlValue::List(CqlList::new(CqlType::Time)));
    }

    // For low bound: if closed, include low.hour; if open, start at low.hour + 1
    let start_hour = if low_closed { low.hour } else { low.hour + 1 };

    // For high bound: if closed, include high.hour
    // If open: include high.hour if there are subcomponents > 0
    let end_hour = if high_closed {
        high.hour
    } else {
        let has_subcomponents = high.minute.unwrap_or(0) > 0
            || high.second.unwrap_or(0) > 0
            || high.millisecond.unwrap_or(0) > 0;
        if has_subcomponents {
            high.hour
        } else {
            high.hour.saturating_sub(1)
        }
    };

    if start_hour > end_hour {
        return Ok(CqlValue::List(CqlList::new(CqlType::Time)));
    }

    let mut elements = Vec::new();
    let mut hour = start_hour;
    while hour <= end_hour {
        // Use hour_only precision for the output
        elements.push(CqlValue::Time(octofhir_cql_types::CqlTime::hour_only(hour)));
        hour += step_hours;
    }

    Ok(CqlValue::List(CqlList {
        element_type: CqlType::Time,
        elements,
    }))
}

fn expand_interval_list(list: &CqlList, per: Option<&CqlValue>) -> EvalResult<CqlValue> {
    let mut all_intervals: Vec<CqlValue> = Vec::new();

    for item in list.iter() {
        match item {
            CqlValue::Interval(i) => {
                // For list input, use unit intervals function
                if let CqlValue::List(expanded) = expand_interval_to_unit_intervals(i, per)? {
                    all_intervals.extend(expanded.elements);
                }
            }
            CqlValue::Null => continue,
            _ => return Err(EvalError::type_mismatch("Interval", item.get_type().name())),
        }
    }

    Ok(CqlValue::List(CqlList::from_elements(all_intervals)))
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
