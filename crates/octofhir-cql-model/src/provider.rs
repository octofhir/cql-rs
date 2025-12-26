//! Data provider traits for CQL evaluation

use async_trait::async_trait;
use serde_json::Value;

/// Trait for providing data to CQL evaluation
#[async_trait]
pub trait DataProvider: Send + Sync {
    /// Retrieve data of a given type with optional code filter
    async fn retrieve(
        &self,
        context: &RetrieveContext,
    ) -> Result<Vec<Value>, DataProviderError>;

    /// Get the model name this provider supports (e.g., "FHIR")
    fn model_name(&self) -> &str;

    /// Get the model version
    fn model_version(&self) -> &str;
}

/// Context for a retrieve operation
#[derive(Debug, Clone)]
pub struct RetrieveContext {
    /// Data type to retrieve (e.g., "Condition", "Observation")
    pub data_type: String,
    /// Template ID / profile URL
    pub template_id: Option<String>,
    /// Code path for filtering
    pub code_path: Option<String>,
    /// Codes to filter by
    pub codes: Option<Vec<CodeValue>>,
    /// Date path for filtering
    pub date_path: Option<String>,
    /// Date range for filtering
    pub date_range: Option<DateRange>,
    /// Context value (e.g., Patient ID)
    pub context_value: Option<String>,
}

impl RetrieveContext {
    pub fn new(data_type: impl Into<String>) -> Self {
        Self {
            data_type: data_type.into(),
            template_id: None,
            code_path: None,
            codes: None,
            date_path: None,
            date_range: None,
            context_value: None,
        }
    }
}

/// Code value for filtering
#[derive(Debug, Clone)]
pub struct CodeValue {
    /// Code system URI
    pub system: Option<String>,
    /// Code value
    pub code: String,
    /// Display text
    pub display: Option<String>,
}

/// Date range for filtering
#[derive(Debug, Clone)]
pub struct DateRange {
    /// Start date (ISO format)
    pub start: Option<String>,
    /// End date (ISO format)
    pub end: Option<String>,
}

/// Data provider error
#[derive(Debug, thiserror::Error)]
pub enum DataProviderError {
    #[error("Retrieve failed: {0}")]
    RetrieveFailed(String),

    #[error("Type not found: {0}")]
    TypeNotFound(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
