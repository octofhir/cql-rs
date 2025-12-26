//! CQL Abstract Syntax Tree definitions
//!
//! This crate defines the AST nodes for CQL (Clinical Quality Language) 1.5.
//! The AST closely mirrors the CQL grammar while providing a clean Rust API.

mod expression;
mod library;
mod literal;
mod operator;
mod query;
mod types;

pub use expression::*;
pub use library::*;
pub use literal::*;
pub use operator::*;
pub use query::*;
pub use types::*;

use octofhir_cql_diagnostics::Span;

/// A node with source span information
pub type Spanned<T> = octofhir_cql_diagnostics::Spanned<T>;

/// Type alias for boxed expressions
pub type BoxExpr = Box<Spanned<Expression>>;

/// Type alias for optional boxed expressions
pub type OptBoxExpr = Option<Box<Spanned<Expression>>>;

/// An identifier with optional qualifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    /// The identifier text
    pub name: String,
    /// Whether this is a quoted (delimited) identifier
    pub quoted: bool,
}

impl Identifier {
    /// Create a new identifier
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            quoted: false,
        }
    }

    /// Create a quoted identifier
    pub fn quoted(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            quoted: true,
        }
    }
}

impl From<&str> for Identifier {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// A qualified identifier (e.g., "Library.Identifier")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedIdentifier {
    /// Optional library qualifier
    pub qualifier: Option<String>,
    /// The identifier
    pub name: Identifier,
}

impl QualifiedIdentifier {
    /// Create a simple unqualified identifier
    pub fn simple(name: impl Into<Identifier>) -> Self {
        Self {
            qualifier: None,
            name: name.into(),
        }
    }

    /// Create a qualified identifier
    pub fn qualified(qualifier: impl Into<String>, name: impl Into<Identifier>) -> Self {
        Self {
            qualifier: Some(qualifier.into()),
            name: name.into(),
        }
    }
}

/// Version specifier for libraries
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSpecifier {
    /// The version string (e.g., "1.0.0")
    pub version: String,
}

impl VersionSpecifier {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }
}

/// Access modifier for definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessModifier {
    /// Public access (default)
    #[default]
    Public,
    /// Private access
    Private,
}
