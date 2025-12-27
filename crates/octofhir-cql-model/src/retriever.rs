//! Data retriever module
//!
//! This module provides utilities for implementing DataRetriever.

use crate::provider::{DataRetriever, DataRetrieverError};
use octofhir_cql_types::{CqlCode, CqlInterval, CqlValue};
use async_trait::async_trait;

/// NoOp data retriever for testing
pub struct NoOpDataRetriever;

#[async_trait]
impl DataRetriever for NoOpDataRetriever {
    async fn retrieve(
        &self,
        _context: &str,
        _data_type: &str,
        _code_path: Option<&str>,
        _codes: Option<&[CqlCode]>,
        _valueset: Option<&str>,
        _date_path: Option<&str>,
        _date_range: Option<&CqlInterval>,
    ) -> Result<Vec<CqlValue>, DataRetrieverError> {
        Ok(vec![])
    }
}

impl NoOpDataRetriever {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpDataRetriever {
    fn default() -> Self {
        Self::new()
    }
}
