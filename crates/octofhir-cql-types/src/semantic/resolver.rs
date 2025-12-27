//! Reference Resolution for CQL Semantic Analysis
//!
//! This module handles resolution of references in CQL including:
//! - Expression references (ExpressionRef)
//! - Parameter references (ParameterRef)
//! - Function calls (FunctionRef)
//! - Code/ValueSet/CodeSystem references
//! - Library-qualified references

use thiserror::Error;

use crate::coercion::TypeCoercer;
use crate::CqlType;
use super::scope::ScopeManager;
use super::symbols::{FunctionParameter, FunctionSignature, Symbol, SymbolKind, SymbolTable};

/// Resolution errors
#[derive(Debug, Clone, Error)]
pub enum ResolutionError {
    /// Symbol not found
    #[error("Cannot resolve symbol '{name}'")]
    SymbolNotFound { name: String },

    /// Qualified symbol not found
    #[error("Cannot resolve symbol '{library}.{name}'")]
    QualifiedSymbolNotFound { library: String, name: String },

    /// Library not found
    #[error("Library '{name}' not found (did you forget to include it?)")]
    LibraryNotFound { name: String },

    /// Ambiguous reference
    #[error("Ambiguous reference to '{name}': multiple definitions found")]
    AmbiguousReference { name: String },

    /// No matching overload
    #[error("No matching overload for function '{name}' with arguments ({args})")]
    NoMatchingOverload { name: String, args: String },

    /// Ambiguous overload
    #[error("Ambiguous call to function '{name}': multiple overloads match")]
    AmbiguousOverload { name: String },

    /// Wrong argument count
    #[error("Function '{name}' expects {expected} arguments, but got {found}")]
    WrongArgumentCount {
        name: String,
        expected: usize,
        found: usize,
    },

    /// Private symbol access
    #[error("Cannot access private symbol '{name}' from outside its library")]
    PrivateAccess { name: String },

    /// Invalid reference kind
    #[error("'{name}' is a {actual}, expected {expected}")]
    InvalidReferenceKind {
        name: String,
        actual: String,
        expected: String,
    },
}

/// Resolution result
pub type ResolutionResult<T> = Result<T, ResolutionError>;

/// Resolved reference
#[derive(Debug, Clone)]
pub struct ResolvedRef {
    /// The resolved symbol
    pub symbol: Symbol,
    /// Library qualifier (if qualified reference)
    pub library: Option<String>,
    /// Reference kind
    pub kind: RefKind,
}

impl ResolvedRef {
    /// Create a new resolved reference
    pub fn new(symbol: Symbol, kind: RefKind) -> Self {
        Self {
            symbol,
            library: None,
            kind,
        }
    }

    /// With library qualifier
    pub fn with_library(mut self, library: impl Into<String>) -> Self {
        self.library = Some(library.into());
        self
    }
}

/// Kind of reference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    /// Expression reference
    Expression,
    /// Parameter reference
    Parameter,
    /// Function reference
    Function,
    /// Code system reference
    CodeSystem,
    /// Value set reference
    ValueSet,
    /// Code reference
    Code,
    /// Concept reference
    Concept,
    /// Alias reference (in query)
    Alias,
    /// Let reference (in query)
    Let,
    /// Variable reference
    Variable,
    /// Context reference
    Context,
}

/// Reference resolver
pub struct Resolver<'a> {
    /// Global symbol table
    symbol_table: &'a SymbolTable,
    /// Current scope manager
    scope_manager: &'a ScopeManager,
    /// Type coercer for overload resolution
    coercer: TypeCoercer,
}

impl<'a> Resolver<'a> {
    /// Create a new resolver
    pub fn new(symbol_table: &'a SymbolTable, scope_manager: &'a ScopeManager) -> Self {
        Self {
            symbol_table,
            scope_manager,
            coercer: TypeCoercer::new(),
        }
    }

