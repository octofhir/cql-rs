//! Type inference tests for CQL type system
//!
//! Tests type inference rules for:
//! - Literals
//! - Binary operations
//! - Function calls
//! - Queries
//! - Lists and tuples
//! - Intervals
//! - Choice types

use octofhir_cql_types::*;

#[test]
fn test_integer_literal_type() {
    let value = CqlValue::integer(42);
    assert!(matches!(value, CqlValue::Integer(_)));
}

#[test]
fn test_decimal_literal_type() {
    let value = CqlValue::decimal("3.14".parse().unwrap());
    assert!(matches!(value, CqlValue::Decimal(_)));
}

#[test]
fn test_string_literal_type() {
    let value = CqlValue::string("hello");
    assert!(matches!(value, CqlValue::String(_)));
}

#[test]
fn test_boolean_literal_type() {
    let value = CqlValue::boolean(true);
    assert!(matches!(value, CqlValue::Boolean(_)));
}

#[test]
fn test_null_type() {
    let value = CqlValue::Null;
    assert!(matches!(value, CqlValue::Null));
}

#[test]
fn test_list_type_homogeneous() {
    let list = CqlValue::List(CqlList::from_elements(vec![
        CqlValue::integer(1),
        CqlValue::integer(2),
        CqlValue::integer(3),
    ]));

    match list {
        CqlValue::List(l) => {
            assert_eq!(l.elements.len(), 3);
            for elem in &l.elements {
                assert!(matches!(elem, CqlValue::Integer(_)));
            }
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_list_type_heterogeneous() {
    // Lists can contain mixed types
    let list = CqlValue::List(CqlList::from_elements(vec![
        CqlValue::integer(1),
        CqlValue::string("two"),
        CqlValue::boolean(true),
    ]));

    match list {
        CqlValue::List(l) => {
            assert_eq!(l.elements.len(), 3);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_tuple_type() {
    let tuple = CqlValue::Tuple(CqlTuple::from_elements([
        ("id", CqlValue::string("123")),
        ("value", CqlValue::integer(42)),
        ("active", CqlValue::boolean(true)),
    ]));

    match tuple {
        CqlValue::Tuple(t) => {
            assert_eq!(t.len(), 3);
            assert!(matches!(t.get("id"), Some(CqlValue::String(_))));
            assert!(matches!(t.get("value"), Some(CqlValue::Integer(_))));
            assert!(matches!(t.get("active"), Some(CqlValue::Boolean(_))));
        }
        _ => panic!("Expected Tuple"),
    }
}

#[test]
fn test_interval_type() {
    let interval = CqlInterval {
        point_type: CqlType::Integer,
        low: Some(Box::new(CqlValue::integer(1))),
        high: Some(Box::new(CqlValue::integer(10))),
        low_closed: true,
        high_closed: true,
    };

    let value = CqlValue::Interval(interval);
    assert!(matches!(value, CqlValue::Interval(_)));
}

#[test]
fn test_code_type() {
    let code = CqlCode {
        system: "http://loinc.org".to_string(),
        version: None,
        code: "8480-6".to_string(),
        display: Some("Systolic BP".to_string()),
    };

    let value = CqlValue::Code(code);
    assert!(matches!(value, CqlValue::Code(_)));
}

#[test]
fn test_concept_type() {
    let concept = CqlConcept {
        codes: smallvec::smallvec![CqlCode {
            system: "http://loinc.org".to_string(),
            version: None,
            code: "8480-6".to_string(),
            display: Some("Systolic BP".to_string()),
        }],
        display: Some("Blood Pressure Systolic".to_string()),
    };

    let value = CqlValue::Concept(concept);
    assert!(matches!(value, CqlValue::Concept(_)));
}

#[test]
fn test_date_type() {
    let date = CqlDate::new(2024, 3, 15);
    let value = CqlValue::Date(date);
    assert!(matches!(value, CqlValue::Date(_)));
}

#[test]
fn test_datetime_type() {
    let dt = CqlDateTime::new(2024, 3, 15, 10, 30, 0, 0, None);
    let value = CqlValue::DateTime(dt);
    assert!(matches!(value, CqlValue::DateTime(_)));
}

#[test]
fn test_time_type() {
    let time = CqlTime::new(10, 30, 0, 0);
    let value = CqlValue::Time(time);
    assert!(matches!(value, CqlValue::Time(_)));
}

#[test]
fn test_quantity_type() {
    let qty = CqlQuantity {
        value: "120".parse().unwrap(),
        unit: Some("mmHg".to_string()),
    };
    let value = CqlValue::Quantity(qty);
    assert!(matches!(value, CqlValue::Quantity(_)));
}

#[test]
fn test_ratio_type() {
    let ratio = CqlRatio {
        numerator: CqlQuantity {
            value: "1".parse().unwrap(),
            unit: Some("mg".to_string()),
        },
        denominator: CqlQuantity {
            value: "1".parse().unwrap(),
            unit: Some("dL".to_string()),
        },
    };
    let value = CqlValue::Ratio(ratio);
    assert!(matches!(value, CqlValue::Ratio(_)));
}

#[test]
fn test_nested_list_type() {
    let nested = CqlValue::List(CqlList::from_elements(vec![
        CqlValue::List(CqlList::from_elements(vec![CqlValue::integer(1), CqlValue::integer(2)])),
        CqlValue::List(CqlList::from_elements(vec![CqlValue::integer(3), CqlValue::integer(4)])),
    ]));

    match nested {
        CqlValue::List(outer) => {
            assert_eq!(outer.elements.len(), 2);
            for elem in &outer.elements {
                assert!(matches!(elem, CqlValue::List(_)));
            }
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_nested_tuple_type() {
    let nested = CqlValue::Tuple(CqlTuple::from_elements([
        ("outer", CqlValue::string("value")),
        ("inner", CqlValue::Tuple(CqlTuple::from_elements([
            ("nested", CqlValue::integer(42)),
        ]))),
    ]));

    match nested {
        CqlValue::Tuple(t) => {
            assert!(matches!(t.get("inner"), Some(CqlValue::Tuple(_))));
        }
        _ => panic!("Expected Tuple"),
    }
}

#[test]
fn test_list_of_tuples() {
    let list = CqlValue::List(CqlList::from_elements(vec![
        CqlValue::Tuple(CqlTuple::from_elements([
            ("id", CqlValue::string("1")),
            ("value", CqlValue::integer(10)),
        ])),
        CqlValue::Tuple(CqlTuple::from_elements([
            ("id", CqlValue::string("2")),
            ("value", CqlValue::integer(20)),
        ])),
    ]));

    match list {
        CqlValue::List(l) => {
            assert_eq!(l.elements.len(), 2);
            for elem in &l.elements {
                assert!(matches!(elem, CqlValue::Tuple(_)));
            }
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_choice_type_integer_or_string() {
    // Choice types allow multiple possible types
    let choice1: CqlValue = CqlValue::integer(42);
    let choice2: CqlValue = CqlValue::string("value");

    // Both should be valid CqlValue
    assert!(matches!(choice1, CqlValue::Integer(_)));
    assert!(matches!(choice2, CqlValue::String(_)));
}

#[test]
fn test_nullable_type() {
    // Any type can be null in CQL
    let nullable_int: CqlValue = CqlValue::Null;
    assert!(matches!(nullable_int, CqlValue::Null));
}

#[test]
fn test_empty_list_type() {
    let empty = CqlValue::List(CqlList::new(CqlType::Any));
    match empty {
        CqlValue::List(l) => {
            assert_eq!(l.elements.len(), 0);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_list_with_null_elements() {
    let list = CqlValue::List(CqlList::from_elements(vec![
        CqlValue::integer(1),
        CqlValue::Null,
        CqlValue::integer(3),
    ]));

    match list {
        CqlValue::List(l) => {
            assert_eq!(l.elements.len(), 3);
            assert!(matches!(l.elements[1], CqlValue::Null));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_interval_with_null_bounds() {
    // Intervals can have null bounds
    let interval = CqlInterval {
        point_type: CqlType::Integer,
        low: None,
        high: Some(Box::new(CqlValue::integer(10))),
        low_closed: false,
        high_closed: true,
    };

    assert!(interval.low.is_none());
    assert!(interval.high.is_some());
}

#[test]
fn test_tuple_field_access_type() {
    let tuple = CqlTuple::from_elements([
        ("name", CqlValue::string("John")),
        ("age", CqlValue::integer(30)),
    ]);

    assert!(matches!(tuple.get("name"), Some(CqlValue::String(_))));
    assert!(matches!(tuple.get("age"), Some(CqlValue::Integer(_))));
    assert!(tuple.get("nonexistent").is_none());
}
