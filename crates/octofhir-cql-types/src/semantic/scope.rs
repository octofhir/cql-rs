//! Scope Management for CQL Semantic Analysis
//!
//! This module handles scope management for CQL, particularly important
//! for query expressions which introduce their own scopes with aliases,
//! let bindings, and iteration variables.

use indexmap::IndexMap;
use std::fmt;

use crate::CqlType;
use super::symbols::{Symbol, SymbolKind};

/// A scope in the CQL semantic analysis
#[derive(Debug, Clone)]
pub struct Scope {
    /// Scope kind
    kind: ScopeKind,
    /// Symbols defined in this scope
    symbols: IndexMap<String, Symbol>,
    /// Parent scope (for nested scopes)
    parent: Option<Box<Scope>>,
    /// Scope depth (0 = top level)
    depth: usize,
}

impl Scope {
    /// Create a new top-level scope
    pub fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            symbols: IndexMap::new(),
            parent: None,
            depth: 0,
        }
    }

    /// Create a child scope
    pub fn child(&self, kind: ScopeKind) -> Self {
        Self {
            kind,
            symbols: IndexMap::new(),
            parent: Some(Box::new(self.clone())),
            depth: self.depth + 1,
        }
    }

    /// Get the scope kind
    pub fn kind(&self) -> &ScopeKind {
        &self.kind
    }

    /// Get the scope depth
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Define a symbol in this scope
    pub fn define(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    /// Define a simple variable
    pub fn define_variable(&mut self, name: impl Into<String>, var_type: CqlType) {
        let name = name.into();
        self.symbols.insert(
            name.clone(),
            Symbol::new(name, SymbolKind::Variable, var_type),
        );
    }

    /// Define an alias (query source alias)
    pub fn define_alias(&mut self, name: impl Into<String>, alias_type: CqlType) {
        let name = name.into();
        self.symbols.insert(
            name.clone(),
            Symbol::new(name, SymbolKind::Alias, alias_type),
        );
    }

    /// Define a let binding
    pub fn define_let(&mut self, name: impl Into<String>, let_type: CqlType) {
        let name = name.into();
        self.symbols.insert(
            name.clone(),
            Symbol::new(name, SymbolKind::Let, let_type),
        );
    }

    /// Look up a symbol in this scope or parent scopes
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    /// Look up a symbol only in this scope (not parents)
    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Check if a symbol is defined in this scope or parents
    pub fn is_defined(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Check if a symbol is defined locally (not in parents)
    pub fn is_defined_local(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Get all locally defined symbols
    pub fn local_symbols(&self) -> impl Iterator<Item = (&String, &Symbol)> {
        self.symbols.iter()
    }

    /// Get the parent scope
    pub fn parent(&self) -> Option<&Scope> {
        self.parent.as_deref()
    }

    /// Check if this is a query scope
    pub fn is_query_scope(&self) -> bool {
        matches!(self.kind, ScopeKind::Query)
    }

    /// Check if this scope is inside a query
    pub fn in_query(&self) -> bool {
        if self.is_query_scope() {
            return true;
        }
        self.parent.as_ref().is_some_and(|p| p.in_query())
    }

    /// Find the nearest query scope
    pub fn nearest_query_scope(&self) -> Option<&Scope> {
        if self.is_query_scope() {
            Some(self)
        } else {
            self.parent.as_ref().and_then(|p| p.nearest_query_scope())
        }
    }

    /// Get the $this variable type if in an iteration context
    pub fn iteration_type(&self) -> Option<&CqlType> {
        self.symbols
            .get("$this")
            .map(|s| &s.symbol_type)
            .or_else(|| self.parent.as_ref().and_then(|p| p.iteration_type()))
    }

    /// Get the $total variable type if in an aggregate context
    pub fn aggregate_type(&self) -> Option<&CqlType> {
        self.symbols
            .get("$total")
            .map(|s| &s.symbol_type)
            .or_else(|| self.parent.as_ref().and_then(|p| p.aggregate_type()))
    }

    /// Get the $index variable if in an iteration context
    pub fn has_index(&self) -> bool {
        self.symbols.contains_key("$index")
            || self.parent.as_ref().is_some_and(|p| p.has_index())
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new(ScopeKind::Global)
    }
}

/// Kind of scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeKind {
    /// Global/library scope
    Global,
    /// Function body scope
    Function,
    /// Query scope (introduces aliases, let bindings)
    Query,
    /// With clause scope
    With,
    /// Aggregate scope (introduces $total)
    Aggregate,
    /// Sort clause scope
    Sort,
    /// Let binding scope
    Let,
    /// Iteration scope (foreach, filter, etc.)
    Iteration,
}

impl fmt::Display for ScopeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Function => write!(f, "function"),
            Self::Query => write!(f, "query"),
            Self::With => write!(f, "with"),
            Self::Aggregate => write!(f, "aggregate"),
            Self::Sort => write!(f, "sort"),
            Self::Let => write!(f, "let"),
            Self::Iteration => write!(f, "iteration"),
        }
    }
}