    /// Resolve an identifier reference
    ///
    /// Resolution order:
    /// 1. Local scope (aliases, let bindings, variables)
    /// 2. Special variables ($this, $index, $total)
    /// 3. Parameters
    /// 4. Expression definitions
    pub fn resolve_identifier(&self, name: &str) -> ResolutionResult<ResolvedRef> {
        // Check local scope first
        if let Some(symbol) = self.scope_manager.lookup(name) {
            let kind = match &symbol.kind {
                SymbolKind::Alias => RefKind::Alias,
                SymbolKind::Let => RefKind::Let,
                SymbolKind::Variable => RefKind::Variable,
                SymbolKind::Iteration => RefKind::Variable,
                SymbolKind::Index => RefKind::Variable,
                SymbolKind::Aggregate => RefKind::Variable,
                SymbolKind::Parameter => RefKind::Parameter,
                SymbolKind::ExpressionDef => RefKind::Expression,
                SymbolKind::Context => RefKind::Context,
                _ => RefKind::Variable,
            };
            return Ok(ResolvedRef::new(symbol.clone(), kind));
        }

        // Check global symbol table
        if let Some(symbol) = self.symbol_table.lookup(name) {
            let kind = match &symbol.kind {
                SymbolKind::Parameter => RefKind::Parameter,
                SymbolKind::ExpressionDef => RefKind::Expression,
                SymbolKind::FunctionDef(_) => RefKind::Function,
                SymbolKind::CodeSystem => RefKind::CodeSystem,
                SymbolKind::ValueSet => RefKind::ValueSet,
                SymbolKind::Code => RefKind::Code,
                SymbolKind::Concept => RefKind::Concept,
                SymbolKind::Context => RefKind::Context,
                _ => RefKind::Expression,
            };
            return Ok(ResolvedRef::new(symbol.clone(), kind));
        }

        Err(ResolutionError::SymbolNotFound {
            name: name.to_string(),
        })
    }

    /// Resolve a qualified identifier (Library.Name)
    pub fn resolve_qualified(
        &self,
        library: &str,
        name: &str,
    ) -> ResolutionResult<ResolvedRef> {
        // Check if library is a known alias
        if self.symbol_table.get_library(library).is_none() {
            return Err(ResolutionError::LibraryNotFound {
                name: library.to_string(),
            });
        }

        // Try to resolve in the library
        if let Some(symbol) = self.symbol_table.lookup_qualified(library, name) {
            let kind = match &symbol.kind {
                SymbolKind::Parameter => RefKind::Parameter,
                SymbolKind::ExpressionDef => RefKind::Expression,
                SymbolKind::FunctionDef(_) => RefKind::Function,
                SymbolKind::CodeSystem => RefKind::CodeSystem,
                SymbolKind::ValueSet => RefKind::ValueSet,
                SymbolKind::Code => RefKind::Code,
                SymbolKind::Concept => RefKind::Concept,
                _ => RefKind::Expression,
            };
            return Ok(ResolvedRef::new(symbol.clone(), kind).with_library(library));
        }

        Err(ResolutionError::QualifiedSymbolNotFound {
            library: library.to_string(),
            name: name.to_string(),
        })
    }

    /// Resolve a function call with arguments
    ///
    /// Performs overload resolution based on argument types.
    pub fn resolve_function(
        &self,
        name: &str,
        arg_types: &[CqlType],
    ) -> ResolutionResult<ResolvedOverload> {
        let overloads = self.symbol_table.function_overloads(name);

        if overloads.is_empty() {
            // Check if it's a non-function symbol
            if let Some(symbol) = self.symbol_table.lookup(name) {
                return Err(ResolutionError::InvalidReferenceKind {
                    name: name.to_string(),
                    actual: format!("{:?}", symbol.kind),
                    expected: "function".to_string(),
                });
            }
            return Err(ResolutionError::SymbolNotFound {
                name: name.to_string(),
            });
        }

        // Find matching overloads
        let mut candidates: Vec<(&FunctionSignature, u32)> = Vec::new();

        for sig in &overloads {
            if let Some(cost) = self.compute_overload_cost(sig, arg_types) {
                candidates.push((sig, cost));
            }
        }

        if candidates.is_empty() {
            let args = arg_types
                .iter()
                .map(|t| t.qualified_name())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ResolutionError::NoMatchingOverload {
                name: name.to_string(),
                args,
            });
        }

        // Sort by cost (lower is better)
        candidates.sort_by_key(|(_, cost)| *cost);

        // Check for ambiguity
        if candidates.len() > 1 && candidates[0].1 == candidates[1].1 {
            return Err(ResolutionError::AmbiguousOverload {
                name: name.to_string(),
            });
        }

