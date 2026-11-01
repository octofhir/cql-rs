#![doc = include_str!("../README.md")]

mod error;
mod error_code;
mod span;

pub use error::*;
pub use error_code::*;
pub use span::*;

/// Result type for CQL operations
pub type Result<T> = std::result::Result<T, CqlError>;