/// Scope manager for tracking scope stack during analysis
#[derive(Debug, Clone)]
pub struct ScopeManager {
    /// Current scope
    current: Scope,
    /// Scope stack for managing nested scopes
    stack: Vec<Scope>,
}

impl ScopeManager {
    /// Create a new scope manager with global scope
    pub fn new() -> Self {
        Self {
            current: Scope::new(ScopeKind::Global),
            stack: Vec::new(),
        }
    }

    /// Get the current scope
    pub fn current(&self) -> &Scope {
        &self.current
    }

    /// Get a mutable reference to the current scope
    pub fn current_mut(&mut self) -> &mut Scope {
        &mut self.current
    }

    /// Enter a new scope
    pub fn enter(&mut self, kind: ScopeKind) {
        let new_scope = self.current.child(kind);
        let old_scope = std::mem::replace(&mut self.current, new_scope);
        self.stack.push(old_scope);
    }

    /// Leave the current scope, returning to parent
    pub fn leave(&mut self) -> Option<Scope> {
        if let Some(parent) = self.stack.pop() {
            let old = std::mem::replace(&mut self.current, parent);
            Some(old)
        } else {
            None
        }
    }

    /// Define a symbol in the current scope
    pub fn define(&mut self, symbol: Symbol) {
        self.current.define(symbol);
    }

    /// Define a variable in the current scope
    pub fn define_variable(&mut self, name: impl Into<String>, var_type: CqlType) {
        self.current.define_variable(name, var_type);
    }

    /// Define an alias in the current scope
    pub fn define_alias(&mut self, name: impl Into<String>, alias_type: CqlType) {
        self.current.define_alias(name, alias_type);
    }

    /// Define a let binding in the current scope
    pub fn define_let(&mut self, name: impl Into<String>, let_type: CqlType) {
        self.current.define_let(name, let_type);
    }

    /// Look up a symbol
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.current.lookup(name)
    }

    /// Check if a symbol is defined
    pub fn is_defined(&self, name: &str) -> bool {
        self.current.is_defined(name)
    }

    /// Get the current scope depth
    pub fn depth(&self) -> usize {
        self.current.depth()
    }

    /// Check if currently in a query scope
    pub fn in_query(&self) -> bool {
        self.current.in_query()
    }

    /// Enter a query scope with source alias
    pub fn enter_query(&mut self, alias: impl Into<String>, source_type: CqlType) {
        self.enter(ScopeKind::Query);
        self.current.define_alias(alias, source_type);
    }

    /// Enter an iteration scope with $this binding
    pub fn enter_iteration(&mut self, element_type: CqlType) {
        self.enter(ScopeKind::Iteration);
        self.current.define(Symbol::new(
            "$this",
            SymbolKind::Iteration,
            element_type.clone(),
        ));
        self.current.define(Symbol::new(
            "$index",
            SymbolKind::Index,
            CqlType::Integer,
        ));
    }

    /// Enter an aggregate scope with $total binding
    pub fn enter_aggregate(&mut self, accumulator_type: CqlType, element_type: CqlType) {
        self.enter(ScopeKind::Aggregate);
        self.current.define(Symbol::new(
            "$total",
            SymbolKind::Aggregate,
            accumulator_type,
        ));
        self.current.define(Symbol::new(
            "$this",
            SymbolKind::Iteration,
            element_type,
        ));
    }

    /// Execute a closure with a temporary scope
    pub fn with_scope<F, R>(&mut self, kind: ScopeKind, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.enter(kind);
        let result = f(self);
        self.leave();
        result
    }
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Query source information
#[derive(Debug, Clone)]
pub struct QuerySource {
    /// Source alias
    pub alias: String,
    /// Source type (the list element type)
    pub element_type: CqlType,
    /// Full source expression type
    pub source_type: CqlType,
}

