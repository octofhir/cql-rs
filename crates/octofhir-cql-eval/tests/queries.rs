//! Query Evaluation Tests
//!
//! Comprehensive tests for CQL query evaluation including:
//! - Single-source queries
//! - Multi-source queries (cartesian product)
//! - Let clauses
//! - With/Without relationship clauses
//! - Where filtering
//! - Return projection
//! - Sort (asc, desc, by expression)
//! - Aggregate clause
//! - Distinct return

use octofhir_cql_eval::{CqlEngine, EvaluationContext};
use octofhir_cql_elm::{
    AggregateClause, AliasedQuerySource, BinaryExpression, Element, Expression, LetClause,
    ListExpression, Literal, NullLiteral, Query, QueryLetRef, ReturnClause, SortByItem,
    SortClause, SortDirection, TupleElementExpression, TupleExpression, WithClause, WithoutClause,
    RelationshipClause, AliasRef,
};
use octofhir_cql_types::{CqlList, CqlTuple, CqlType, CqlValue};

// ============================================================================
// Test Helpers
// ============================================================================

fn engine() -> CqlEngine {
    CqlEngine::new()
}

fn ctx() -> EvaluationContext {
    EvaluationContext::new()
}

fn int_expr(i: i32) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
        value: Some(i.to_string()),
    }))
}

fn string_expr(s: &str) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
        value: Some(s.to_string()),
    }))
}

fn bool_expr(b: bool) -> Box<Expression> {
    Box::new(Expression::Literal(Literal {
        element: Element::default(),
        value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
        value: Some(b.to_string()),
    }))
}

fn null_expr() -> Box<Expression> {
    Box::new(Expression::Null(NullLiteral {
        element: Element::default(),
    }))
}

fn alias_ref(name: &str) -> Box<Expression> {
    Box::new(Expression::AliasRef(AliasRef {
        element: Element::default(),
        name: name.to_string(),
    }))
}

fn let_ref(name: &str) -> Box<Expression> {
    Box::new(Expression::QueryLetRef(QueryLetRef {
        element: Element::default(),
        name: name.to_string(),
    }))
}

fn list_expr(values: Vec<i32>) -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(values.into_iter().map(|v| int_expr(v)).collect()),
    }))
}

fn string_list_expr(values: Vec<&str>) -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(values.into_iter().map(|v| string_expr(v)).collect()),
    }))
}

fn empty_list_expr() -> Box<Expression> {
    Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: None,
    }))
}

fn make_source(alias: &str, expr: Box<Expression>) -> AliasedQuerySource {
    AliasedQuerySource {
        expression: expr,
        alias: alias.to_string(),
    }
}

fn make_binary(op: fn(BinaryExpression) -> Expression, left: Box<Expression>, right: Box<Expression>) -> Box<Expression> {
    Box::new(op(BinaryExpression {
        element: Element::default(),
        operand: vec![left, right],
    }))
}

fn assert_list_values(result: &CqlValue, expected: &[i32]) {
    match result {
        CqlValue::List(list) => {
            assert_eq!(
                list.len(),
                expected.len(),
                "List length mismatch: got {:?}",
                list
            );
            for (i, exp) in expected.iter().enumerate() {
                assert_eq!(
                    list.get(i),
                    Some(&CqlValue::Integer(*exp)),
                    "Element {} mismatch",
                    i
                );
            }
        }
        _ => panic!("Expected List, got: {:?}", result),
    }
}

fn assert_list_len(result: &CqlValue, expected_len: usize) {
    match result {
        CqlValue::List(list) => {
            assert_eq!(list.len(), expected_len, "List length mismatch");
        }
        _ => panic!("Expected List, got: {:?}", result),
    }
}

// ============================================================================
// Simple Query Tests
// ============================================================================

#[test]
fn test_simple_query_return_source() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
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
    assert_list_values(&result, &[1, 2, 3]);
}

#[test]
fn test_query_default_return() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X (no explicit return)
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[1, 2, 3]);
}

#[test]
fn test_query_empty_source() {
    let e = engine();
    let mut c = ctx();

    // from [] X return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", empty_list_expr())],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_len(&result, 0);
}

#[test]
fn test_query_null_source() {
    let e = engine();
    let mut c = ctx();

    // from null X return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", null_expr())],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_len(&result, 0);
}

#[test]
fn test_query_single_value_source() {
    let e = engine();
    let mut c = ctx();

    // from 42 X return X (single value treated as singleton list)
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", int_expr(42))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[42]);
}

// ============================================================================
// Where Clause Tests
// ============================================================================

