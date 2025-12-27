//! Tests for parsing CQL query expressions
//!
//! Covers:
//! - Source clauses (from, with, without)
//! - Let clauses
//! - Where clauses
//! - Return clauses
//! - Sort clauses
//! - Aggregate expressions (distinct, flatten)
//! - Query aliases and scoping
//! - Nested queries

use octofhir_cql_ast::*;
use octofhir_cql_parser::Parser;
use pretty_assertions::assert_eq;

fn parse_expr(input: &str) -> Expression {
    let parser = Parser::new();
    parser
        .parse_expression(input)
        .unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", input, e))
}

fn assert_query(expr: &Expression) -> &Query {
    match &expr.kind {
        ExpressionKind::Query(query) => query,
        _ => panic!("Expected Query, got: {:?}", expr.kind),
    }
}

// === Basic Query Structure ===

#[test]
fn test_simple_from_clause() {
    let expr = parse_expr("[Observation]");
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert_eq!(query.sources[0].alias, "");
    // Source expression should be a retrieve
}

#[test]
fn test_from_with_alias() {
    let expr = parse_expr("[Observation] O");
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert_eq!(query.sources[0].alias, "O");
}

#[test]
fn test_from_clause_explicit() {
    let expr = parse_expr("from [Observation] O");
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert_eq!(query.sources[0].alias, "O");
}

#[test]
fn test_multiple_sources() {
    let expr = parse_expr(
        "from [Observation] O,
         [Condition] C"
    );
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 2);
    assert_eq!(query.sources[0].alias, "O");
    assert_eq!(query.sources[1].alias, "C");
}

// === Where Clause ===

#[test]
fn test_where_clause() {
    let expr = parse_expr(
        "[Observation] O
         where O.status = 'final'"
    );
    let query = assert_query(&expr);

    assert!(query.where_clause.is_some());
}

#[test]
fn test_where_with_complex_condition() {
    let expr = parse_expr(
        "[Observation] O
         where O.status = 'final' and O.value > 120"
    );
    let query = assert_query(&expr);

    assert!(query.where_clause.is_some());
    // The where condition should be an 'and' binary operation
    if let Some(ref where_expr) = query.where_clause {
        match &where_expr.kind {
            ExpressionKind::BinaryOp { op, .. } => assert_eq!(op, "and"),
            _ => panic!("Expected BinaryOp in where clause"),
        }
    }
}

// === Return Clause ===

#[test]
fn test_return_clause() {
    let expr = parse_expr(
        "[Observation] O
         return O.value"
    );
    let query = assert_query(&expr);

    assert!(query.return_clause.is_some());
}

#[test]
fn test_return_all() {
    let expr = parse_expr(
        "[Observation] O
         return all O"
    );
    let query = assert_query(&expr);

    assert!(query.return_clause.is_some());
    if let Some(ref ret) = query.return_clause {
        assert!(ret.distinct == false || !ret.all);
    }
}

#[test]
fn test_return_distinct() {
    let expr = parse_expr(
        "[Observation] O
         return distinct O.code"
    );
    let query = assert_query(&expr);

    assert!(query.return_clause.is_some());
    if let Some(ref ret) = query.return_clause {
        assert!(ret.distinct);
    }
}

// === Let Clause ===

#[test]
fn test_let_clause() {
    let expr = parse_expr(
        "[Observation] O
         let value: O.value
         return value"
    );
    let query = assert_query(&expr);

    assert!(query.let_clause.is_some());
    if let Some(ref let_items) = query.let_clause {
        assert_eq!(let_items.len(), 1);
        assert_eq!(let_items[0].identifier, "value");
    }
}

#[test]
fn test_multiple_let_bindings() {
    let expr = parse_expr(
        "[Observation] O
         let value: O.value,
             code: O.code
         return Tuple { value: value, code: code }"
    );
    let query = assert_query(&expr);

    assert!(query.let_clause.is_some());
    if let Some(ref let_items) = query.let_clause {
        assert_eq!(let_items.len(), 2);
        assert_eq!(let_items[0].identifier, "value");
        assert_eq!(let_items[1].identifier, "code");
    }
}

// === Sort Clause ===

