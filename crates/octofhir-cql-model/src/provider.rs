//! Data provider traits for CQL evaluation

use async_trait::async_trait;
use serde_json::Value;
use crate::model_info::{TypeInfo, PropertyInfo};
use octofhir_cql_types::{CqlCode, CqlInterval, CqlValue};

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

/// Trait for providing model metadata (type and property information)
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Get type information by type name
    async fn get_type(&self, type_name: &str) -> Result<Option<TypeInfo>, ModelProviderError>;

    /// Get property type information for a given parent type and property name
    async fn get_property_type(&self, parent: &str, property: &str) -> Result<Option<PropertyInfo>, ModelProviderError>;

    /// Check if a type is retrievable (can be used in Retrieve expressions)
    fn is_retrievable(&self, type_name: &str) -> bool;

    /// Get the primary code path for a type (used for terminology filtering)
    fn get_primary_code_path(&self, type_name: &str) -> Option<String>;
}

/// Model provider error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModelProviderError {
    #[error("Type not found: {0}")]
    TypeNotFound(String),

    #[error("Property not found: {parent}.{property}")]
    PropertyNotFound { parent: String, property: String },

    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Trait for retrieving data from a data source (aligned with CQL Retrieve expression)
#[async_trait]
pub trait DataRetriever: Send + Sync {
    /// Retrieve data with optional filtering
    async fn retrieve(
        &self,
        context: &str,                     // "Patient", "Encounter", etc.
        data_type: &str,                   // "Observation", "Condition", etc.
        code_path: Option<&str>,           // "code"
        codes: Option<&[CqlCode]>,         // Specific codes to filter by
        valueset: Option<&str>,            // ValueSet URL for terminology filtering
        date_path: Option<&str>,           // "effective", "onset", etc.
        date_range: Option<&CqlInterval>,  // Date range for filtering
    ) -> Result<Vec<CqlValue>, DataRetrieverError>;
}

/// Data retriever error
#[derive(Debug, thiserror::Error)]
pub enum DataRetrieverError {
    #[error("Retrieve failed: {0}")]
    RetrieveFailed(String),

    #[error("Type not retrievable: {0}")]
    TypeNotRetrievable(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Terminology error: {0}")]
    TerminologyError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
