//! Terminology integration
//!
//! This module provides integration with octofhir-fhir-model's TerminologyProvider
//! for CQL terminology operations (InValueSet, InCodeSystem, etc.)

use crate::context::TerminologyProvider as EvalTerminologyProvider;
use octofhir_cql_types::{CqlCode, CqlConcept, CqlValue};
use octofhir_fhir_model::TerminologyProvider as FhirTerminologyProvider;
use std::sync::Arc;

/// Adapter that wraps a FHIR TerminologyProvider to implement the eval crate's TerminologyProvider trait
pub struct TerminologyAdapter {
    provider: Arc<dyn FhirTerminologyProvider>,
}

impl TerminologyAdapter {
    /// Create a new adapter for a FHIR TerminologyProvider
    pub fn new(provider: Arc<dyn FhirTerminologyProvider>) -> Self {
        Self { provider }
    }

    /// Get the underlying FHIR terminology provider
    pub fn inner(&self) -> &Arc<dyn FhirTerminologyProvider> {
        &self.provider
    }
}

impl EvalTerminologyProvider for TerminologyAdapter {
    fn in_value_set(&self, code: &CqlValue, value_set_id: &str) -> Option<bool> {
        // Extract code from CqlValue
        let cql_code = match code {
            CqlValue::Code(c) => c,
            CqlValue::Concept(concept) => concept.codes.first()?,
            _ => return None,
        };

        // Call the FHIR terminology provider
        // This is async in the FHIR provider, so we need to handle it
        let code_str = &cql_code.code;
        let system = Some(cql_code.system.as_str());
        let display = cql_code.display.as_deref();

        let result = tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| {
                handle.block_on(async {
                    self.provider
                        .validate_code_vs(value_set_id, system, code_str, display)
                        .await
                        .ok()
                })
            });

        result.map(|validation_result| validation_result.result)
    }

    fn in_code_system(&self, code: &CqlValue, code_system_id: &str) -> Option<bool> {
        // Extract code from CqlValue
        let cql_code = match code {
            CqlValue::Code(c) => c,
            CqlValue::Concept(concept) => concept.codes.first()?,
            _ => return None,
        };

        // Check if the code's system matches the given code system
        Some(&cql_code.system == code_system_id)
    }

    fn expand_value_set(&self, value_set_id: &str) -> Option<Vec<CqlValue>> {
        // Call the FHIR terminology provider to expand the value set
        let result = tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| {
                handle.block_on(async {
                    self.provider
                        .expand_valueset(value_set_id, None)
                        .await
                        .ok()
                })
            });

        result.map(|expansion| {
            expansion
                .contains
                .into_iter()
                .map(|concept| {
                    CqlValue::Code(CqlCode {
                        system: concept.system.unwrap_or_default(),
                        version: None,
                        code: concept.code,
                        display: concept.display,
                    })
                })
                .collect()
        })
    }

    fn lookup_display(&self, code: &CqlValue) -> Option<String> {
        // Extract code from CqlValue
        let cql_code = match code {
            CqlValue::Code(c) => c,
            CqlValue::Concept(concept) => concept.codes.first()?,
            _ => return None,
        };

        // If the code already has a display, return it
        if let Some(ref display) = cql_code.display {
            return Some(display.clone());
        }

        // Otherwise, try to look it up
        let system = &cql_code.system;
        let code_str = &cql_code.code;
        let version = cql_code.version.as_deref();

        let result = tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| {
                handle.block_on(async {
                    self.provider
                        .lookup_code(system, code_str, version, None)
                        .await
                        .ok()
                })
            });

        result.and_then(|lookup_result| lookup_result.display)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use octofhir_fhir_model::NoOpTerminologyProvider;

    #[test]
    fn test_adapter_creation() {
        let provider = Arc::new(NoOpTerminologyProvider) as Arc<dyn FhirTerminologyProvider>;
        let _adapter = TerminologyAdapter::new(provider);
    }

    #[tokio::test]
    async fn test_in_value_set() {
        let provider = Arc::new(NoOpTerminologyProvider) as Arc<dyn FhirTerminologyProvider>;
        let adapter = TerminologyAdapter::new(provider);

        let code = CqlValue::Code(CqlCode {
            system: "http://loinc.org".to_string(),
            version: None,
            code: "8480-6".to_string(),
            display: Some("Systolic blood pressure".to_string()),
        });

        let result = adapter.in_value_set(&code, "http://test.com/vs");
        assert!(result.is_some());
    }
}
