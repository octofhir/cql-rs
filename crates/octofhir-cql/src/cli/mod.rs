//! CLI functionality for the CQL tool
//!
//! This module contains all CLI-related functionality including:
//! - Command execution
//! - Translation
//! - Validation
//! - REPL
//! - Library resolution
//! - Output formatting

#[cfg(feature = "cli")]
pub mod execute;
#[cfg(feature = "cli")]
pub mod output;
#[cfg(feature = "cli")]
pub mod repl;
#[cfg(feature = "cli")]
pub mod resolver;
#[cfg(feature = "cli")]
pub mod translate;
#[cfg(feature = "cli")]
pub mod validate;
