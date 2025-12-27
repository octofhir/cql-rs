//! Symbol Table for CQL Semantic Analysis
//!
//! This module implements the symbol table for tracking definitions
//! during CQL compilation.

use indexmap::IndexMap;
use std::sync::Arc;

use crate::CqlType;

/// A symbol in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Symbol type
    pub symbol_type: CqlType,
    /// Access level
    pub access: AccessLevel,
    /// Library where symbol is defined
    pub library: Option<String>,
    /// Documentation/description
    pub doc: Option<String>,
}

impl Symbol {
    /// Create a new symbol
    pub fn new(name: impl Into<String>, kind: SymbolKind, symbol_type: CqlType) -> Self {
        Self {
            name: name.into(),
            kind,
            symbol_type,
            access: AccessLevel::Public,
            library: None,
            doc: None,
        }
    }

    /// Set the access level
    pub fn with_access(mut self, access: AccessLevel) -> Self {
        self.access = access;
        self
    }

    /// Set the library
    pub fn with_library(mut self, library: impl Into<String>) -> Self {
        self.library = Some(library.into());
        self
    }

    /// Set the documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = Some(doc.into());
        self
    }
}

/// Kind of symbol
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    /// Parameter definition
    Parameter,
    /// Expression definition
    ExpressionDef,
    /// Function definition
    FunctionDef(FunctionSignature),
    /// Code system definition
    CodeSystem,
    /// Value set definition
    ValueSet,
    /// Code definition
    Code,
    /// Concept definition
    Concept,
    /// Context (e.g., Patient, Practitioner)
    Context,
    /// Alias (query source alias)
    Alias,
    /// Let binding in query
    Let,
    /// Using definition (model)
    Using,
    /// Include definition (library)
    Include,
    /// Local variable
    Variable,
    /// Aggregate ($total)
    Aggregate,
    /// Iteration variable ($this)
    Iteration,
    /// Index variable ($index)
    Index,
}

/// Function signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    /// Function parameters
    pub parameters: Vec<FunctionParameter>,
    /// Return type
    pub return_type: CqlType,
    /// Whether this is a fluent function
    pub fluent: bool,
    /// Whether this is an external function
    pub external: bool,
}

impl FunctionSignature {
    /// Create a new function signature
    pub fn new(parameters: Vec<FunctionParameter>, return_type: CqlType) -> Self {
        Self {
            parameters,
            return_type,
            fluent: false,
            external: false,
        }
    }

    /// Set fluent flag
    pub fn fluent(mut self) -> Self {
        self.fluent = true;
        self
    }

    /// Set external flag
    pub fn external(mut self) -> Self {
        self.external = true;
        self
    }

    /// Get parameter count
    pub fn arity(&self) -> usize {
        self.parameters.len()
    }
}

/// Function parameter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: CqlType,
}

impl FunctionParameter {
    /// Create a new function parameter
    pub fn new(name: impl Into<String>, param_type: CqlType) -> Self {
        Self {
            name: name.into(),
            param_type,
        }
    }
}

/// Access level for definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessLevel {
    /// Public access (default)
    #[default]
    Public,
    /// Private access
    Private,
}

/// Symbol table for managing definitions
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    /// Symbols indexed by name
    symbols: IndexMap<String, Vec<Symbol>>,
    /// Library aliases (local name -> library identifier)
    library_aliases: IndexMap<String, LibraryRef>,
    /// Model aliases (local name -> model info)
    model_aliases: IndexMap<String, ModelRef>,
    /// Current context (e.g., "Patient")
    current_context: Option<String>,
    /// Library identifier
    library_id: Option<LibraryIdentifier>,
}

impl SymbolTable {
    /// Create a new symbol table
    pub fn new() -> Self {
        Self {
            symbols: IndexMap::new(),
            library_aliases: IndexMap::new(),
            model_aliases: IndexMap::new(),
            current_context: None,
            library_id: None,
        }
    }

    /// Set the library identifier
    pub fn set_library(&mut self, id: LibraryIdentifier) {
        self.library_id = Some(id);
    }

    /// Get the library identifier
    pub fn library(&self) -> Option<&LibraryIdentifier> {
        self.library_id.as_ref()
    }

    /// Set the current context
    pub fn set_context(&mut self, context: impl Into<String>) {
        self.current_context = Some(context.into());
    }

    /// Get the current context
    pub fn context(&self) -> Option<&str> {
        self.current_context.as_deref()
    }

    /// Define a symbol
    pub fn define(&mut self, symbol: Symbol) {
        let name = symbol.name.clone();
        self.symbols.entry(name).or_default().push(symbol);
    }

