//! Query Evaluation for CQL
//!
//! This module implements CQL query evaluation including:
//! - Single-source queries
//! - Multi-source queries (cartesian product)
//! - Let clauses
//! - With/Without relationship clauses
//! - Where filtering
//! - Return projection
//! - Sort (asc, desc, by expression)
//! - Aggregate clause
//! - Distinct return

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use crate::operators::comparison::cql_compare;
use octofhir_cql_elm::{Query, RelationshipClause, Retrieve, SortByItem, SortDirection};
use octofhir_cql_types::{CqlList, CqlValue};
use std::cmp::Ordering;

impl CqlEngine {
    /// Evaluate a Query expression
    ///
    /// A query in CQL consists of:
    /// 1. One or more sources (each with an alias)
    /// 2. Optional let clauses (computed values available in scope)
    /// 3. Optional relationship clauses (with/without)
    /// 4. Optional where clause (filtering)
    /// 5. Optional return clause (projection)
    /// 6. Optional aggregate clause (aggregation)
    /// 7. Optional sort clause (ordering)
    pub fn eval_query(&self, query: &Query, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        // Step 1: Evaluate all sources
        let sources = self.evaluate_query_sources(query, ctx)?;

        // Track if this is a singleton source query (non-list source)
        // A singleton source should return a scalar, not a list
        let is_singleton_source = sources.len() == 1 && sources[0].2;

        // Step 2: Generate combinations (cartesian product for multi-source)
        let mut combinations = self.generate_source_combinations(&sources)?;

        // Step 3: Apply relationship clauses (with/without)
        if let Some(relationships) = &query.relationship {
            combinations = self.apply_relationship_clauses(combinations, relationships, query, ctx)?;
        }

        // Step 4: Apply let clauses
        // Let clauses are evaluated for each combination and made available in scope

        // Step 5: Apply where clause (filtering)
        if let Some(where_expr) = &query.where_clause {
            combinations = self.apply_where_clause(combinations, where_expr, query, ctx)?;
        }

        // Step 6: Check for aggregate clause
        if let Some(aggregate) = &query.aggregate {
            return self.apply_aggregate_clause(combinations, aggregate, query, ctx);
        }

        // Step 7: Apply return clause (projection)
        let mut results = if let Some(return_clause) = &query.return_clause {
            self.apply_return_clause(combinations, return_clause, query, ctx)?
        } else {
            // Default: return the source value(s) directly
            // For single source, return the aliased value
            // For multi-source, return a tuple of all aliases
            self.default_return(combinations, query)?
        };

        // Step 8: Apply distinct if specified
        if query.return_clause.as_ref().map_or(false, |r| r.distinct.unwrap_or(false)) {
            results = self.apply_distinct(results)?;
        }

        // Step 9: Apply sort clause
        if let Some(sort) = &query.sort {
            results = self.apply_sort_clause(results, sort, ctx)?;
        }

        // Step 10: For singleton source, unwrap the result
        // If the source was a non-list value, return the single result directly
        if is_singleton_source && results.len() == 1 {
            return Ok(results.into_iter().next().unwrap());
        }

        Ok(CqlValue::List(CqlList::from_elements(results)))
    }

