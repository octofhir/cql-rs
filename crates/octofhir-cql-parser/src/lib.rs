//! CQL parser using Winnow
//!
//! This crate provides a complete CQL 1.5 parser using Winnow with recursive descent
//! and precedence climbing for operator precedence.

mod combinators;
mod expression;
mod library;

pub use library::parse;
pub use library::parse_expression;
pub use library::parse_with_mode;

use octofhir_cql_ast::Library;
use octofhir_cql_diagnostics::{CqlError, Result};

/// Parser mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParseMode {
    /// Fast mode - fail on first error (for production)
    #[default]
    Fast,
    /// Analysis mode - collect all errors (for IDE/tooling)
    Analysis,
}

/// Parse result with optional errors
pub struct ParseResult {
    /// Parsed library (may be partial in analysis mode)
    pub library: Option<Library>,
    /// Parse errors (empty in fast mode on success)
    pub errors: Vec<CqlError>,
}

impl ParseResult {
    /// Create successful result
    pub fn success(library: Library) -> Self {
        Self {
            library: Some(library),
            errors: Vec::new(),
        }
    }

    /// Create error result
    pub fn error(errors: Vec<CqlError>) -> Self {
        Self {
            library: None,
            errors,
        }
    }

    /// Check if parsing succeeded without errors
    pub fn is_success(&self) -> bool {
        self.library.is_some() && self.errors.is_empty()
    }

    /// Convert to Result, returning first error if any
    pub fn into_result(self) -> Result<Library> {
        if self.errors.is_empty() {
            self.library.ok_or_else(|| {
                CqlError::parse(
                    octofhir_cql_diagnostics::CQL0002,
                    "Unexpected end of input",
                    "",
                )
            })
        } else if self.errors.len() == 1 {
            Err(self.errors.into_iter().next().unwrap())
        } else {
            Err(CqlError::Multiple(self.errors))
        }
    }
}
