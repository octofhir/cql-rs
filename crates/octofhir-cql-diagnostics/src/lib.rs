//! CQL diagnostics and error handling
//!
//! This crate provides the error handling infrastructure for the CQL implementation,
//! including error codes, source locations, and diagnostic reporting.

mod error;
mod error_code;
mod span;

pub use error::*;
pub use error_code::*;
pub use span::*;

/// Result type for CQL operations
pub type Result<T> = std::result::Result<T, CqlError>;