    /// Evaluate query sources and return as (alias, values, was_singleton) tuples
    ///
    /// The third element indicates if the source was a non-list value (singleton).
    /// This is used to determine if the result should be unwrapped.
    fn evaluate_query_sources(
        &self,
        query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<(String, Vec<CqlValue>, bool)>> {
        let mut sources = Vec::new();

        for source in &query.source {
            let value = self.evaluate(&source.expression, ctx)?;

            // Convert to list if needed, and track if it was originally a singleton
            let (values, was_singleton) = match value {
                CqlValue::List(list) => (list.iter().cloned().collect(), false),
                CqlValue::Null => (vec![], false),
                other => (vec![other], true), // Non-list value treated as singleton
            };

            sources.push((source.alias.clone(), values, was_singleton));
        }

        Ok(sources)
    }

    /// Generate all combinations of source values (cartesian product)
    ///
    /// For single-source queries, this just wraps each value.
    /// For multi-source queries, this produces the cartesian product.
    fn generate_source_combinations(
        &self,
        sources: &[(String, Vec<CqlValue>, bool)],
    ) -> EvalResult<Vec<Vec<(String, CqlValue)>>> {
        if sources.is_empty() {
            return Ok(vec![]);
        }

        // Start with first source
        let (first_alias, first_values, _) = &sources[0];
        let mut combinations: Vec<Vec<(String, CqlValue)>> = first_values
            .iter()
            .map(|v| vec![(first_alias.clone(), v.clone())])
            .collect();

        // Add each subsequent source (cartesian product)
        for (alias, values, _) in sources.iter().skip(1) {
            let mut new_combinations = Vec::new();
            for combo in combinations {
                for value in values {
                    let mut new_combo = combo.clone();
                    new_combo.push((alias.clone(), value.clone()));
                    new_combinations.push(new_combo);
                }
            }
            combinations = new_combinations;
        }

        Ok(combinations)
    }

    /// Apply relationship clauses (with/without)
    fn apply_relationship_clauses(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
        relationships: &[RelationshipClause],
        query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<Vec<(String, CqlValue)>>> {
        let mut result = combinations;

        for relationship in relationships {
            result = match relationship {
                RelationshipClause::With(with) => {
                    self.apply_with_clause(result, with, query, ctx)?
                }
                RelationshipClause::Without(without) => {
                    self.apply_without_clause(result, without, query, ctx)?
                }
            };
        }

        Ok(result)
    }

    /// Apply a With clause
    ///
    /// Returns combinations where at least one related item satisfies the such-that condition
    fn apply_with_clause(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
        with: &octofhir_cql_elm::WithClause,
        _query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<Vec<(String, CqlValue)>>> {
        // Evaluate the with source
        let related_value = self.evaluate(&with.expression, ctx)?;
        let related_values: Vec<CqlValue> = match related_value {
            CqlValue::List(list) => list.iter().cloned().collect(),
            CqlValue::Null => vec![],
            other => vec![other],
        };

        let mut result = Vec::new();

        for combo in combinations {
            // Set up scope with current combination aliases
            ctx.push_scope();
            for (alias, value) in &combo {
                ctx.set_alias(alias, value.clone());
            }

            // Check if any related value satisfies the such-that condition
            let mut has_match = false;
            for related in &related_values {
                ctx.set_alias(&with.alias, related.clone());
                let condition = self.evaluate(&with.such_that, ctx)?;
                if condition.is_true() {
                    has_match = true;
                    break;
                }
            }

            ctx.pop_scope();

            if has_match {
                result.push(combo);
            }
        }

        Ok(result)
    }

    /// Apply a Without clause
    ///
    /// Returns combinations where NO related item satisfies the such-that condition
    fn apply_without_clause(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
        without: &octofhir_cql_elm::WithoutClause,
        _query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<Vec<(String, CqlValue)>>> {
        // Evaluate the without source
        let related_value = self.evaluate(&without.expression, ctx)?;
        let related_values: Vec<CqlValue> = match related_value {
            CqlValue::List(list) => list.iter().cloned().collect(),
            CqlValue::Null => vec![],
            other => vec![other],
        };

        let mut result = Vec::new();

        for combo in combinations {
            // Set up scope with current combination aliases
            ctx.push_scope();
            for (alias, value) in &combo {
                ctx.set_alias(alias, value.clone());
            }

            // Check that NO related value satisfies the such-that condition
            let mut has_match = false;
            for related in &related_values {
                ctx.set_alias(&without.alias, related.clone());
                let condition = self.evaluate(&without.such_that, ctx)?;
                if condition.is_true() {
                    has_match = true;
                    break;
                }
            }

            ctx.pop_scope();

            // Include only if there's no match
            if !has_match {
                result.push(combo);
            }
        }

        Ok(result)
    }

    /// Apply where clause filtering
    fn apply_where_clause(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
        where_expr: &octofhir_cql_elm::Expression,
        query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<Vec<(String, CqlValue)>>> {
        let mut result = Vec::new();

        for combo in combinations {
            ctx.push_scope();

            // Set all aliases
            for (alias, value) in &combo {
                ctx.set_alias(alias, value.clone());
            }

            // Evaluate let clauses
            if let Some(let_clauses) = &query.let_clause {
                for let_clause in let_clauses {
                    let value = self.evaluate(&let_clause.expression, ctx)?;
                    ctx.set_let(&let_clause.identifier, value);
                }
            }

            // Evaluate where condition
            let condition = self.evaluate(where_expr, ctx)?;
            ctx.pop_scope();

            // Include if condition is true (not false or null)
            if condition.is_true() {
                result.push(combo);
            }
        }

        Ok(result)
    }

    /// Apply return clause projection
    fn apply_return_clause(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
        return_clause: &octofhir_cql_elm::ReturnClause,
        query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<CqlValue>> {
        let mut results = Vec::new();

        for combo in combinations {
            ctx.push_scope();

            // Set all aliases
            for (alias, value) in &combo {
                ctx.set_alias(alias, value.clone());
            }

            // Evaluate let clauses
            if let Some(let_clauses) = &query.let_clause {
                for let_clause in let_clauses {
                    let value = self.evaluate(&let_clause.expression, ctx)?;
                    ctx.set_let(&let_clause.identifier, value);
                }
            }

            // Evaluate return expression
            let result = self.evaluate(&return_clause.expression, ctx)?;
            ctx.pop_scope();

            results.push(result);
        }

        Ok(results)
    }

    /// Default return when no return clause is specified
    fn default_return(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
        query: &Query,
    ) -> EvalResult<Vec<CqlValue>> {
        let single_source = query.source.len() == 1;

        Ok(combinations
            .into_iter()
            .map(|combo| {
                if single_source {
                    // Single source: return the value directly
                    combo.into_iter().next().map(|(_, v)| v).unwrap_or(CqlValue::Null)
                } else {
                    // Multi-source: return a tuple of all values
                    CqlValue::Tuple(octofhir_cql_types::CqlTuple::from_elements(
                        combo.into_iter().collect::<Vec<_>>(),
                    ))
                }
            })
            .collect())
    }

    /// Apply aggregate clause
    fn apply_aggregate_clause(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
        aggregate: &octofhir_cql_elm::AggregateClause,
        query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        // Get initial value
        let mut accumulator = if let Some(starting) = &aggregate.starting {
            self.evaluate(starting, ctx)?
        } else {
            CqlValue::Null
        };

        // Apply distinct if specified
        let items = if aggregate.distinct.unwrap_or(false) {
            self.make_combinations_distinct(combinations)?
        } else {
            combinations
        };

        // Process each combination
        for combo in items {
            ctx.push_scope();

            // Set all aliases
            for (alias, value) in &combo {
                ctx.set_alias(alias, value.clone());
            }

            // Evaluate let clauses
            if let Some(let_clauses) = &query.let_clause {
                for let_clause in let_clauses {
                    let value = self.evaluate(&let_clause.expression, ctx)?;
                    ctx.set_let(&let_clause.identifier, value);
                }
            }

            // Set the accumulator as an alias so it can be referenced in the expression
            ctx.set_alias(&aggregate.identifier, accumulator.clone());

            // Evaluate aggregate expression
            accumulator = self.evaluate(&aggregate.expression, ctx)?;
            ctx.pop_scope();
        }

        Ok(accumulator)
    }

    /// Make combinations distinct by comparing tuples
    fn make_combinations_distinct(
        &self,
        combinations: Vec<Vec<(String, CqlValue)>>,
    ) -> EvalResult<Vec<Vec<(String, CqlValue)>>> {
        let mut result: Vec<Vec<(String, CqlValue)>> = Vec::new();

        for combo in combinations {
            let is_duplicate = result.iter().any(|existing| {
                if existing.len() != combo.len() {
                    return false;
                }
                existing
                    .iter()
                    .zip(combo.iter())
                    .all(|((_, v1), (_, v2))| crate::operators::comparison::cql_equal(v1, v2).unwrap_or(Some(false)).unwrap_or(false))
            });

            if !is_duplicate {
                result.push(combo);
            }
        }

        Ok(result)
    }

    /// Apply distinct to results
    fn apply_distinct(&self, values: Vec<CqlValue>) -> EvalResult<Vec<CqlValue>> {
        let mut result = Vec::new();

        for value in values {
            let is_duplicate = result.iter().any(|existing| {
                crate::operators::comparison::cql_equal(existing, &value).unwrap_or(Some(false)).unwrap_or(false)
            });

            if !is_duplicate {
                result.push(value);
            }
        }

        Ok(result)
    }

    /// Apply sort clause
    fn apply_sort_clause(
        &self,
        mut values: Vec<CqlValue>,
        sort: &octofhir_cql_elm::SortClause,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<CqlValue>> {
        // Sort by each sort key in order
        for sort_item in sort.by.iter().rev() {
            self.sort_by_item(&mut values, sort_item, ctx)?;
        }

        Ok(values)
    }

    /// Sort values by a single sort item
    fn sort_by_item(
        &self,
        values: &mut [CqlValue],
        sort_item: &SortByItem,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<()> {
        // Extract sort keys for each value
        let mut keyed: Vec<(CqlValue, CqlValue)> = Vec::new();

        for value in values.iter() {
            let key = if let Some(path) = &sort_item.path {
                // Sort by path
                self.get_property_value_for_sort(value, path, ctx)?
            } else {
                // Sort by value itself
                value.clone()
            };
            keyed.push((value.clone(), key));
        }

        // Sort based on direction
        let direction = sort_item.direction;
        keyed.sort_by(|(_, k1), (_, k2)| {
            let cmp_result = cql_compare(k1, k2);
            let cmp = match &cmp_result {
                Ok(Some(ord)) => *ord,
                Ok(None) => {
                    // When comparison is indeterminate (e.g., different precision),
                    // use secondary sort: less precise values come first
                    datetime_precision_compare(k1, k2)
                }
                Err(_) => Ordering::Equal,
            };
            match direction {
                SortDirection::Asc | SortDirection::Ascending => cmp,
                SortDirection::Desc | SortDirection::Descending => cmp.reverse(),
            }
        });

        // Write back sorted values
        for (i, (v, _)) in keyed.into_iter().enumerate() {
            values[i] = v;
        }

        Ok(())
    }

    /// Get property value for sorting
    fn get_property_value_for_sort(
        &self,
        value: &CqlValue,
        path: &str,
        _ctx: &EvaluationContext,
    ) -> EvalResult<CqlValue> {
        // Navigate into tuples/structures to get the property value
        match value {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Tuple(t) => t
                .get(path)
                .cloned()
                .ok_or_else(|| EvalError::invalid_property(path, "Tuple")),
            _ => {
                // For other types, return the value itself if sorting by identity
                Ok(value.clone())
            }
        }
    }

    /// Evaluate a Retrieve expression
    ///
    /// Retrieves data from the data provider based on the query criteria
    pub fn eval_retrieve(&self, retrieve: &Retrieve, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        // Evaluate code path if present
        let codes = if let Some(codes_expr) = &retrieve.codes {
            Some(self.evaluate(codes_expr, ctx)?)
        } else {
            None
        };

        // Evaluate date range if present
        let date_range = if let Some(date_expr) = &retrieve.date_range {
            Some(self.evaluate(date_expr, ctx)?)
        } else {
            None
        };

        // Get data provider and perform retrieve
        let provider = ctx.data_provider().ok_or_else(|| {
            EvalError::internal("No data provider configured for retrieve")
        })?;

        // Perform the retrieve
        let results = provider.retrieve(
            &retrieve.data_type,
            ctx.context_type.as_deref(),
            ctx.context_value.as_ref(),
            retrieve.template_id.as_deref(),
            retrieve.code_property.as_deref(),
            codes.as_ref(),
            retrieve.date_property.as_deref(),
            date_range.as_ref(),
        );

        Ok(CqlValue::List(CqlList::from_elements(results)))
    }

    /// Evaluate a Tuple expression
    pub fn eval_tuple(
        &self,
        tuple: &octofhir_cql_elm::TupleExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let mut result_elements = Vec::new();

        if let Some(elems) = &tuple.elements {
            for elem in elems {
                let value = self.evaluate(&elem.value, ctx)?;
                result_elements.push((elem.name.clone(), value));
            }
        }

        Ok(CqlValue::Tuple(octofhir_cql_types::CqlTuple::from_elements(result_elements)))
    }

    /// Evaluate an Instance expression (typed tuple)
    pub fn eval_instance(
        &self,
        instance: &octofhir_cql_elm::InstanceExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let mut result_elements = Vec::new();

        if let Some(elems) = &instance.elements {
            for elem in elems {
                let value = self.evaluate(&elem.value, ctx)?;
                result_elements.push((elem.name.clone(), value));
            }
        }

        // Extract simple type name from qualified name
        let type_name = instance.class_type
            .rsplit('}')
            .next()
            .unwrap_or(&instance.class_type)
            .rsplit('.')
            .next()
            .unwrap_or(&instance.class_type);

        // Handle CQL system types specially
        match type_name {
            "Code" => {
                let code = get_string_element(&result_elements, "code").unwrap_or_default();
                let system = get_string_element(&result_elements, "system").unwrap_or_default();
                let version = get_string_element(&result_elements, "version");
                let display = get_string_element(&result_elements, "display");
                Ok(CqlValue::Code(octofhir_cql_types::CqlCode {
                    code,
                    system,
                    version,
                    display,
                }))
            }
            "Concept" => {
                let codes: Vec<octofhir_cql_types::CqlCode> = result_elements.iter()
                    .filter(|(name, _)| name == "codes")
                    .filter_map(|(_, v)| match v {
                        CqlValue::Code(c) => Some(c.clone()),
                        CqlValue::List(l) => {
                            let codes: Vec<_> = l.elements.iter()
                                .filter_map(|e| match e {
                                    CqlValue::Code(c) => Some(c.clone()),
                                    _ => None,
                                })
                                .collect();
                            if codes.is_empty() { None } else { Some(codes[0].clone()) }
                        }
                        _ => None,
                    })
                    .collect();
                let display = get_string_element(&result_elements, "display");
                Ok(CqlValue::Concept(octofhir_cql_types::CqlConcept {
                    codes: codes.into(),
                    display,
                }))
            }
            "Quantity" => {
                let value = result_elements.iter()
                    .find(|(name, _)| name == "value")
                    .and_then(|(_, v)| match v {
                        CqlValue::Decimal(d) => Some(*d),
                        CqlValue::Integer(i) => Some(rust_decimal::Decimal::from(*i)),
                        _ => None,
                    })
                    .unwrap_or_default();
                let unit = get_string_element(&result_elements, "unit");
                Ok(CqlValue::Quantity(octofhir_cql_types::CqlQuantity { value, unit }))
            }
            _ => {
                // Default: return as Tuple with type information
                result_elements.push(("__type".to_string(), CqlValue::string(&instance.class_type)));
                Ok(CqlValue::Tuple(octofhir_cql_types::CqlTuple::from_elements(result_elements)))
            }
        }
    }

    /// Evaluate a Message expression
    ///
    /// Returns the source value after optionally logging a message
    pub fn eval_message(
        &self,
        message: &octofhir_cql_elm::MessageExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let source = self.evaluate(&message.source, ctx)?;
        let _condition = self.evaluate(&message.condition, ctx)?;
        let _code = self.evaluate(&message.code, ctx)?;
        let _severity = self.evaluate(&message.severity, ctx)?;
        let _msg = self.evaluate(&message.message, ctx)?;

        // In a real implementation, we would log the message here
        // For now, we just return the source

        Ok(source)
    }
}

/// Helper function to extract a string value from tuple elements
fn get_string_element(elements: &[(String, CqlValue)], name: &str) -> Option<String> {
    elements.iter()
        .find(|(n, _)| n == name)
        .and_then(|(_, v)| match v {
            CqlValue::String(s) => Some(s.clone()),
            _ => None,
        })
}

/// Compare DateTime/Date values by precision when cql_compare returns None.
/// Less precise values come first in ascending order.
fn datetime_precision_compare(a: &CqlValue, b: &CqlValue) -> Ordering {
    match (a, b) {
        (CqlValue::DateTime(da), CqlValue::DateTime(db)) => {
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

/// Calculate DateTime precision level (higher = more precise)
fn datetime_precision(dt: &octofhir_cql_types::CqlDateTime) -> u8 {
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
fn date_precision(d: &octofhir_cql_types::CqlDate) -> u8 {
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
    use octofhir_cql_elm::{
        AliasedQuerySource, Element, Expression, LetClause, Literal, ReturnClause, SortClause,
    };

    fn engine() -> CqlEngine {
        CqlEngine::new()
    }

    fn ctx() -> EvaluationContext {
        EvaluationContext::new()
    }

    fn list_expr(values: Vec<i32>) -> Box<Expression> {
        Box::new(Expression::List(octofhir_cql_elm::ListExpression {
            element: Element::default(),
            type_specifier: None,
            elements: Some(
                values
                    .into_iter()
                    .map(|v| {
                        Box::new(Expression::Literal(Literal {
                            element: Element::default(),
                            value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                            value: Some(v.to_string()),
                        }))
                    })
                    .collect(),
            ),
        }))
    }

    fn alias_ref(name: &str) -> Box<Expression> {
        Box::new(Expression::AliasRef(octofhir_cql_elm::AliasRef {
            element: Element::default(),
            name: name.to_string(),
        }))
    }

    #[test]
    fn test_simple_query() {
        let e = engine();
        let mut c = ctx();

        // from [1, 2, 3] X return X
        let query = Query {
            element: Element::default(),
            source: vec![AliasedQuerySource {
                expression: list_expr(vec![1, 2, 3]),
                alias: "X".to_string(),
            }],
            let_clause: None,
            relationship: None,
            where_clause: None,
            return_clause: Some(ReturnClause {
                expression: alias_ref("X"),
                distinct: None,
            }),
            aggregate: None,
            sort: None,
        };

        let result = e.eval_query(&query, &mut c).unwrap();
        match result {
            CqlValue::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list.get(0), Some(&CqlValue::Integer(1)));
                assert_eq!(list.get(1), Some(&CqlValue::Integer(2)));
                assert_eq!(list.get(2), Some(&CqlValue::Integer(3)));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_query_with_where() {
        let e = engine();
        let mut c = ctx();

        // from [1, 2, 3, 4, 5] X where X > 2 return X
        let query = Query {
            element: Element::default(),
            source: vec![AliasedQuerySource {
                expression: list_expr(vec![1, 2, 3, 4, 5]),
                alias: "X".to_string(),
            }],
            let_clause: None,
            relationship: None,
            where_clause: Some(Box::new(Expression::Greater(
                octofhir_cql_elm::BinaryExpression {
                    element: Element::default(),
                    operand: vec![alias_ref("X"), Box::new(Expression::Literal(Literal {
                        element: Element::default(),
                        value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                        value: Some("2".to_string()),
                    }))],
                },
            ))),
            return_clause: Some(ReturnClause {
                expression: alias_ref("X"),
                distinct: None,
            }),
            aggregate: None,
            sort: None,
        };

        let result = e.eval_query(&query, &mut c).unwrap();
        match result {
            CqlValue::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list.get(0), Some(&CqlValue::Integer(3)));
                assert_eq!(list.get(1), Some(&CqlValue::Integer(4)));
                assert_eq!(list.get(2), Some(&CqlValue::Integer(5)));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_query_with_let() {
        let e = engine();
        let mut c = ctx();

        // from [1, 2, 3] X let Y: X + 1 return Y
        let query = Query {
            element: Element::default(),
            source: vec![AliasedQuerySource {
                expression: list_expr(vec![1, 2, 3]),
                alias: "X".to_string(),
            }],
            let_clause: Some(vec![LetClause {
                identifier: "Y".to_string(),
                expression: Box::new(Expression::Add(octofhir_cql_elm::BinaryExpression {
                    element: Element::default(),
                    operand: vec![
                        alias_ref("X"),
                        Box::new(Expression::Literal(Literal {
                            element: Element::default(),
                            value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                            value: Some("1".to_string()),
                        })),
                    ],
                })),
            }]),
            relationship: None,
            where_clause: None,
            return_clause: Some(ReturnClause {
                expression: Box::new(Expression::QueryLetRef(octofhir_cql_elm::QueryLetRef {
                    element: Element::default(),
                    name: "Y".to_string(),
                })),
                distinct: None,
            }),
            aggregate: None,
            sort: None,
        };

        let result = e.eval_query(&query, &mut c).unwrap();
        match result {
            CqlValue::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list.get(0), Some(&CqlValue::Integer(2)));
                assert_eq!(list.get(1), Some(&CqlValue::Integer(3)));
                assert_eq!(list.get(2), Some(&CqlValue::Integer(4)));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_query_with_sort() {
        let e = engine();
        let mut c = ctx();

        // from [3, 1, 2] X return X sort desc
        let query = Query {
            element: Element::default(),
            source: vec![AliasedQuerySource {
                expression: list_expr(vec![3, 1, 2]),
                alias: "X".to_string(),
            }],
            let_clause: None,
            relationship: None,
            where_clause: None,
            return_clause: Some(ReturnClause {
                expression: alias_ref("X"),
                distinct: None,
            }),
            aggregate: None,
            sort: Some(SortClause {
                by: vec![SortByItem {
                    direction: SortDirection::Desc,
                    path: None,
                }],
            }),
        };

        let result = e.eval_query(&query, &mut c).unwrap();
        match result {
            CqlValue::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list.get(0), Some(&CqlValue::Integer(3)));
                assert_eq!(list.get(1), Some(&CqlValue::Integer(2)));
                assert_eq!(list.get(2), Some(&CqlValue::Integer(1)));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_query_with_distinct() {
        let e = engine();
        let mut c = ctx();

        // from [1, 2, 2, 3, 3, 3] X return distinct X
        let query = Query {
            element: Element::default(),
            source: vec![AliasedQuerySource {
                expression: list_expr(vec![1, 2, 2, 3, 3, 3]),
                alias: "X".to_string(),
            }],
            let_clause: None,
            relationship: None,
            where_clause: None,
            return_clause: Some(ReturnClause {
                expression: alias_ref("X"),
                distinct: Some(true),
            }),
            aggregate: None,
            sort: None,
        };

        let result = e.eval_query(&query, &mut c).unwrap();
        match result {
            CqlValue::List(list) => {
                assert_eq!(list.len(), 3);
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_empty_query() {
        let e = engine();
        let mut c = ctx();

        // from [] X return X
        let query = Query {
            element: Element::default(),
            source: vec![AliasedQuerySource {
                expression: list_expr(vec![]),
                alias: "X".to_string(),
            }],
            let_clause: None,
            relationship: None,
            where_clause: None,
            return_clause: None,
            aggregate: None,
            sort: None,
        };

        let result = e.eval_query(&query, &mut c).unwrap();
        match result {
            CqlValue::List(list) => {
                assert_eq!(list.len(), 0);
            }
            _ => panic!("Expected list"),
        }
    }
}
