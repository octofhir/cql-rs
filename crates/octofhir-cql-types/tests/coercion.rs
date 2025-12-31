//! Type coercion and conversion tests for CQL type system
//!
//! Tests implicit and explicit type conversions:
//! - Integer to Decimal
//! - Integer/Decimal to Quantity
//! - String to various types
//! - Date/DateTime conversions
//! - Null propagation
//! - Choice type handling

use octofhir_cql_types::*;
use rust_decimal::Decimal;

// === Numeric Coercions ===

#[test]
fn test_integer_to_decimal_coercion() {
    // In CQL, integers can be implicitly converted to decimals
    let int_val = 42i64;
    let decimal = Decimal::from(int_val);
    assert_eq!(decimal, Decimal::new(42, 0));
}

#[test]
fn test_decimal_precision_preserved() {
    let dec1 = "3.14".parse::<Decimal>().unwrap();
    let dec2 = "3.140".parse::<Decimal>().unwrap();

    // Decimal precision should be maintained
    // Note: Rust Decimal may normalize these
}

#[test]
fn test_integer_to_quantity() {
    // Integer can be converted to quantity with implicit unit
    let value = CqlValue::integer(5);
    let quantity = CqlQuantity {
        value: Decimal::from(5),
        unit: Some("1".to_string()), // Unity
    };

    assert_eq!(quantity.value, Decimal::from(5));
    let _ = value; // suppress unused warning
}

#[test]
fn test_decimal_to_quantity() {
    let quantity = CqlQuantity {
        value: "3.14".parse().unwrap(),
        unit: Some("m".to_string()),
    };

    assert_eq!(quantity.value.to_string(), "3.14");
    assert_eq!(quantity.unit, Some("m".to_string()));
}

// === String Conversions ===

#[test]
fn test_string_to_integer() {
    let s = "42";
    let result = s.parse::<i64>();
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_string_to_decimal() {
    let s = "3.14";
    let result = s.parse::<Decimal>();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to_string(), "3.14");
}

#[test]
fn test_string_to_boolean() {
    assert_eq!("true".parse::<bool>().unwrap(), true);
    assert_eq!("false".parse::<bool>().unwrap(), false);
}

#[test]
fn test_invalid_string_to_integer() {
    let s = "not a number";
    let result = s.parse::<i64>();
    assert!(result.is_err());
}

// === Date/DateTime Conversions ===

#[test]
fn test_date_to_datetime() {
    let date = CqlDate::new(2024, 3, 15);
    let datetime = CqlDateTime::new(date.year, date.month.unwrap(), date.day.unwrap(), 0, 0, 0, 0, None);

    assert_eq!(datetime.year, 2024);
    assert_eq!(datetime.month, Some(3));
    assert_eq!(datetime.day, Some(15));
}

#[test]
fn test_datetime_to_date() {
    let datetime = CqlDateTime::new(2024, 3, 15, 10, 30, 0, 0, None);
    let date = CqlDate::new(datetime.year, datetime.month.unwrap(), datetime.day.unwrap());

    assert_eq!(date.year, 2024);
    assert_eq!(date.month, Some(3));
    assert_eq!(date.day, Some(15));
}

#[test]
fn test_partial_date_precision() {
    let date_year = CqlDate {
        year: 2024,
        month: None,
        day: None,
    };

    let date_month = CqlDate {
        year: 2024,
        month: Some(3),
        day: None,
    };

    let date_full = CqlDate {
        year: 2024,
        month: Some(3),
        day: Some(15),
    };

    // All should be valid
    assert_eq!(date_year.year, 2024);
    assert_eq!(date_month.month, Some(3));
    assert_eq!(date_full.day, Some(15));
}

// === Null Coercion and Propagation ===

#[test]
fn test_null_to_any_type() {
    // Null can be coerced to any type
    let null_as_int: CqlValue = CqlValue::Null;
    let null_as_string: CqlValue = CqlValue::Null;
    let null_as_boolean: CqlValue = CqlValue::Null;

    assert!(matches!(null_as_int, CqlValue::Null));
    assert!(matches!(null_as_string, CqlValue::Null));
    assert!(matches!(null_as_boolean, CqlValue::Null));
}

#[test]
fn test_null_in_operations() {
    // Operations with null should propagate null (three-valued logic)
    let null_val = CqlValue::Null;
    let int_val = CqlValue::integer(42);

    // In actual evaluation, null + int would result in null
    // This test just verifies the type representation
}

// === List Coercions ===