#[test]
fn test_query_where_greater_than() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4, 5] X where X > 2 return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4, 5]))],
        let_clause: None,
        relationship: None,
        where_clause: Some(make_binary(Expression::Greater, alias_ref("X"), int_expr(2))),
        return_clause: Some(ReturnClause {
            expression: alias_ref("X"),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[3, 4, 5]);
}

#[test]
fn test_query_where_equals() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 2, 1] X where X = 2 return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 2, 1]))],
        let_clause: None,
        relationship: None,
        where_clause: Some(make_binary(Expression::Equal, alias_ref("X"), int_expr(2))),
        return_clause: Some(ReturnClause {
            expression: alias_ref("X"),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[2, 2]);
}

#[test]
fn test_query_where_filters_all() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X where X > 10 return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: None,
        relationship: None,
        where_clause: Some(make_binary(Expression::Greater, alias_ref("X"), int_expr(10))),
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_len(&result, 0);
}

#[test]
fn test_query_where_with_compound_condition() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4, 5] X where X >= 2 and X <= 4 return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4, 5]))],
        let_clause: None,
        relationship: None,
        where_clause: Some(make_binary(
            Expression::And,
            make_binary(Expression::GreaterOrEqual, alias_ref("X"), int_expr(2)),
            make_binary(Expression::LessOrEqual, alias_ref("X"), int_expr(4)),
        )),
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[2, 3, 4]);
}

// ============================================================================
// Let Clause Tests
// ============================================================================

#[test]
fn test_query_with_let_clause() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X let Y: X + 10 return Y
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: Some(vec![LetClause {
            identifier: "Y".to_string(),
            expression: make_binary(Expression::Add, alias_ref("X"), int_expr(10)),
        }]),
        relationship: None,
        where_clause: None,
        return_clause: Some(ReturnClause {
            expression: let_ref("Y"),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[11, 12, 13]);
}

#[test]
fn test_query_with_multiple_let_clauses() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X let Y: X + 1, Z: X * 2 return Y + Z
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: Some(vec![
            LetClause {
                identifier: "Y".to_string(),
                expression: make_binary(Expression::Add, alias_ref("X"), int_expr(1)),
            },
            LetClause {
                identifier: "Z".to_string(),
                expression: make_binary(Expression::Multiply, alias_ref("X"), int_expr(2)),
            },
        ]),
        relationship: None,
        where_clause: None,
        return_clause: Some(ReturnClause {
            // Return Y + Z: (X+1) + (X*2)
            // X=1: 2 + 2 = 4
            // X=2: 3 + 4 = 7
            // X=3: 4 + 6 = 10
            expression: make_binary(Expression::Add, let_ref("Y"), let_ref("Z")),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[4, 7, 10]);
}

#[test]
fn test_query_let_used_in_where() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4, 5] X let Double: X * 2 where Double > 5 return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4, 5]))],
        let_clause: Some(vec![LetClause {
            identifier: "Double".to_string(),
            expression: make_binary(Expression::Multiply, alias_ref("X"), int_expr(2)),
        }]),
        relationship: None,
        where_clause: Some(make_binary(Expression::Greater, let_ref("Double"), int_expr(5))),
        return_clause: Some(ReturnClause {
            expression: alias_ref("X"),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // X=3: Double=6 > 5 -> include
    // X=4: Double=8 > 5 -> include
    // X=5: Double=10 > 5 -> include
    assert_list_values(&result, &[3, 4, 5]);
}

// ============================================================================
// Return Clause Tests
// ============================================================================

#[test]
fn test_query_return_computed_value() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X return X * X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: Some(ReturnClause {
            expression: make_binary(Expression::Multiply, alias_ref("X"), alias_ref("X")),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[1, 4, 9]);
}

#[test]
fn test_query_return_distinct() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 2, 3, 3, 3] X return distinct X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 2, 3, 3, 3]))],
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
    assert_list_values(&result, &[1, 2, 3]);
}

#[test]
fn test_query_return_distinct_computed() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4] X return distinct (X mod 2)
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: Some(ReturnClause {
            expression: make_binary(Expression::Modulo, alias_ref("X"), int_expr(2)),
            distinct: Some(true),
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // 1%2=1, 2%2=0, 3%2=1, 4%2=0 -> distinct [1, 0]
    assert_list_len(&result, 2);
}

// ============================================================================
// Sort Clause Tests
// ============================================================================

#[test]
fn test_query_sort_ascending() {
    let e = engine();
    let mut c = ctx();

    // from [3, 1, 4, 1, 5, 9, 2] X return X sort asc
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![3, 1, 4, 1, 5, 9, 2]))],
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
                direction: SortDirection::Asc,
                path: None,
            }],
        }),
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[1, 1, 2, 3, 4, 5, 9]);
}

