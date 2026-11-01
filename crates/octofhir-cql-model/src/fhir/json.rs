//! FHIR JSON ⇄ CQL values
//!
//! CQL authored against FHIR (with `FHIRHelpers`) works on CQL system values:
//! the reference translator inserts `FHIRHelpers.ToDate`, `ToConcept`,
//! `ToInterval`, ... wherever a FHIR element is used. This module applies
//! those conversions once, when FHIR JSON is loaded, so the evaluator sees
//! system values directly:
//!
//! | FHIR | CQL |
//! |------|-----|
//! | `boolean` | `Boolean` |
//! | `integer`, `positiveInt`, `unsignedInt` | `Integer` (`Long` if it does not fit) |
//! | `decimal` | `Decimal` |
//! | `date` | `Date` |
//! | `dateTime`, `instant` | `DateTime` |
//! | `time` | `Time` |
//! | `string`, `code`, `uri`, `id`, ... | `String` |
//! | `Coding` | `Code` |
//! | `CodeableConcept` | `Concept` (`text` becomes the display) |
//! | `Quantity` (and `Age`, `Duration`, ...) | `Quantity` (UCUM `code`, else `unit`) |
//! | `Period` | `Interval<DateTime>` (FHIRHelpers.ToInterval semantics) |
//! | `Range` | `Interval<Quantity>` |
//! | `Ratio` | `Ratio` |
//! | resources and other complex types | `Tuple` (keeps `resourceType`) |
//!
//! Choice elements are exposed under their CQL name with the type given by
//! the JSON suffix: `effectiveDateTime` becomes `effective: DateTime`,
//! `valueQuantity` becomes `value: Quantity`. Date-typed elements that are
//! not choices are recognised by name (`birthDate`, `recordedDate`,
//! `authoredOn`, ...) or, for any element, by a full `dateTime`/`instant`
//! lexical form with a time part.
//!
//! The mapping works without StructureDefinitions, so a `string` element
//! whose value happens to look like a `dateTime` with a time part is read as a
//! `DateTime`; elements are otherwise typed from the JSON value itself.

use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use octofhir_cql_types::{
    CqlCode, CqlConcept, CqlDate, CqlDateTime, CqlInterval, CqlList, CqlQuantity, CqlRatio,
    CqlTime, CqlTuple, CqlType, CqlValue,
};
use serde_json::{Map, Number, Value};
use std::str::FromStr;

/// UCUM code system URI.
pub const UCUM_SYSTEM: &str = "http://unitsofmeasure.org";

/// Base names of FHIR R4 choice elements (`<base>[x]`).
const CHOICE_BASES: &[&str] = &[
    "abatement",
    "age",
    "allowed",
    "answer",
    "asNeeded",
    "born",
    "collected",
    "deceased",
    "defaultValue",
    "dose",
    "effective",
    "fixed",
    "initial",
    "item",
    "maxValue",
    "medication",
    "minValue",
    "multipleBirth",
    "occurrence",
    "onset",
    "pattern",
    "performed",
    "product",
    "rate",
    "reported",
    "serviced",
    "subject",
    "timing",
    "used",
    "value",
];

/// FHIR data type names that can appear as choice suffixes.
const CHOICE_TYPES: &[&str] = &[
    "Address",
    "Age",
    "Annotation",
    "Attachment",
    "Base64Binary",
    "Boolean",
    "Canonical",
    "Code",
    "CodeableConcept",
    "Coding",
    "ContactPoint",
    "Count",
    "Date",
    "DateTime",
    "Decimal",
    "Distance",
    "Dosage",
    "Duration",
    "Expression",
    "HumanName",
    "Id",
    "Identifier",
    "Instant",
    "Integer",
    "Markdown",
    "Money",
    "Oid",
    "Period",
    "PositiveInt",
    "Quantity",
    "Range",
    "Ratio",
    "Reference",
    "SampledData",
    "Signature",
    "String",
    "Time",
    "Timing",
    "UnsignedInt",
    "Uri",
    "Url",
    "Uuid",
];

/// Non-choice FHIR R4 elements of type `date`, `dateTime` or `instant`
/// whose values may be partial (`1980`, `2024-03`), so they cannot be
/// recognised from their lexical form alone.
const DATE_ELEMENTS: &[&str] = &[
    "assertedDate",
    "authoredOn",
    "birthDate",
    "created",
    "date",
    "dateAsserted",
    "dateWritten",
    "end",
    "expirationDate",
    "issued",
    "lastUpdated",
    "lockedDate",
    "recorded",
    "recordedDate",
    "start",
    "whenHandedOver",
    "whenPrepared",
];

