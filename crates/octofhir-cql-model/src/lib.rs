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
pub mod registry;
pub mod retriever;

pub use model_info::*;
pub use provider::*;
pub use registry::*;
pub use retriever::*;

// Re-export terminology provider from octofhir-fhir-model
pub use octofhir_fhir_model::TerminologyProvider;
