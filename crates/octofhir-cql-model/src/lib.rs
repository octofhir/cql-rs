//! CQL data model abstraction
//!
//! This crate provides:
//! - ModelInfo abstraction for FHIR and other data models
//! - Version-agnostic FHIR support
//! - Data provider traits
//! - Model provider implementation
//! - Data retriever abstraction

pub mod fhir;
pub mod model_info;
pub mod provider;
pub mod retriever;
pub mod registry;

pub use model_info::*;
pub use provider::*;
pub use retriever::*;
pub use registry::*;

// Re-export terminology provider from octofhir-fhir-model
pub use octofhir_fhir_model::TerminologyProvider;