        let (best_sig, cost) = candidates.remove(0);
        Ok(ResolvedOverload {
            name: name.to_string(),
            signature: best_sig.clone(),
            conversion_cost: cost,
        })
    }

    /// Compute the cost of using an overload with given argument types
    ///
    /// Returns None if the overload doesn't match.
    fn compute_overload_cost(
        &self,
        sig: &FunctionSignature,
        arg_types: &[CqlType],
    ) -> Option<u32> {
        if sig.parameters.len() != arg_types.len() {
            return None;
        }

        let mut total_cost = 0u32;
        for (param, arg) in sig.parameters.iter().zip(arg_types.iter()) {
            if let Some(cost) = self.coercer.conversion_cost(arg, &param.param_type) {
                total_cost += cost;
            } else {
                return None;
            }
        }

        Some(total_cost)
    }

    /// Resolve a code system reference
    pub fn resolve_codesystem(&self, name: &str) -> ResolutionResult<ResolvedRef> {
        let resolved = self.resolve_identifier(name)?;
        if resolved.kind != RefKind::CodeSystem {
            return Err(ResolutionError::InvalidReferenceKind {
                name: name.to_string(),
                actual: format!("{:?}", resolved.kind),
                expected: "codesystem".to_string(),
            });
        }
        Ok(resolved)
    }

    /// Resolve a value set reference
    pub fn resolve_valueset(&self, name: &str) -> ResolutionResult<ResolvedRef> {
        let resolved = self.resolve_identifier(name)?;
        if resolved.kind != RefKind::ValueSet {
            return Err(ResolutionError::InvalidReferenceKind {
                name: name.to_string(),
                actual: format!("{:?}", resolved.kind),
                expected: "valueset".to_string(),
            });
        }
        Ok(resolved)
    }

    /// Resolve a code reference
    pub fn resolve_code(&self, name: &str) -> ResolutionResult<ResolvedRef> {
        let resolved = self.resolve_identifier(name)?;
        if resolved.kind != RefKind::Code {
            return Err(ResolutionError::InvalidReferenceKind {
                name: name.to_string(),
                actual: format!("{:?}", resolved.kind),
                expected: "code".to_string(),
            });
        }
        Ok(resolved)
    }

    /// Resolve a concept reference
    pub fn resolve_concept(&self, name: &str) -> ResolutionResult<ResolvedRef> {
        let resolved = self.resolve_identifier(name)?;
        if resolved.kind != RefKind::Concept {
            return Err(ResolutionError::InvalidReferenceKind {
                name: name.to_string(),
                actual: format!("{:?}", resolved.kind),
                expected: "concept".to_string(),
            });
        }
        Ok(resolved)
    }

    /// Resolve an expression reference
    pub fn resolve_expression(&self, name: &str) -> ResolutionResult<ResolvedRef> {
        let resolved = self.resolve_identifier(name)?;
        if resolved.kind != RefKind::Expression {
            return Err(ResolutionError::InvalidReferenceKind {
                name: name.to_string(),
                actual: format!("{:?}", resolved.kind),
                expected: "expression".to_string(),
            });
        }
        Ok(resolved)
    }

    /// Resolve a parameter reference
    pub fn resolve_parameter(&self, name: &str) -> ResolutionResult<ResolvedRef> {
        let resolved = self.resolve_identifier(name)?;
        if resolved.kind != RefKind::Parameter {
            return Err(ResolutionError::InvalidReferenceKind {
                name: name.to_string(),
                actual: format!("{:?}", resolved.kind),
                expected: "parameter".to_string(),
            });
        }
        Ok(resolved)
    }
}

/// Result of function overload resolution
#[derive(Debug, Clone)]
pub struct ResolvedOverload {
    /// Function name
    pub name: String,
    /// Resolved function signature
    pub signature: FunctionSignature,
    /// Conversion cost (for ranking)
    pub conversion_cost: u32,
}

impl ResolvedOverload {
    /// Get the return type
    pub fn return_type(&self) -> &CqlType {
        &self.signature.return_type
    }

    /// Get the parameter count
    pub fn arity(&self) -> usize {
        self.signature.arity()
    }

