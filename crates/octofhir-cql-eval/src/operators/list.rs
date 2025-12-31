//! List Operators for CQL
//!
//! Implements: List constructor, Exists, Times, Filter, First, Last, Slice,
//! IndexOf, Flatten, Sort, ForEach, Repeat, Distinct, Current, Iteration, Total,
//! SingletonFrom, and aggregate functions (Count, Sum, Avg, Min, Max, etc.)

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use crate::operators::comparison::{cql_compare, cql_equal};
use octofhir_cql_elm::{
    AggregateExpression, BinaryExpression, FilterExpression, FirstLastExpression, ForEachExpression,
    IndexOfExpression, ListExpression, NaryExpression, RepeatExpression, SliceExpression,
    SortExpression, UnaryExpression,
};
use octofhir_cql_types::{CqlDate, CqlDateTime, CqlList, CqlQuantity, CqlType, CqlValue};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::cmp::Ordering;

impl CqlEngine {
    /// Evaluate List constructor
    pub fn eval_list_constructor(&self, expr: &ListExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let mut elements = Vec::new();

        if let Some(element_exprs) = &expr.elements {
            for elem_expr in element_exprs {
                elements.push(self.evaluate(elem_expr, ctx)?);
            }
        }

        // Determine element type
        let element_type = if let Some(first) = elements.first() {
            first.get_type()
        } else {
            // TODO: Parse type_specifier when type conversion is implemented
            CqlType::Any
        };

        Ok(CqlValue::List(CqlList {
            element_type,
            elements,
        }))
    }

