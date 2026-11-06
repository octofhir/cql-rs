#![doc = include_str!("../README.md")]

mod blocking;
pub mod context;
pub mod engine;
pub mod error;
pub mod fhir;
pub mod operators;
pub mod query;
pub mod references;
pub mod registry;
pub mod retrieve;
pub mod terminology;
pub mod trace;
pub mod value;

// Re-export main types
pub use context::{
    DataProvider, EvaluationContext, EvaluationContextBuilder, Scope, TerminologyProvider,
};
pub use engine::CqlEngine;
pub use error::{EvalError, EvalResult};
pub use references::TerminologyId;
pub use registry::{FunctionRegistry, OperatorRegistry};
pub use retrieve::{DataRetrieverAdapter, extract_codes};
pub use terminology::TerminologyAdapter;
pub use trace::{TraceEvent, TracePhase, TraceRecorder, TraceVariable};
pub use value::*;

// Re-export commonly used operator helpers
pub use operators::clinical::{code_in_codes, codes_equivalent, concept_in_codes};
pub use operators::comparison::{cql_compare, cql_equal, cql_equivalent};