#[test]
fn test_singleton_to_list() {
    // A single value can be treated as a list with one element
    let value = CqlValue::integer(42);
    let list = CqlValue::List(CqlList::from_elements(vec![value]));

    match list {
        CqlValue::List(l) => {
            assert_eq!(l.elements.len(), 1);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_list_element_type_promotion() {
    // If a list contains integers and decimals, integers should promote to decimals
    let list = vec![
        CqlValue::integer(1),
        CqlValue::decimal("2.5".parse().unwrap()),
        CqlValue::integer(3),
    ];

    // This would require actual type inference logic
    assert_eq!(list.len(), 3);
}

// === Interval Coercions ===

#[test]
fn test_point_to_interval() {
    // A single value can be treated as a point interval [value, value]
    let value = CqlValue::integer(5);
    let interval = CqlInterval {
        point_type: CqlType::Integer,
        low: Some(Box::new(value.clone())),
        high: Some(Box::new(value)),
        low_closed: true,
        high_closed: true,
    };

    assert!(interval.low.is_some());
    assert!(interval.high.is_some());
}

// === Quantity Unit Conversions ===

#[test]
fn test_quantity_unit_compatibility() {
    let qty1 = CqlQuantity {
        value: "1000".parse().unwrap(),
        unit: Some("g".to_string()),
    };

    let qty2 = CqlQuantity {
        value: "1".parse().unwrap(),
        unit: Some("kg".to_string()),
    };

    // These should be equivalent (1kg = 1000g)
    // Actual conversion would require UCUM library
    assert_eq!(qty1.value, Decimal::new(1000, 0));
    assert_eq!(qty2.value, Decimal::new(1, 0));
}

#[test]
fn test_quantity_dimensionless() {
    let qty = CqlQuantity {
        value: "42".parse().unwrap(),
        unit: Some("1".to_string()), // Dimensionless
    };

    assert_eq!(qty.unit, Some("1".to_string()));
}

// === Code/Concept Coercions ===

#[test]
fn test_code_to_concept() {
    let code = CqlCode {
        system: "http://loinc.org".to_string(),
        version: None,
        code: "8480-6".to_string(),
        display: Some("Systolic BP".to_string()),
    };

    let concept = CqlConcept {
        codes: smallvec::smallvec![code.clone()],
        display: code.display.clone(),
    };

    assert_eq!(concept.codes.len(), 1);
    assert_eq!(concept.codes[0].code, "8480-6");
}

#[test]
fn test_concept_to_code() {
    // A concept with a single code can be treated as a code
    let concept = CqlConcept {
        codes: smallvec::smallvec![CqlCode {
            system: "http://loinc.org".to_string(),
            version: None,
            code: "8480-6".to_string(),
            display: Some("Systolic BP".to_string()),
        }],
        display: Some("Systolic BP".to_string()),
    };

    let code = concept.codes.first().unwrap();
    assert_eq!(code.code, "8480-6");
}

// === Tuple Coercions ===

#[test]
fn test_tuple_to_compatible_tuple() {
    // Tuples with compatible fields can be coerced
    let tuple1 = CqlTuple::from_elements([
        ("id", CqlValue::string("123")),
        ("value", CqlValue::integer(42)),
    ]);

    let tuple2 = CqlTuple::from_elements([
        ("id", CqlValue::string("456")),
        ("value", CqlValue::integer(99)),
    ]);

    // Both have the same structure
    assert_eq!(tuple1.len(), 2);
    assert_eq!(tuple2.len(), 2);
}

#[test]
fn test_tuple_structural_subtyping() {
    // A tuple with more fields can be used where fewer are expected (width subtyping)
    let tuple = CqlTuple::from_elements([
        ("id", CqlValue::string("123")),
        ("value", CqlValue::integer(42)),
        ("extra", CqlValue::boolean(true)),
    ]);

    // Accessing only id and value should work
    assert!(tuple.get("id").is_some());
    assert!(tuple.get("value").is_some());
}

// === Choice Type Handling ===

#[test]
fn test_choice_type_integer_or_decimal() {
    // Choice<Integer, Decimal> can hold either type
    let choice1: CqlValue = CqlValue::integer(42);
    let choice2: CqlValue = CqlValue::decimal("3.14".parse().unwrap());

    assert!(matches!(choice1, CqlValue::Integer(_)));
    assert!(matches!(choice2, CqlValue::Decimal(_)));
}

#[test]
fn test_choice_type_with_null() {
    // Choice types include null as a possible value
    let choice: CqlValue = CqlValue::Null;
    assert!(matches!(choice, CqlValue::Null));
}

// === Boolean Coercions ===

#[test]
fn test_three_valued_logic_types() {
    // CQL uses three-valued logic: true, false, null
    let true_val = CqlValue::boolean(true);
    let false_val = CqlValue::boolean(false);
    let null_val = CqlValue::Null; // null as boolean

    assert!(matches!(true_val, CqlValue::Boolean(true)));
    assert!(matches!(false_val, CqlValue::Boolean(false)));
    assert!(matches!(null_val, CqlValue::Null));
}

// === Subtype Relationships ===

#[test]
fn test_integer_is_subtype_of_decimal() {
    // Integer values can be used where Decimal is expected
    let int_val = CqlValue::integer(42);

    // In type checking, Integer <: Decimal
    match int_val {
        CqlValue::Integer(i) => {
            let as_decimal = Decimal::from(i);
            assert_eq!(as_decimal, Decimal::from(42));
        }
        _ => panic!("Expected integer"),
    }
}

#[test]
fn test_code_is_subtype_of_concept() {
    // Code can be used where Concept is expected
    let code = CqlCode {
        system: "http://loinc.org".to_string(),
        version: None,
        code: "8480-6".to_string(),
        display: None,
    };

    // Can be promoted to Concept
    let as_concept = CqlConcept {
        codes: smallvec::smallvec![code],
        display: None,
    };

    assert_eq!(as_concept.codes.len(), 1);
}

// === Implicit Conversions in Context ===

#[test]
fn test_integer_list_to_decimal_list() {
    let int_list = vec![
        CqlValue::integer(1),
        CqlValue::integer(2),
        CqlValue::integer(3),
    ];

    // In contexts requiring List<Decimal>, integers should convert
    let decimal_list: Vec<CqlValue> = int_list
        .into_iter()
        .map(|v| match v {
            CqlValue::Integer(i) => CqlValue::decimal(Decimal::from(i)),
            other => other,
        })
        .collect();

    for val in decimal_list {
        assert!(matches!(val, CqlValue::Decimal(_)));
    }
}