#[test]
fn test_query_sort_descending() {
    let e = engine();
    let mut c = ctx();

    // from [3, 1, 4, 1, 5, 9, 2] X return X sort desc
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![3, 1, 4, 1, 5, 9, 2]))],
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
    assert_list_values(&result, &[9, 5, 4, 3, 2, 1, 1]);
}

#[test]
fn test_query_sort_ascending_spelled_out() {
    let e = engine();
    let mut c = ctx();

    // from [3, 1, 2] X return X sort ascending
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![3, 1, 2]))],
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
                direction: SortDirection::Ascending,
                path: None,
            }],
        }),
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[1, 2, 3]);
}

#[test]
fn test_query_sort_already_sorted() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X return X sort asc
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: Some(SortClause {
            by: vec![SortByItem {
                direction: SortDirection::Asc,
                path: None,
            }],
        }),
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[1, 2, 3]);
}

#[test]
fn test_query_sort_by_path() {
    let e = engine();
    let mut c = ctx();

    // Create tuples with 'value' field
    // from [{value: 3}, {value: 1}, {value: 2}] X return X sort by value asc
    let tuples = Box::new(Expression::List(ListExpression {
        element: Element::default(),
        type_specifier: None,
        elements: Some(vec![
            Box::new(Expression::Tuple(TupleExpression {
                element: Element::default(),
                elements: Some(vec![TupleElementExpression {
                    name: "value".to_string(),
                    value: int_expr(3),
                }]),
            })),
            Box::new(Expression::Tuple(TupleExpression {
                element: Element::default(),
                elements: Some(vec![TupleElementExpression {
                    name: "value".to_string(),
                    value: int_expr(1),
                }]),
            })),
            Box::new(Expression::Tuple(TupleExpression {
                element: Element::default(),
                elements: Some(vec![TupleElementExpression {
                    name: "value".to_string(),
                    value: int_expr(2),
                }]),
            })),
        ]),
    }));

    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", tuples)],
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
                direction: SortDirection::Asc,
                path: Some("value".to_string()),
            }],
        }),
    };

    let result = e.eval_query(&query, &mut c).unwrap();

    // Verify sorted by value
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 3);
        if let Some(CqlValue::Tuple(t)) = list.get(0) {
            assert_eq!(t.get("value"), Some(&CqlValue::Integer(1)));
        } else {
            panic!("Expected Tuple at index 0");
        }
        if let Some(CqlValue::Tuple(t)) = list.get(1) {
            assert_eq!(t.get("value"), Some(&CqlValue::Integer(2)));
        } else {
            panic!("Expected Tuple at index 1");
        }
        if let Some(CqlValue::Tuple(t)) = list.get(2) {
            assert_eq!(t.get("value"), Some(&CqlValue::Integer(3)));
        } else {
            panic!("Expected Tuple at index 2");
        }
    } else {
        panic!("Expected List");
    }
}

// ============================================================================
// Multi-Source Query Tests (Cartesian Product)
// ============================================================================

#[test]
fn test_query_multi_source_cartesian_product() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2] X, [10, 20] Y
    // Default return creates tuples with all source values
    let query = Query {
        element: Element::default(),
        source: vec![
            make_source("X", list_expr(vec![1, 2])),
            make_source("Y", list_expr(vec![10, 20])),
        ],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();

    // Should have 2 * 2 = 4 combinations
    if let CqlValue::List(list) = result {
        assert_eq!(list.len(), 4);
        // Combinations: (1,10), (1,20), (2,10), (2,20)
    } else {
        panic!("Expected List");
    }
}

