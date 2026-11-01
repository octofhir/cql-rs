//! Terminology integration
//!
//! This module provides integration with octofhir-fhir-model's TerminologyProvider
//! for CQL terminology operations (InValueSet, InCodeSystem, etc.)

use crate::context::TerminologyProvider as EvalTerminologyProvider;
use octofhir_cql_types::{CqlCode, CqlValue};
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
        // A Concept is in the value set if any of its codes is.
        let codes: Vec<&CqlCode> = match code {
            CqlValue::Code(c) => vec![c],
            CqlValue::Concept(concept) => concept.codes.iter().collect(),
            _ => return None,
        };

        let mut unknown = false;
        for cql_code in codes {
            let result = crate::blocking::block_on(self.provider.validate_code_vs(
                value_set_id,
                Some(cql_code.system.as_str()),
                &cql_code.code,
                cql_code.display.as_deref(),
            ))
            .and_then(Result::ok);
            match result {
                Some(validation) if validation.result => return Some(true),
                Some(_) => {}
                None => unknown = true,
            }
        }
        if unknown { None } else { Some(false) }
    }

    fn in_code_system(&self, code: &CqlValue, code_system_id: &str) -> Option<bool> {
        match code {
            CqlValue::Code(c) => Some(c.system == code_system_id),
            CqlValue::Concept(concept) => {
                Some(concept.codes.iter().any(|c| c.system == code_system_id))
            }
            _ => None,
        }
    }

    fn expand_value_set(&self, value_set_id: &str) -> Option<Vec<CqlValue>> {
        let expansion =
            crate::blocking::block_on(self.provider.expand_valueset(value_set_id, None))?.ok()?;
        Some(
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
                .collect(),
        )
    }

    fn lookup_display(&self, code: &CqlValue) -> Option<String> {
        let cql_code = match code {
            CqlValue::Code(c) => c,
            CqlValue::Concept(concept) => concept.codes.first()?,
            _ => return None,
        };

        if let Some(display) = &cql_code.display {
            return Some(display.clone());
        }

        crate::blocking::block_on(self.provider.lookup_code(
            &cql_code.system,
            &cql_code.code,
            cql_code.version.as_deref(),
            None,
        ))?
        .ok()?
        .display
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

    #[tokio::test(flavor = "multi_thread")]
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
