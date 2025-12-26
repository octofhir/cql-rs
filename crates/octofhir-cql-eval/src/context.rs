//! Evaluation context for CQL execution

use std::collections::HashMap;
use crate::CqlValue;

/// Evaluation context for CQL expression execution
pub struct EvaluationContext {
    /// Current context type (e.g., "Patient")
    pub context_type: Option<String>,
    /// Current context value (e.g., Patient resource)
    pub context_value: Option<CqlValue>,
    /// Parameter values
    pub parameters: HashMap<String, CqlValue>,
    /// Local variables/definitions
    pub locals: HashMap<String, CqlValue>,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EvaluationContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            context_type: None,
            context_value: None,
            parameters: HashMap::new(),
            locals: HashMap::new(),
        }
    }

    /// Set the context type and value
    pub fn with_context(mut self, context_type: impl Into<String>, value: CqlValue) -> Self {
        self.context_type = Some(context_type.into());
        self.context_value = Some(value);
        self
    }

    /// Set a parameter value
    pub fn set_parameter(&mut self, name: impl Into<String>, value: CqlValue) {
        self.parameters.insert(name.into(), value);
    }

    /// Get a parameter value
    pub fn get_parameter(&self, name: &str) -> Option<&CqlValue> {
        self.parameters.get(name)
    }

    /// Set a local variable
    pub fn set_local(&mut self, name: impl Into<String>, value: CqlValue) {
        self.locals.insert(name.into(), value);
    }

    /// Get a local variable
    pub fn get_local(&self, name: &str) -> Option<&CqlValue> {
        self.locals.get(name)
    }
}
