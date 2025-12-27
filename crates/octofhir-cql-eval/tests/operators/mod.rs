//! Operator integration tests for CQL evaluation
//!
//! These tests verify operator behavior including:
//! - Correct computation for various input types
//! - Null propagation according to CQL specification
//! - Three-valued logic for logical operators
//! - Edge cases and boundary conditions

pub mod arithmetic;
pub mod comparison;
pub mod logical;
pub mod string;
pub mod datetime;
pub mod interval;
pub mod list;
pub mod aggregate;
