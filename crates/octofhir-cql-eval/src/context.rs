//! Evaluation context for CQL execution
//!
//! This module provides the EvaluationContext which maintains all state
//! needed during CQL expression evaluation including parameters, libraries,
//! temporal context, and scope management.

use chrono::{DateTime, Datelike, FixedOffset, Local, Timelike};
use indexmap::IndexMap;
use octofhir_cql_elm::Library;
use octofhir_cql_types::{CqlDate, CqlDateTime, CqlTime, CqlValue};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Evaluation context for CQL expression execution
///
/// The context maintains all state needed during evaluation including:
/// - The current evaluation context (e.g., Patient)
/// - Parameter values
/// - Library definitions
/// - Temporal values (Now, Today)
/// - Variable scopes (aliases, let bindings)
/// - Cached expression results
pub struct EvaluationContext {
    /// Current context type (e.g., "Patient")
    pub context_type: Option<String>,
    /// Current context value (e.g., Patient resource)
    pub context_value: Option<CqlValue>,
    /// Parameter values by name
    parameters: HashMap<String, CqlValue>,
    /// Loaded libraries by name
    libraries: HashMap<String, Arc<Library>>,
    /// The main library being evaluated
    main_library: Option<Arc<Library>>,
    /// Evaluation timestamp (used for Now())
    evaluation_timestamp: DateTime<FixedOffset>,
    /// Scope stack for query aliases and let bindings
    scope_stack: Vec<Scope>,
    /// Expression cache for memoization
    expression_cache: RwLock<HashMap<String, CqlValue>>,
    /// Maximum recursion depth
    max_recursion_depth: usize,
    /// Current recursion depth
    current_recursion_depth: usize,
    /// Terminology service provider
    terminology_provider: Option<Arc<dyn TerminologyProvider>>,
    /// Data retrieval provider
    data_provider: Option<Arc<dyn DataProvider>>,
}

/// A scope for variable bindings during query evaluation
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Query aliases ($this, etc.)
    aliases: IndexMap<String, CqlValue>,
    /// Let clause bindings
    let_bindings: IndexMap<String, CqlValue>,
    /// Special variables ($index, $total)
    special_vars: IndexMap<String, CqlValue>,
}

impl Scope {
    /// Create a new empty scope
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an alias value
    pub fn set_alias(&mut self, name: impl Into<String>, value: CqlValue) {
        self.aliases.insert(name.into(), value);
    }

    /// Get an alias value
    pub fn get_alias(&self, name: &str) -> Option<&CqlValue> {
        self.aliases.get(name)
    }

    /// Set a let binding
    pub fn set_let(&mut self, name: impl Into<String>, value: CqlValue) {
        self.let_bindings.insert(name.into(), value);
    }

    /// Get a let binding
    pub fn get_let(&self, name: &str) -> Option<&CqlValue> {
        self.let_bindings.get(name)
    }

    /// Set a special variable ($this, $index, $total)
    pub fn set_special(&mut self, name: impl Into<String>, value: CqlValue) {
        self.special_vars.insert(name.into(), value);
    }

    /// Get a special variable
    pub fn get_special(&self, name: &str) -> Option<&CqlValue> {
        self.special_vars.get(name)
    }
}

/// Terminology service provider trait
///
/// Implementations provide terminology operations like InValueSet, InCodeSystem
pub trait TerminologyProvider: Send + Sync {
    /// Check if a code is in a value set
    fn in_value_set(&self, code: &CqlValue, value_set_id: &str) -> Option<bool>;

    /// Check if a code is in a code system
    fn in_code_system(&self, code: &CqlValue, code_system_id: &str) -> Option<bool>;

    /// Expand a value set to its codes
    fn expand_value_set(&self, value_set_id: &str) -> Option<Vec<CqlValue>>;

    /// Lookup a code's display name
    fn lookup_display(&self, code: &CqlValue) -> Option<String>;
}