    /// Get the parameters
    pub fn parameters(&self) -> &[FunctionParameter] {
        &self.signature.parameters
    }
}

/// Overload resolver for system functions
pub struct OverloadResolver {
    /// Type coercer
    coercer: TypeCoercer,
    /// Registered overloads (function name -> signatures)
    overloads: indexmap::IndexMap<String, Vec<FunctionSignature>>,
}

impl OverloadResolver {
    /// Create a new overload resolver
    pub fn new() -> Self {
        Self {
            coercer: TypeCoercer::new(),
            overloads: indexmap::IndexMap::new(),
        }
    }

    /// Create with standard CQL operators registered
    pub fn with_standard_operators() -> Self {
        let mut resolver = Self::new();
        resolver.register_standard_operators();
        resolver
    }

    /// Register a function overload
    pub fn register(&mut self, name: impl Into<String>, signature: FunctionSignature) {
        self.overloads
            .entry(name.into())
            .or_default()
            .push(signature);
    }

    /// Register standard CQL operators
    fn register_standard_operators(&mut self) {
        // Add - unary negate
        self.register(
            "Negate",
            FunctionSignature::new(
                vec![FunctionParameter::new("operand", CqlType::Integer)],
                CqlType::Integer,
            ),
        );
        self.register(
            "Negate",
            FunctionSignature::new(
                vec![FunctionParameter::new("operand", CqlType::Long)],
                CqlType::Long,
            ),
        );
        self.register(
            "Negate",
            FunctionSignature::new(
                vec![FunctionParameter::new("operand", CqlType::Decimal)],
                CqlType::Decimal,
            ),
        );
        self.register(
            "Negate",
            FunctionSignature::new(
                vec![FunctionParameter::new("operand", CqlType::Quantity)],
                CqlType::Quantity,
            ),
        );

        // Add - binary add
        self.register(
            "Add",
            FunctionSignature::new(
                vec![
                    FunctionParameter::new("left", CqlType::Integer),
                    FunctionParameter::new("right", CqlType::Integer),
                ],
                CqlType::Integer,
            ),
        );
        self.register(
            "Add",
            FunctionSignature::new(
                vec![
                    FunctionParameter::new("left", CqlType::Long),
                    FunctionParameter::new("right", CqlType::Long),
                ],
                CqlType::Long,
            ),
        );
        self.register(
            "Add",
            FunctionSignature::new(
                vec![
                    FunctionParameter::new("left", CqlType::Decimal),
                    FunctionParameter::new("right", CqlType::Decimal),
                ],
                CqlType::Decimal,
            ),
        );
        self.register(
            "Add",
            FunctionSignature::new(
                vec![
                    FunctionParameter::new("left", CqlType::Quantity),
                    FunctionParameter::new("right", CqlType::Quantity),
                ],
                CqlType::Quantity,
            ),
        );

        // String concatenation
        self.register(
            "Concatenate",
            FunctionSignature::new(
                vec![
                    FunctionParameter::new("left", CqlType::String),
                    FunctionParameter::new("right", CqlType::String),
                ],
                CqlType::String,
            ),
        );

        // Comparison
        for op in ["Equal", "NotEqual", "Less", "LessOrEqual", "Greater", "GreaterOrEqual"] {
            for ty in [
                CqlType::Integer,
                CqlType::Long,
                CqlType::Decimal,
                CqlType::String,
                CqlType::Date,
                CqlType::DateTime,
                CqlType::Time,
            ] {
                self.register(
                    op,
                    FunctionSignature::new(
                        vec![
                            FunctionParameter::new("left", ty.clone()),
                            FunctionParameter::new("right", ty),
                        ],
                        CqlType::Boolean,
                    ),
                );
            }
        }

        // Boolean operations
        for op in ["And", "Or", "Xor", "Implies"] {
            self.register(
                op,
                FunctionSignature::new(
                    vec![
                        FunctionParameter::new("left", CqlType::Boolean),
                        FunctionParameter::new("right", CqlType::Boolean),
                    ],
                    CqlType::Boolean,
                ),
            );
        }

        self.register(
            "Not",
            FunctionSignature::new(
                vec![FunctionParameter::new("operand", CqlType::Boolean)],
                CqlType::Boolean,
            ),
        );
    }

