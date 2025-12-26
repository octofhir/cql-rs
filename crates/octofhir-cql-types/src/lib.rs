//! CQL type system
//!
//! This crate defines the CQL type system including:
//! - System types (Boolean, Integer, Decimal, String, Date, DateTime, Time, etc.)
//! - Type inference and checking
//! - Type conversion rules

pub mod system_types;

pub use system_types::*;