/// Convert a FHIR JSON resource (or any FHIR element) to a CQL value.
pub fn fhir_to_cql(json: &Value) -> CqlValue {
    convert_untyped(None, json)
}

/// Convert a value whose FHIR type is unknown.
fn convert_untyped(key: Option<&str>, json: &Value) -> CqlValue {
    match json {
        Value::Null => CqlValue::Null,
        Value::Bool(b) => CqlValue::Boolean(*b),
        Value::Number(n) => number(n),
        Value::String(s) => {
            let date_element = key.is_some_and(|k| DATE_ELEMENTS.contains(&k));
            if date_element || looks_like_datetime(s) {
                parse_date_or_datetime(s).unwrap_or_else(|| CqlValue::String(s.clone()))
            } else {
                CqlValue::String(s.clone())
            }
        }
        Value::Array(items) => list(items.iter().map(|item| convert_untyped(key, item))),
        Value::Object(map) => convert_object(map),
    }
}

/// Convert a value of a known FHIR type (from a choice suffix).
fn convert_typed(fhir_type: &str, json: &Value) -> CqlValue {
    if let Value::Array(items) = json {
        return list(items.iter().map(|item| convert_typed(fhir_type, item)));
    }
    match (fhir_type, json) {
        (_, Value::Null) => CqlValue::Null,
        ("Boolean", Value::Bool(b)) => CqlValue::Boolean(*b),
        ("Integer" | "PositiveInt" | "UnsignedInt", Value::Number(n)) => number(n),
        ("Decimal", Value::Number(n)) => decimal(n).map_or(CqlValue::Null, CqlValue::Decimal),
        ("Date", Value::String(s)) => CqlDate::parse(s)
            .map(CqlValue::Date)
            .unwrap_or_else(|| CqlValue::String(s.clone())),
        ("DateTime" | "Instant", Value::String(s)) => parse_datetime(s)
            .map(CqlValue::DateTime)
            .unwrap_or_else(|| CqlValue::String(s.clone())),
        ("Time", Value::String(s)) => CqlTime::parse(s)
            .map(CqlValue::Time)
            .unwrap_or_else(|| CqlValue::String(s.clone())),
        (_, Value::String(s)) => CqlValue::String(s.clone()),
        ("Coding", Value::Object(map)) => coding(map).unwrap_or_else(|| convert_object(map)),
        ("CodeableConcept", Value::Object(map)) => {
            codeable_concept(map).unwrap_or_else(|| convert_object(map))
        }
        (
            "Quantity" | "Age" | "Count" | "Distance" | "Duration" | "SimpleQuantity",
            Value::Object(map),
        ) => quantity(map)
            .map(CqlValue::Quantity)
            .unwrap_or_else(|| convert_object(map)),
        ("Period", Value::Object(map)) => period(map).unwrap_or_else(|| convert_object(map)),
        ("Range", Value::Object(map)) => range(map).unwrap_or_else(|| convert_object(map)),
        ("Ratio", Value::Object(map)) => ratio(map).unwrap_or_else(|| convert_object(map)),
        _ => convert_untyped(None, json),
    }
}

/// Convert an object: a recognised FHIR data type, else a tuple whose
/// elements are converted recursively.
fn convert_object(map: &Map<String, Value>) -> CqlValue {
    if !map.contains_key("resourceType")
        && let Some(value) = recognise_data_type(map)
    {
        return value;
    }

    let mut elements: IndexMap<String, CqlValue> = IndexMap::with_capacity(map.len());
    for (key, value) in map {
        match split_choice(key) {
            Some((base, fhir_type)) => {
                elements.insert(base.to_string(), convert_typed(fhir_type, value));
            }
            None => {
                elements.insert(key.clone(), convert_untyped(Some(key), value));
            }
        }
    }
    CqlValue::Tuple(CqlTuple { elements })
}

