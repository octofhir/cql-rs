#![doc = include_str!("../README.md")]

pub mod coercion;
pub mod inference;
pub mod semantic;
pub mod system_types;
pub mod type_system;
pub mod value;

// Re-export main types
pub use coercion::{CoercionError, TypeCoercer};
pub use inference::{TypeEnvironment, TypeInferenceError, TypeInferrer};
pub use semantic::{
    AccessLevel, FunctionParameter, FunctionSignature, LibraryIdentifier, LibraryRef, ModelRef,
    OverloadResolver, RefKind, ResolutionError, ResolvedOverload, ResolvedRef, Resolver, Scope,
    ScopeKind, ScopeManager, Symbol, SymbolKind, SymbolTable,
};
pub use system_types::SystemType;
pub use type_system::{
    ChoiceTypeSpecifier, CqlType, IntervalTypeSpecifier, ListTypeSpecifier, NamedTypeSpecifier,
    TupleElementDefinition, TupleTypeElement, TupleTypeSpecifier, TypeSpecifier,
};
pub use value::{
    CqlCode, CqlConcept, CqlDate, CqlDateTime, CqlInterval, CqlList, CqlQuantity, CqlRatio,
    CqlTime, CqlTuple, CqlValue, DateTimePrecision,
};
