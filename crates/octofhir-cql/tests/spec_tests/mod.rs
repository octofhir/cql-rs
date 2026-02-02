//! CQFramework Specification Tests
//!
//! This module provides infrastructure for running the official CQL specification
//! tests from cqframework/cql-tests repository.
//!
//! The tests are expressed in XML format and verify correct behavior of CQL
//! language capabilities.

pub mod runner;
pub mod xml_parser;

pub use runner::*;
pub use xml_parser::*;
