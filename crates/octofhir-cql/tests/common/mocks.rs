//! Mock implementations for testing
//!
//! Provides configurable mock implementations of DataProvider, TerminologyProvider,
//! and DataRetriever for comprehensive testing scenarios.

use indexmap::IndexMap;
use octofhir_cql_types::{CqlCode, CqlConcept, CqlTuple, CqlValue};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Mock terminology provider with configurable responses
pub struct MockTerminologyProvider {
    value_set_memberships: Arc<RwLock<HashMap<(String, String), bool>>>,
    code_system_memberships: Arc<RwLock<HashMap<(String, String), bool>>>,
    value_set_expansions: Arc<RwLock<HashMap<String, Vec<CqlValue>>>>,
    displays: Arc<RwLock<HashMap<String, String>>>,
}

impl MockTerminologyProvider {
    pub fn new() -> Self {
        Self {
            value_set_memberships: Arc::new(RwLock::new(HashMap::new())),
            code_system_memberships: Arc::new(RwLock::new(HashMap::new())),
            value_set_expansions: Arc::new(RwLock::new(HashMap::new())),
            displays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Configure a code to be in a value set
    pub fn add_to_value_set(&self, code: impl Into<String>, value_set: impl Into<String>) {
        self.value_set_memberships
            .write()
            .insert((code.into(), value_set.into()), true);
    }

    /// Configure a code to be in a code system
    pub fn add_to_code_system(&self, code: impl Into<String>, code_system: impl Into<String>) {
        self.code_system_memberships
            .write()
            .insert((code.into(), code_system.into()), true);
    }

    /// Configure a value set expansion
    pub fn set_expansion(&self, value_set: impl Into<String>, codes: Vec<CqlValue>) {
        self.value_set_expansions
            .write()
            .insert(value_set.into(), codes);
    }

    /// Configure a display name for a code
    pub fn set_display(&self, code: impl Into<String>, display: impl Into<String>) {
        self.displays.write().insert(code.into(), display.into());
    }
}

impl Default for MockTerminologyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl octofhir_cql_eval::context::TerminologyProvider for MockTerminologyProvider {
    fn in_value_set(&self, code: &CqlValue, value_set_id: &str) -> Option<bool> {
        let code_str = match code {
            CqlValue::Code(c) => &c.code,
            CqlValue::Concept(concept) => &concept.codes.first()?.code,
            _ => return None,
        };

        Some(
            *self
                .value_set_memberships
                .read()
                .get(&(code_str.clone(), value_set_id.to_string()))
                .unwrap_or(&false),
        )
    }

    fn in_code_system(&self, code: &CqlValue, code_system_id: &str) -> Option<bool> {
        let code_str = match code {
            CqlValue::Code(c) => &c.code,
            CqlValue::Concept(concept) => &concept.codes.first()?.code,
            _ => return None,
        };

        Some(
            *self
                .code_system_memberships
                .read()
                .get(&(code_str.clone(), code_system_id.to_string()))
                .unwrap_or(&false),
        )
    }

    fn expand_value_set(&self, value_set_id: &str) -> Option<Vec<CqlValue>> {
        self.value_set_expansions
            .read()
            .get(value_set_id)
            .cloned()
    }

    fn lookup_display(&self, code: &CqlValue) -> Option<String> {
        let code_str = match code {
            CqlValue::Code(c) => &c.code,
            CqlValue::Concept(concept) => &concept.codes.first()?.code,
            _ => return None,
        };

        self.displays.read().get(code_str).cloned()
    }
}

/// Mock data provider with configurable data sets
pub struct MockDataProvider {
    data: Arc<RwLock<HashMap<String, Vec<CqlValue>>>>,
}

impl MockDataProvider {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add data for a specific type
    pub fn add_data(&self, data_type: impl Into<String>, values: Vec<CqlValue>) {
        self.data.write().insert(data_type.into(), values);
    }

    /// Add a single resource
    pub fn add_resource(&self, data_type: impl Into<String>, value: CqlValue) {
        let data_type = data_type.into();
        self.data
            .write()
            .entry(data_type)
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Clear all data
    pub fn clear(&self) {
        self.data.write().clear();
    }
}

impl Default for MockDataProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl octofhir_cql_eval::context::DataProvider for MockDataProvider {
    fn retrieve(
        &self,
        data_type: &str,
        _context_type: Option<&str>,
        _context_value: Option<&CqlValue>,
        _template_id: Option<&str>,
        code_property: Option<&str>,
        codes: Option<&CqlValue>,
        _date_property: Option<&str>,
        _date_range: Option<&CqlValue>,
    ) -> Vec<CqlValue> {
        let data = self.data.read();
        let resources = match data.get(data_type) {
            Some(res) => res.clone(),
            None => return vec![],
        };

        // If no code filter, return all resources
        if codes.is_none() || code_property.is_none() {
            return resources;
        }

        // Apply code filtering
        let filter_codes = codes.unwrap();
        let code_path = code_property.unwrap();

        resources
            .into_iter()
            .filter(|resource| {
                // Extract the code from the resource
                let resource_code = self.get_property(resource, code_path);
                if let Some(rc) = resource_code {
                    // Check if it matches any of the filter codes
                    match filter_codes {
                        CqlValue::Code(fc) => {
                            if let CqlValue::Code(rc) = rc {
                                fc.code == rc.code
                            } else {
                                false
                            }
                        }
                        CqlValue::List(list) => list.elements.iter().any(|fc| {
                            if let (CqlValue::Code(fc), CqlValue::Code(rc)) = (fc, &rc) {
                                fc.code == rc.code
                            } else {
                                false
                            }
                        }),
                        _ => false,
                    }
                } else {
                    false
                }
            })
            .collect()
    }

    fn get_property(&self, resource: &CqlValue, path: &str) -> Option<CqlValue> {
        match resource {
            CqlValue::Tuple(tuple) => tuple.get(path).cloned(),
            _ => None,
        }
    }
}

/// Mock data retriever for model layer testing
pub struct MockDataRetriever {
    data: Arc<RwLock<HashMap<String, Vec<CqlValue>>>>,
}

impl MockDataRetriever {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add data for a specific type
    pub fn add_data(&self, data_type: impl Into<String>, values: Vec<CqlValue>) {
        self.data.write().insert(data_type.into(), values);
    }

    /// Add a single resource
    pub fn add_resource(&self, data_type: impl Into<String>, value: CqlValue) {
        let data_type = data_type.into();
        self.data
            .write()
            .entry(data_type)
            .or_insert_with(Vec::new)
            .push(value);
    }
}

impl Default for MockDataRetriever {
    fn default() -> Self {
        Self::new()
    }
}

use async_trait::async_trait;
use octofhir_cql_model::{DataRetriever, DataRetrieverError};
use octofhir_cql_types::CqlInterval;

#[async_trait]
impl DataRetriever for MockDataRetriever {
    async fn retrieve(
        &self,
        _context: &str,
        data_type: &str,
        _code_path: Option<&str>,
        codes: Option<&[CqlCode]>,
        _valueset: Option<&str>,
        _date_path: Option<&str>,
        _date_range: Option<&CqlInterval>,
    ) -> Result<Vec<CqlValue>, DataRetrieverError> {
        let data = self.data.read();
        let resources = match data.get(data_type) {
            Some(res) => res.clone(),
            None => return Ok(vec![]),
        };

        // If no code filter, return all resources
        if codes.is_none() {
            return Ok(resources);
        }

        // Apply code filtering
        let filter_codes = codes.unwrap();
        let filtered: Vec<CqlValue> = resources
            .into_iter()
            .filter(|resource| {
                // Simple code matching for testing
                if let CqlValue::Tuple(tuple) = resource {
                    if let Some(CqlValue::Code(code)) = tuple.get("code") {
                        return filter_codes.iter().any(|fc| fc.code == code.code);
                    }
                }
                false
            })
            .collect();

        Ok(filtered)
    }
}

/// Helper to create a simple FHIR-like patient resource
pub fn mock_patient(id: &str, name: &str) -> CqlValue {
    CqlValue::Tuple(CqlTuple::from_elements([
        ("resourceType", CqlValue::string("Patient")),
        ("id", CqlValue::string(id)),
        ("name", CqlValue::string(name)),
    ]))
}

/// Helper to create a simple FHIR-like observation resource
pub fn mock_observation(id: &str, code: CqlCode, value: CqlValue) -> CqlValue {
    CqlValue::Tuple(CqlTuple::from_elements([
        ("resourceType", CqlValue::string("Observation")),
        ("id", CqlValue::string(id)),
        ("code", CqlValue::Code(code)),
        ("value", value),
    ]))
}

/// Helper to create a simple FHIR-like condition resource
pub fn mock_condition(id: &str, code: CqlCode) -> CqlValue {
    CqlValue::Tuple(CqlTuple::from_elements([
        ("resourceType", CqlValue::string("Condition")),
        ("id", CqlValue::string(id)),
        ("code", CqlValue::Code(code)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_terminology_provider() {
        let provider = MockTerminologyProvider::new();
        provider.add_to_value_set("8480-6", "http://test.com/vs/bp");

        let code = CqlValue::Code(CqlCode {
            system: "http://loinc.org".to_string(),
            version: None,
            code: "8480-6".to_string(),
            display: Some("Systolic BP".to_string()),
        });

        assert_eq!(
            provider.in_value_set(&code, "http://test.com/vs/bp"),
            Some(true)
        );
        assert_eq!(
            provider.in_value_set(&code, "http://test.com/vs/other"),
            Some(false)
        );
    }

    #[test]
    fn test_mock_data_provider() {
        let provider = MockDataProvider::new();
        let patient = mock_patient("p1", "John Doe");
        provider.add_resource("Patient", patient.clone());

        let results = provider.retrieve("Patient", None, None, None, None, None, None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(
            provider.get_property(&results[0], "id"),
            Some(CqlValue::string("p1"))
        );
    }

    #[tokio::test]
    async fn test_mock_data_retriever() {
        let retriever = MockDataRetriever::new();
        let obs = mock_observation(
            "o1",
            CqlCode {
                system: "http://loinc.org".to_string(),
                version: None,
                code: "8480-6".to_string(),
                display: Some("Systolic BP".to_string()),
            },
            CqlValue::integer(120),
        );
        retriever.add_resource("Observation", obs);

        let results = retriever
            .retrieve("Patient", "Observation", None, None, None, None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }
}
