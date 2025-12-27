//! FHIR ModelInfo support
//!
//! This module provides embedded FHIR ModelInfo for R4 and R5.

pub mod r4;
pub mod r5;

pub use r4::*;
pub use r5::*;
