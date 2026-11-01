//! FHIR ModelInfo support
//!
//! - [`json`]: FHIR JSON ⇄ CQL value conversion (FHIRHelpers semantics)
//! - embedded FHIR ModelInfo for R4 and R5

pub mod json;
pub mod r4;
pub mod r5;

pub use r4::*;
pub use r5::*;