/// Recognise Coding, CodeableConcept, Quantity, Period, Range and Ratio by
/// their element names.
fn recognise_data_type(map: &Map<String, Value>) -> Option<CqlValue> {
    let only = |allowed: &[&str]| map.keys().all(|k| allowed.contains(&k.as_str()));
    if map.contains_key("coding") && only(&["coding", "text", "id", "extension"]) {
        return codeable_concept(map);
    }
    if map.get("value").is_some_and(Value::is_number)
        && only(&[
            "value",
            "comparator",
            "unit",
            "system",
            "code",
            "id",
            "extension",
        ])
    {
        return quantity(map).map(CqlValue::Quantity);
    }
    if map.get("code").is_some_and(Value::is_string)
        && only(&[
            "system",
            "version",
            "code",
            "display",
            "userSelected",
            "id",
            "extension",
        ])
    {
        return coding(map);
    }
    if (map.contains_key("start") || map.contains_key("end"))
        && only(&["start", "end", "id", "extension"])
    {
        return period(map);
    }
    if (map.contains_key("low") || map.contains_key("high"))
        && only(&["low", "high", "id", "extension"])
        && ["low", "high"]
            .iter()
            .filter_map(|k| map.get(*k))
            .all(|v| v.get("value").is_some_and(Value::is_number))
    {
        return range(map);
    }
    if map.contains_key("numerator")
        && map.contains_key("denominator")
        && only(&["numerator", "denominator", "id", "extension"])
    {
        return ratio(map);
    }
    None
}

/// `effectiveDateTime` → (`effective`, `DateTime`)
fn split_choice(key: &str) -> Option<(&str, &str)> {
    CHOICE_BASES.iter().find_map(|base| {
        let suffix = key.strip_prefix(base)?;
        CHOICE_TYPES.contains(&suffix).then_some((*base, suffix))
    })
}

fn string_field<'m>(map: &'m Map<String, Value>, key: &str) -> Option<&'m str> {
    map.get(key).and_then(Value::as_str)
}

fn coding(map: &Map<String, Value>) -> Option<CqlValue> {
    Some(CqlValue::Code(CqlCode {
        code: string_field(map, "code")?.to_string(),
        system: string_field(map, "system").unwrap_or_default().to_string(),
        version: string_field(map, "version").map(str::to_string),
        display: string_field(map, "display").map(str::to_string),
    }))
}

fn codeable_concept(map: &Map<String, Value>) -> Option<CqlValue> {
    let codes: Vec<CqlCode> = map
        .get("coding")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| match c.as_object().and_then(coding) {
            Some(CqlValue::Code(code)) => Some(code),
            _ => None,
        })
        .collect();
    Some(CqlValue::Concept(CqlConcept {
        codes: codes.into(),
        display: string_field(map, "text").map(str::to_string),
    }))
}

/// FHIRHelpers.ToQuantity: the UCUM `code` when the system is UCUM (or
/// absent), otherwise the human-readable `unit`.
fn quantity(map: &Map<String, Value>) -> Option<CqlQuantity> {
    let value = decimal(map.get("value")?.as_number()?)?;
    let system = string_field(map, "system");
    let unit = match (system, string_field(map, "code")) {
        (None | Some(UCUM_SYSTEM), Some(code)) => Some(code),
        _ => string_field(map, "unit"),
    };
    Some(CqlQuantity {
        value,
        unit: unit.map(str::to_string),
    })
}

/// FHIRHelpers.ToInterval(Period): `Interval(null, end]` when `start` is
/// absent, `Interval[start, end]` otherwise.
fn period(map: &Map<String, Value>) -> Option<CqlValue> {
    let bound = |key: &str| {
        string_field(map, key)
            .and_then(parse_datetime)
            .map(CqlValue::DateTime)
    };
    let low = bound("start");
    let high = bound("end");
    let low_closed = low.is_some();
    Some(CqlValue::Interval(CqlInterval::new(
        CqlType::DateTime,
        low,
        low_closed,
        high,
        true,
    )))
}

/// FHIRHelpers.ToInterval(Range): `Interval[low, high]`.
fn range(map: &Map<String, Value>) -> Option<CqlValue> {
    let bound = |key: &str| {
        map.get(key)
            .and_then(Value::as_object)
            .and_then(quantity)
            .map(CqlValue::Quantity)
    };
    Some(CqlValue::Interval(CqlInterval::new(
        CqlType::Quantity,
        bound("low"),
        true,
        bound("high"),
        true,
    )))
}

fn ratio(map: &Map<String, Value>) -> Option<CqlValue> {
    let part = |key: &str| map.get(key).and_then(Value::as_object).and_then(quantity);
    Some(CqlValue::Ratio(CqlRatio {
        numerator: part("numerator")?,
        denominator: part("denominator")?,
    }))
}

fn list(items: impl Iterator<Item = CqlValue>) -> CqlValue {
    CqlValue::List(CqlList::from_elements(items.collect()))
}

fn number(n: &Number) -> CqlValue {
    if let Some(i) = n.as_i64() {
        return i32::try_from(i).map_or(CqlValue::Long(i), CqlValue::Integer);
    }
    decimal(n).map_or(CqlValue::Null, CqlValue::Decimal)
}