    /// Look up a symbol by name
    ///
    /// For non-function symbols, returns the first match.
    /// For functions, all overloads are available via `lookup_all`.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name).and_then(|syms| syms.first())
    }

    /// Look up all symbols with a given name (for overloading)
    pub fn lookup_all(&self, name: &str) -> Option<&[Symbol]> {
        self.symbols.get(name).map(|v| v.as_slice())
    }

    /// Look up a qualified symbol (Library.Name)
    pub fn lookup_qualified(&self, library: &str, name: &str) -> Option<&Symbol> {
        // First check if we have an alias for this library
        if let Some(lib_ref) = self.library_aliases.get(library) {
            // Would need to resolve from the referenced library
            // For now, just check local symbols with matching library
            self.symbols.get(name).and_then(|syms| {
                syms.iter()
                    .find(|s| s.library.as_deref() == Some(&lib_ref.identifier.id))
            })
        } else {
            None
        }
    }

    /// Check if a symbol is defined
    pub fn is_defined(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Register a library alias (from include statement)
    pub fn add_library_alias(&mut self, alias: impl Into<String>, lib_ref: LibraryRef) {
        self.library_aliases.insert(alias.into(), lib_ref);
    }

    /// Get a library reference by alias
    pub fn get_library(&self, alias: &str) -> Option<&LibraryRef> {
        self.library_aliases.get(alias)
    }

    /// Register a model alias (from using statement)
    pub fn add_model_alias(&mut self, alias: impl Into<String>, model_ref: ModelRef) {
        self.model_aliases.insert(alias.into(), model_ref);
    }

    /// Get a model reference by alias
    pub fn get_model(&self, alias: &str) -> Option<&ModelRef> {
        self.model_aliases.get(alias)
    }

    /// Get all defined symbol names
    pub fn all_names(&self) -> impl Iterator<Item = &String> {
        self.symbols.keys()
    }

    /// Get all symbols
    pub fn all_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values().flatten()
    }

    /// Get symbols by kind
    pub fn symbols_of_kind(&self, kind: &SymbolKind) -> impl Iterator<Item = &Symbol> {
        self.all_symbols()
            .filter(move |s| std::mem::discriminant(&s.kind) == std::mem::discriminant(kind))
    }

    /// Get all function overloads for a name
    pub fn function_overloads(&self, name: &str) -> Vec<&FunctionSignature> {
        self.symbols
            .get(name)
            .map(|syms| {
                syms.iter()
                    .filter_map(|s| {
                        if let SymbolKind::FunctionDef(sig) = &s.kind {
                            Some(sig)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Remove a symbol
    pub fn remove(&mut self, name: &str) {
        self.symbols.shift_remove(name);
    }

    /// Clear all symbols
    pub fn clear(&mut self) {
        self.symbols.clear();
        self.library_aliases.clear();
        self.model_aliases.clear();
        self.current_context = None;
    }
}

/// Library identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryIdentifier {
    /// Library name/id
    pub id: String,
    /// System/namespace
    pub system: Option<String>,
    /// Version
    pub version: Option<String>,
}

impl LibraryIdentifier {
    /// Create a new library identifier
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            system: None,
            version: None,
        }
    }

    /// With version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// With system
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }
}

/// Reference to an included library
#[derive(Debug, Clone)]
pub struct LibraryRef {
    /// Library identifier
    pub identifier: LibraryIdentifier,
    /// Local alias
    pub local_alias: String,
    /// Resolved symbol table (populated during resolution)
    pub symbols: Option<Arc<SymbolTable>>,
}

impl LibraryRef {
    /// Create a new library reference
    pub fn new(identifier: LibraryIdentifier, local_alias: impl Into<String>) -> Self {
        Self {
            identifier,
            local_alias: local_alias.into(),
            symbols: None,
        }
    }
}

/// Reference to a using model
#[derive(Debug, Clone)]
pub struct ModelRef {
    /// Model name
    pub name: String,
    /// Model URI
    pub uri: Option<String>,
    /// Version
    pub version: Option<String>,
}

impl ModelRef {
    /// Create a new model reference
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            uri: None,
            version: None,
        }
    }

    /// With URI
    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// With version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();

        // Define a parameter
        table.define(Symbol::new("MeasurementPeriod", SymbolKind::Parameter, CqlType::Interval(Box::new(CqlType::DateTime))));

        // Define an expression
        table.define(Symbol::new("InitialPopulation", SymbolKind::ExpressionDef, CqlType::list(CqlType::Any)));

        // Lookup
        assert!(table.is_defined("MeasurementPeriod"));
        assert!(table.is_defined("InitialPopulation"));
        assert!(!table.is_defined("Unknown"));

        let param = table.lookup("MeasurementPeriod").unwrap();
        assert_eq!(param.kind, SymbolKind::Parameter);
    }

    #[test]
    fn test_function_overloads() {
        let mut table = SymbolTable::new();

        // Define two overloads of a function
        let sig1 = FunctionSignature::new(
            vec![FunctionParameter::new("x", CqlType::Integer)],
            CqlType::Integer,
        );
        let sig2 = FunctionSignature::new(
            vec![FunctionParameter::new("x", CqlType::Decimal)],
            CqlType::Decimal,
        );

        table.define(Symbol::new("Abs", SymbolKind::FunctionDef(sig1), CqlType::Integer));
        table.define(Symbol::new("Abs", SymbolKind::FunctionDef(sig2), CqlType::Decimal));

        // Get overloads
        let overloads = table.function_overloads("Abs");
        assert_eq!(overloads.len(), 2);
    }

    #[test]
    fn test_library_aliases() {
        let mut table = SymbolTable::new();

        let lib_ref = LibraryRef::new(
            LibraryIdentifier::new("MATGlobalCommonFunctions").with_version("1.0.0"),
            "Global",
        );

        table.add_library_alias("Global", lib_ref);

        assert!(table.get_library("Global").is_some());
        assert!(table.get_library("Unknown").is_none());
    }
}
