//! CQL Evaluation Engine
//!
//! This crate provides complete CQL (Clinical Quality Language) expression evaluation
//! against in-memory data. It implements the full CQL specification including:
//!
//! - **Arithmetic Operators**: Add, Subtract, Multiply, Divide, Power, etc.
//! - **Comparison Operators**: Equal, NotEqual, Less, Greater, etc.
//! - **Logical Operators**: And, Or, Not, Xor, Implies with three-valued logic
//! - **String Operators**: Concatenate, Split, Upper, Lower, Matches, etc.
//! - **DateTime Operators**: Date constructors, DurationBetween, SameAs, etc.
//! - **Interval Operators**: Contains, Overlaps, Union, Intersect, etc.
//! - **List Operators**: First, Last, Count, Flatten, Sort, etc.
//! - **Aggregate Functions**: Sum, Avg, Min, Max, Median, StdDev, etc.
//! - **Query Evaluation**: Sources, Let, Where, Return, Sort, Aggregate
//! - **Clinical Operators**: CalculateAge, InValueSet, InCodeSystem
//! - **Type Operators**: As, Is, Convert, ToXxx converters
//!
//! # Example
//!
//! ```ignore
//! use octofhir_cql_eval::{CqlEngine, EvaluationContext};
//! use octofhir_cql_elm::Library;
//!
//! let engine = CqlEngine::new();
//! let mut ctx = EvaluationContext::new();
//!
//! // Evaluate a library
//! let results = engine.evaluate_library(&library, &mut ctx).unwrap();
//! ```
//!
//! # Architecture
//!
//! The evaluation engine consists of:
//!
//! - `CqlEngine`: The main evaluation engine that dispatches to operator implementations
//! - `EvaluationContext`: Maintains state during evaluation (parameters, scopes, providers)
//! - `operators`: Module containing all operator implementations
//! - `query`: Query evaluation with support for complex CQL queries
//!
//! # Three-Valued Logic
//!
//! CQL implements three-valued logic where operations can return true, false, or null.
//! This is correctly handled throughout the engine:
//!
//! - `And`: false dominates (null and false = false)
//! - `Or`: true dominates (null or true = true)
//! - Comparisons return null when operands have insufficient precision

pub mod context;
pub mod engine;
pub mod error;
pub mod operators;
pub mod query;
pub mod registry;
pub mod retrieve;
pub mod terminology;
pub mod value;

// Re-export main types
pub use context::{DataProvider, EvaluationContext, EvaluationContextBuilder, Scope, TerminologyProvider};
pub use engine::CqlEngine;
pub use error::{EvalError, EvalResult};
pub use registry::{FunctionRegistry, OperatorRegistry};
pub use retrieve::{DataRetrieverAdapter, extract_codes};
pub use terminology::TerminologyAdapter;
pub use value::*;

// Re-export commonly used operator helpers
pub use operators::comparison::{cql_compare, cql_equal, cql_equivalent};
pub use operators::clinical::{code_in_codes, codes_equivalent, concept_in_codes};