fn decimal(n: &Number) -> Option<BigDecimal> {
    BigDecimal::from_str(&n.to_string()).ok()
}

/// Whether `s` has the lexical form of a FHIR `dateTime`/`instant` with a
/// time part (`YYYY-MM-DDThh:mm...`).
fn looks_like_datetime(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 16
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
        && b[10] == b'T'
        && b[11..13].iter().all(u8::is_ascii_digit)
        && b[13] == b':'
}

/// FHIR `date` (no time part) → Date, otherwise DateTime.
fn parse_date_or_datetime(s: &str) -> Option<CqlValue> {
    if s.contains('T') {
        parse_datetime(s).map(CqlValue::DateTime)
    } else {
        CqlDate::parse(s).map(CqlValue::Date)
    }
}

/// Parse a FHIR `dateTime`/`instant`, normalising fractional seconds to
/// millisecond precision (FHIR allows any number of digits).
fn parse_datetime(s: &str) -> Option<CqlDateTime> {
    let Some(dot) = s.find('.') else {
        return CqlDateTime::parse(s);
    };
    let fraction_end = s[dot + 1..]
        .find(|c: char| !c.is_ascii_digit())
        .map_or(s.len(), |i| dot + 1 + i);
    let digits = &s[dot + 1..fraction_end];
    let millis: String = digits.chars().chain("000".chars()).take(3).collect();
    let normalised = format!("{}.{}{}", &s[..dot], millis, &s[fraction_end..]);
    CqlDateTime::parse(&normalised)
}

/// Render a CQL value as JSON.
///
/// Temporal values become ISO 8601 strings, Quantity `{ value, unit }`,
/// Ratio `{ numerator, denominator }`, Code `{ system, code, version?,
/// display? }`, Concept `{ codes, display? }`, Interval `{ low, high,
/// lowClosed, highClosed }`, List an array and Tuple an object.
pub fn cql_to_json(value: &CqlValue) -> Value {
    match value {
        CqlValue::Null => Value::Null,
        CqlValue::Boolean(b) => Value::Bool(*b),
        CqlValue::Integer(i) => Value::from(*i),
        CqlValue::Long(l) => Value::from(*l),
        CqlValue::Decimal(d) => decimal_to_json(d),
        CqlValue::String(s) => Value::String(s.clone()),
        CqlValue::Date(d) => Value::String(d.to_string()),
        CqlValue::DateTime(dt) => Value::String(dt.to_string()),
        CqlValue::Time(t) => Value::String(t.to_string()),
        CqlValue::Quantity(q) => quantity_to_json(q),
        CqlValue::Ratio(r) => {
            let mut map = Map::new();
            map.insert("numerator".into(), quantity_to_json(&r.numerator));
            map.insert("denominator".into(), quantity_to_json(&r.denominator));
            Value::Object(map)
        }
        CqlValue::Code(c) => code_to_json(c),
        CqlValue::Concept(c) => {
            let mut map = Map::new();
            map.insert(
                "codes".into(),
                Value::Array(c.codes.iter().map(code_to_json).collect()),
            );
            if let Some(display) = &c.display {
                map.insert("display".into(), Value::String(display.clone()));
            }
            Value::Object(map)
        }
        CqlValue::List(list) => Value::Array(list.iter().map(cql_to_json).collect()),
        CqlValue::Interval(i) => {
            let mut map = Map::new();
            map.insert(
                "low".into(),
                i.low.as_deref().map_or(Value::Null, cql_to_json),
            );
            map.insert(
                "high".into(),
                i.high.as_deref().map_or(Value::Null, cql_to_json),
            );
            map.insert("lowClosed".into(), Value::Bool(i.low_closed));
            map.insert("highClosed".into(), Value::Bool(i.high_closed));
            Value::Object(map)
        }
        CqlValue::Tuple(t) => {
            Value::Object(t.iter().map(|(k, v)| (k.clone(), cql_to_json(v))).collect())
        }
    }
}

fn decimal_to_json(d: &BigDecimal) -> Value {
    let text = d.normalized().to_string();
    match Number::from_str(&text) {
        Ok(n) => Value::Number(n),
        Err(_) => Value::String(text),
    }
}

fn quantity_to_json(q: &CqlQuantity) -> Value {
    let mut map = Map::new();
    map.insert("value".into(), decimal_to_json(&q.value));
    if let Some(unit) = &q.unit {
        map.insert("unit".into(), Value::String(unit.clone()));
    }
    Value::Object(map)
}