/// Data provider trait for data retrieval
///
/// Implementations provide access to clinical data (FHIR resources, etc.)
pub trait DataProvider: Send + Sync {
    /// Retrieve data of a given type
    fn retrieve(
        &self,
        data_type: &str,
        context_type: Option<&str>,
        context_value: Option<&CqlValue>,
        template_id: Option<&str>,
        code_property: Option<&str>,
        codes: Option<&CqlValue>,
        date_property: Option<&str>,
        date_range: Option<&CqlValue>,
    ) -> Vec<CqlValue>;

    /// Get a property value from a resource
    fn get_property(&self, resource: &CqlValue, path: &str) -> Option<CqlValue>;
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EvaluationContext {
    /// Create a new empty context with current timestamp
    pub fn new() -> Self {
        Self {
            context_type: None,
            context_value: None,
            parameters: HashMap::new(),
            libraries: HashMap::new(),
            main_library: None,
            evaluation_timestamp: Local::now().fixed_offset(),
            scope_stack: Vec::new(),
            expression_cache: RwLock::new(HashMap::new()),
            max_recursion_depth: 100,
            current_recursion_depth: 0,
            terminology_provider: None,
            data_provider: None,
        }
    }

    /// Create a context with a specific evaluation timestamp
    pub fn with_timestamp(timestamp: DateTime<FixedOffset>) -> Self {
        Self {
            evaluation_timestamp: timestamp,
            ..Self::new()
        }
    }

    /// Set the context type and value
    pub fn with_context(mut self, context_type: impl Into<String>, value: CqlValue) -> Self {
        self.context_type = Some(context_type.into());
        self.context_value = Some(value);
        self
    }

    /// Set the main library
    pub fn with_library(mut self, library: Library) -> Self {
        self.main_library = Some(Arc::new(library));
        self
    }

    /// Set the terminology provider
    pub fn with_terminology_provider(
        mut self,
        provider: Arc<dyn TerminologyProvider>,
    ) -> Self {
        self.terminology_provider = Some(provider);
        self
    }

    /// Set the data provider
    pub fn with_data_provider(mut self, provider: Arc<dyn DataProvider>) -> Self {
        self.data_provider = Some(provider);
        self
    }

    /// Set maximum recursion depth
    pub fn with_max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = depth;
        self
    }

    // === Parameter Management ===

    /// Set a parameter value
    pub fn set_parameter(&mut self, name: impl Into<String>, value: CqlValue) {
        self.parameters.insert(name.into(), value);
    }

    /// Get a parameter value
    pub fn get_parameter(&self, name: &str) -> Option<&CqlValue> {
        self.parameters.get(name)
    }

    /// Get a parameter value with library qualifier
    pub fn get_parameter_qualified(
        &self,
        library_name: Option<&str>,
        name: &str,
    ) -> Option<&CqlValue> {
        // TODO: Handle library-qualified parameter lookup
        let _ = library_name;
        self.parameters.get(name)
    }

    // === Library Management ===

    /// Add a library
    pub fn add_library(&mut self, name: impl Into<String>, library: Library) {
        self.libraries.insert(name.into(), Arc::new(library));
    }

    /// Get the main library
    pub fn main_library(&self) -> Option<&Library> {
        self.main_library.as_deref()
    }

    /// Get the main library as a cloned Arc (useful for avoiding borrow conflicts)
    pub fn main_library_arc(&self) -> Option<Arc<Library>> {
        self.main_library.clone()
    }

    /// Get a library by name
    pub fn get_library(&self, name: &str) -> Option<&Library> {
        self.libraries.get(name).map(|l| l.as_ref())
    }

    // === Temporal Context ===

    /// Get the evaluation timestamp as CQL DateTime
    pub fn now(&self) -> CqlDateTime {
        let dt = self.evaluation_timestamp;
        CqlDateTime::new(
            dt.year(),
            dt.month() as u8,
            dt.day() as u8,
            dt.hour() as u8,
            dt.minute() as u8,
            dt.second() as u8,
            (dt.nanosecond() / 1_000_000) as u16,
            Some((dt.offset().local_minus_utc() / 60) as i16),
        )
    }

    /// Get today's date as CQL Date
    pub fn today(&self) -> CqlDate {
        let dt = self.evaluation_timestamp;
        CqlDate::new(dt.year(), dt.month() as u8, dt.day() as u8)
    }

