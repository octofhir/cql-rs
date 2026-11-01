//! Retrieve expression evaluation
//!
//! This module provides integration between the evaluation engine and DataRetriever
//! for evaluating CQL Retrieve expressions.

use crate::context::DataProvider;
use octofhir_cql_model::DataRetriever;
use octofhir_cql_types::{CqlCode, CqlValue};
use std::sync::Arc;

/// Adapter that wraps a DataRetriever to implement the eval crate's DataProvider trait
pub struct DataRetrieverAdapter {
    retriever: Arc<dyn DataRetriever>,
    context: String,
}

impl DataRetrieverAdapter {
    /// Create a new adapter for a DataRetriever with the given context (e.g., "Patient")
    pub fn new(retriever: Arc<dyn DataRetriever>, context: impl Into<String>) -> Self {
        Self {
            retriever,
            context: context.into(),
        }
    }

    /// Update the context
    pub fn set_context(&mut self, context: impl Into<String>) {
        self.context = context.into();
    }

    /// Get the current context
    pub fn context(&self) -> &str {
        &self.context
    }
}

impl DataProvider for DataRetrieverAdapter {
    fn retrieve(
        &self,
        data_type: &str,
        _context_type: Option<&str>,
        _context_value: Option<&CqlValue>,
        template_id: Option<&str>,
        code_property: Option<&str>,
        codes: Option<&CqlValue>,
        date_property: Option<&str>,
        date_range: Option<&CqlValue>,
    ) -> Vec<CqlValue> {
        // Convert codes from CqlValue to Vec<CqlCode>
        let code_list = codes.and_then(|c| match c {
            CqlValue::Code(code) => Some(vec![code.clone()]),
            CqlValue::List(list) => {
                let codes: Option<Vec<CqlCode>> = list
                    .elements
                    .iter()
                    .map(|v| match v {
                        CqlValue::Code(c) => Some(c.clone()),
                        _ => None,
                    })
                    .collect();
                codes
            }
            _ => None,
        });

        // Convert date_range from CqlValue to CqlInterval
        let interval = date_range.and_then(|d| match d {
            CqlValue::Interval(interval) => Some(interval.clone()),
            _ => None,
        });

        // The evaluator is synchronous; drive the async retriever to completion.
        // Retrieval errors yield no resources (the `DataProvider` contract has no
        // error channel); implement `DataProvider` directly to surface them.
        let result = crate::blocking::block_on(self.retriever.retrieve(
            &self.context,
            data_type,
            code_property,
            code_list.as_deref(),
            template_id,
            date_property,
            interval.as_ref(),
        ))
        .and_then(Result::ok);

        result.unwrap_or_default()
    }

    fn get_property(&self, resource: &CqlValue, path: &str) -> Option<CqlValue> {
        // Basic property access for tuples
        match resource {
            CqlValue::Tuple(tuple) => tuple.get(path).cloned(),
            _ => None,
        }
    }
}

/// Helper to extract codes from a CqlValue (useful for FHIR CodeableConcept structures)
pub fn extract_codes(value: &CqlValue) -> Vec<CqlCode> {
    match value {
        CqlValue::Code(code) => vec![code.clone()],
        CqlValue::Concept(concept) => concept.codes.to_vec(),
        CqlValue::List(list) => list.elements.iter().flat_map(extract_codes).collect(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use octofhir_cql_model::NoOpDataRetriever;

    #[test]
    fn test_adapter_creation() {
        let retriever = Arc::new(NoOpDataRetriever::new()) as Arc<dyn DataRetriever>;
        let adapter = DataRetrieverAdapter::new(retriever, "Patient");
        assert_eq!(adapter.context(), "Patient");
    }

    #[test]
    fn test_extract_codes() {
        let code = CqlCode {
            system: "http://loinc.org".to_string(),
            version: None,
            code: "8480-6".to_string(),
            display: Some("Systolic blood pressure".to_string()),
        };

        let value = CqlValue::Code(code.clone());
        let codes = extract_codes(&value);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].code, "8480-6");
    }
}
