//! In-memory FHIR data and terminology
//!
//! [`InMemoryFhirData`] holds FHIR resources loaded from JSON (a single
//! resource, a `Bundle`, a JSON array or NDJSON) and implements both
//! [`DataProvider`] and [`TerminologyProvider`], so a library can be evaluated
//! offline against a patient bundle:
//!
//! ```
//! use octofhir_cql_eval::fhir::InMemoryFhirData;
//! use octofhir_cql_eval::{DataProvider, TerminologyProvider};
//! use serde_json::json;
//!
//! let mut data = InMemoryFhirData::new();
//! data.add_json(&json!({
//!     "resourceType": "Bundle",
//!     "entry": [
//!         { "resource": { "resourceType": "Patient", "id": "p1", "birthDate": "1980-05-20" } },
//!         { "resource": { "resourceType": "ValueSet", "url": "http://example.org/vs",
//!             "expansion": { "contains": [{ "system": "http://loinc.org", "code": "4548-4" }] } } }
//!     ]
//! }));
//! assert_eq!(data.resources("Patient").len(), 1);
//! assert_eq!(data.expand_value_set("http://example.org/vs").map(|c| c.len()), Some(1));
//! ```
//!
//! Resources are converted with [`fhir_to_cql`], i.e. FHIRHelpers conversions
//! are applied at load time.
//!
//! Retrieve semantics:
//! - the data type is matched against `resourceType` (`{http://hl7.org/fhir}`
//!   and `FHIR.` prefixes are ignored);
//! - in `Patient` context, a `Patient` retrieve returns the context patient,
//!   and resources carrying a `subject`, `patient` or `beneficiary`
//!   reference are returned only when it points at that patient (resources
//!   without such a reference, e.g. `Medication`, are shared by all patients);
//! - a terminology filter keeps resources whose code (the retrieve's code
//!   property) is equivalent to one of the filter's codes; a value set filter
//!   is expanded from the loaded `ValueSet` resources;
//! - a date filter keeps resources whose date property lies in the range.
//!
//! Terminology: value sets are read from `expansion.contains` (nested
//! entries included) and from `compose.include` entries that enumerate
//! `concept`s. Intensional definitions (`filter`, included `valueSet`s) are
//! not evaluated; such value sets contribute only their enumerated codes.

use crate::context::{DataProvider, TerminologyProvider};
use crate::operators::comparison::{cql_compare, cql_equivalent};
use octofhir_cql_elm::fhir_primary_code_path;
use octofhir_cql_model::fhir::json::fhir_to_cql;
use octofhir_cql_types::{CqlCode, CqlValue};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Reference elements that tie a resource to a patient.
const PATIENT_REFERENCES: &[&str] = &["subject", "patient", "beneficiary"];

/// An enumerated value set.
#[derive(Debug, Clone, Default)]
struct ValueSetCodes {
    codes: Vec<CqlCode>,
}

/// FHIR resources and value sets held in memory.
#[derive(Debug, Clone, Default)]
pub struct InMemoryFhirData {
    /// Converted resources by resource type, in load order
    resources: HashMap<String, Vec<CqlValue>>,
    /// Bundle `fullUrl` → `Type/id`, for `urn:uuid:` references
    full_urls: HashMap<String, String>,
    /// Value sets by canonical URL (and `url|version`)
    value_sets: HashMap<String, ValueSetCodes>,
}

impl InMemoryFhirData {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a resource, a `Bundle` (recursively) or a JSON array of either.
    /// Returns the number of resources added.
    pub fn add_json(&mut self, json: &Value) -> usize {
        match json {
            Value::Array(items) => items.iter().map(|item| self.add_json(item)).sum(),
            Value::Object(map) => match map.get("resourceType").and_then(Value::as_str) {
                Some("Bundle") => {
                    let entries = map
                        .get("entry")
                        .and_then(Value::as_array)
                        .map_or(&[][..], Vec::as_slice);
                    entries
                        .iter()
                        .map(|entry| {
                            let Some(resource) = entry.get("resource") else {
                                return 0;
                            };
                            if let (Some(full_url), Some(local)) = (
                                entry.get("fullUrl").and_then(Value::as_str),
                                local_reference(resource),
                            ) {
                                self.full_urls.insert(full_url.to_string(), local);
                            }
                            self.add_json(resource)
                        })
                        .sum()
                }
                Some(resource_type) => {
                    if resource_type == "ValueSet" {
                        self.add_value_set(json);
                    }
                    self.resources
                        .entry(resource_type.to_string())
                        .or_default()
                        .push(fhir_to_cql(json));
                    1
                }
                None => 0,
            },
            _ => 0,
        }
    }

