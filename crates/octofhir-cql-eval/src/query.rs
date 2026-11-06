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
use octofhir_cql_elm::{
    Expression, Query, RelationshipClause, Retrieve, SORT_ELEMENT_ALIAS, SortByItem, SortDirection,
};
use octofhir_cql_types::{CqlList, CqlValue};
use std::cmp::Ordering;

/// One row of a query: the alias bindings of a source combination plus the
/// values of the query's `let` clauses for that combination.
#[derive(Debug, Clone)]
struct Row {
    aliases: Vec<(String, CqlValue)>,
    lets: Vec<(String, CqlValue)>,
}

impl Row {
    /// Push a scope binding this row's aliases and lets.
    fn enter(&self, ctx: &mut EvaluationContext) {
        ctx.push_scope();
        for (alias, value) in &self.aliases {
            ctx.set_alias(alias, value.clone());
        }
        for (name, value) in &self.lets {
            ctx.set_let(name, value.clone());
        }
    }
}

impl CqlEngine {
    /// Evaluate a Query expression
    ///
    /// A query in CQL consists of:
    /// 1. One or more sources (each with an alias)
    /// 2. Optional let clauses (computed values available in every later clause)
    /// 3. Optional relationship clauses (with/without)
    /// 4. Optional where clause (filtering)
    /// 5. Optional return clause (projection)
    /// 6. Optional aggregate clause (aggregation)
    /// 7. Optional sort clause (ordering)
    pub fn eval_query(&self, query: &Query, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        // Step 1: Evaluate all sources
        let sources = self.evaluate_query_sources(query, ctx)?;

        // A singleton source (non-list value) yields a singleton result
        let is_singleton_source = sources.len() == 1 && sources[0].2;

        // Step 2: Generate combinations (cartesian product for multi-source)
        let combinations = self.generate_source_combinations(&sources)?;

        // Step 3: Bind let clauses for each combination
        let mut rows = combinations
            .into_iter()
            .map(|aliases| self.bind_lets(aliases, query, ctx))
            .collect::<EvalResult<Vec<_>>>()?;

        // Step 4: Apply relationship clauses (with/without)
        if let Some(relationships) = &query.relationship {
            for relationship in relationships {
                rows = self.apply_relationship(rows, relationship, ctx)?;
            }
        }

        // Step 5: Apply where clause (filtering)
        if let Some(where_expr) = &query.where_clause {
            rows = self.apply_where_clause(rows, where_expr, ctx)?;
        }

        // Step 6: Aggregate clause replaces return/sort
        if let Some(aggregate) = &query.aggregate {
            return self.apply_aggregate_clause(rows, aggregate, ctx);
        }

        // Step 7: Apply return clause (projection)
        let mut results = if let Some(return_clause) = &query.return_clause {
            self.apply_return_clause(rows, return_clause, ctx)?
        } else {
            // Single source: the aliased value; multi-source: a tuple of aliases
            Self::default_return(rows, query)
        };

        // Step 8: Apply distinct if specified
        if query
            .return_clause
            .as_ref()
            .is_some_and(|r| r.distinct.unwrap_or(false))
        {
            results = self.apply_distinct(results)?;
        }

        // Step 9: Apply sort clause
        if let Some(sort) = &query.sort {
            results = self.apply_sort_clause(results, sort, ctx)?;
        }

        // Step 10: For singleton source, unwrap the result
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

    /// Evaluate the let clauses for one combination; each let sees the
    /// aliases and the lets before it.
    fn bind_lets(
        &self,
        aliases: Vec<(String, CqlValue)>,
        query: &Query,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Row> {
        let mut row = Row {
            aliases,
            lets: Vec::new(),
        };
        let Some(let_clauses) = &query.let_clause else {
            return Ok(row);
        };
        row.enter(ctx);
        let result = let_clauses.iter().try_for_each(|clause| {
            let value = self.evaluate(&clause.expression, ctx)?;
            ctx.set_let(&clause.identifier, value.clone());
            row.lets.push((clause.identifier.clone(), value));
            Ok(())
        });
        ctx.pop_scope();
        result.map(|()| row)
    }

    /// Apply a with/without clause: keep rows for which some (with) or no
    /// (without) related element satisfies the such-that condition.
    fn apply_relationship(
        &self,
        rows: Vec<Row>,
        relationship: &RelationshipClause,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<Row>> {
        let (expression, alias, such_that, keep_matching) = match relationship {
            RelationshipClause::With(w) => (&w.expression, &w.alias, &w.such_that, true),
            RelationshipClause::Without(w) => (&w.expression, &w.alias, &w.such_that, false),
        };

        let related_values: Vec<CqlValue> = match self.evaluate(expression, ctx)? {
            CqlValue::List(list) => list.iter().cloned().collect(),
            CqlValue::Null => vec![],
            other => vec![other],
        };

        let mut result = Vec::new();
        for row in rows {
            row.enter(ctx);
            let mut has_match = false;
            for related in &related_values {
                ctx.set_alias(alias, related.clone());
                match self.evaluate(such_that, ctx) {
                    Ok(condition) if condition.is_true() => {
                        has_match = true;
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        ctx.pop_scope();
                        return Err(e);
                    }
                }
            }
            ctx.pop_scope();

            if has_match == keep_matching {
                result.push(row);
            }
        }

        Ok(result)
    }

    /// Apply where clause filtering
    fn apply_where_clause(
        &self,
        rows: Vec<Row>,
        where_expr: &octofhir_cql_elm::Expression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<Row>> {
        let mut result = Vec::new();

        for row in rows {
            row.enter(ctx);
            let condition = self.evaluate(where_expr, ctx);
            ctx.pop_scope();

            // Include if condition is true (not false or null)
            if condition?.is_true() {
                result.push(row);
            }
        }

        Ok(result)
    }

    /// Apply return clause projection
    fn apply_return_clause(
        &self,
        rows: Vec<Row>,
        return_clause: &octofhir_cql_elm::ReturnClause,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<Vec<CqlValue>> {
        let mut results = Vec::with_capacity(rows.len());

        for row in rows {
            row.enter(ctx);
            let result = self.evaluate(&return_clause.expression, ctx);
            ctx.pop_scope();
            results.push(result?);
        }

        Ok(results)
    }

    /// Default return when no return clause is specified
    fn default_return(rows: Vec<Row>, query: &Query) -> Vec<CqlValue> {
        let single_source = query.source.len() == 1;

        rows.into_iter()
            .map(|row| {
                if single_source {
                    row.aliases
                        .into_iter()
                        .next()
                        .map(|(_, v)| v)
                        .unwrap_or(CqlValue::Null)
                } else {
                    CqlValue::Tuple(octofhir_cql_types::CqlTuple::from_elements(row.aliases))
                }
            })
            .collect()
    }

    /// Apply aggregate clause
    fn apply_aggregate_clause(
        &self,
        rows: Vec<Row>,
        aggregate: &octofhir_cql_elm::AggregateClause,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        // Get initial value
        let mut accumulator = if let Some(starting) = &aggregate.starting {
            self.evaluate(starting, ctx)?
        } else {
            CqlValue::Null
        };

        // Apply distinct if specified
        let rows = if aggregate.distinct.unwrap_or(false) {
            Self::distinct_rows(rows)
        } else {
            rows
        };

        for row in rows {
            row.enter(ctx);
            // The accumulator is visible to the aggregate expression by name
            ctx.set_alias(&aggregate.identifier, accumulator.clone());
            let next = self.evaluate(&aggregate.expression, ctx);
            ctx.pop_scope();
            accumulator = next?;
        }

        Ok(accumulator)
    }

    /// Remove rows whose alias values equal an earlier row's
    fn distinct_rows(rows: Vec<Row>) -> Vec<Row> {
        let mut result: Vec<Row> = Vec::new();

        for row in rows {
            let is_duplicate =
                result.iter().any(|existing| {
                    existing.aliases.len() == row.aliases.len()
                        && existing.aliases.iter().zip(row.aliases.iter()).all(
                            |((_, v1), (_, v2))| {
                                crate::operators::comparison::cql_equal(v1, v2)
                                    .unwrap_or(Some(false))
                                    .unwrap_or(false)
                            },
                        )
                });

            if !is_duplicate {
                result.push(row);
            }
        }

        result
    }

    /// Apply distinct to results
    fn apply_distinct(&self, values: Vec<CqlValue>) -> EvalResult<Vec<CqlValue>> {
        let mut result = Vec::new();

        for value in values {
            let is_duplicate = result.iter().any(|existing| {
                crate::operators::comparison::cql_equal(existing, &value)
                    .unwrap_or(Some(false))
                    .unwrap_or(false)
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
            let key = if let Some(expression) = &sort_item.expression {
                // Sort by expression, evaluated with the element bound to `$this`
                ctx.push_scope();
                ctx.set_alias(SORT_ELEMENT_ALIAS, value.clone());
                let key = self.evaluate(expression, ctx);
                ctx.pop_scope();
                key?
            } else if let Some(path) = &sort_item.path {
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
        if let Some((head, rest)) = path.split_once('.') {
            let inner = self.get_property_value_for_sort(value, head, _ctx)?;
            return self.get_property_value_for_sort(&inner, rest, _ctx);
        }
        match value {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::Tuple(t) => Ok(t.get(path).cloned().unwrap_or(CqlValue::Null)),
            CqlValue::Interval(_)
            | CqlValue::Quantity(_)
            | CqlValue::Code(_)
            | CqlValue::Concept(_)
            | CqlValue::Ratio(_) => self.get_property_value(value, path, _ctx),
            _ => {
                // For other types, return the value itself if sorting by identity
                Ok(value.clone())
            }
        }
    }

    /// Evaluate a Retrieve expression
    ///
    /// Retrieves data from the data provider based on the query criteria
    pub fn eval_retrieve(
        &self,
        retrieve: &Retrieve,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        // Evaluate the terminology filter. A value set is passed to the data
        // provider as its expansion when the terminology provider can expand
        // it, otherwise as the value set reference `{ name, id, version }`.
        let codes = match &retrieve.codes {
            Some(codes_expr) => {
                let value = self.evaluate(codes_expr, ctx)?;
                Some(match codes_expr.as_ref() {
                    Expression::ValueSetRef(_) => self.expand_value_set_value(value, ctx),
                    _ => value,
                })
            }
            None => None,
        };

        // Evaluate date range if present
        let date_range = if let Some(date_expr) = &retrieve.date_range {
            Some(self.evaluate(date_expr, ctx)?)
        } else {
            None
        };

        // Get data provider and perform retrieve
        let provider = ctx
            .data_provider()
            .ok_or_else(|| EvalError::internal("No data provider configured for retrieve"))?;

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

    /// Expand a value set reference value into a list of codes using the
    /// terminology provider; returns the reference unchanged if it cannot be
    /// expanded.
    fn expand_value_set_value(&self, value: CqlValue, ctx: &EvaluationContext) -> CqlValue {
        let expansion = crate::references::TerminologyId::from_value(&value).and_then(|vs| {
            ctx.terminology_provider()
                .and_then(|provider| provider.expand_value_set(&vs.id))
        });
        match expansion {
            Some(codes) => CqlValue::List(CqlList::from_elements(codes)),
            None => value,
        }
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

        Ok(CqlValue::Tuple(
            octofhir_cql_types::CqlTuple::from_elements(result_elements),
        ))
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
        let type_name = instance
            .class_type
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
                let codes: Vec<octofhir_cql_types::CqlCode> = result_elements
                    .iter()
                    .filter(|(name, _)| name == "codes")
                    .filter_map(|(_, v)| match v {
                        CqlValue::Code(c) => Some(c.clone()),
                        CqlValue::List(l) => {
                            let codes: Vec<_> = l
                                .elements
                                .iter()
                                .filter_map(|e| match e {
                                    CqlValue::Code(c) => Some(c.clone()),
                                    _ => None,
                                })
                                .collect();
                            if codes.is_empty() {
                                None
                            } else {
                                Some(codes[0].clone())
                            }
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
                let value = result_elements
                    .iter()
                    .find(|(name, _)| name == "value")
                    .and_then(|(_, v)| match v {
                        CqlValue::Decimal(d) => Some(d.clone()),
                        CqlValue::Integer(i) => Some(bigdecimal::BigDecimal::from(*i)),
                        _ => None,
                    })
                    .unwrap_or_default();
                let unit = get_string_element(&result_elements, "unit");
                Ok(CqlValue::Quantity(octofhir_cql_types::CqlQuantity {
                    value,
                    unit,
                }))
            }
            _ => {
                // Default: return as Tuple with type information
                result_elements
                    .push(("__type".to_string(), CqlValue::string(&instance.class_type)));
                Ok(CqlValue::Tuple(
                    octofhir_cql_types::CqlTuple::from_elements(result_elements),
                ))
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
        let condition = self.evaluate(&message.condition, ctx)?;
        let code = self.evaluate(&message.code, ctx)?;
        let severity = self.evaluate(&message.severity, ctx)?;
        let text = self.evaluate(&message.message, ctx)?;

        if condition == CqlValue::Boolean(true) {
            let library = ctx
                .current_library()
                .map_or_else(String::new, |library| library.identifier.id.clone());
            if let Some(trace) = ctx.trace_mut() {
                let part = |value: &CqlValue| match value {
                    CqlValue::String(text) => text.clone(),
                    _ => String::new(),
                };
                let line = format!("{} {}: {}", part(&severity), part(&code), part(&text));
                trace.message(&library, line.trim().to_owned());
            }
        }

        Ok(source)
    }
}

/// Helper function to extract a string value from tuple elements
fn get_string_element(elements: &[(String, CqlValue)], name: &str) -> Option<String> {
    elements
        .iter()
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
                    operand: vec![
                        alias_ref("X"),
                        Box::new(Expression::Literal(Literal {
                            element: Element::default(),
                            value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                            value: Some("2".to_string()),
                        })),
                    ],
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
                    expression: None,
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