    /// Evaluate Exists - returns true if list contains at least one non-null element
    pub fn eval_exists(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Boolean(false)),
            CqlValue::List(list) => {
                // Exists returns true if there is at least one non-null element
                let has_non_null = list.iter().any(|elem| !elem.is_null());
                Ok(CqlValue::Boolean(has_non_null))
            }
            // Single value is like a list with one element
            _ => Ok(CqlValue::Boolean(true)),
        }
    }

    /// Evaluate Times (cartesian product)
    pub fn eval_times(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        let left_list = match &left {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", left.get_type().name())),
        };

        let right_list = match &right {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", right.get_type().name())),
        };

        // Cartesian product - create tuples
        let mut result = Vec::new();
        for l_item in left_list.iter() {
            for r_item in right_list.iter() {
                let tuple = octofhir_cql_types::CqlTuple::from_elements([
                    ("X", l_item.clone()),
                    ("Y", r_item.clone()),
                ]);
                result.push(CqlValue::Tuple(tuple));
            }
        }

        Ok(CqlValue::List(CqlList::from_elements(result)))
    }

    /// Evaluate Filter
    pub fn eval_filter(&self, expr: &FilterExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let scope = expr.scope.as_deref().unwrap_or("$this");

        let mut result = Vec::new();
        ctx.push_scope();

        for (idx, item) in list.iter().enumerate() {
            ctx.set_alias(scope, item.clone());
            ctx.set_special("$this", item.clone());
            ctx.set_special("$index", CqlValue::Integer(idx as i32));

            let condition = self.evaluate(&expr.condition, ctx)?;

            if condition.is_true() {
                result.push(item.clone());
            }
        }

        ctx.pop_scope();

        Ok(CqlValue::List(CqlList {
            element_type: list.element_type.clone(),
            elements: result,
        }))
    }

    /// Evaluate First
    pub fn eval_first(&self, expr: &FirstLastExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        match &source {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::List(list) => Ok(list.first().cloned().unwrap_or(CqlValue::Null)),
            _ => Err(EvalError::type_mismatch("List", source.get_type().name())),
        }
    }

    /// Evaluate Last
    pub fn eval_last(&self, expr: &FirstLastExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        match &source {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::List(list) => Ok(list.last().cloned().unwrap_or(CqlValue::Null)),
            _ => Err(EvalError::type_mismatch("List", source.get_type().name())),
        }
    }

    /// Evaluate Slice
    pub fn eval_slice(&self, expr: &SliceExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let start_index = self.evaluate(&expr.start_index, ctx)?;

        // Track if end_index was explicitly provided (vs not specified)
        let (end_index, end_was_specified) = if let Some(end_expr) = &expr.end_index {
            (self.evaluate(end_expr, ctx)?, true)
        } else {
            (CqlValue::Null, false) // If no end index, use list length
        };

        let start = match &start_index {
            CqlValue::Null => 0,
            CqlValue::Integer(i) => (*i).max(0) as usize,
            _ => return Err(EvalError::type_mismatch("Integer", start_index.get_type().name())),
        };

        let end = match &end_index {
            CqlValue::Null => {
                if end_was_specified {
                    // Explicitly null end_index (e.g., Take with null count) -> empty list
                    return Ok(CqlValue::List(CqlList::new(list.element_type.clone())));
                } else {
                    // Not specified -> go to end of list
                    list.len()
                }
            }
            CqlValue::Integer(i) => ((*i).max(0) as usize).min(list.len()),
            _ => return Err(EvalError::type_mismatch("Integer", end_index.get_type().name())),
        };

        if start >= list.len() || start >= end {
            return Ok(CqlValue::List(CqlList::new(list.element_type.clone())));
        }

        let elements: Vec<CqlValue> = list.elements[start..end].to_vec();

        Ok(CqlValue::List(CqlList {
            element_type: list.element_type.clone(),
            elements,
        }))
    }

    /// Evaluate IndexOf - returns 0-based index of element in list
    pub fn eval_index_of(&self, expr: &IndexOfExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;
        let element = self.evaluate(&expr.element_to_find, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        // If searching for null, return null
        if element.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        for (i, item) in list.iter().enumerate() {
            if cql_equal(item, &element)?.unwrap_or(false) {
                return Ok(CqlValue::Integer(i as i32));
            }
        }

        Ok(CqlValue::Integer(-1))
    }

    /// Evaluate Flatten - flattens nested lists
    pub fn eval_flatten(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::List(list) => {
                let mut result = Vec::new();
                for item in list.iter() {
                    match item {
                        CqlValue::List(inner) => {
                            result.extend(inner.elements.clone());
                        }
                        CqlValue::Null => {
                            // Skip nulls when flattening
                        }
                        other => {
                            result.push(other.clone());
                        }
                    }
                }
                Ok(CqlValue::List(CqlList::from_elements(result)))
            }
            _ => Err(EvalError::type_mismatch("List", operand.get_type().name())),
        }
    }

    /// Evaluate Sort
    pub fn eval_sort(&self, expr: &SortExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let mut elements = list.elements.clone();

        // Sort using CQL comparison with special handling for DateTimes
        elements.sort_by(|a, b| {
            let cmp_result = cql_compare(a, b);
            let result = match &cmp_result {
                Ok(Some(ord)) => *ord,
                Ok(None) => {
                    // When comparison is indeterminate (e.g., different precision),
                    // use secondary sort: less precise values come first
                    match (a, b) {
                        (CqlValue::DateTime(da), CqlValue::DateTime(db)) => {
                            // Compare precision: less precise comes first (ascending)
                            let precision_a = datetime_precision(da);
                            let precision_b = datetime_precision(db);
                            precision_a.cmp(&precision_b)
                        }
                        (CqlValue::Date(da), CqlValue::Date(db)) => {
                            let precision_a = date_precision(da);
                            let precision_b = date_precision(db);
                            precision_a.cmp(&precision_b)
                        }
                        _ => Ordering::Equal,
                    }
                }
                Err(_) => Ordering::Equal,
            };
            result
        });

        // Handle sort direction
        if let Some(first_by) = expr.by.first() {
            if matches!(
                first_by.direction,
                octofhir_cql_elm::SortDirection::Descending
                    | octofhir_cql_elm::SortDirection::Desc
            ) {
                elements.reverse();
            }
        }

        Ok(CqlValue::List(CqlList {
            element_type: list.element_type.clone(),
            elements,
        }))
    }

    /// Evaluate ForEach
    pub fn eval_for_each(&self, expr: &ForEachExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let scope = expr.scope.as_deref().unwrap_or("$this");

        let mut result = Vec::new();
        ctx.push_scope();

        for (idx, item) in list.iter().enumerate() {
            ctx.set_alias(scope, item.clone());
            ctx.set_special("$this", item.clone());
            ctx.set_special("$index", CqlValue::Integer(idx as i32));

            let mapped = self.evaluate(&expr.element_expr, ctx)?;
            result.push(mapped);
        }

        ctx.pop_scope();

        Ok(CqlValue::List(CqlList::from_elements(result)))
    }

    /// Evaluate Repeat
    pub fn eval_repeat(&self, expr: &RepeatExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l.clone(),
            _ => CqlList::from_elements(vec![source]),
        };

        let scope = expr.scope.as_deref().unwrap_or("$this");

        let mut result = list.elements.clone();
        let mut to_process = list.elements.clone();
        let max_iterations = 1000; // Prevent infinite loops
        let mut iterations = 0;

        ctx.push_scope();

        while !to_process.is_empty() && iterations < max_iterations {
            iterations += 1;
            let mut new_items = Vec::new();

            for item in to_process.iter() {
                ctx.set_alias(scope, item.clone());
                ctx.set_special("$this", item.clone());

                let expanded = self.evaluate(&expr.element_expr, ctx)?;

                match expanded {
                    CqlValue::List(inner) => {
                        for inner_item in inner.iter() {
                            if !result.iter().any(|r| cql_equal(r, inner_item).unwrap_or(Some(false)).unwrap_or(false)) {
                                new_items.push(inner_item.clone());
                            }
                        }
                    }
                    CqlValue::Null => {}
                    other => {
                        if !result.iter().any(|r| cql_equal(r, &other).unwrap_or(Some(false)).unwrap_or(false)) {
                            new_items.push(other);
                        }
                    }
                }
            }

            result.extend(new_items.clone());
            to_process = new_items;
        }

        ctx.pop_scope();

        Ok(CqlValue::List(CqlList::from_elements(result)))
    }

    /// Evaluate Distinct - removes duplicates
    ///
    /// For the Distinct operator, null values are considered equivalent to each other,
    /// so multiple nulls are reduced to a single null.
    pub fn eval_distinct(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::List(list) => {
                let mut result: Vec<CqlValue> = Vec::new();
                let mut has_null = false;

                for item in list.iter() {
                    // Special handling for nulls - only keep one null
                    if item.is_null() {
                        if !has_null {
                            result.push(item.clone());
                            has_null = true;
                        }
                        continue;
                    }

                    // For non-null values, use cql_equal to check for duplicates
                    let is_duplicate = result.iter().any(|r| {
                        if r.is_null() {
                            false // Null is never equal to non-null for comparison
                        } else {
                            cql_equal(r, item).unwrap_or(Some(false)).unwrap_or(false)
                        }
                    });

                    if !is_duplicate {
                        result.push(item.clone());
                    }
                }
                Ok(CqlValue::List(CqlList {
                    element_type: list.element_type.clone(),
                    elements: result,
                }))
            }
            _ => Err(EvalError::type_mismatch("List", operand.get_type().name())),
        }
    }

    /// Evaluate Current ($this)
    pub fn eval_current(&self, _expr: &octofhir_cql_elm::CurrentExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        ctx.get_special("$this")
            .cloned()
            .ok_or_else(|| EvalError::undefined_alias("$this"))
    }

    /// Evaluate Iteration ($index)
    pub fn eval_iteration(&self, _expr: &octofhir_cql_elm::IterationExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        ctx.get_special("$index")
            .cloned()
            .ok_or_else(|| EvalError::undefined_alias("$index"))
    }

    /// Evaluate Total ($total)
    pub fn eval_total(&self, _expr: &octofhir_cql_elm::TotalExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        ctx.get_special("$total")
            .cloned()
            .ok_or_else(|| EvalError::undefined_alias("$total"))
    }

    /// Evaluate SingletonFrom - returns single element or null/error
    pub fn eval_singleton_from(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::List(list) => {
                match list.len() {
                    0 => Ok(CqlValue::Null),
                    1 => Ok(list.first().unwrap().clone()),
                    _ => Err(EvalError::invalid_operand("SingletonFrom", "list has more than one element")),
                }
            }
            // Single value returns itself
            _ => Ok(operand),
        }
    }

    // =========================================================================
    // Aggregate Functions
    // =========================================================================

    /// Helper to evaluate the source from an AggregateExpression
    fn eval_aggregate_source(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if let Some(source_expr) = &expr.source {
            self.evaluate(source_expr, ctx)
        } else {
            Err(EvalError::internal("AggregateExpression missing source"))
        }
    }

    /// Evaluate Aggregate expression
    pub fn eval_aggregate(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let scope = "$this"; // Default scope for aggregate iterations

        // Initialize accumulator
        let mut accumulator = if let Some(init_expr) = &expr.starting {
            self.evaluate(init_expr, ctx)?
        } else {
            CqlValue::Null
        };

        ctx.push_scope();

        for (idx, item) in list.iter().enumerate() {
            ctx.set_alias(scope, item.clone());
            ctx.set_special("$this", item.clone());
            ctx.set_special("$index", CqlValue::Integer(idx as i32));
            ctx.set_special("$total", accumulator.clone());

            if let Some(iteration_expr) = &expr.iteration {
                accumulator = self.evaluate(iteration_expr, ctx)?;
            }
        }

        ctx.pop_scope();

        Ok(accumulator)
    }

    /// Evaluate Count
    pub fn eval_count(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        match &source {
            CqlValue::Null => Ok(CqlValue::Integer(0)),
            CqlValue::List(list) => {
                let count = list.iter().filter(|v| !v.is_null()).count();
                Ok(CqlValue::Integer(count as i32))
            }
            _ => Ok(CqlValue::Integer(1)), // Single non-null value
        }
    }

    /// Evaluate Sum
    pub fn eval_sum(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        if list.is_empty() {
            return Ok(CqlValue::Null);
        }

        let mut sum: Option<CqlValue> = None;

        for item in list.iter() {
            if item.is_null() {
                continue;
            }

            sum = Some(match sum {
                None => item.clone(),
                Some(CqlValue::Integer(a)) => {
                    if let Some(b) = item.as_integer() {
                        CqlValue::Integer(a + b)
                    } else if let Some(b) = item.as_decimal() {
                        CqlValue::Decimal(Decimal::from(a) + b)
                    } else {
                        return Err(EvalError::type_mismatch("numeric", item.get_type().name()));
                    }
                }
                Some(CqlValue::Long(a)) => {
                    if let Some(b) = item.as_long() {
                        CqlValue::Long(a + b)
                    } else if let Some(b) = item.as_decimal() {
                        CqlValue::Decimal(Decimal::from(a) + b)
                    } else {
                        return Err(EvalError::type_mismatch("numeric", item.get_type().name()));
                    }
                }
                Some(CqlValue::Decimal(a)) => {
                    if let Some(b) = item.as_decimal() {
                        CqlValue::Decimal(a + b)
                    } else {
                        return Err(EvalError::type_mismatch("numeric", item.get_type().name()));
                    }
                }
                Some(CqlValue::Quantity(q)) => {
                    if let CqlValue::Quantity(q2) = item {
                        if q.unit == q2.unit {
                            CqlValue::Quantity(CqlQuantity {
                                value: q.value + q2.value,
                                unit: q.unit.clone(),
                            })
                        } else {
                            return Err(EvalError::IncompatibleUnits {
                                unit1: q.unit.clone().unwrap_or_default(),
                                unit2: q2.unit.clone().unwrap_or_default(),
                            });
                        }
                    } else {
                        return Err(EvalError::type_mismatch("Quantity", item.get_type().name()));
                    }
                }
                _ => return Err(EvalError::type_mismatch("numeric", source.get_type().name())),
            });
        }

        Ok(sum.unwrap_or(CqlValue::Null))
    }

    /// Evaluate Product
    pub fn eval_product(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        if list.is_empty() {
            return Ok(CqlValue::Null);
        }

        let mut product: Option<CqlValue> = None;

        for item in list.iter() {
            if item.is_null() {
                continue;
            }

            product = Some(match product {
                None => item.clone(),
                Some(CqlValue::Integer(a)) => {
                    if let Some(b) = item.as_integer() {
                        CqlValue::Integer(a * b)
                    } else if let Some(b) = item.as_decimal() {
                        CqlValue::Decimal(Decimal::from(a) * b)
                    } else {
                        return Err(EvalError::type_mismatch("numeric", item.get_type().name()));
                    }
                }
                Some(CqlValue::Long(a)) => {
                    match item {
                        CqlValue::Long(b) => CqlValue::Long(a * b),
                        CqlValue::Integer(b) => CqlValue::Long(a * (*b as i64)),
                        CqlValue::Decimal(b) => CqlValue::Decimal(Decimal::from(a) * b),
                        _ => return Err(EvalError::type_mismatch("numeric", item.get_type().name())),
                    }
                }
                Some(CqlValue::Decimal(a)) => {
                    if let Some(b) = item.as_decimal() {
                        CqlValue::Decimal(a * b)
                    } else {
                        return Err(EvalError::type_mismatch("numeric", item.get_type().name()));
                    }
                }
                _ => return Err(EvalError::type_mismatch("numeric", source.get_type().name())),
            });
        }

        Ok(product.unwrap_or(CqlValue::Null))
    }

    /// Evaluate Min
    pub fn eval_min(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let mut min: Option<CqlValue> = None;

        for item in list.iter() {
            if item.is_null() {
                continue;
            }

            min = Some(match min {
                None => item.clone(),
                Some(current) => {
                    match cql_compare(&current, item)? {
                        Some(Ordering::Greater) => item.clone(),
                        _ => current,
                    }
                }
            });
        }

        Ok(min.unwrap_or(CqlValue::Null))
    }

    /// Evaluate Max
    pub fn eval_max(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let mut max: Option<CqlValue> = None;

        for item in list.iter() {
            if item.is_null() {
                continue;
            }

            max = Some(match max {
                None => item.clone(),
                Some(current) => {
                    match cql_compare(&current, item)? {
                        Some(Ordering::Less) => item.clone(),
                        _ => current,
                    }
                }
            });
        }

        Ok(max.unwrap_or(CqlValue::Null))
    }

    /// Evaluate Avg
    pub fn eval_avg(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let sum_result = self.eval_sum(expr, ctx)?;
        let count_result = self.eval_count(expr, ctx)?;

        match (&sum_result, &count_result) {
            (CqlValue::Null, _) | (_, CqlValue::Integer(0)) => Ok(CqlValue::Null),
            (CqlValue::Integer(sum), CqlValue::Integer(count)) => {
                Ok(CqlValue::Decimal(Decimal::from(*sum) / Decimal::from(*count)))
            }
            (CqlValue::Long(sum), CqlValue::Integer(count)) => {
                Ok(CqlValue::Decimal(Decimal::from(*sum) / Decimal::from(*count)))
            }
            (CqlValue::Decimal(sum), CqlValue::Integer(count)) => {
                Ok(CqlValue::Decimal(*sum / Decimal::from(*count)))
            }
            (CqlValue::Quantity(q), CqlValue::Integer(count)) => {
                Ok(CqlValue::Quantity(CqlQuantity {
                    value: q.value / Decimal::from(*count),
                    unit: q.unit.clone(),
                }))
            }
            _ => Err(EvalError::type_mismatch("numeric", sum_result.get_type().name())),
        }
    }

    /// Evaluate GeometricMean
    pub fn eval_geometric_mean(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let non_null: Vec<f64> = list
            .iter()
            .filter_map(|v| match v {
                CqlValue::Integer(i) => Some(*i as f64),
                CqlValue::Long(l) => Some(*l as f64),
                CqlValue::Decimal(d) => d.to_f64(),
                _ => None,
            })
            .collect();

        if non_null.is_empty() {
            return Ok(CqlValue::Null);
        }

        let product: f64 = non_null.iter().product();
        let n = non_null.len() as f64;
        let geo_mean = product.powf(1.0 / n);

        Ok(CqlValue::Decimal(Decimal::from_f64(geo_mean).unwrap_or(Decimal::ZERO)))
    }

    /// Evaluate Median
    pub fn eval_median(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let mut values: Vec<CqlValue> = list.iter().filter(|v| !v.is_null()).cloned().collect();

        if values.is_empty() {
            return Ok(CqlValue::Null);
        }

        values.sort_by(|a, b| cql_compare(a, b).unwrap_or(Some(Ordering::Equal)).unwrap_or(Ordering::Equal));

        let len = values.len();
        if len % 2 == 1 {
            Ok(values[len / 2].clone())
        } else {
            // Average of two middle values
            let mid1 = &values[len / 2 - 1];
            let mid2 = &values[len / 2];

            match (mid1.as_decimal(), mid2.as_decimal()) {
                (Some(d1), Some(d2)) => Ok(CqlValue::Decimal((d1 + d2) / Decimal::from(2))),
                _ => Ok(mid1.clone()),
            }
        }
    }

    /// Evaluate Mode
    pub fn eval_mode(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let non_null: Vec<&CqlValue> = list.iter().filter(|v| !v.is_null()).collect();

        if non_null.is_empty() {
            return Ok(CqlValue::Null);
        }

        // Count occurrences
        let mut counts: Vec<(&CqlValue, usize)> = Vec::new();
        for item in &non_null {
            let found = counts.iter_mut().find(|(v, _)| cql_equal(v, item).unwrap_or(Some(false)).unwrap_or(false));
            match found {
                Some((_, count)) => *count += 1,
                None => counts.push((item, 1)),
            }
        }

        // Find max count
        let max_count = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);

        // Return first with max count
        counts
            .into_iter()
            .find(|(_, c)| *c == max_count)
            .map(|(v, _)| v.clone())
            .ok_or_else(|| EvalError::internal("Mode calculation failed"))
    }

    /// Evaluate Variance
    pub fn eval_variance(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        self.eval_variance_impl(expr, ctx, false)
    }

    /// Evaluate PopulationVariance
    pub fn eval_population_variance(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        self.eval_variance_impl(expr, ctx, true)
    }

    fn eval_variance_impl(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext, population: bool) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        let values: Vec<f64> = list
            .iter()
            .filter_map(|v| match v {
                CqlValue::Integer(i) => Some(*i as f64),
                CqlValue::Long(l) => Some(*l as f64),
                CqlValue::Decimal(d) => d.to_f64(),
                _ => None,
            })
            .collect();

        let n = values.len();
        if n == 0 || (!population && n == 1) {
            return Ok(CqlValue::Null);
        }

        let mean: f64 = values.iter().sum::<f64>() / n as f64;
        let sum_sq_diff: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();

        let variance = if population {
            sum_sq_diff / n as f64
        } else {
            sum_sq_diff / (n - 1) as f64
        };

        Ok(CqlValue::Decimal(Decimal::from_f64(variance).unwrap_or(Decimal::ZERO)))
    }

    /// Evaluate StdDev
    pub fn eval_stddev(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let variance = self.eval_variance(expr, ctx)?;
        match variance {
            CqlValue::Decimal(v) => {
                let stddev = v.to_f64().map(|f| f.sqrt()).unwrap_or(0.0);
                let result = Decimal::from_f64(stddev).unwrap_or(Decimal::ZERO);
                // Round to 8 decimal places (CQL Decimal precision)
                Ok(CqlValue::Decimal(result.round_dp(8)))
            }
            CqlValue::Null => Ok(CqlValue::Null),
            _ => Err(EvalError::type_mismatch("Decimal", variance.get_type().name())),
        }
    }

    /// Evaluate PopulationStdDev
    pub fn eval_population_stddev(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let variance = self.eval_population_variance(expr, ctx)?;
        match variance {
            CqlValue::Decimal(v) => {
                let stddev = v.to_f64().map(|f| f.sqrt()).unwrap_or(0.0);
                let result = Decimal::from_f64(stddev).unwrap_or(Decimal::ZERO);
                // Round to 8 decimal places (CQL Decimal precision)
                Ok(CqlValue::Decimal(result.round_dp(8)))
            }
            CqlValue::Null => Ok(CqlValue::Null),
            _ => Err(EvalError::type_mismatch("Decimal", variance.get_type().name())),
        }
    }

    /// Evaluate AllTrue
    pub fn eval_all_true(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Boolean(true));
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        for item in list.iter() {
            match item {
                CqlValue::Boolean(false) => return Ok(CqlValue::Boolean(false)),
                CqlValue::Null => continue, // null doesn't make AllTrue false
                CqlValue::Boolean(true) => continue,
                _ => return Err(EvalError::type_mismatch("Boolean", item.get_type().name())),
            }
        }

        Ok(CqlValue::Boolean(true))
    }

    /// Evaluate AnyTrue
    pub fn eval_any_true(&self, expr: &AggregateExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.eval_aggregate_source(expr, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Boolean(false));
        }

        let list = match &source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List", source.get_type().name())),
        };

        for item in list.iter() {
            match item {
                CqlValue::Boolean(true) => return Ok(CqlValue::Boolean(true)),
                _ => continue,
            }
        }

        Ok(CqlValue::Boolean(false))
    }
}