    /// Load newline-delimited JSON (one resource or bundle per line).
    pub fn add_ndjson(&mut self, text: &str) -> Result<usize, serde_json::Error> {
        let mut added = 0;
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            added += self.add_json(&serde_json::from_str(line)?);
        }
        Ok(added)
    }

    /// Register a `ValueSet` resource's enumerated codes.
    pub fn add_value_set(&mut self, json: &Value) {
        let Some(url) = json.get("url").and_then(Value::as_str) else {
            return;
        };
        let mut codes = Vec::new();
        if let Some(contains) = json
            .pointer("/expansion/contains")
            .and_then(Value::as_array)
        {
            collect_contains(contains, &mut codes);
        }
        for include in json
            .pointer("/compose/include")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let system = include
                .get("system")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let version = include.get("version").and_then(Value::as_str);
            for concept in include
                .get("concept")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(code) = concept.get("code").and_then(Value::as_str) {
                    codes.push(CqlCode {
                        code: code.to_string(),
                        system: system.to_string(),
                        version: version.map(str::to_string),
                        display: concept
                            .get("display")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    });
                }
            }
        }
        let entry = ValueSetCodes { codes };
        if let Some(version) = json.get("version").and_then(Value::as_str) {
            self.value_sets
                .insert(format!("{url}|{version}"), entry.clone());
        }
        self.value_sets.insert(url.to_string(), entry);
    }

    /// The loaded resources of a type, in load order.
    pub fn resources(&self, resource_type: &str) -> &[CqlValue] {
        self.resources
            .get(resource_type)
            .map_or(&[][..], Vec::as_slice)
    }

    /// Number of loaded resources.
    pub fn len(&self) -> usize {
        self.resources.values().map(Vec::len).sum()
    }

    /// Whether no resources are loaded.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether a value set with this canonical URL (or `url|version`) is loaded.
    pub fn has_value_set(&self, url: &str) -> bool {
        self.value_sets.contains_key(url)
    }

    /// Whether `reference` (e.g. `Patient/p1`, a full URL or a bundle
    /// `urn:uuid:`) points at the patient with `patient_id`.
    fn references_patient(&self, reference: &str, patient_id: &str) -> bool {
        let reference = self
            .full_urls
            .get(reference)
            .map_or(reference, String::as_str);
        let target = format!("Patient/{patient_id}");
        reference == target || reference.ends_with(&format!("/{target}"))
    }

    /// Whether a resource belongs to the context patient.
    fn in_patient_context(
        &self,
        resource: &CqlValue,
        resource_type: &str,
        patient_id: &str,
    ) -> bool {
        let CqlValue::Tuple(tuple) = resource else {
            return false;
        };
        if resource_type == "Patient" {
            return tuple.get("id").and_then(CqlValue::as_string) == Some(patient_id);
        }
        let mut has_reference = false;
        for element in PATIENT_REFERENCES {
            if let Some(CqlValue::Tuple(reference)) = tuple.get(element)
                && let Some(target) = reference.get("reference").and_then(CqlValue::as_string)
            {
                if self.references_patient(target, patient_id) {
                    return true;
                }
                has_reference = true;
            }
        }
        !has_reference
    }

    /// Codes a terminology filter value stands for.
    fn filter_codes(&self, codes: &CqlValue) -> Vec<CqlValue> {
        match codes {
            CqlValue::List(list) => list.iter().flat_map(|c| self.filter_codes(c)).collect(),
            CqlValue::Tuple(t) => t
                .get("id")
                .and_then(CqlValue::as_string)
                .and_then(|id| self.expand_value_set(id))
                .unwrap_or_default(),
            CqlValue::Null => Vec::new(),
            other => vec![other.clone()],
        }
    }
}

