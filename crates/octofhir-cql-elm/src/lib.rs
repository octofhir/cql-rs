//! CQL ELM (Expression Logical Model) representation and serialization
//!
//! This crate provides:
//! - ELM data structures for CQL compilation output
//! - JSON and XML serialization (compatible with HL7 ELM spec)
//! - AST to ELM translation

pub mod converter;
pub mod model;
pub mod serialize;

pub use converter::AstToElmConverter;
pub use model::*;
pub use serialize::{ElmSerializer, JsonSerializer, XmlSerializer};
