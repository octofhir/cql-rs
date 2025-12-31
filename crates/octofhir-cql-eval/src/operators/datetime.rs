//! DateTime Operators for CQL
//!
//! Implements: Date, DateTime, Time constructors, Now, Today, TimeOfDay,
//! DateFrom, TimeFrom, TimezoneFrom, TimezoneOffsetFrom,
//! DateTimeComponentFrom (year, month, day, hour, minute, second, millisecond),
//! DurationBetween, DifferenceBetween, SameAs, SameOrBefore, SameOrAfter

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use octofhir_cql_elm::{
    DateExpression, DateTimeComponentFromExpression, DateTimeExpression, DifferenceBetweenExpression,
    DurationBetweenExpression, SameAsExpression, SameOrAfterExpression, SameOrBeforeExpression,
    TimeExpression, UnaryExpression,
};
use octofhir_cql_types::{CqlDate, CqlDateTime, CqlInterval, CqlTime, CqlType, CqlValue, DateTimePrecision};
use chrono::{Datelike, Timelike};

/// Convert ELM DateTimePrecision to types DateTimePrecision
fn convert_precision(elm_precision: &octofhir_cql_elm::DateTimePrecision) -> DateTimePrecision {
    match elm_precision {
        octofhir_cql_elm::DateTimePrecision::Year => DateTimePrecision::Year,
        octofhir_cql_elm::DateTimePrecision::Month => DateTimePrecision::Month,
        octofhir_cql_elm::DateTimePrecision::Week => DateTimePrecision::Day, // Map week to day
        octofhir_cql_elm::DateTimePrecision::Day => DateTimePrecision::Day,
        octofhir_cql_elm::DateTimePrecision::Hour => DateTimePrecision::Hour,
        octofhir_cql_elm::DateTimePrecision::Minute => DateTimePrecision::Minute,
        octofhir_cql_elm::DateTimePrecision::Second => DateTimePrecision::Second,
        octofhir_cql_elm::DateTimePrecision::Millisecond => DateTimePrecision::Millisecond,
    }
}

impl CqlEngine {
    /// Evaluate Date constructor
    ///
    /// Creates a Date from year, optional month, optional day components.
    pub fn eval_date_constructor(&self, expr: &DateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let year = self.evaluate(&expr.year, ctx)?;

        if year.is_null() {
            return Ok(CqlValue::Null);
        }

        let year_val = match &year {
            CqlValue::Integer(y) => *y,
            _ => return Err(EvalError::type_mismatch("Integer", year.get_type().name())),
        };

        // CQL spec: valid years are 1-9999
        if year_val < 1 || year_val > 9999 {
            return Err(EvalError::overflow(format!(
                "Date constructor: year {} out of range (1-9999)",
                year_val
            )));
        }

        let month_val = if let Some(month_expr) = &expr.month {
            match self.evaluate(month_expr, ctx)? {
                CqlValue::Null => None,
                CqlValue::Integer(m) => Some(m as u8),
                other => return Err(EvalError::type_mismatch("Integer", other.get_type().name())),
            }
        } else {
            None
        };

        let day_val = if let Some(day_expr) = &expr.day {
            match self.evaluate(day_expr, ctx)? {
                CqlValue::Null => None,
                CqlValue::Integer(d) => Some(d as u8),
                other => return Err(EvalError::type_mismatch("Integer", other.get_type().name())),
            }
        } else {
            None
        };

        // Validate the date
        if let Some(m) = month_val {
            if m < 1 || m > 12 {
                return Err(EvalError::InvalidDateTimeComponent {
                    component: "month".to_string(),
                    value: m.to_string(),
                });
            }
        }

        if let Some(d) = day_val {
            let max_day = month_val.map(|m| days_in_month(year_val, m)).unwrap_or(31);
            if d < 1 || d > max_day {
                return Err(EvalError::InvalidDateTimeComponent {
                    component: "day".to_string(),
                    value: d.to_string(),
                });
            }
        }

        Ok(CqlValue::Date(CqlDate {
            year: year_val,
            month: month_val,
            day: day_val,
        }))
    }

    /// Evaluate DateTime constructor
    pub fn eval_datetime_constructor(&self, expr: &DateTimeExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let year = self.evaluate(&expr.year, ctx)?;

        if year.is_null() {
            return Ok(CqlValue::Null);
        }

        let year_val = match &year {
            CqlValue::Integer(y) => *y,
            _ => return Err(EvalError::type_mismatch("Integer", year.get_type().name())),
        };

        // CQL spec: valid years are 1-9999
        if year_val < 1 || year_val > 9999 {
            return Err(EvalError::overflow(format!(
                "DateTime constructor: year {} out of range (1-9999)",
                year_val
            )));
        }

        let month_val = eval_optional_int(&expr.month, self, ctx)?;
        let day_val = eval_optional_int(&expr.day, self, ctx)?;
        let hour_val = eval_optional_int(&expr.hour, self, ctx)?;
        let minute_val = eval_optional_int(&expr.minute, self, ctx)?;
        let second_val = eval_optional_int(&expr.second, self, ctx)?;
        let millisecond_val = eval_optional_int(&expr.millisecond, self, ctx)?;

        // Get timezone offset
        let tz_offset = if let Some(tz_expr) = &expr.timezone_offset {
            match self.evaluate(tz_expr, ctx)? {
                CqlValue::Null => None,
                CqlValue::Decimal(d) => Some((d.to_string().parse::<f64>().unwrap_or(0.0) * 60.0) as i16),
                CqlValue::Integer(i) => Some((i * 60) as i16),
                other => return Err(EvalError::type_mismatch("Decimal", other.get_type().name())),
            }
        } else {
            None
        };

        Ok(CqlValue::DateTime(CqlDateTime {
            year: year_val,
            month: month_val.map(|v| v as u8),
            day: day_val.map(|v| v as u8),
            hour: hour_val.map(|v| v as u8),
            minute: minute_val.map(|v| v as u8),
            second: second_val.map(|v| v as u8),
            millisecond: millisecond_val.map(|v| v as u16),
            timezone_offset: tz_offset,
        }))
    }

    /// Evaluate Time constructor
    pub fn eval_time_constructor(&self, expr: &TimeExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let hour = self.evaluate(&expr.hour, ctx)?;

        if hour.is_null() {
            return Ok(CqlValue::Null);
        }

        let hour_val = match &hour {
            CqlValue::Integer(h) => *h as u8,
            _ => return Err(EvalError::type_mismatch("Integer", hour.get_type().name())),
        };

        let minute_val = eval_optional_int(&expr.minute, self, ctx)?;
        let second_val = eval_optional_int(&expr.second, self, ctx)?;
        let millisecond_val = eval_optional_int(&expr.millisecond, self, ctx)?;

        // Validate time component bounds
        if hour_val > 23 {
            return Err(EvalError::invalid_operand("Time", "hour must be 0-23"));
        }
        if let Some(m) = minute_val {
            if m > 59 {
                return Err(EvalError::invalid_operand("Time", "minute must be 0-59"));
            }
        }
        if let Some(s) = second_val {
            if s > 59 {
                return Err(EvalError::invalid_operand("Time", "second must be 0-59"));
            }
        }
        if let Some(ms) = millisecond_val {
            if ms > 999 {
                return Err(EvalError::invalid_operand("Time", "millisecond must be 0-999"));
            }
        }

        Ok(CqlValue::Time(CqlTime {
            hour: hour_val,
            minute: minute_val.map(|v| v as u8),
            second: second_val.map(|v| v as u8),
            millisecond: millisecond_val.map(|v| v as u16),
        }))
    }