impl DataProvider for InMemoryFhirData {
    fn retrieve(
        &self,
        data_type: &str,
        context_type: Option<&str>,
        context_value: Option<&CqlValue>,
        _template_id: Option<&str>,
        code_property: Option<&str>,
        codes: Option<&CqlValue>,
        date_property: Option<&str>,
        date_range: Option<&CqlValue>,
    ) -> Vec<CqlValue> {
        let resource_type = data_type
            .strip_prefix("{http://hl7.org/fhir}")
            .or_else(|| data_type.strip_prefix("FHIR."))
            .unwrap_or(data_type);

        let patient_id = match (context_type, context_value) {
            (Some("Patient"), Some(CqlValue::Tuple(patient))) => {
                patient.get("id").and_then(CqlValue::as_string)
            }
            _ => None,
        };

        let filter = codes.map(|c| self.filter_codes(c));
        let code_path = code_property.or_else(|| fhir_primary_code_path(resource_type));

        self.resources(resource_type)
            .iter()
            .filter(|resource| {
                patient_id.is_none_or(|id| self.in_patient_context(resource, resource_type, id))
            })
            .filter(|resource| match (&filter, code_path) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(wanted), Some(path)) => {
                    let actual = property_path(resource, path);
                    code_matches(&actual, wanted)
                }
            })
            .filter(|resource| match (date_property, date_range) {
                (Some(path), Some(range)) => in_date_range(&property_path(resource, path), range),
                _ => true,
            })
            .cloned()
            .collect()
    }

    fn get_property(&self, _resource: &CqlValue, _path: &str) -> Option<CqlValue> {
        // Resources are tuples; the engine navigates them directly.
        None
    }
}

impl TerminologyProvider for InMemoryFhirData {
    fn in_value_set(&self, code: &CqlValue, value_set_id: &str) -> Option<bool> {
        let value_set = self.value_sets.get(value_set_id)?;
        let candidates: Vec<&CqlCode> = match code {
            CqlValue::Code(c) => vec![c],
            CqlValue::Concept(c) => c.codes.iter().collect(),
            CqlValue::String(s) => {
                return Some(value_set.codes.iter().any(|c| &c.code == s));
            }
            CqlValue::List(items) => {
                return Some(
                    items
                        .iter()
                        .any(|item| self.in_value_set(item, value_set_id) == Some(true)),
                );
            }
            _ => return None,
        };
        Some(candidates.iter().any(|candidate| {
            value_set
                .codes
                .iter()
                .any(|c| c.code == candidate.code && c.system == candidate.system)
        }))
    }

    fn in_code_system(&self, code: &CqlValue, code_system_id: &str) -> Option<bool> {
        match code {
            CqlValue::Code(c) => Some(c.system == code_system_id),
            CqlValue::Concept(c) => Some(c.codes.iter().any(|c| c.system == code_system_id)),
            _ => None,
        }
    }

    fn expand_value_set(&self, value_set_id: &str) -> Option<Vec<CqlValue>> {
        self.value_sets
            .get(value_set_id)
            .map(|vs| vs.codes.iter().cloned().map(CqlValue::Code).collect())
    }

    fn lookup_display(&self, code: &CqlValue) -> Option<String> {
        let CqlValue::Code(code) = code else {
            return None;
        };
        self.value_sets
            .values()
            .flat_map(|vs| vs.codes.iter())
            .find(|c| c.code == code.code && c.system == code.system)
            .and_then(|c| c.display.clone())
    }
}

/// `Type/id` of a resource.
fn local_reference(resource: &Value) -> Option<String> {
    let resource_type = resource.get("resourceType")?.as_str()?;
    let id = resource.get("id")?.as_str()?;
    Some(format!("{resource_type}/{id}"))
}