    /// Get current time as CQL Time
    pub fn time_of_day(&self) -> CqlTime {
        let dt = self.evaluation_timestamp;
        CqlTime::new(
            dt.hour() as u8,
            dt.minute() as u8,
            dt.second() as u8,
            (dt.nanosecond() / 1_000_000) as u16,
        )
    }

    /// Get the timezone offset in minutes
    pub fn timezone_offset(&self) -> i16 {
        (self.evaluation_timestamp.offset().local_minus_utc() / 60) as i16
    }

    // === Scope Management ===

    /// Push a new scope
    pub fn push_scope(&mut self) {
        self.scope_stack.push(Scope::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Get the current scope (mutable)
    pub fn current_scope_mut(&mut self) -> Option<&mut Scope> {
        self.scope_stack.last_mut()
    }

    /// Get the current scope
    pub fn current_scope(&self) -> Option<&Scope> {
        self.scope_stack.last()
    }

    /// Set an alias in the current scope
    pub fn set_alias(&mut self, name: impl Into<String>, value: CqlValue) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.set_alias(name, value);
        }
    }

    /// Get an alias from any scope (searches from innermost to outermost)
    pub fn get_alias(&self, name: &str) -> Option<&CqlValue> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(value) = scope.get_alias(name) {
                return Some(value);
            }
        }
        None
    }

    /// Set a let binding in the current scope
    pub fn set_let(&mut self, name: impl Into<String>, value: CqlValue) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.set_let(name, value);
        }
    }

    /// Get a let binding from any scope
    pub fn get_let(&self, name: &str) -> Option<&CqlValue> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(value) = scope.get_let(name) {
                return Some(value);
            }
        }
        None
    }

    /// Set a special variable ($this, $index, $total)
    pub fn set_special(&mut self, name: impl Into<String>, value: CqlValue) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.set_special(name, value);
        }
    }

    /// Get a special variable
    pub fn get_special(&self, name: &str) -> Option<&CqlValue> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(value) = scope.get_special(name) {
                return Some(value);
            }
        }
        None
    }

    // === Expression Cache ===

    /// Get a cached expression result
    pub fn get_cached(&self, key: &str) -> Option<CqlValue> {
        self.expression_cache.read().get(key).cloned()
    }

    /// Cache an expression result
    pub fn cache_result(&self, key: impl Into<String>, value: CqlValue) {
        self.expression_cache.write().insert(key.into(), value);
    }

    /// Clear the expression cache
    pub fn clear_cache(&self) {
        self.expression_cache.write().clear();
    }

    // === Recursion Management ===

    /// Enter a recursive call, returns false if limit exceeded
    pub fn enter_recursion(&mut self) -> bool {
        if self.current_recursion_depth >= self.max_recursion_depth {
            return false;
        }
        self.current_recursion_depth += 1;
        true
    }

    /// Exit a recursive call
    pub fn exit_recursion(&mut self) {
        if self.current_recursion_depth > 0 {
            self.current_recursion_depth -= 1;
        }
    }

    /// Get current recursion depth
    pub fn recursion_depth(&self) -> usize {
        self.current_recursion_depth
    }

    // === Provider Access ===

    /// Get the terminology provider
    pub fn terminology_provider(&self) -> Option<&dyn TerminologyProvider> {
        self.terminology_provider.as_deref()
    }

    /// Get the data provider
    pub fn data_provider(&self) -> Option<&dyn DataProvider> {
        self.data_provider.as_deref()
    }
}

/// Builder for constructing EvaluationContext
pub struct EvaluationContextBuilder {
    context: EvaluationContext,
}