fn code_to_json(c: &CqlCode) -> Value {
    let mut map = Map::new();
    map.insert("system".into(), Value::String(c.system.clone()));
    map.insert("code".into(), Value::String(c.code.clone()));
    if let Some(version) = &c.version {
        map.insert("version".into(), Value::String(version.clone()));
    }
    if let Some(display) = &c.display {
        map.insert("display".into(), Value::String(display.clone()));
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tuple(value: &CqlValue) -> &CqlTuple {
        match value {
            CqlValue::Tuple(t) => t,
            other => panic!("expected tuple, got {other:?}"),
        }
    }

    #[test]
    fn primitives_and_dates() {
        let patient = fhir_to_cql(&json!({
            "resourceType": "Patient",
            "id": "2024",
            "active": true,
            "birthDate": "1980-05-20",
            "deceasedDateTime": "2024-03-01T10:15:30.1234+02:00",
            "meta": { "lastUpdated": "2024-01-01T00:00:00Z" }
        }));
        let p = tuple(&patient);
        assert_eq!(p.get("id"), Some(&CqlValue::string("2024")));
        assert_eq!(p.get("active"), Some(&CqlValue::Boolean(true)));
        assert_eq!(
            p.get("birthDate"),
            Some(&CqlValue::Date(CqlDate::new(1980, 5, 20)))
        );
        match p.get("deceased") {
            Some(CqlValue::DateTime(dt)) => {
                assert_eq!(dt.millisecond, Some(123));
                assert_eq!(dt.timezone_offset, Some(120));
            }
            other => panic!("expected DateTime, got {other:?}"),
        }
        let meta = tuple(p.get("meta").unwrap());
        assert!(matches!(
            meta.get("lastUpdated"),
            Some(CqlValue::DateTime(_))
        ));
    }

    #[test]
    fn clinical_data_types() {
        let obs = fhir_to_cql(&json!({
            "resourceType": "Observation",
            "status": "final",
            "code": { "coding": [{ "system": "http://loinc.org", "code": "4548-4" }], "text": "HbA1c" },
            "valueQuantity": { "value": 7.2, "unit": "percent", "system": UCUM_SYSTEM, "code": "%" },
            "effectivePeriod": { "start": "2024-02-01T08:00:00Z" },
            "referenceRange": [{ "low": { "value": 4, "code": "%" }, "high": { "value": 5.6, "code": "%" } }]
        }));
        let o = tuple(&obs);
        match o.get("code") {
            Some(CqlValue::Concept(c)) => {
                assert_eq!(c.codes[0].code, "4548-4");
                assert_eq!(c.display.as_deref(), Some("HbA1c"));
            }
            other => panic!("expected Concept, got {other:?}"),
        }
        match o.get("value") {
            Some(CqlValue::Quantity(q)) => {
                assert_eq!(q.unit.as_deref(), Some("%"));
                assert_eq!(q.value.to_string(), "7.2");
            }
            other => panic!("expected Quantity, got {other:?}"),
        }
        match o.get("effective") {
            Some(CqlValue::Interval(i)) => {
                assert!(i.low_closed && i.high_closed && i.high.is_none());
            }
            other => panic!("expected Interval, got {other:?}"),
        }
        let CqlValue::List(ranges) = o.get("referenceRange").unwrap() else {
            panic!("expected list");
        };
        // A backbone with only `low`/`high` quantities has the Range shape;
        // `.low`/`.high` stay available as interval properties.
        match ranges.get(0) {
            Some(CqlValue::Interval(i)) => {
                assert!(matches!(i.low.as_deref(), Some(CqlValue::Quantity(_))));
            }
            other => panic!("expected Interval, got {other:?}"),
        }
    }

    #[test]
    fn period_without_start_is_open_at_low() {
        let CqlValue::Interval(i) = convert_typed("Period", &json!({ "end": "2024-01-01" })) else {
            panic!("expected interval");
        };
        assert!(!i.low_closed && i.low.is_none() && i.high.is_some());
    }

    #[test]
    fn element_named_like_a_type_is_not_a_choice() {
        let value = fhir_to_cql(&json!({ "valueSet": "http://example.org/vs" }));
        assert_eq!(
            tuple(&value).get("valueSet"),
            Some(&CqlValue::string("http://example.org/vs"))
        );
    }

    #[test]
    fn renders_json() {
        let value = fhir_to_cql(&json!({
            "code": { "coding": [{ "system": "s", "code": "c", "display": "d" }] },
            "n": 2
        }));
        assert_eq!(
            cql_to_json(&value),
            json!({ "code": { "codes": [{ "system": "s", "code": "c", "display": "d" }] }, "n": 2 })
        );
    }
}