impl QuerySource {
    /// Create a new query source
    pub fn new(alias: impl Into<String>, element_type: CqlType, source_type: CqlType) -> Self {
        Self {
            alias: alias.into(),
            element_type,
            source_type,
        }
    }
}

/// Query scope builder for constructing complex query scopes
#[derive(Debug, Clone)]
pub struct QueryScopeBuilder {
    /// Sources in the query
    sources: Vec<QuerySource>,
    /// Let definitions
    lets: Vec<(String, CqlType)>,
    /// With relationships
    withs: Vec<(String, CqlType)>,
}

impl QueryScopeBuilder {
    /// Create a new query scope builder
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            lets: Vec::new(),
            withs: Vec::new(),
        }
    }

    /// Add a source
    pub fn add_source(&mut self, source: QuerySource) -> &mut Self {
        self.sources.push(source);
        self
    }

    /// Add a let binding
    pub fn add_let(&mut self, name: impl Into<String>, let_type: CqlType) -> &mut Self {
        self.lets.push((name.into(), let_type));
        self
    }

    /// Add a with relationship
    pub fn add_with(&mut self, alias: impl Into<String>, with_type: CqlType) -> &mut Self {
        self.withs.push((alias.into(), with_type));
        self
    }

    /// Build the query scope
    pub fn build(&self, parent: &Scope) -> Scope {
        let mut scope = parent.child(ScopeKind::Query);

        // Add sources as aliases
        for source in &self.sources {
            scope.define_alias(source.alias.clone(), source.element_type.clone());
        }

        // Add let bindings
        for (name, let_type) in &self.lets {
            scope.define_let(name.clone(), let_type.clone());
        }

        // Add with relationships as aliases
        for (alias, with_type) in &self.withs {
            scope.define_alias(alias.clone(), with_type.clone());
        }

        scope
    }

    /// Apply to a scope manager
    pub fn apply(&self, manager: &mut ScopeManager) {
        manager.enter(ScopeKind::Query);

        for source in &self.sources {
            manager.define_alias(source.alias.clone(), source.element_type.clone());
        }

        for (name, let_type) in &self.lets {
            manager.define_let(name.clone(), let_type.clone());
        }

        for (alias, with_type) in &self.withs {
            manager.define_alias(alias.clone(), with_type.clone());
        }
    }
}

impl Default for QueryScopeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_basic() {
        let mut scope = Scope::new(ScopeKind::Global);
        scope.define_variable("x", CqlType::Integer);

        assert!(scope.is_defined("x"));
        assert!(!scope.is_defined("y"));
    }

    #[test]
    fn test_scope_nesting() {
        let mut global = Scope::new(ScopeKind::Global);
        global.define_variable("x", CqlType::Integer);

        let mut query = global.child(ScopeKind::Query);
        query.define_alias("P", CqlType::Any);

        // Can see both local and parent
        assert!(query.is_defined("x"));
        assert!(query.is_defined("P"));
        assert!(query.is_defined_local("P"));
        assert!(!query.is_defined_local("x"));

        // Parent doesn't see child
        assert!(!global.is_defined("P"));
    }

    #[test]
    fn test_scope_manager() {
        let mut manager = ScopeManager::new();

        manager.define_variable("x", CqlType::Integer);
        manager.enter(ScopeKind::Query);
        manager.define_alias("P", CqlType::Any);

        assert!(manager.is_defined("x"));
        assert!(manager.is_defined("P"));
        assert!(manager.in_query());

        manager.leave();

        assert!(manager.is_defined("x"));
        assert!(!manager.is_defined("P"));
        assert!(!manager.in_query());
    }

    #[test]
    fn test_iteration_scope() {
        let mut manager = ScopeManager::new();

        manager.enter_iteration(CqlType::Integer);

        assert!(manager.is_defined("$this"));
        assert!(manager.is_defined("$index"));

        let this_type = manager.current().iteration_type();
        assert_eq!(this_type, Some(&CqlType::Integer));

        manager.leave();
        assert!(!manager.is_defined("$this"));
    }

    #[test]
    fn test_query_scope_builder() {
        let mut builder = QueryScopeBuilder::new();
        builder
            .add_source(QuerySource::new(
                "E",
                CqlType::Any,
                CqlType::list(CqlType::Any),
            ))
            .add_let("startDate", CqlType::Date);

        let mut manager = ScopeManager::new();
        builder.apply(&mut manager);

        assert!(manager.is_defined("E"));
        assert!(manager.is_defined("startDate"));
        assert!(manager.in_query());
    }
}
