//! CQL data model abstraction
//!
//! This crate provides:
//! - ModelInfo abstraction for FHIR and other data models
//! - Version-agnostic FHIR support
//! - Data provider traits

pub mod model_info;
pub mod provider;

pub use model_info::*;
pub use provider::*;