/// Calculate DateTime precision level (higher = more precise)
fn datetime_precision(dt: &CqlDateTime) -> u8 {
    if dt.millisecond.is_some() {
        7
    } else if dt.second.is_some() {
        6
    } else if dt.minute.is_some() {
        5
    } else if dt.hour.is_some() {
        4
    } else if dt.day.is_some() {
        3
    } else if dt.month.is_some() {
        2
    } else {
        1
    }
}

/// Calculate Date precision level (higher = more precise)
fn date_precision(d: &CqlDate) -> u8 {
    if d.day.is_some() {
        3
    } else if d.month.is_some() {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_int_list(values: Vec<i32>) -> CqlValue {
        CqlValue::List(CqlList {
            element_type: CqlType::Integer,
            elements: values.into_iter().map(CqlValue::Integer).collect(),
        })
    }

    #[test]
    fn test_list_operations() {
        let list = CqlList::from_elements(vec![
            CqlValue::Integer(1),
            CqlValue::Integer(2),
            CqlValue::Integer(3),
        ]);

        assert_eq!(list.len(), 3);
        assert_eq!(list.first(), Some(&CqlValue::Integer(1)));
        assert_eq!(list.last(), Some(&CqlValue::Integer(3)));
    }

    #[test]
    fn test_distinct() {
        let list = CqlList::from_elements(vec![
            CqlValue::Integer(1),
            CqlValue::Integer(2),
            CqlValue::Integer(1),
            CqlValue::Integer(3),
            CqlValue::Integer(2),
        ]);

        let mut result: Vec<CqlValue> = Vec::new();
        for item in list.iter() {
            if !result.iter().any(|r| cql_equal(r, item).unwrap_or(Some(false)).unwrap_or(false)) {
                result.push(item.clone());
            }
        }

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_flatten() {
        let inner1 = CqlValue::List(CqlList::from_elements(vec![
            CqlValue::Integer(1),
            CqlValue::Integer(2),
        ]));
        let inner2 = CqlValue::List(CqlList::from_elements(vec![
            CqlValue::Integer(3),
            CqlValue::Integer(4),
        ]));
        let outer = CqlList::from_elements(vec![inner1, inner2]);

        let mut result = Vec::new();
        for item in outer.iter() {
            if let CqlValue::List(inner) = item {
                result.extend(inner.elements.clone());
            }
        }

        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_datetime_precision_comparison() {
        // Test that cql_compare returns None for DateTimes with different precisions
        // and that our precision fallback works correctly
        let dt_day = CqlValue::DateTime(CqlDateTime {
            year: 2012,
            month: Some(10),
            day: Some(5),
            hour: None,
            minute: None,
            second: None,
            millisecond: None,
            timezone_offset: None,
        });
        let dt_hour = CqlValue::DateTime(CqlDateTime {
            year: 2012,
            month: Some(10),
            day: Some(5),
            hour: Some(10),
            minute: None,
            second: None,
            millisecond: None,
            timezone_offset: None,
        });

        // cql_compare should return None for different precisions
        let cmp_result = cql_compare(&dt_day, &dt_hour);
        assert_eq!(cmp_result.unwrap(), None, "Expected None for different precisions");

        // datetime_precision should return 3 for day precision, 4 for hour precision
        if let CqlValue::DateTime(dt) = &dt_day {
            assert_eq!(datetime_precision(dt), 3, "Day precision should be 3");
        }
        if let CqlValue::DateTime(dt) = &dt_hour {
            assert_eq!(datetime_precision(dt), 4, "Hour precision should be 4");
        }
    }

    #[test]
    fn test_datetime_sort_with_precision() {
        // Test sorting DateTimes with different precisions
        // Input: [Oct 5 hour 10, Jan 1 no hour, Jan 1 hour 12, Oct 5 no hour]
        // Expected ascending: [Jan 1 no hour, Jan 1 hour 12, Oct 5 no hour, Oct 5 hour 10]
        let oct5_hour10 = CqlValue::DateTime(CqlDateTime {
            year: 2012, month: Some(10), day: Some(5), hour: Some(10),
            minute: None, second: None, millisecond: None, timezone_offset: None,
        });
        let jan1_nohour = CqlValue::DateTime(CqlDateTime {
            year: 2012, month: Some(1), day: Some(1), hour: None,
            minute: None, second: None, millisecond: None, timezone_offset: None,
        });
        let jan1_hour12 = CqlValue::DateTime(CqlDateTime {
            year: 2012, month: Some(1), day: Some(1), hour: Some(12),
            minute: None, second: None, millisecond: None, timezone_offset: None,
        });
        let oct5_nohour = CqlValue::DateTime(CqlDateTime {
            year: 2012, month: Some(10), day: Some(5), hour: None,
            minute: None, second: None, millisecond: None, timezone_offset: None,
        });

        let mut elements = vec![
            oct5_hour10.clone(),
            jan1_nohour.clone(),
            jan1_hour12.clone(),
            oct5_nohour.clone(),
        ];

        // Sort using the same logic as eval_sort
        elements.sort_by(|a, b| {
            let cmp_result = cql_compare(a, b);
            match &cmp_result {
                Ok(Some(ord)) => *ord,
                Ok(None) => {
                    match (a, b) {
                        (CqlValue::DateTime(da), CqlValue::DateTime(db)) => {
                            let precision_a = datetime_precision(da);
                            let precision_b = datetime_precision(db);
                            precision_a.cmp(&precision_b)
                        }
                        _ => Ordering::Equal,
                    }
                }
                Err(_) => Ordering::Equal,
            }
        });

        // Expected order: [Jan 1 no hour, Jan 1 hour 12, Oct 5 no hour, Oct 5 hour 10]
        assert_eq!(elements[0], jan1_nohour, "First should be Jan 1 no hour");
        assert_eq!(elements[1], jan1_hour12, "Second should be Jan 1 hour 12");
        assert_eq!(elements[2], oct5_nohour, "Third should be Oct 5 no hour");
        assert_eq!(elements[3], oct5_hour10, "Fourth should be Oct 5 hour 10");
    }
}