#[test]
fn test_query_multi_source_with_return() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2] X, [10, 20] Y return X + Y
    let query = Query {
        element: Element::default(),
        source: vec![
            make_source("X", list_expr(vec![1, 2])),
            make_source("Y", list_expr(vec![10, 20])),
        ],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: Some(ReturnClause {
            expression: make_binary(Expression::Add, alias_ref("X"), alias_ref("Y")),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // (1+10, 1+20, 2+10, 2+20) = (11, 21, 12, 22)
    assert_list_values(&result, &[11, 21, 12, 22]);
}

#[test]
fn test_query_multi_source_with_where() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2] X, [10, 20] Y where X + Y > 20 return X + Y
    let query = Query {
        element: Element::default(),
        source: vec![
            make_source("X", list_expr(vec![1, 2])),
            make_source("Y", list_expr(vec![10, 20])),
        ],
        let_clause: None,
        relationship: None,
        where_clause: Some(make_binary(
            Expression::Greater,
            make_binary(Expression::Add, alias_ref("X"), alias_ref("Y")),
            int_expr(20),
        )),
        return_clause: Some(ReturnClause {
            expression: make_binary(Expression::Add, alias_ref("X"), alias_ref("Y")),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // (1+10=11, 1+20=21, 2+10=12, 2+20=22) -> filter > 20 -> (21, 22)
    assert_list_values(&result, &[21, 22]);
}

#[test]
fn test_query_multi_source_empty() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2] X, [] Y
    // Cartesian product with empty list is empty
    let query = Query {
        element: Element::default(),
        source: vec![
            make_source("X", list_expr(vec![1, 2])),
            make_source("Y", empty_list_expr()),
        ],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_len(&result, 0);
}

// ============================================================================
// With Clause Tests
// ============================================================================

#[test]
fn test_query_with_clause() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4, 5] X with [2, 4, 6] Y such that X = Y return X
    // Returns X values that have a matching Y
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4, 5]))],
        let_clause: None,
        relationship: Some(vec![RelationshipClause::With(WithClause {
            expression: list_expr(vec![2, 4, 6]),
            alias: "Y".to_string(),
            such_that: make_binary(Expression::Equal, alias_ref("X"), alias_ref("Y")),
        })]),
        where_clause: None,
        return_clause: Some(ReturnClause {
            expression: alias_ref("X"),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // X=2 matches Y=2, X=4 matches Y=4
    assert_list_values(&result, &[2, 4]);
}

#[test]
fn test_query_with_clause_no_matches() {
    let e = engine();
    let mut c = ctx();

    // from [1, 3, 5] X with [2, 4, 6] Y such that X = Y return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 3, 5]))],
        let_clause: None,
        relationship: Some(vec![RelationshipClause::With(WithClause {
            expression: list_expr(vec![2, 4, 6]),
            alias: "Y".to_string(),
            such_that: make_binary(Expression::Equal, alias_ref("X"), alias_ref("Y")),
        })]),
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_len(&result, 0);
}

// ============================================================================
// Without Clause Tests
// ============================================================================

#[test]
fn test_query_without_clause() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4, 5] X without [2, 4, 6] Y such that X = Y return X
    // Returns X values that do NOT have a matching Y
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4, 5]))],
        let_clause: None,
        relationship: Some(vec![RelationshipClause::Without(WithoutClause {
            expression: list_expr(vec![2, 4, 6]),
            alias: "Y".to_string(),
            such_that: make_binary(Expression::Equal, alias_ref("X"), alias_ref("Y")),
        })]),
        where_clause: None,
        return_clause: Some(ReturnClause {
            expression: alias_ref("X"),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // X=1, X=3, X=5 have no matching Y
    assert_list_values(&result, &[1, 3, 5]);
}

#[test]
fn test_query_without_clause_empty_related() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X without [] Y such that X = Y return X
    // All values pass since there's nothing to match against
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: None,
        relationship: Some(vec![RelationshipClause::Without(WithoutClause {
            expression: empty_list_expr(),
            alias: "Y".to_string(),
            such_that: make_binary(Expression::Equal, alias_ref("X"), alias_ref("Y")),
        })]),
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[1, 2, 3]);
}

// ============================================================================
// Aggregate Clause Tests
// ============================================================================

#[test]
fn test_query_aggregate_sum() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4, 5] X aggregate Sum starting 0: Sum + X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4, 5]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: Some(AggregateClause {
            identifier: "Sum".to_string(),
            starting: Some(int_expr(0)),
            expression: make_binary(Expression::Add,
                Box::new(Expression::OperandRef(octofhir_cql_elm::OperandRef {
                    element: Element::default(),
                    name: "Sum".to_string(),
                })),
                alias_ref("X"),
            ),
            distinct: None,
        }),
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(15)); // 1+2+3+4+5 = 15
}

#[test]
fn test_query_aggregate_product() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4] X aggregate Product starting 1: Product * X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: Some(AggregateClause {
            identifier: "Product".to_string(),
            starting: Some(int_expr(1)),
            expression: make_binary(Expression::Multiply,
                Box::new(Expression::OperandRef(octofhir_cql_elm::OperandRef {
                    element: Element::default(),
                    name: "Product".to_string(),
                })),
                alias_ref("X"),
            ),
            distinct: None,
        }),
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(24)); // 1*2*3*4 = 24
}