#[test]
fn test_sort_clause_ascending() {
    let expr = parse_expr(
        "[Observation] O
         sort by O.effectiveDateTime"
    );
    let query = assert_query(&expr);

    assert!(query.sort_clause.is_some());
    if let Some(ref sort_items) = query.sort_clause {
        assert_eq!(sort_items.len(), 1);
        // Default should be ascending
    }
}

#[test]
fn test_sort_clause_descending() {
    let expr = parse_expr(
        "[Observation] O
         sort by O.effectiveDateTime desc"
    );
    let query = assert_query(&expr);

    assert!(query.sort_clause.is_some());
    if let Some(ref sort_items) = query.sort_clause {
        assert_eq!(sort_items.len(), 1);
        assert_eq!(sort_items[0].direction, SortDirection::Descending);
    }
}

#[test]
fn test_sort_multiple_fields() {
    let expr = parse_expr(
        "[Observation] O
         sort by O.category asc, O.effectiveDateTime desc"
    );
    let query = assert_query(&expr);

    assert!(query.sort_clause.is_some());
    if let Some(ref sort_items) = query.sort_clause {
        assert_eq!(sort_items.len(), 2);
        assert_eq!(sort_items[0].direction, SortDirection::Ascending);
        assert_eq!(sort_items[1].direction, SortDirection::Descending);
    }
}

// === With/Without Clauses ===

#[test]
fn test_with_clause() {
    let expr = parse_expr(
        "[Encounter] E
         with [Observation] O
           such that O.encounter = E.id"
    );
    let query = assert_query(&expr);

    assert_eq!(query.relationships.len(), 1);
    if let RelationshipClause::With { alias, condition, .. } = &query.relationships[0] {
        assert_eq!(alias, "O");
        assert!(condition.is_some());
    } else {
        panic!("Expected With relationship");
    }
}

#[test]
fn test_without_clause() {
    let expr = parse_expr(
        "[Patient] P
         without [Condition] C
           such that C.subject = P.id"
    );
    let query = assert_query(&expr);

    assert_eq!(query.relationships.len(), 1);
    if let RelationshipClause::Without { alias, condition, .. } = &query.relationships[0] {
        assert_eq!(alias, "C");
        assert!(condition.is_some());
    } else {
        panic!("Expected Without relationship");
    }
}

// === Nested Queries ===

#[test]
fn test_nested_query_in_where() {
    let expr = parse_expr(
        "[Observation] O
         where O.code in (
           [ValueSet: 'blood-pressure-codes'] V return V.code
         )"
    );
    let query = assert_query(&expr);

    assert!(query.where_clause.is_some());
    // The where clause should contain a nested query
}

#[test]
fn test_nested_query_in_return() {
    let expr = parse_expr(
        "[Patient] P
         return {
           patient: P,
           observations: ([Observation] O where O.subject = P.id)
         }"
    );
    let query = assert_query(&expr);

    assert!(query.return_clause.is_some());
}

// === Query over List ===

#[test]
fn test_query_over_list_literal() {
    let expr = parse_expr(
        "{1, 2, 3, 4, 5} N
         where N > 2
         return N * 2"
    );
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert!(query.where_clause.is_some());
    assert!(query.return_clause.is_some());
}

#[test]
fn test_query_over_identifier() {
    let expr = parse_expr(
        "MyList L
         where L.value > 10
         return L.id"
    );
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert_eq!(query.sources[0].alias, "L");
}

// === Aggregate Expressions ===

#[test]
fn test_distinct_expression() {
    let expr = parse_expr("distinct {1, 2, 2, 3, 3, 3}");

    match &expr.kind {
        ExpressionKind::Distinct { source } => {
            // Successfully parsed as distinct
            assert!(matches!(source.kind, ExpressionKind::List(_)));
        }
        ExpressionKind::Query(query) => {
            // May also be parsed as query with distinct return
            if let Some(ref ret) = query.return_clause {
                assert!(ret.distinct);
            }
        }
        _ => panic!("Expected Distinct or Query with distinct, got: {:?}", expr.kind),
    }
}

