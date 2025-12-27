//! Clinical Quality Language (CQL) implementation for Rust
//!
//! This crate provides a complete CQL 1.5 implementation including:
//! - Parsing CQL expressions and libraries
//! - Type checking and semantic analysis
//! - ELM (Expression Logical Model) output
//! - Expression evaluation
//! - Version-agnostic FHIR support
//!
//! # Example
//!
//! ```ignore
//! use octofhir_cql::parse;
//!
//! let cql = r#"
//! library Example version '1.0.0'
//!
//! define InPopulation:
//!     AgeInYears() >= 18
//! "#;
//!
//! let library = parse(cql)?;
//! ```

// Re-export all public APIs from internal crates
pub use octofhir_cql_ast as ast;
pub use octofhir_cql_diagnostics as diagnostics;
pub use octofhir_cql_elm as elm;
pub use octofhir_cql_eval as eval;
pub use octofhir_cql_model as model;
pub use octofhir_cql_parser as parser;
pub use octofhir_cql_types as types;

// Convenience re-exports
pub use octofhir_cql_ast::{Expression, Library};
pub use octofhir_cql_diagnostics::{CqlError, Result};
pub use octofhir_cql_parser::parse;

// CLI module (only available with cli feature)
#[cfg(feature = "cli")]
pub mod cli;