#[test]
fn test_query_aggregate_count() {
    let e = engine();
    let mut c = ctx();

    // from [10, 20, 30] X aggregate Count starting 0: Count + 1
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![10, 20, 30]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: Some(AggregateClause {
            identifier: "Count".to_string(),
            starting: Some(int_expr(0)),
            expression: make_binary(Expression::Add,
                Box::new(Expression::OperandRef(octofhir_cql_elm::OperandRef {
                    element: Element::default(),
                    name: "Count".to_string(),
                })),
                int_expr(1),
            ),
            distinct: None,
        }),
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(3));
}

#[test]
fn test_query_aggregate_distinct() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 2, 3, 3, 3] X aggregate distinct Sum starting 0: Sum + X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 2, 3, 3, 3]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: Some(AggregateClause {
            identifier: "Sum".to_string(),
            starting: Some(int_expr(0)),
            expression: make_binary(Expression::Add,
                Box::new(Expression::OperandRef(octofhir_cql_elm::OperandRef {
                    element: Element::default(),
                    name: "Sum".to_string(),
                })),
                alias_ref("X"),
            ),
            distinct: Some(true),
        }),
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(6)); // 1+2+3 = 6 (distinct values)
}

#[test]
fn test_query_aggregate_empty() {
    let e = engine();
    let mut c = ctx();

    // from [] X aggregate Sum starting 0: Sum + X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", empty_list_expr())],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: Some(AggregateClause {
            identifier: "Sum".to_string(),
            starting: Some(int_expr(0)),
            expression: make_binary(Expression::Add,
                Box::new(Expression::OperandRef(octofhir_cql_elm::OperandRef {
                    element: Element::default(),
                    name: "Sum".to_string(),
                })),
                alias_ref("X"),
            ),
            distinct: None,
        }),
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_eq!(result, CqlValue::Integer(0)); // Starting value returned for empty
}

// ============================================================================
// Combined Features Tests
// ============================================================================

#[test]
fn test_query_where_return_sort() {
    let e = engine();
    let mut c = ctx();

    // from [5, 3, 1, 4, 2] X where X > 1 return X * 2 sort asc
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![5, 3, 1, 4, 2]))],
        let_clause: None,
        relationship: None,
        where_clause: Some(make_binary(Expression::Greater, alias_ref("X"), int_expr(1))),
        return_clause: Some(ReturnClause {
            expression: make_binary(Expression::Multiply, alias_ref("X"), int_expr(2)),
            distinct: None,
        }),
        aggregate: None,
        sort: Some(SortClause {
            by: vec![SortByItem {
                direction: SortDirection::Asc,
                path: None,
            }],
        }),
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // Filter: [5, 3, 4, 2]
    // Return: [10, 6, 8, 4]
    // Sort asc: [4, 6, 8, 10]
    assert_list_values(&result, &[4, 6, 8, 10]);
}

#[test]
fn test_query_let_where_return() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3, 4, 5] X let Sq: X * X where Sq > 5 return Sq
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3, 4, 5]))],
        let_clause: Some(vec![LetClause {
            identifier: "Sq".to_string(),
            expression: make_binary(Expression::Multiply, alias_ref("X"), alias_ref("X")),
        }]),
        relationship: None,
        where_clause: Some(make_binary(Expression::Greater, let_ref("Sq"), int_expr(5))),
        return_clause: Some(ReturnClause {
            expression: let_ref("Sq"),
            distinct: None,
        }),
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    // Sq values: 1, 4, 9, 16, 25
    // Filter > 5: 9, 16, 25
    assert_list_values(&result, &[9, 16, 25]);
}

#[test]
fn test_query_distinct_and_sort() {
    let e = engine();
    let mut c = ctx();

    // from [1, 3, 2, 3, 1, 2] X return distinct X sort desc
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 3, 2, 3, 1, 2]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: Some(ReturnClause {
            expression: alias_ref("X"),
            distinct: Some(true),
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
    // Distinct: [1, 3, 2] (order of first occurrence)
    // Sort desc: [3, 2, 1]
    assert_list_values(&result, &[3, 2, 1]);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_query_with_single_element() {
    let e = engine();
    let mut c = ctx();

    // from [42] X return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![42]))],
        let_clause: None,
        relationship: None,
        where_clause: None,
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[42]);
}

#[test]
fn test_query_filter_to_single() {
    let e = engine();
    let mut c = ctx();

    // from [1, 2, 3] X where X = 2 return X
    let query = Query {
        element: Element::default(),
        source: vec![make_source("X", list_expr(vec![1, 2, 3]))],
        let_clause: None,
        relationship: None,
        where_clause: Some(make_binary(Expression::Equal, alias_ref("X"), int_expr(2))),
        return_clause: None,
        aggregate: None,
        sort: None,
    };

    let result = e.eval_query(&query, &mut c).unwrap();
    assert_list_values(&result, &[2]);
}