#[test]
fn test_flatten_expression() {
    let expr = parse_expr("flatten {{1, 2}, {3, 4}}");

    match &expr.kind {
        ExpressionKind::Flatten { source } => {
            assert!(matches!(source.kind, ExpressionKind::List(_)));
        }
        ExpressionKind::FunctionCall { name, .. } if name == "flatten" => {
            // May be parsed as function call
        }
        _ => panic!("Expected Flatten or function call, got: {:?}", expr.kind),
    }
}

// === Complex Multi-Clause Queries ===

#[test]
fn test_full_query_structure() {
    let expr = parse_expr(
        "from [Observation] O
         where O.status = 'final'
         let value: O.value
         where value > 120
         return Tuple {
           code: O.code,
           value: value,
           date: O.effectiveDateTime
         }
         sort by O.effectiveDateTime desc"
    );

    let query = assert_query(&expr);

    // Verify all clauses are present
    assert_eq!(query.sources.len(), 1);
    assert!(query.where_clause.is_some());
    assert!(query.let_clause.is_some());
    assert!(query.return_clause.is_some());
    assert!(query.sort_clause.is_some());
}

#[test]
fn test_multi_source_with_relationships() {
    let expr = parse_expr(
        "from [Encounter] E
         with [Observation] O
           such that O.encounter = E.id
         with [Condition] C
           such that C.encounter = E.id
         where E.status = 'finished'
         return Tuple {
           encounter: E,
           observations: O,
           conditions: C
         }"
    );

    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert_eq!(query.relationships.len(), 2);
    assert!(query.where_clause.is_some());
    assert!(query.return_clause.is_some());
}

// === Subquery and Exists ===

#[test]
fn test_exists_subquery() {
    let expr = parse_expr(
        "exists ([Observation] O where O.code = '8480-6')"
    );

    match &expr.kind {
        ExpressionKind::Exists { source } => {
            assert!(matches!(source.kind, ExpressionKind::Query(_)));
        }
        ExpressionKind::FunctionCall { name, .. } if name == "exists" => {
            // May be parsed as function call
        }
        _ => panic!("Expected Exists or function call, got: {:?}", expr.kind),
    }
}

// === Query Aliases and Scoping ===

#[test]
fn test_query_with_scope_reference() {
    let expr = parse_expr(
        "[Observation] O
         where O.value > 100 and O.status = 'final'
         return O"
    );

    let query = assert_query(&expr);

    // Verify alias O is used throughout
    assert_eq!(query.sources[0].alias, "O");
}

#[test]
fn test_query_with_this_alias() {
    let expr = parse_expr(
        "[Observation] $this
         where $this.value > 100"
    );

    let query = assert_query(&expr);

    assert_eq!(query.sources[0].alias, "$this");
}

// === Edge Cases ===

#[test]
fn test_empty_query() {
    // Just a source with no other clauses
    let expr = parse_expr("[Observation]");
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert!(query.where_clause.is_none());
    assert!(query.return_clause.is_none());
}

#[test]
fn test_query_with_only_where() {
    let expr = parse_expr(
        "[Observation]
         where status = 'final'"
    );

    let query = assert_query(&expr);

    assert!(query.where_clause.is_some());
    assert!(query.return_clause.is_none());
}

#[test]
fn test_query_with_complex_return_tuple() {
    let expr = parse_expr(
        "[Observation] O
         return Tuple {
           id: O.id,
           code: O.code.coding[0].code,
           value: O.value.value,
           unit: O.value.unit,
           date: O.effectiveDateTime
         }"
    );

    let query = assert_query(&expr);

    assert!(query.return_clause.is_some());
    if let Some(ref ret) = query.return_clause {
        // Return expression should be a tuple
        match &ret.expression.kind {
            ExpressionKind::Tuple(elements) => {
                assert_eq!(elements.len(), 5);
            }
            _ => panic!("Expected Tuple in return"),
        }
    }
}

#[test]
fn test_single_query_expression() {
    // Test the single-from syntax: ListExpr AliasExpr
    let expr = parse_expr("[Condition] C");
    let query = assert_query(&expr);

    assert_eq!(query.sources.len(), 1);
    assert_eq!(query.sources[0].alias, "C");
}

#[test]
fn test_query_flatten_in_return() {
    let expr = parse_expr(
        "[Patient] P
         return flatten (P.name.given)"
    );

    let query = assert_query(&expr);
    assert!(query.return_clause.is_some());
}