    /// Resolve an overload
    pub fn resolve(&self, name: &str, arg_types: &[CqlType]) -> ResolutionResult<ResolvedOverload> {
        let overloads = self.overloads.get(name).ok_or_else(|| {
            ResolutionError::SymbolNotFound {
                name: name.to_string(),
            }
        })?;

        let mut candidates: Vec<(&FunctionSignature, u32)> = Vec::new();

        for sig in overloads {
            if sig.parameters.len() != arg_types.len() {
                continue;
            }

            let mut total_cost = 0u32;
            let mut matches = true;

            for (param, arg) in sig.parameters.iter().zip(arg_types.iter()) {
                if let Some(cost) = self.coercer.conversion_cost(arg, &param.param_type) {
                    total_cost += cost;
                } else {
                    matches = false;
                    break;
                }
            }

            if matches {
                candidates.push((sig, total_cost));
            }
        }

        if candidates.is_empty() {
            let args = arg_types
                .iter()
                .map(|t| t.qualified_name())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ResolutionError::NoMatchingOverload {
                name: name.to_string(),
                args,
            });
        }

        candidates.sort_by_key(|(_, cost)| *cost);

        if candidates.len() > 1 && candidates[0].1 == candidates[1].1 {
            return Err(ResolutionError::AmbiguousOverload {
                name: name.to_string(),
            });
        }

        let (best_sig, cost) = candidates.remove(0);
        Ok(ResolvedOverload {
            name: name.to_string(),
            signature: best_sig.clone(),
            conversion_cost: cost,
        })
    }
}

impl Default for OverloadResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_symbol_table() -> SymbolTable {
        let mut table = SymbolTable::new();

        // Add a parameter
        table.define(Symbol::new(
            "MeasurementPeriod",
            SymbolKind::Parameter,
            CqlType::interval(CqlType::DateTime),
        ));

        // Add an expression
        table.define(Symbol::new(
            "InitialPopulation",
            SymbolKind::ExpressionDef,
            CqlType::list(CqlType::Any),
        ));

        // Add function overloads
        let sig_int = FunctionSignature::new(
            vec![FunctionParameter::new("x", CqlType::Integer)],
            CqlType::Integer,
        );
        let sig_dec = FunctionSignature::new(
            vec![FunctionParameter::new("x", CqlType::Decimal)],
            CqlType::Decimal,
        );

        table.define(Symbol::new("Abs", SymbolKind::FunctionDef(sig_int), CqlType::Integer));
        table.define(Symbol::new("Abs", SymbolKind::FunctionDef(sig_dec), CqlType::Decimal));

        table
    }

    #[test]
    fn test_resolve_identifier() {
        let table = setup_symbol_table();
        let scope = ScopeManager::new();
        let resolver = Resolver::new(&table, &scope);

        let result = resolver.resolve_identifier("MeasurementPeriod");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind, RefKind::Parameter);

        let result = resolver.resolve_identifier("InitialPopulation");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind, RefKind::Expression);

        let result = resolver.resolve_identifier("Unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_function() {
        let table = setup_symbol_table();
        let scope = ScopeManager::new();
        let resolver = Resolver::new(&table, &scope);

        // Should resolve to Integer overload
        let result = resolver.resolve_function("Abs", &[CqlType::Integer]);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.return_type(), &CqlType::Integer);

        // Should resolve to Decimal overload with promotion
        let result = resolver.resolve_function("Abs", &[CqlType::Long]);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.return_type(), &CqlType::Decimal);
    }

    #[test]
    fn test_scope_resolution() {
        let table = SymbolTable::new();
        let mut scope = ScopeManager::new();
        scope.define_alias("P", CqlType::Any);

        let resolver = Resolver::new(&table, &scope);

        let result = resolver.resolve_identifier("P");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind, RefKind::Alias);
    }

    #[test]
    fn test_overload_resolver() {
        let resolver = OverloadResolver::with_standard_operators();

        // Integer + Integer -> Integer
        let result = resolver.resolve("Add", &[CqlType::Integer, CqlType::Integer]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().return_type(), &CqlType::Integer);

        // Integer + Long -> Long (with promotion)
        let result = resolver.resolve("Add", &[CqlType::Integer, CqlType::Long]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().return_type(), &CqlType::Long);
    }
}