    /// Evaluate DateFrom - extracts date from DateTime
    pub fn eval_date_from(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::DateTime(dt) => Ok(CqlValue::Date(dt.date())),
            CqlValue::Date(d) => Ok(CqlValue::Date(d.clone())),
            _ => Err(EvalError::type_mismatch("DateTime", operand.get_type().name())),
        }
    }

    /// Evaluate TimeFrom - extracts time from DateTime
    pub fn eval_time_from(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::DateTime(dt) => {
                match dt.time() {
                    Some(t) => Ok(CqlValue::Time(t)),
                    None => Ok(CqlValue::Null),
                }
            }
            CqlValue::Time(t) => Ok(CqlValue::Time(t.clone())),
            _ => Err(EvalError::type_mismatch("DateTime", operand.get_type().name())),
        }
    }

    /// Evaluate TimezoneFrom - extracts timezone string from DateTime
    pub fn eval_timezone_from(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::DateTime(dt) => {
                match dt.timezone_offset {
                    Some(offset) => {
                        let hours = offset / 60;
                        let mins = offset.abs() % 60;
                        let sign = if offset >= 0 { "+" } else { "-" };
                        Ok(CqlValue::String(format!("{}{:02}:{:02}", sign, hours.abs(), mins)))
                    }
                    None => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::type_mismatch("DateTime", operand.get_type().name())),
        }
    }

    /// Evaluate TimezoneOffsetFrom - extracts timezone offset as Decimal hours
    pub fn eval_timezone_offset_from(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::DateTime(dt) => {
                match dt.timezone_offset {
                    Some(offset) => {
                        let hours = rust_decimal::Decimal::from(offset) / rust_decimal::Decimal::from(60);
                        Ok(CqlValue::Decimal(hours))
                    }
                    None => Ok(CqlValue::Null),
                }
            }
            _ => Err(EvalError::type_mismatch("DateTime", operand.get_type().name())),
        }
    }

    /// Evaluate DateTimeComponentFrom - extracts a component from Date/DateTime/Time
    pub fn eval_datetime_component_from(
        &self,
        expr: &DateTimeComponentFromExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        if operand.is_null() {
            return Ok(CqlValue::Null);
        }

        let precision = convert_precision(&expr.precision);

        match &operand {
            CqlValue::Date(d) => extract_date_component(d, &precision),
            CqlValue::DateTime(dt) => extract_datetime_component(dt, &precision),
            CqlValue::Time(t) => extract_time_component(t, &precision),
            _ => Err(EvalError::unsupported_operator("DateTimeComponentFrom", operand.get_type().name())),
        }
    }

    /// Evaluate DurationBetween - returns duration in specified precision
    pub fn eval_duration_between(
        &self,
        expr: &DurationBetweenExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::internal("DurationBetween requires 2 operands"));
        }

        let low = self.evaluate(&expr.operand[0], ctx)?;
        let high = self.evaluate(&expr.operand[1], ctx)?;

        if low.is_null() || high.is_null() {
            return Ok(CqlValue::Null);
        }

        // Handle Week precision specially - calculate days and divide by 7
        let is_week = matches!(expr.precision, octofhir_cql_elm::DateTimePrecision::Week);
        let precision = convert_precision(&expr.precision);

        let result = match (&low, &high) {
            (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                duration_between_dates(d1, d2, &precision)?
            }
            (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => {
                duration_between_datetimes(dt1, dt2, &precision)?
            }
            (CqlValue::Time(t1), CqlValue::Time(t2)) => {
                duration_between_times(t1, t2, &precision)?
            }
            _ => return Err(EvalError::unsupported_operator(
                "DurationBetween",
                format!("{}, {}", low.get_type().name(), high.get_type().name()),
            )),
        };

        // For Week precision, divide days by 7
        if is_week {
            match result {
                CqlValue::Integer(days) => Ok(CqlValue::Integer(days / 7)),
                other => Ok(other),
            }
        } else {
            Ok(result)
        }
    }

    /// Evaluate DifferenceBetween - returns whole periods difference
    pub fn eval_difference_between(
        &self,
        expr: &DifferenceBetweenExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::internal("DifferenceBetween requires 2 operands"));
        }

        let low = self.evaluate(&expr.operand[0], ctx)?;
        let high = self.evaluate(&expr.operand[1], ctx)?;

        if low.is_null() || high.is_null() {
            return Ok(CqlValue::Null);
        }

        // Handle Week precision specially - calculate days and divide by 7
        let is_week = matches!(expr.precision, octofhir_cql_elm::DateTimePrecision::Week);
        let precision = convert_precision(&expr.precision);

        // DifferenceBetween is like DurationBetween but truncates to whole periods
        let result = match (&low, &high) {
            (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                difference_between_dates(d1, d2, &precision)?
            }
            (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => {
                difference_between_datetimes(dt1, dt2, &precision)?
            }
            (CqlValue::Time(t1), CqlValue::Time(t2)) => {
                difference_between_times(t1, t2, &precision)?
            }
            _ => return Err(EvalError::unsupported_operator(
                "DifferenceBetween",
                format!("{}, {}", low.get_type().name(), high.get_type().name()),
            )),
        };

        // For Week precision, divide days by 7
        if is_week {
            match result {
                CqlValue::Integer(days) => Ok(CqlValue::Integer(days / 7)),
                other => Ok(other),
            }
        } else {
            Ok(result)
        }
    }

    /// Evaluate SameAs - tests if two dates/times are the same at given precision
    pub fn eval_same_as(&self, expr: &SameAsExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::internal("SameAs requires 2 operands"));
        }

        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        let precision = expr.precision.as_ref().map(convert_precision);

        match (&left, &right) {
            (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                same_as_dates(d1, d2, precision.as_ref())
            }
            (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => {
                same_as_datetimes(dt1, dt2, precision.as_ref())
            }
            (CqlValue::Time(t1), CqlValue::Time(t2)) => {
                same_as_times(t1, t2, precision.as_ref())
            }
            _ => Err(EvalError::unsupported_operator(
                "SameAs",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate SameOrBefore - tests if first is same or before second at precision
    pub fn eval_same_or_before(&self, expr: &SameOrBeforeExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::internal("SameOrBefore requires 2 operands"));
        }

        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        let precision = expr.precision.as_ref().map(convert_precision);

        match (&left, &right) {
            (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                same_or_before_dates(d1, d2, precision.as_ref())
            }
            (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => {
                same_or_before_datetimes(dt1, dt2, precision.as_ref())
            }
            (CqlValue::Time(t1), CqlValue::Time(t2)) => {
                same_or_before_times(t1, t2, precision.as_ref())
            }
            // Interval on or before point: interval.end <= point
            (CqlValue::Interval(interval), point) if !matches!(point, CqlValue::Interval(_)) => {
                if let Some(end) = &interval.high {
                    value_same_or_before(end, point, precision.as_ref())
                } else {
                    // Unbounded high means interval extends to infinity, so never on or before a point
                    Ok(CqlValue::Boolean(false))
                }
            }
            // Point on or before interval: point <= interval.start
            (point, CqlValue::Interval(interval)) if !matches!(point, CqlValue::Interval(_)) => {
                if let Some(start) = &interval.low {
                    value_same_or_before(point, start, precision.as_ref())
                } else {
                    // Unbounded low means interval extends from -infinity, so any point is not <= start
                    Ok(CqlValue::Boolean(false))
                }
            }
            // Interval on or before interval: first.end <= second.start
            (CqlValue::Interval(i1), CqlValue::Interval(i2)) => {
                if let (Some(end1), Some(start2)) = (&i1.high, &i2.low) {
                    value_same_or_before(end1, start2, precision.as_ref())
                } else {
                    Ok(CqlValue::Null)
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "SameOrBefore",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate SameOrAfter - tests if first is same or after second at precision
    pub fn eval_same_or_after(&self, expr: &SameOrAfterExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::internal("SameOrAfter requires 2 operands"));
        }

        let left = self.evaluate(&expr.operand[0], ctx)?;
        let right = self.evaluate(&expr.operand[1], ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        let precision = expr.precision.as_ref().map(convert_precision);

        match (&left, &right) {
            (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                same_or_after_dates(d1, d2, precision.as_ref())
            }
            (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => {
                same_or_after_datetimes(dt1, dt2, precision.as_ref())
            }
            (CqlValue::Time(t1), CqlValue::Time(t2)) => {
                same_or_after_times(t1, t2, precision.as_ref())
            }
            // Interval on or after point: interval.start >= point
            (CqlValue::Interval(interval), point) if !matches!(point, CqlValue::Interval(_)) => {
                if let Some(start) = &interval.low {
                    value_same_or_after(start, point, precision.as_ref())
                } else {
                    // Unbounded low means interval extends from -infinity, so never on or after a point
                    Ok(CqlValue::Boolean(false))
                }
            }
            // Point on or after interval: point >= interval.end
            (point, CqlValue::Interval(interval)) if !matches!(point, CqlValue::Interval(_)) => {
                if let Some(end) = &interval.high {
                    value_same_or_after(point, end, precision.as_ref())
                } else {
                    // Unbounded high means interval extends to infinity, so point is never >= end
                    Ok(CqlValue::Boolean(false))
                }
            }
            // Interval on or after interval: first.start >= second.end
            (CqlValue::Interval(i1), CqlValue::Interval(i2)) => {
                if let (Some(start1), Some(end2)) = (&i1.low, &i2.high) {
                    value_same_or_after(start1, end2, precision.as_ref())
                } else {
                    Ok(CqlValue::Null)
                }
            }
            _ => Err(EvalError::unsupported_operator(
                "SameOrAfter",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate before with precision for temporal types
    pub fn temporal_before_with_precision(
        &self,
        left: &CqlValue,
        right: &CqlValue,
        precision: &octofhir_cql_elm::DateTimePrecision,
    ) -> EvalResult<CqlValue> {
        let precision = convert_precision(precision);

        match (left, right) {
            (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                before_dates_with_precision(d1, d2, &precision)
            }
            (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => {
                before_datetimes_with_precision(dt1, dt2, &precision)
            }
            (CqlValue::Time(t1), CqlValue::Time(t2)) => {
                before_times_with_precision(t1, t2, &precision)
            }
            _ => Err(EvalError::unsupported_operator(
                "Before",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }

    /// Evaluate after with precision for temporal types
    pub fn temporal_after_with_precision(
        &self,
        left: &CqlValue,
        right: &CqlValue,
        precision: &octofhir_cql_elm::DateTimePrecision,
    ) -> EvalResult<CqlValue> {
        let precision = convert_precision(precision);

        match (left, right) {
            (CqlValue::Date(d1), CqlValue::Date(d2)) => {
                after_dates_with_precision(d1, d2, &precision)
            }
            (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => {
                after_datetimes_with_precision(dt1, dt2, &precision)
            }
            (CqlValue::Time(t1), CqlValue::Time(t2)) => {
                after_times_with_precision(t1, t2, &precision)
            }
            _ => Err(EvalError::unsupported_operator(
                "After",
                format!("{}, {}", left.get_type().name(), right.get_type().name()),
            )),
        }
    }
}

// Helper functions

fn eval_optional_int(
    expr: &Option<Box<octofhir_cql_elm::Expression>>,
    engine: &CqlEngine,
    ctx: &mut EvaluationContext,
) -> EvalResult<Option<i32>> {
    if let Some(e) = expr {
        match engine.evaluate(e, ctx)? {
            CqlValue::Null => Ok(None),
            CqlValue::Integer(i) => Ok(Some(i)),
            other => Err(EvalError::type_mismatch("Integer", other.get_type().name())),
        }
    } else {
        Ok(None)
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 31,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn extract_date_component(date: &CqlDate, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    match precision {
        DateTimePrecision::Year => Ok(CqlValue::Integer(date.year)),
        DateTimePrecision::Month => Ok(date.month.map(|m| CqlValue::Integer(m as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Day => Ok(date.day.map(|d| CqlValue::Integer(d as i32)).unwrap_or(CqlValue::Null)),
        _ => Ok(CqlValue::Null), // Hour, Minute, Second, Millisecond not applicable to Date
    }
}

fn extract_datetime_component(dt: &CqlDateTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    match precision {
        DateTimePrecision::Year => Ok(CqlValue::Integer(dt.year)),
        DateTimePrecision::Month => Ok(dt.month.map(|m| CqlValue::Integer(m as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Day => Ok(dt.day.map(|d| CqlValue::Integer(d as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Hour => Ok(dt.hour.map(|h| CqlValue::Integer(h as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Minute => Ok(dt.minute.map(|m| CqlValue::Integer(m as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Second => Ok(dt.second.map(|s| CqlValue::Integer(s as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Millisecond => Ok(dt.millisecond.map(|ms| CqlValue::Integer(ms as i32)).unwrap_or(CqlValue::Null)),
    }
}

fn extract_time_component(time: &CqlTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    match precision {
        DateTimePrecision::Hour => Ok(CqlValue::Integer(time.hour as i32)),
        DateTimePrecision::Minute => Ok(time.minute.map(|m| CqlValue::Integer(m as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Second => Ok(time.second.map(|s| CqlValue::Integer(s as i32)).unwrap_or(CqlValue::Null)),
        DateTimePrecision::Millisecond => Ok(time.millisecond.map(|ms| CqlValue::Integer(ms as i32)).unwrap_or(CqlValue::Null)),
        _ => Ok(CqlValue::Null), // Year, Month, Day not applicable to Time
    }
}

fn duration_between_dates(d1: &CqlDate, d2: &CqlDate, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // Handle different precision levels with partial dates
    match precision {
        DateTimePrecision::Year => {
            // Year difference only needs year components
            let year_diff = d2.year - d1.year;

            // If both have month components, adjust for whether the "anniversary" has passed
            let adjustment = match (d1.month, d2.month) {
                (Some(m1), Some(m2)) => {
                    if m2 < m1 {
                        -1
                    } else if m2 > m1 {
                        0
                    } else {
                        // Same month - check days
                        match (d1.day, d2.day) {
                            (Some(day1), Some(day2)) if day2 < day1 => -1,
                            _ => 0,
                        }
                    }
                }
                _ => 0, // Partial dates - just use year difference
            };

            let result = if year_diff >= 0 {
                year_diff + adjustment
            } else {
                // Negative direction - flip the adjustment
                year_diff - adjustment
            };
            Ok(CqlValue::Integer(result))
        }
        DateTimePrecision::Month => {
            // Month difference needs year and month
            match (d1.month, d2.month) {
                (Some(m1), Some(m2)) => {
                    let years = d2.year - d1.year;
                    let months = m2 as i32 - m1 as i32;
                    let base_diff = years * 12 + months;

                    // Adjust for days if both have day components
                    let adjustment = match (d1.day, d2.day) {
                        (Some(day1), Some(day2)) if day2 < day1 => {
                            if base_diff >= 0 { -1 } else { 1 }
                        }
                        _ => 0,
                    };

                    Ok(CqlValue::Integer(base_diff + adjustment))
                }
                _ => Ok(CqlValue::Null), // Can't calculate months without month component
            }
        }
        DateTimePrecision::Day => {
            // Day precision requires full dates
            let date1 = d1.to_naive_date();
            let date2 = d2.to_naive_date();
            match (date1, date2) {
                (Some(nd1), Some(nd2)) => {
                    let days = nd2.signed_duration_since(nd1).num_days() as i32;
                    Ok(CqlValue::Integer(days))
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => {
            // Sub-day precision requires full dates
            let date1 = d1.to_naive_date();
            let date2 = d2.to_naive_date();
            match (date1, date2) {
                (Some(nd1), Some(nd2)) => {
                    let duration = nd2.signed_duration_since(nd1);
                    let result = match precision {
                        DateTimePrecision::Hour => duration.num_hours() as i32,
                        DateTimePrecision::Minute => duration.num_minutes() as i32,
                        DateTimePrecision::Second => duration.num_seconds() as i32,
                        DateTimePrecision::Millisecond => duration.num_milliseconds() as i32,
                        _ => 0,
                    };
                    Ok(CqlValue::Integer(result))
                }
                _ => Ok(CqlValue::Null),
            }
        }
    }
}

fn duration_between_datetimes(dt1: &CqlDateTime, dt2: &CqlDateTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // Check if either DateTime has uncertainty that affects the precision we're calculating
    let dt1_has_uncertainty = has_uncertainty_at_precision(dt1, precision);
    let dt2_has_uncertainty = has_uncertainty_at_precision(dt2, precision);

    if dt1_has_uncertainty || dt2_has_uncertainty {
        // Calculate duration using boundaries to get an interval
        // Min duration: later start (high boundary of dt1) to earlier end (low boundary of dt2)
        // Max duration: earlier start (low boundary of dt1) to later end (high boundary of dt2)
        let min_duration = calculate_single_duration(
            &dt1.high_boundary(),
            &dt2.low_boundary(),
            precision,
        )?;
        let max_duration = calculate_single_duration(
            &dt1.low_boundary(),
            &dt2.high_boundary(),
            precision,
        )?;

        match (min_duration, max_duration) {
            (CqlValue::Integer(min), CqlValue::Integer(max)) => {
                Ok(CqlValue::Interval(CqlInterval::new(
                    CqlType::Integer,
                    Some(CqlValue::Integer(min)),
                    true,
                    Some(CqlValue::Integer(max)),
                    true,
                )))
            }
            _ => Ok(CqlValue::Null),
        }
    } else {
        // No uncertainty - calculate single duration value
        calculate_single_duration(dt1, dt2, precision)
    }
}

/// Check if a DateTime has uncertainty that affects the given precision
fn has_uncertainty_at_precision(dt: &CqlDateTime, precision: &DateTimePrecision) -> bool {
    match precision {
        // For year precision, we need month to know if the "anniversary" has passed
        DateTimePrecision::Year => dt.month.is_none(),
        // For month precision, we need day to know if the "month anniversary" has passed
        DateTimePrecision::Month => dt.day.is_none(),
        DateTimePrecision::Day => dt.month.is_none() || dt.day.is_none(),
        DateTimePrecision::Hour => dt.month.is_none() || dt.day.is_none() || dt.hour.is_none(),
        DateTimePrecision::Minute => {
            dt.month.is_none() || dt.day.is_none() || dt.hour.is_none() || dt.minute.is_none()
        }
        DateTimePrecision::Second => {
            dt.month.is_none()
                || dt.day.is_none()
                || dt.hour.is_none()
                || dt.minute.is_none()
                || dt.second.is_none()
        }
        DateTimePrecision::Millisecond => {
            dt.month.is_none()
                || dt.day.is_none()
                || dt.hour.is_none()
                || dt.minute.is_none()
                || dt.second.is_none()
                || dt.millisecond.is_none()
        }
    }
}

/// Check if a DateTime has uncertainty for DifferenceBetween at the given precision
/// Unlike Duration, Difference just counts boundaries and only needs the precision component itself
fn has_uncertainty_for_difference(dt: &CqlDateTime, precision: &DateTimePrecision) -> bool {
    match precision {
        // Year is always present, so no uncertainty
        DateTimePrecision::Year => false,
        // Month precision: uncertain if month is missing
        DateTimePrecision::Month => dt.month.is_none(),
        // Day precision: uncertain if month or day is missing
        DateTimePrecision::Day => dt.month.is_none() || dt.day.is_none(),
        // Hour precision: uncertain if any date/time component is missing
        DateTimePrecision::Hour => dt.month.is_none() || dt.day.is_none() || dt.hour.is_none(),
        DateTimePrecision::Minute => {
            dt.month.is_none() || dt.day.is_none() || dt.hour.is_none() || dt.minute.is_none()
        }
        DateTimePrecision::Second => {
            dt.month.is_none()
                || dt.day.is_none()
                || dt.hour.is_none()
                || dt.minute.is_none()
                || dt.second.is_none()
        }
        DateTimePrecision::Millisecond => {
            dt.month.is_none()
                || dt.day.is_none()
                || dt.hour.is_none()
                || dt.minute.is_none()
                || dt.second.is_none()
                || dt.millisecond.is_none()
        }
    }
}

/// Calculate a single duration value between two fully specified DateTimes
fn calculate_single_duration(
    dt1: &CqlDateTime,
    dt2: &CqlDateTime,
    precision: &DateTimePrecision,
) -> EvalResult<CqlValue> {
    // For Day precision and finer, we need to account for timezone offsets
    // to compute actual elapsed time, not just calendar differences
    match precision {
        DateTimePrecision::Day | DateTimePrecision::Hour | DateTimePrecision::Minute |
        DateTimePrecision::Second | DateTimePrecision::Millisecond => {
            // Need to create full datetime for proper calculation
            // IMPORTANT: Apply timezone offset to get UTC time for accurate comparison
            let naive_dt1 = chrono::NaiveDate::from_ymd_opt(
                dt1.year,
                dt1.month.unwrap_or(1) as u32,
                dt1.day.unwrap_or(1) as u32,
            )
            .and_then(|d| {
                chrono::NaiveTime::from_hms_milli_opt(
                    dt1.hour.unwrap_or(0) as u32,
                    dt1.minute.unwrap_or(0) as u32,
                    dt1.second.unwrap_or(0) as u32,
                    dt1.millisecond.unwrap_or(0) as u32,
                )
                .map(|t| chrono::NaiveDateTime::new(d, t))
            });

            let naive_dt2 = chrono::NaiveDate::from_ymd_opt(
                dt2.year,
                dt2.month.unwrap_or(1) as u32,
                dt2.day.unwrap_or(1) as u32,
            )
            .and_then(|d| {
                chrono::NaiveTime::from_hms_milli_opt(
                    dt2.hour.unwrap_or(0) as u32,
                    dt2.minute.unwrap_or(0) as u32,
                    dt2.second.unwrap_or(0) as u32,
                    dt2.millisecond.unwrap_or(0) as u32,
                )
                .map(|t| chrono::NaiveDateTime::new(d, t))
            });

            match (naive_dt1, naive_dt2) {
                (Some(ndt1), Some(ndt2)) => {
                    // Apply timezone offsets to normalize to UTC
                    // Timezone offset is in minutes, stored as i16
                    // To convert local to UTC: subtract offset
                    let offset1_minutes = dt1.timezone_offset.unwrap_or(0) as i64;
                    let offset2_minutes = dt2.timezone_offset.unwrap_or(0) as i64;

                    // Convert offsets to chrono Duration
                    let utc_dt1 = ndt1 - chrono::TimeDelta::minutes(offset1_minutes);
                    let utc_dt2 = ndt2 - chrono::TimeDelta::minutes(offset2_minutes);

                    let duration = utc_dt2.signed_duration_since(utc_dt1);
                    let result = match precision {
                        DateTimePrecision::Day => duration.num_days() as i32,
                        DateTimePrecision::Hour => duration.num_hours() as i32,
                        DateTimePrecision::Minute => duration.num_minutes() as i32,
                        DateTimePrecision::Second => duration.num_seconds() as i32,
                        DateTimePrecision::Millisecond => duration.num_milliseconds() as i32,
                        _ => 0,
                    };
                    Ok(CqlValue::Integer(result))
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => {
            // For month-level precision and above, use date-only comparison
            let d1 = dt1.date();
            let d2 = dt2.date();
            duration_between_dates(&d1, &d2, precision)
        }
    }
}

fn duration_between_times(t1: &CqlTime, t2: &CqlTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    let ms1 = t1.to_milliseconds().unwrap_or(0);
    let ms2 = t2.to_milliseconds().unwrap_or(0);
    let diff_ms = ms2 as i64 - ms1 as i64;

    let result = match precision {
        DateTimePrecision::Hour => (diff_ms / 3_600_000) as i32,
        DateTimePrecision::Minute => (diff_ms / 60_000) as i32,
        DateTimePrecision::Second => (diff_ms / 1_000) as i32,
        DateTimePrecision::Millisecond => diff_ms as i32,
        _ => return Ok(CqlValue::Null),
    };

    Ok(CqlValue::Integer(result))
}

fn difference_between_dates(d1: &CqlDate, d2: &CqlDate, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // DifferenceBetween is similar to DurationBetween for dates
    duration_between_dates(d1, d2, precision)
}

fn difference_between_datetimes(dt1: &CqlDateTime, dt2: &CqlDateTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // Check if either DateTime has uncertainty that affects the precision we're calculating
    // For DifferenceBetween, uncertainty occurs when the required precision component is missing
    // (unlike Duration, Difference just counts boundaries and doesn't need finer precision)
    let dt1_has_uncertainty = has_uncertainty_for_difference(dt1, precision);
    let dt2_has_uncertainty = has_uncertainty_for_difference(dt2, precision);

    if dt1_has_uncertainty || dt2_has_uncertainty {
        // Calculate difference using boundaries to get an interval
        // Min difference: later start (high boundary of dt1) to earlier end (low boundary of dt2)
        // Max difference: earlier start (low boundary of dt1) to later end (high boundary of dt2)
        let min_diff = calculate_single_difference(
            &dt1.high_boundary(),
            &dt2.low_boundary(),
            precision,
        )?;
        let max_diff = calculate_single_difference(
            &dt1.low_boundary(),
            &dt2.high_boundary(),
            precision,
        )?;

        match (min_diff, max_diff) {
            (CqlValue::Integer(min), CqlValue::Integer(max)) => {
                Ok(CqlValue::Interval(CqlInterval::new(
                    CqlType::Integer,
                    Some(CqlValue::Integer(min)),
                    true,
                    Some(CqlValue::Integer(max)),
                    true,
                )))
            }
            _ => Ok(CqlValue::Null),
        }
    } else {
        // No uncertainty - calculate single difference value
        calculate_single_difference(dt1, dt2, precision)
    }
}

/// Calculate a single difference value between two fully specified DateTimes
fn calculate_single_difference(
    dt1: &CqlDateTime,
    dt2: &CqlDateTime,
    precision: &DateTimePrecision,
) -> EvalResult<CqlValue> {
    // DifferenceBetween counts the number of boundaries crossed (not complete periods)
    match precision {
        // Year difference: just count year boundaries (year2 - year1)
        DateTimePrecision::Year => {
            Ok(CqlValue::Integer(dt2.year - dt1.year))
        }
        // Month difference: count month boundaries
        DateTimePrecision::Month => {
            let m1 = dt1.month.unwrap_or(1) as i32;
            let m2 = dt2.month.unwrap_or(1) as i32;
            let months = (dt2.year - dt1.year) * 12 + (m2 - m1);
            Ok(CqlValue::Integer(months))
        }
        DateTimePrecision::Day | DateTimePrecision::Hour | DateTimePrecision::Minute |
        DateTimePrecision::Second | DateTimePrecision::Millisecond => {
            // Normalize to UTC first
            let utc1 = normalize_datetime_to_utc(dt1);
            let utc2 = normalize_datetime_to_utc(dt2);

            // For DifferenceBetween, compute calendar differences (not elapsed time)
            match precision {
                DateTimePrecision::Day => {
                    // Count day boundaries crossed
                    let d1 = utc1.date();
                    let d2 = utc2.date();
                    duration_between_dates(&d1, &d2, precision)
                }
                DateTimePrecision::Hour | DateTimePrecision::Minute |
                DateTimePrecision::Second | DateTimePrecision::Millisecond => {
                    // For sub-day precision, use the UTC-normalized elapsed time
                    calculate_single_duration(&utc1, &utc2, precision)
                }
                _ => Ok(CqlValue::Null),
            }
        }
    }
}

/// Normalize a CqlDateTime to UTC by applying timezone offset
fn normalize_datetime_to_utc(dt: &CqlDateTime) -> CqlDateTime {
    let offset_minutes = dt.timezone_offset.unwrap_or(0);
    if offset_minutes == 0 {
        return dt.clone();
    }

    // Create a NaiveDateTime and apply offset
    let naive = chrono::NaiveDate::from_ymd_opt(
        dt.year,
        dt.month.unwrap_or(1) as u32,
        dt.day.unwrap_or(1) as u32,
    )
    .and_then(|d| {
        chrono::NaiveTime::from_hms_milli_opt(
            dt.hour.unwrap_or(0) as u32,
            dt.minute.unwrap_or(0) as u32,
            dt.second.unwrap_or(0) as u32,
            dt.millisecond.unwrap_or(0) as u32,
        )
        .map(|t| chrono::NaiveDateTime::new(d, t))
    });

    match naive {
        Some(ndt) => {
            let utc = ndt - chrono::TimeDelta::minutes(offset_minutes as i64);
            CqlDateTime {
                year: utc.date().year(),
                month: Some(utc.date().month() as u8),
                day: Some(utc.date().day() as u8),
                hour: Some(utc.time().hour() as u8),
                minute: Some(utc.time().minute() as u8),
                second: Some(utc.time().second() as u8),
                millisecond: Some((utc.time().nanosecond() / 1_000_000) as u16),
                timezone_offset: Some(0), // UTC
            }
        }
        None => dt.clone(),
    }
}

fn difference_between_times(t1: &CqlTime, t2: &CqlTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    duration_between_times(t1, t2, precision)
}

fn same_as_dates(d1: &CqlDate, d2: &CqlDate, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let default_precision = d1.precision().min(d2.precision());
    let prec = precision.unwrap_or(&default_precision);

    match prec {
        DateTimePrecision::Year => Ok(CqlValue::Boolean(d1.year == d2.year)),
        DateTimePrecision::Month => {
            match (d1.month, d2.month) {
                (Some(m1), Some(m2)) => Ok(CqlValue::Boolean(d1.year == d2.year && m1 == m2)),
                _ => Ok(CqlValue::Null),
            }
        }
        DateTimePrecision::Day => {
            match ((d1.month, d1.day), (d2.month, d2.day)) {
                ((Some(m1), Some(day1)), (Some(m2), Some(day2))) => {
                    Ok(CqlValue::Boolean(d1.year == d2.year && m1 == m2 && day1 == day2))
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

/// Normalize a DateTime to UTC by applying timezone offset
fn normalize_to_utc(dt: &CqlDateTime) -> CqlDateTime {
    let offset = dt.timezone_offset.unwrap_or(0);
    if offset == 0 {
        return dt.clone();
    }

    // Convert datetime components to total minutes from year start
    let mut total_minutes: i64 = 0;
    total_minutes += dt.hour.unwrap_or(0) as i64 * 60;
    total_minutes += dt.minute.unwrap_or(0) as i64;
    total_minutes -= offset as i64; // Subtract offset to get UTC

    // Handle overflow/underflow across days
    let mut day = dt.day.unwrap_or(1) as i32;
    let mut month = dt.month.unwrap_or(1) as i32;
    let mut year = dt.year;
    let mut hour = (total_minutes / 60) % 24;
    let mut minute = total_minutes % 60;

    if total_minutes < 0 {
        // Went back to previous day
        hour = 24 + (total_minutes / 60);
        minute = (60 + (total_minutes % 60)) % 60;
        day -= 1;
        if day < 1 {
            month -= 1;
            if month < 1 {
                month = 12;
                year -= 1;
            }
            day = days_in_month(year, month as u8) as i32;
        }
    } else if hour >= 24 {
        // Went to next day
        hour %= 24;
        day += 1;
        let days = days_in_month(year, month as u8) as i32;
        if day > days {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }

    CqlDateTime {
        year,
        month: Some(month as u8),
        day: Some(day as u8),
        hour: Some(hour as u8),
        minute: Some(minute as u8),
        second: dt.second,
        millisecond: dt.millisecond,
        timezone_offset: Some(0), // Now in UTC
    }
}

fn same_as_datetimes(dt1: &CqlDateTime, dt2: &CqlDateTime, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let default_precision = dt1.precision().min(dt2.precision());
    let prec = precision.unwrap_or(&default_precision);

    // Normalize to UTC if both have timezone offsets and we're comparing at hour precision or finer
    let (dt1, dt2) = if *prec >= DateTimePrecision::Hour
        && dt1.timezone_offset.is_some()
        && dt2.timezone_offset.is_some()
    {
        (normalize_to_utc(dt1), normalize_to_utc(dt2))
    } else {
        (dt1.clone(), dt2.clone())
    };

    // Compare up to the specified precision
    if dt1.year != dt2.year {
        return Ok(CqlValue::Boolean(false));
    }
    if *prec == DateTimePrecision::Year {
        return Ok(CqlValue::Boolean(true));
    }

    match (dt1.month, dt2.month) {
        (Some(m1), Some(m2)) if m1 != m2 => return Ok(CqlValue::Boolean(false)),
        (None, _) | (_, None) => return Ok(CqlValue::Null),
        _ => {}
    }
    if *prec == DateTimePrecision::Month {
        return Ok(CqlValue::Boolean(true));
    }

    match (dt1.day, dt2.day) {
        (Some(d1), Some(d2)) if d1 != d2 => return Ok(CqlValue::Boolean(false)),
        (None, _) | (_, None) => return Ok(CqlValue::Null),
        _ => {}
    }
    if *prec == DateTimePrecision::Day {
        return Ok(CqlValue::Boolean(true));
    }

    match (dt1.hour, dt2.hour) {
        (Some(h1), Some(h2)) if h1 != h2 => return Ok(CqlValue::Boolean(false)),
        (None, _) | (_, None) => return Ok(CqlValue::Null),
        _ => {}
    }
    if *prec == DateTimePrecision::Hour {
        return Ok(CqlValue::Boolean(true));
    }

    match (dt1.minute, dt2.minute) {
        (Some(m1), Some(m2)) if m1 != m2 => return Ok(CqlValue::Boolean(false)),
        (None, _) | (_, None) => return Ok(CqlValue::Null),
        _ => {}
    }
    if *prec == DateTimePrecision::Minute {
        return Ok(CqlValue::Boolean(true));
    }

    match (dt1.second, dt2.second) {
        (Some(s1), Some(s2)) if s1 != s2 => return Ok(CqlValue::Boolean(false)),
        (None, _) | (_, None) => return Ok(CqlValue::Null),
        _ => {}
    }
    if *prec == DateTimePrecision::Second {
        return Ok(CqlValue::Boolean(true));
    }

    match (dt1.millisecond, dt2.millisecond) {
        (Some(ms1), Some(ms2)) => Ok(CqlValue::Boolean(ms1 == ms2)),
        _ => Ok(CqlValue::Null),
    }
}

fn same_as_times(t1: &CqlTime, t2: &CqlTime, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let default_precision = t1.precision().min(t2.precision());
    let prec = precision.unwrap_or(&default_precision);

    if t1.hour != t2.hour {
        return Ok(CqlValue::Boolean(false));
    }
    if *prec == DateTimePrecision::Hour {
        return Ok(CqlValue::Boolean(true));
    }

    match (t1.minute, t2.minute) {
        (Some(m1), Some(m2)) if m1 != m2 => return Ok(CqlValue::Boolean(false)),
        (None, _) | (_, None) => return Ok(CqlValue::Null),
        _ => {}
    }
    if *prec == DateTimePrecision::Minute {
        return Ok(CqlValue::Boolean(true));
    }

    match (t1.second, t2.second) {
        (Some(s1), Some(s2)) if s1 != s2 => return Ok(CqlValue::Boolean(false)),
        (None, _) | (_, None) => return Ok(CqlValue::Null),
        _ => {}
    }
    if *prec == DateTimePrecision::Second {
        return Ok(CqlValue::Boolean(true));
    }

    match (t1.millisecond, t2.millisecond) {
        (Some(ms1), Some(ms2)) => Ok(CqlValue::Boolean(ms1 == ms2)),
        _ => Ok(CqlValue::Null),
    }
}

fn same_or_before_dates(d1: &CqlDate, d2: &CqlDate, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let same = same_as_dates(d1, d2, precision)?;
    if let CqlValue::Boolean(true) = same {
        return Ok(CqlValue::Boolean(true));
    }

    // Check if d1 < d2 at precision
    match d1.partial_cmp(d2) {
        Some(std::cmp::Ordering::Less) => Ok(CqlValue::Boolean(true)),
        Some(std::cmp::Ordering::Greater) => Ok(CqlValue::Boolean(false)),
        _ => Ok(same), // Equal or uncertain
    }
}

fn same_or_before_datetimes(dt1: &CqlDateTime, dt2: &CqlDateTime, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let same = same_as_datetimes(dt1, dt2, precision)?;
    if let CqlValue::Boolean(true) = same {
        return Ok(CqlValue::Boolean(true));
    }

    // Normalize to UTC if both have timezone offsets
    let (dt1_norm, dt2_norm) = if dt1.timezone_offset.is_some() && dt2.timezone_offset.is_some() {
        (normalize_to_utc(dt1), normalize_to_utc(dt2))
    } else {
        (dt1.clone(), dt2.clone())
    };

    // Compare the normalized datetimes
    match datetime_cmp(&dt1_norm, &dt2_norm) {
        Some(std::cmp::Ordering::Less) => Ok(CqlValue::Boolean(true)),
        Some(std::cmp::Ordering::Greater) => Ok(CqlValue::Boolean(false)),
        _ => Ok(same),
    }
}

/// Compare two datetimes component by component
fn datetime_cmp(dt1: &CqlDateTime, dt2: &CqlDateTime) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;

    match dt1.year.cmp(&dt2.year) {
        Ordering::Equal => {}
        other => return Some(other),
    }

    match (dt1.month, dt2.month) {
        (Some(m1), Some(m2)) => match m1.cmp(&m2) {
            Ordering::Equal => {}
            other => return Some(other),
        },
        _ => return None,
    }

    match (dt1.day, dt2.day) {
        (Some(d1), Some(d2)) => match d1.cmp(&d2) {
            Ordering::Equal => {}
            other => return Some(other),
        },
        _ => return None,
    }

    match (dt1.hour, dt2.hour) {
        (Some(h1), Some(h2)) => match h1.cmp(&h2) {
            Ordering::Equal => {}
            other => return Some(other),
        },
        _ => return None,
    }

    match (dt1.minute, dt2.minute) {
        (Some(m1), Some(m2)) => match m1.cmp(&m2) {
            Ordering::Equal => {}
            other => return Some(other),
        },
        _ => return None,
    }

    match (dt1.second, dt2.second) {
        (Some(s1), Some(s2)) => match s1.cmp(&s2) {
            Ordering::Equal => {}
            other => return Some(other),
        },
        _ => return None,
    }

    match (dt1.millisecond, dt2.millisecond) {
        (Some(ms1), Some(ms2)) => Some(ms1.cmp(&ms2)),
        _ => None,
    }
}

fn same_or_before_times(t1: &CqlTime, t2: &CqlTime, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let same = same_as_times(t1, t2, precision)?;
    if let CqlValue::Boolean(true) = same {
        return Ok(CqlValue::Boolean(true));
    }

    match t1.partial_cmp(t2) {
        Some(std::cmp::Ordering::Less) => Ok(CqlValue::Boolean(true)),
        Some(std::cmp::Ordering::Greater) => Ok(CqlValue::Boolean(false)),
        _ => Ok(same),
    }
}

fn same_or_after_dates(d1: &CqlDate, d2: &CqlDate, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let same = same_as_dates(d1, d2, precision)?;
    if let CqlValue::Boolean(true) = same {
        return Ok(CqlValue::Boolean(true));
    }

    match d1.partial_cmp(d2) {
        Some(std::cmp::Ordering::Greater) => Ok(CqlValue::Boolean(true)),
        Some(std::cmp::Ordering::Less) => Ok(CqlValue::Boolean(false)),
        _ => Ok(same),
    }
}

fn same_or_after_datetimes(dt1: &CqlDateTime, dt2: &CqlDateTime, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let same = same_as_datetimes(dt1, dt2, precision)?;
    if let CqlValue::Boolean(true) = same {
        return Ok(CqlValue::Boolean(true));
    }

    // Normalize to UTC if both have timezone offsets
    let (dt1_norm, dt2_norm) = if dt1.timezone_offset.is_some() && dt2.timezone_offset.is_some() {
        (normalize_to_utc(dt1), normalize_to_utc(dt2))
    } else {
        (dt1.clone(), dt2.clone())
    };

    // Compare the normalized datetimes
    match datetime_cmp(&dt1_norm, &dt2_norm) {
        Some(std::cmp::Ordering::Greater) => Ok(CqlValue::Boolean(true)),
        Some(std::cmp::Ordering::Less) => Ok(CqlValue::Boolean(false)),
        _ => Ok(same),
    }
}

fn same_or_after_times(t1: &CqlTime, t2: &CqlTime, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    let same = same_as_times(t1, t2, precision)?;
    if let CqlValue::Boolean(true) = same {
        return Ok(CqlValue::Boolean(true));
    }

    match t1.partial_cmp(t2) {
        Some(std::cmp::Ordering::Greater) => Ok(CqlValue::Boolean(true)),
        Some(std::cmp::Ordering::Less) => Ok(CqlValue::Boolean(false)),
        _ => Ok(same),
    }
}

// Before with precision helper functions

fn before_dates_with_precision(d1: &CqlDate, d2: &CqlDate, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    match precision {
        DateTimePrecision::Year => Ok(CqlValue::Boolean(d1.year < d2.year)),
        DateTimePrecision::Month => {
            match (d1.month, d2.month) {
                (Some(m1), Some(m2)) => {
                    if d1.year < d2.year {
                        Ok(CqlValue::Boolean(true))
                    } else if d1.year > d2.year {
                        Ok(CqlValue::Boolean(false))
                    } else {
                        Ok(CqlValue::Boolean(m1 < m2))
                    }
                }
                _ => Ok(CqlValue::Null),
            }
        }
        DateTimePrecision::Day => {
            match ((d1.month, d1.day), (d2.month, d2.day)) {
                ((Some(m1), Some(day1)), (Some(m2), Some(day2))) => {
                    if d1.year < d2.year {
                        Ok(CqlValue::Boolean(true))
                    } else if d1.year > d2.year {
                        Ok(CqlValue::Boolean(false))
                    } else if m1 < m2 {
                        Ok(CqlValue::Boolean(true))
                    } else if m1 > m2 {
                        Ok(CqlValue::Boolean(false))
                    } else {
                        Ok(CqlValue::Boolean(day1 < day2))
                    }
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

fn before_datetimes_with_precision(dt1: &CqlDateTime, dt2: &CqlDateTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // Normalize to UTC if both have timezone offsets and we're comparing at hour precision or finer
    let (dt1, dt2) = if *precision >= DateTimePrecision::Hour
        && dt1.timezone_offset.is_some()
        && dt2.timezone_offset.is_some()
    {
        (normalize_to_utc(dt1), normalize_to_utc(dt2))
    } else {
        (dt1.clone(), dt2.clone())
    };

    // Compare year
    if dt1.year < dt2.year {
        return Ok(CqlValue::Boolean(true));
    }
    if dt1.year > dt2.year {
        return Ok(CqlValue::Boolean(false));
    }
    if *precision == DateTimePrecision::Year {
        return Ok(CqlValue::Boolean(false)); // Same year, not before
    }

    // Compare month
    match (dt1.month, dt2.month) {
        (Some(m1), Some(m2)) => {
            if m1 < m2 {
                return Ok(CqlValue::Boolean(true));
            }
            if m1 > m2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Month {
        return Ok(CqlValue::Boolean(false)); // Same month, not before
    }

    // Compare day
    match (dt1.day, dt2.day) {
        (Some(d1), Some(d2)) => {
            if d1 < d2 {
                return Ok(CqlValue::Boolean(true));
            }
            if d1 > d2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Day {
        return Ok(CqlValue::Boolean(false)); // Same day, not before
    }

    // Compare hour
    match (dt1.hour, dt2.hour) {
        (Some(h1), Some(h2)) => {
            if h1 < h2 {
                return Ok(CqlValue::Boolean(true));
            }
            if h1 > h2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Hour {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare minute
    match (dt1.minute, dt2.minute) {
        (Some(min1), Some(min2)) => {
            if min1 < min2 {
                return Ok(CqlValue::Boolean(true));
            }
            if min1 > min2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Minute {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare second
    match (dt1.second, dt2.second) {
        (Some(s1), Some(s2)) => {
            if s1 < s2 {
                return Ok(CqlValue::Boolean(true));
            }
            if s1 > s2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Second {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare millisecond
    match (dt1.millisecond, dt2.millisecond) {
        (Some(ms1), Some(ms2)) => Ok(CqlValue::Boolean(ms1 < ms2)),
        _ => Ok(CqlValue::Null),
    }
}

fn before_times_with_precision(t1: &CqlTime, t2: &CqlTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // Compare hour
    if t1.hour < t2.hour {
        return Ok(CqlValue::Boolean(true));
    }
    if t1.hour > t2.hour {
        return Ok(CqlValue::Boolean(false));
    }
    if *precision == DateTimePrecision::Hour {
        return Ok(CqlValue::Boolean(false)); // Same hour, not before
    }

    // Compare minute
    match (t1.minute, t2.minute) {
        (Some(min1), Some(min2)) => {
            if min1 < min2 {
                return Ok(CqlValue::Boolean(true));
            }
            if min1 > min2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Minute {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare second
    match (t1.second, t2.second) {
        (Some(s1), Some(s2)) => {
            if s1 < s2 {
                return Ok(CqlValue::Boolean(true));
            }
            if s1 > s2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Second {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare millisecond
    match (t1.millisecond, t2.millisecond) {
        (Some(ms1), Some(ms2)) => Ok(CqlValue::Boolean(ms1 < ms2)),
        _ => Ok(CqlValue::Null),
    }
}

// After with precision helper functions

fn after_dates_with_precision(d1: &CqlDate, d2: &CqlDate, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    match precision {
        DateTimePrecision::Year => Ok(CqlValue::Boolean(d1.year > d2.year)),
        DateTimePrecision::Month => {
            match (d1.month, d2.month) {
                (Some(m1), Some(m2)) => {
                    if d1.year > d2.year {
                        Ok(CqlValue::Boolean(true))
                    } else if d1.year < d2.year {
                        Ok(CqlValue::Boolean(false))
                    } else {
                        Ok(CqlValue::Boolean(m1 > m2))
                    }
                }
                _ => Ok(CqlValue::Null),
            }
        }
        DateTimePrecision::Day => {
            match ((d1.month, d1.day), (d2.month, d2.day)) {
                ((Some(m1), Some(day1)), (Some(m2), Some(day2))) => {
                    if d1.year > d2.year {
                        Ok(CqlValue::Boolean(true))
                    } else if d1.year < d2.year {
                        Ok(CqlValue::Boolean(false))
                    } else if m1 > m2 {
                        Ok(CqlValue::Boolean(true))
                    } else if m1 < m2 {
                        Ok(CqlValue::Boolean(false))
                    } else {
                        Ok(CqlValue::Boolean(day1 > day2))
                    }
                }
                _ => Ok(CqlValue::Null),
            }
        }
        _ => Ok(CqlValue::Null),
    }
}

fn after_datetimes_with_precision(dt1: &CqlDateTime, dt2: &CqlDateTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // Compare year
    if dt1.year > dt2.year {
        return Ok(CqlValue::Boolean(true));
    }
    if dt1.year < dt2.year {
        return Ok(CqlValue::Boolean(false));
    }
    if *precision == DateTimePrecision::Year {
        return Ok(CqlValue::Boolean(false)); // Same year, not after
    }

    // Compare month
    match (dt1.month, dt2.month) {
        (Some(m1), Some(m2)) => {
            if m1 > m2 {
                return Ok(CqlValue::Boolean(true));
            }
            if m1 < m2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Month {
        return Ok(CqlValue::Boolean(false)); // Same month, not after
    }

    // Compare day
    match (dt1.day, dt2.day) {
        (Some(d1), Some(d2)) => {
            if d1 > d2 {
                return Ok(CqlValue::Boolean(true));
            }
            if d1 < d2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Day {
        return Ok(CqlValue::Boolean(false)); // Same day, not after
    }

    // Compare hour
    match (dt1.hour, dt2.hour) {
        (Some(h1), Some(h2)) => {
            if h1 > h2 {
                return Ok(CqlValue::Boolean(true));
            }
            if h1 < h2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Hour {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare minute
    match (dt1.minute, dt2.minute) {
        (Some(min1), Some(min2)) => {
            if min1 > min2 {
                return Ok(CqlValue::Boolean(true));
            }
            if min1 < min2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Minute {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare second
    match (dt1.second, dt2.second) {
        (Some(s1), Some(s2)) => {
            if s1 > s2 {
                return Ok(CqlValue::Boolean(true));
            }
            if s1 < s2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Second {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare millisecond
    match (dt1.millisecond, dt2.millisecond) {
        (Some(ms1), Some(ms2)) => Ok(CqlValue::Boolean(ms1 > ms2)),
        _ => Ok(CqlValue::Null),
    }
}

fn after_times_with_precision(t1: &CqlTime, t2: &CqlTime, precision: &DateTimePrecision) -> EvalResult<CqlValue> {
    // Compare hour
    if t1.hour > t2.hour {
        return Ok(CqlValue::Boolean(true));
    }
    if t1.hour < t2.hour {
        return Ok(CqlValue::Boolean(false));
    }
    if *precision == DateTimePrecision::Hour {
        return Ok(CqlValue::Boolean(false)); // Same hour, not after
    }

    // Compare minute
    match (t1.minute, t2.minute) {
        (Some(min1), Some(min2)) => {
            if min1 > min2 {
                return Ok(CqlValue::Boolean(true));
            }
            if min1 < min2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Minute {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare second
    match (t1.second, t2.second) {
        (Some(s1), Some(s2)) => {
            if s1 > s2 {
                return Ok(CqlValue::Boolean(true));
            }
            if s1 < s2 {
                return Ok(CqlValue::Boolean(false));
            }
        }
        _ => return Ok(CqlValue::Null),
    }
    if *precision == DateTimePrecision::Second {
        return Ok(CqlValue::Boolean(false));
    }

    // Compare millisecond
    match (t1.millisecond, t2.millisecond) {
        (Some(ms1), Some(ms2)) => Ok(CqlValue::Boolean(ms1 > ms2)),
        _ => Ok(CqlValue::Null),
    }
}

/// Generic value comparison helper for same_or_before operations
/// Supports Integer, Decimal, Quantity, Date, DateTime, Time
fn value_same_or_before(left: &CqlValue, right: &CqlValue, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    match (left, right) {
        (CqlValue::Integer(l), CqlValue::Integer(r)) => Ok(CqlValue::Boolean(*l <= *r)),
        (CqlValue::Decimal(l), CqlValue::Decimal(r)) => Ok(CqlValue::Boolean(*l <= *r)),
        (CqlValue::Long(l), CqlValue::Long(r)) => Ok(CqlValue::Boolean(*l <= *r)),
        (CqlValue::Quantity(l), CqlValue::Quantity(r)) => {
            // TODO: Handle unit conversion if needed
            Ok(CqlValue::Boolean(l.value <= r.value))
        }
        (CqlValue::Date(d1), CqlValue::Date(d2)) => same_or_before_dates(d1, d2, precision),
        (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => same_or_before_datetimes(dt1, dt2, precision),
        (CqlValue::Time(t1), CqlValue::Time(t2)) => same_or_before_times(t1, t2, precision),
        _ => Err(EvalError::unsupported_operator(
            "SameOrBefore",
            format!("{}, {}", left.get_type().name(), right.get_type().name()),
        )),
    }
}

/// Generic value comparison helper for same_or_after operations
/// Supports Integer, Decimal, Quantity, Date, DateTime, Time
fn value_same_or_after(left: &CqlValue, right: &CqlValue, precision: Option<&DateTimePrecision>) -> EvalResult<CqlValue> {
    match (left, right) {
        (CqlValue::Integer(l), CqlValue::Integer(r)) => Ok(CqlValue::Boolean(*l >= *r)),
        (CqlValue::Decimal(l), CqlValue::Decimal(r)) => Ok(CqlValue::Boolean(*l >= *r)),
        (CqlValue::Long(l), CqlValue::Long(r)) => Ok(CqlValue::Boolean(*l >= *r)),
        (CqlValue::Quantity(l), CqlValue::Quantity(r)) => {
            // TODO: Handle unit conversion if needed
            Ok(CqlValue::Boolean(l.value >= r.value))
        }
        (CqlValue::Date(d1), CqlValue::Date(d2)) => same_or_after_dates(d1, d2, precision),
        (CqlValue::DateTime(dt1), CqlValue::DateTime(dt2)) => same_or_after_datetimes(dt1, dt2, precision),
        (CqlValue::Time(t1), CqlValue::Time(t2)) => same_or_after_times(t1, t2, precision),
        _ => Err(EvalError::unsupported_operator(
            "SameOrAfter",
            format!("{}, {}", left.get_type().name(), right.get_type().name()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 1), 31); // January
        assert_eq!(days_in_month(2024, 2), 29); // February (leap year)
        assert_eq!(days_in_month(2023, 2), 28); // February (non-leap)
        assert_eq!(days_in_month(2024, 4), 30); // April
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(1900)); // Century not divisible by 400
        assert!(is_leap_year(2000)); // Century divisible by 400
    }

    #[test]
    fn test_same_as_dates() {
        let d1 = CqlDate::new(2024, 1, 15);
        let d2 = CqlDate::new(2024, 1, 15);
        let d3 = CqlDate::new(2024, 1, 16);

        assert_eq!(
            same_as_dates(&d1, &d2, Some(&DateTimePrecision::Day)).unwrap(),
            CqlValue::Boolean(true)
        );
        assert_eq!(
            same_as_dates(&d1, &d3, Some(&DateTimePrecision::Day)).unwrap(),
            CqlValue::Boolean(false)
        );
        assert_eq!(
            same_as_dates(&d1, &d3, Some(&DateTimePrecision::Month)).unwrap(),
            CqlValue::Boolean(true)
        );
    }
}
