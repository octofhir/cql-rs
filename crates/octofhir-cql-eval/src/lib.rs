//! CQL evaluation engine
//!
//! This crate provides:
//! - Expression evaluation
//! - Operator implementations
//! - Function implementations
//! - Evaluation context management

pub mod context;
pub mod value;

pub use context::*;
pub use value::*;