impl EvaluationContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            context: EvaluationContext::new(),
        }
    }

    /// Set the context type and value
    pub fn context(mut self, context_type: impl Into<String>, value: CqlValue) -> Self {
        self.context.context_type = Some(context_type.into());
        self.context.context_value = Some(value);
        self
    }

    /// Set the evaluation timestamp
    pub fn timestamp(mut self, timestamp: DateTime<FixedOffset>) -> Self {
        self.context.evaluation_timestamp = timestamp;
        self
    }

    /// Set the main library
    pub fn library(mut self, library: Library) -> Self {
        self.context.main_library = Some(Arc::new(library));
        self
    }

    /// Add a parameter
    pub fn parameter(mut self, name: impl Into<String>, value: CqlValue) -> Self {
        self.context.parameters.insert(name.into(), value);
        self
    }

    /// Add multiple parameters
    pub fn parameters(mut self, params: impl IntoIterator<Item = (String, CqlValue)>) -> Self {
        for (name, value) in params {
            self.context.parameters.insert(name, value);
        }
        self
    }

    /// Set the terminology provider
    pub fn terminology_provider(mut self, provider: Arc<dyn TerminologyProvider>) -> Self {
        self.context.terminology_provider = Some(provider);
        self
    }

    /// Set the data provider
    pub fn data_provider(mut self, provider: Arc<dyn DataProvider>) -> Self {
        self.context.data_provider = Some(provider);
        self
    }

    /// Set maximum recursion depth
    pub fn max_recursion_depth(mut self, depth: usize) -> Self {
        self.context.max_recursion_depth = depth;
        self
    }

    /// Build the context
    pub fn build(self) -> EvaluationContext {
        self.context
    }
}

impl Default for EvaluationContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = EvaluationContext::new();
        assert!(ctx.context_type.is_none());
        assert!(ctx.context_value.is_none());
    }

    #[test]
    fn test_context_with_patient() {
        let patient = CqlValue::Tuple(octofhir_cql_types::CqlTuple::from_elements([
            ("id", CqlValue::string("123")),
        ]));
        let ctx = EvaluationContext::new().with_context("Patient", patient);
        assert_eq!(ctx.context_type, Some("Patient".to_string()));
    }

    #[test]
    fn test_parameter_management() {
        let mut ctx = EvaluationContext::new();
        ctx.set_parameter("MeasurementPeriod", CqlValue::integer(2024));
        assert_eq!(
            ctx.get_parameter("MeasurementPeriod"),
            Some(&CqlValue::integer(2024))
        );
    }

    #[test]
    fn test_scope_management() {
        let mut ctx = EvaluationContext::new();
        ctx.push_scope();
        ctx.set_alias("E", CqlValue::integer(1));
        assert_eq!(ctx.get_alias("E"), Some(&CqlValue::integer(1)));
        ctx.pop_scope();
        assert_eq!(ctx.get_alias("E"), None);
    }

    #[test]
    fn test_nested_scopes() {
        let mut ctx = EvaluationContext::new();
        ctx.push_scope();
        ctx.set_alias("outer", CqlValue::integer(1));
        ctx.push_scope();
        ctx.set_alias("inner", CqlValue::integer(2));
        // Inner scope can access outer
        assert_eq!(ctx.get_alias("outer"), Some(&CqlValue::integer(1)));
        assert_eq!(ctx.get_alias("inner"), Some(&CqlValue::integer(2)));
        ctx.pop_scope();
        // After pop, inner is gone but outer remains
        assert_eq!(ctx.get_alias("outer"), Some(&CqlValue::integer(1)));
        assert_eq!(ctx.get_alias("inner"), None);
    }

    #[test]
    fn test_temporal_context() {
        let ctx = EvaluationContext::new();
        let now = ctx.now();
        let today = ctx.today();
        let time = ctx.time_of_day();

        // Just verify they return valid values
        assert!(now.year >= 2024);
        assert!(today.year >= 2024);
        assert!(time.hour < 24);
    }

    #[test]
    fn test_recursion_limit() {
        let mut ctx = EvaluationContext::new().with_max_recursion_depth(3);
        assert!(ctx.enter_recursion()); // 1
        assert!(ctx.enter_recursion()); // 2
        assert!(ctx.enter_recursion()); // 3
        assert!(!ctx.enter_recursion()); // exceeds limit
        ctx.exit_recursion();
        assert!(ctx.enter_recursion()); // now works again
    }

    #[test]
    fn test_builder() {
        let ctx = EvaluationContextBuilder::new()
            .parameter("test", CqlValue::boolean(true))
            .max_recursion_depth(50)
            .build();

        assert_eq!(ctx.get_parameter("test"), Some(&CqlValue::boolean(true)));
    }
}
