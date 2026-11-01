#![doc = include_str!("../README.md")]

// Re-export all public APIs from internal crates
pub use octofhir_cql_ast as ast;
pub use octofhir_cql_diagnostics as diagnostics;
pub use octofhir_cql_elm as elm;
pub use octofhir_cql_eval as eval;
pub use octofhir_cql_model as model;
pub use octofhir_cql_parser as parser;
pub use octofhir_cql_types as types;

pub mod library;

// Convenience re-exports
pub use library::{CompiledLibrary, DirectoryLibrarySource, LibrarySource, NoLibraries};
pub use octofhir_cql_ast::{Expression, Library};
pub use octofhir_cql_diagnostics::{CqlError, Result};
pub use octofhir_cql_parser::parse;

// CLI module (only available with cli feature)
#[cfg(feature = "cli")]
pub mod cli;

/// The workspace README's examples, compiled and run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
pub struct WorkspaceReadmeDoctests;