fn collect_contains(contains: &[Value], codes: &mut Vec<CqlCode>) {
    for entry in contains {
        if let (Some(system), Some(code)) = (
            entry.get("system").and_then(Value::as_str),
            entry.get("code").and_then(Value::as_str),
        ) {
            codes.push(CqlCode {
                code: code.to_string(),
                system: system.to_string(),
                version: entry
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                display: entry
                    .get("display")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        if let Some(nested) = entry.get("contains").and_then(Value::as_array) {
            collect_contains(nested, codes);
        }
    }
}

/// Follow a dotted property path through tuples (lists are flattened).
fn property_path(value: &CqlValue, path: &str) -> CqlValue {
    path.split('.')
        .fold(value.clone(), |current, segment| match current {
            CqlValue::Tuple(t) => t.get(segment).cloned().unwrap_or(CqlValue::Null),
            CqlValue::List(items) => CqlValue::List(octofhir_cql_types::CqlList::from_elements(
                items
                    .iter()
                    .map(|item| property_path(item, segment))
                    .filter(|v| !v.is_null())
                    .collect(),
            )),
            _ => CqlValue::Null,
        })
}

/// Whether a resource's code value is equivalent to one of the wanted codes.
fn code_matches(actual: &CqlValue, wanted: &[CqlValue]) -> bool {
    match actual {
        CqlValue::List(items) => items.iter().any(|item| code_matches(item, wanted)),
        CqlValue::Null => false,
        _ => wanted
            .iter()
            .any(|w| cql_equivalent(actual, w).unwrap_or(false)),
    }
}

/// Whether a date (or interval) value lies within / overlaps a date range.
fn in_date_range(value: &CqlValue, range: &CqlValue) -> bool {
    let CqlValue::Interval(range) = range else {
        return true;
    };
    let after_low = |point: &CqlValue| match range.low.as_deref() {
        None => true,
        Some(low) => matches!(
            cql_compare(point, low),
            Ok(Some(Ordering::Greater)) | Ok(Some(Ordering::Equal))
        ),
    };
    let before_high = |point: &CqlValue| match range.high.as_deref() {
        None => true,
        Some(high) => matches!(
            cql_compare(point, high),
            Ok(Some(Ordering::Less)) | Ok(Some(Ordering::Equal))
        ),
    };
    match value {
        CqlValue::Interval(i) => {
            i.high.as_deref().is_none_or(after_low) && i.low.as_deref().is_none_or(before_high)
        }
        CqlValue::Null => false,
        point => after_low(point) && before_high(point),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::TerminologyProvider;
    use serde_json::json;

    fn data() -> InMemoryFhirData {
        let mut data = InMemoryFhirData::new();
        data.add_json(&json!({
            "resourceType": "Bundle",
            "entry": [
                { "fullUrl": "urn:uuid:1", "resource": { "resourceType": "Patient", "id": "p1" } },
                { "resource": { "resourceType": "Patient", "id": "p2" } },
                { "resource": { "resourceType": "Condition", "id": "c1",
                    "subject": { "reference": "urn:uuid:1" },
                    "code": { "coding": [{ "system": "http://snomed.info/sct", "code": "44054006" }] } } },
                { "resource": { "resourceType": "Condition", "id": "c2",
                    "subject": { "reference": "Patient/p2" },
                    "code": { "coding": [{ "system": "http://snomed.info/sct", "code": "1" }] } } },
                { "resource": { "resourceType": "ValueSet", "url": "http://example.org/diabetes",
                    "compose": { "include": [{ "system": "http://snomed.info/sct",
                        "concept": [{ "code": "44054006" }] }] } } }
            ]
        }));
        data
    }

    fn patient(data: &InMemoryFhirData, id: &str) -> CqlValue {
        data.resources("Patient")
            .iter()
            .find(|p| matches!(p, CqlValue::Tuple(t) if t.get("id") == Some(&CqlValue::string(id))))
            .cloned()
            .unwrap()
    }

    #[test]
    fn retrieves_in_patient_context() {
        let data = data();
        let p1 = patient(&data, "p1");
        let conditions = data.retrieve(
            "{http://hl7.org/fhir}Condition",
            Some("Patient"),
            Some(&p1),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn filters_by_value_set() {
        let data = data();
        let value_set = CqlValue::Tuple(octofhir_cql_types::CqlTuple::from_elements([(
            "id",
            CqlValue::string("http://example.org/diabetes"),
        )]));
        let conditions = data.retrieve(
            "Condition",
            None,
            None,
            None,
            Some("code"),
            Some(&value_set),
            None,
            None,
        );
        assert_eq!(conditions.len(), 1);
        assert_eq!(
            data.in_value_set(
                &CqlValue::Code(CqlCode::new(
                    "44054006",
                    "http://snomed.info/sct",
                    None::<String>,
                    None::<String>
                )),
                "http://example.org/diabetes"
            ),
            Some(true)
        );
        assert_eq!(
            data.in_value_set(&CqlValue::string("x"), "http://unknown"),
            None
        );
    }
}
