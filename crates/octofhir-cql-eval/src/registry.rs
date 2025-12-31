//! Operator and function registries for the CQL evaluation engine
//!
//! This module provides registries that map operator/function names to their implementations.

use crate::error::{EvalError, EvalResult};
use crate::EvaluationContext;
use octofhir_cql_types::{CqlList, CqlType, CqlValue};
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for unary operator implementations
pub type UnaryOpFn = Arc<dyn Fn(&CqlValue, &mut EvaluationContext) -> EvalResult<CqlValue> + Send + Sync>;

/// Type alias for binary operator implementations
pub type BinaryOpFn = Arc<dyn Fn(&CqlValue, &CqlValue, &mut EvaluationContext) -> EvalResult<CqlValue> + Send + Sync>;

/// Type alias for n-ary operator implementations
pub type NaryOpFn = Arc<dyn Fn(&[CqlValue], &mut EvaluationContext) -> EvalResult<CqlValue> + Send + Sync>;

/// Type alias for aggregate function implementations
pub type AggregateFn = Arc<dyn Fn(&[CqlValue], Option<&str>, &mut EvaluationContext) -> EvalResult<CqlValue> + Send + Sync>;

/// Operator signature for type checking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperatorSignature {
    /// Operator name
    pub name: String,
    /// Operand types
    pub operand_types: Vec<CqlType>,
    /// Return type
    pub return_type: CqlType,
}

impl OperatorSignature {
    /// Create a new operator signature
    pub fn new(name: impl Into<String>, operand_types: Vec<CqlType>, return_type: CqlType) -> Self {
        Self {
            name: name.into(),
            operand_types,
            return_type,
        }
    }

    /// Create a unary operator signature
    pub fn unary(name: impl Into<String>, operand_type: CqlType, return_type: CqlType) -> Self {
        Self::new(name, vec![operand_type], return_type)
    }

    /// Create a binary operator signature
    pub fn binary(
        name: impl Into<String>,
        left_type: CqlType,
        right_type: CqlType,
        return_type: CqlType,
    ) -> Self {
        Self::new(name, vec![left_type, right_type], return_type)
    }

    /// Check if this signature matches given operand types
    pub fn matches(&self, operand_types: &[CqlType]) -> bool {
        if self.operand_types.len() != operand_types.len() {
            return false;
        }
        self.operand_types
            .iter()
            .zip(operand_types.iter())
            .all(|(sig_type, actual_type)| {
                sig_type.is_any() || actual_type.is_subtype_of(sig_type)
            })
    }
}

/// Registry for unary operators
#[derive(Default)]
pub struct UnaryOperatorRegistry {
    operators: HashMap<String, Vec<(OperatorSignature, UnaryOpFn)>>,
}

impl UnaryOperatorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a unary operator
    pub fn register(
        &mut self,
        name: impl Into<String>,
        operand_type: CqlType,
        return_type: CqlType,
        implementation: UnaryOpFn,
    ) {
        let name = name.into();
        let signature = OperatorSignature::unary(&name, operand_type, return_type);
        self.operators
            .entry(name)
            .or_default()
            .push((signature, implementation));
    }

    /// Get an operator implementation for given operand type
    pub fn get(&self, name: &str, operand_type: &CqlType) -> Option<&UnaryOpFn> {
        self.operators.get(name).and_then(|overloads| {
            overloads
                .iter()
                .find(|(sig, _)| sig.matches(&[operand_type.clone()]))
                .map(|(_, f)| f)
        })
    }

    /// Get all overloads for an operator
    pub fn get_overloads(&self, name: &str) -> Option<&[(OperatorSignature, UnaryOpFn)]> {
        self.operators.get(name).map(|v| v.as_slice())
    }
}

/// Registry for binary operators
#[derive(Default)]
pub struct BinaryOperatorRegistry {
    operators: HashMap<String, Vec<(OperatorSignature, BinaryOpFn)>>,
}

impl BinaryOperatorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a binary operator
    pub fn register(
        &mut self,
        name: impl Into<String>,
        left_type: CqlType,
        right_type: CqlType,
        return_type: CqlType,
        implementation: BinaryOpFn,
    ) {
        let name = name.into();
        let signature = OperatorSignature::binary(&name, left_type, right_type, return_type);
        self.operators
            .entry(name)
            .or_default()
            .push((signature, implementation));
    }

    /// Get an operator implementation for given operand types
    pub fn get(&self, name: &str, left_type: &CqlType, right_type: &CqlType) -> Option<&BinaryOpFn> {
        self.operators.get(name).and_then(|overloads| {
            overloads
                .iter()
                .find(|(sig, _)| sig.matches(&[left_type.clone(), right_type.clone()]))
                .map(|(_, f)| f)
        })
    }

    /// Get all overloads for an operator
    pub fn get_overloads(&self, name: &str) -> Option<&[(OperatorSignature, BinaryOpFn)]> {
        self.operators.get(name).map(|v| v.as_slice())
    }
}

/// Registry for n-ary operators (like Concatenate, Coalesce)
#[derive(Default)]
pub struct NaryOperatorRegistry {
    operators: HashMap<String, NaryOpFn>,
}

impl NaryOperatorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an n-ary operator
    pub fn register(&mut self, name: impl Into<String>, implementation: NaryOpFn) {
        self.operators.insert(name.into(), implementation);
    }

    /// Get an operator implementation
    pub fn get(&self, name: &str) -> Option<&NaryOpFn> {
        self.operators.get(name)
    }
}

/// Registry for aggregate functions
#[derive(Default)]
pub struct AggregateFunctionRegistry {
    functions: HashMap<String, AggregateFn>,
}

impl AggregateFunctionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an aggregate function
    pub fn register(&mut self, name: impl Into<String>, implementation: AggregateFn) {
        self.functions.insert(name.into(), implementation);
    }

    /// Get a function implementation
    pub fn get(&self, name: &str) -> Option<&AggregateFn> {
        self.functions.get(name)
    }
}

/// Function parameter definition
#[derive(Debug, Clone)]
pub struct FunctionParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: CqlType,
    /// Whether optional
    pub optional: bool,
}

impl FunctionParameter {
    /// Create a required parameter
    pub fn required(name: impl Into<String>, param_type: CqlType) -> Self {
        Self {
            name: name.into(),
            param_type,
            optional: false,
        }
    }

    /// Create an optional parameter
    pub fn optional(name: impl Into<String>, param_type: CqlType) -> Self {
        Self {
            name: name.into(),
            param_type,
            optional: true,
        }
    }
}

/// Function definition for user-defined and built-in functions
#[derive(Clone)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,
    /// Parameters
    pub parameters: Vec<FunctionParameter>,
    /// Return type
    pub return_type: CqlType,
    /// Whether this is a fluent function
    pub fluent: bool,
    /// Implementation (None for external/user-defined)
    pub implementation: Option<NaryOpFn>,
}

impl FunctionDefinition {
    /// Create a new function definition
    pub fn new(
        name: impl Into<String>,
        parameters: Vec<FunctionParameter>,
        return_type: CqlType,
    ) -> Self {
        Self {
            name: name.into(),
            parameters,
            return_type,
            fluent: false,
            implementation: None,
        }
    }

    /// Set as fluent function
    pub fn fluent(mut self) -> Self {
        self.fluent = true;
        self
    }

    /// Set implementation
    pub fn with_implementation(mut self, implementation: NaryOpFn) -> Self {
        self.implementation = Some(implementation);
        self
    }

    /// Check if this function matches given argument types
    pub fn matches(&self, arg_types: &[CqlType]) -> bool {
        // Count required parameters
        let required_count = self.parameters.iter().filter(|p| !p.optional).count();

        // Check argument count
        if arg_types.len() < required_count || arg_types.len() > self.parameters.len() {
            return false;
        }

        // Check each argument type
        for (i, arg_type) in arg_types.iter().enumerate() {
            if !arg_type.is_subtype_of(&self.parameters[i].param_type) {
                return false;
            }
        }

        true
    }
}

/// Registry for function definitions
#[derive(Default)]
pub struct FunctionRegistry {
    functions: HashMap<String, Vec<FunctionDefinition>>,
}

impl FunctionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a function
    pub fn register(&mut self, definition: FunctionDefinition) {
        self.functions
            .entry(definition.name.clone())
            .or_default()
            .push(definition);
    }

    /// Get a function definition matching the given name and argument types
    pub fn get(&self, name: &str, arg_types: &[CqlType]) -> Option<&FunctionDefinition> {
        self.functions.get(name).and_then(|overloads| {
            overloads.iter().find(|def| def.matches(arg_types))
        })
    }

    /// Get all overloads for a function
    pub fn get_overloads(&self, name: &str) -> Option<&[FunctionDefinition]> {
        self.functions.get(name).map(|v| v.as_slice())
    }
}

/// Combined operator registry containing all operator types
pub struct OperatorRegistry {
    /// Unary operators
    pub unary: UnaryOperatorRegistry,
    /// Binary operators
    pub binary: BinaryOperatorRegistry,
    /// N-ary operators
    pub nary: NaryOperatorRegistry,
    /// Aggregate functions
    pub aggregate: AggregateFunctionRegistry,
    /// User-defined functions
    pub functions: FunctionRegistry,
}

impl Default for OperatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            unary: UnaryOperatorRegistry::new(),
            binary: BinaryOperatorRegistry::new(),
            nary: NaryOperatorRegistry::new(),
            aggregate: AggregateFunctionRegistry::new(),
            functions: FunctionRegistry::new(),
        }
    }

    /// Create a registry with all standard operators registered
    pub fn with_standard_operators() -> Self {
        let mut registry = Self::new();
        registry.register_standard_operators();
        registry
    }

    /// Register all standard CQL operators
    pub fn register_standard_operators(&mut self) {
        // Register built-in functions

        // descendents function - returns all descendant elements of a value
        // For null input, returns null (null propagation)
        let descendents_fn: NaryOpFn = Arc::new(|args, _ctx| {
            if args.is_empty() {
                return Ok(CqlValue::Null);
            }
            let source = &args[0];
            match source {
                CqlValue::Null => Ok(CqlValue::Null),
                CqlValue::List(list) => {
                    // For a list, collect all elements and their descendants
                    let mut result = Vec::new();
                    collect_descendants_list(&list.elements, &mut result);
                    Ok(CqlValue::List(CqlList::from_elements(result)))
                }
                CqlValue::Tuple(tuple) => {
                    // For a tuple, collect all values and their descendants
                    let mut result = Vec::new();
                    for (_, value) in tuple.iter() {
                        result.push(value.clone());
                        collect_descendants(value, &mut result);
                    }
                    Ok(CqlValue::List(CqlList::from_elements(result)))
                }
                // For scalar values, return empty list
                _ => Ok(CqlValue::List(CqlList::from_elements(vec![]))),
            }
        });

        self.functions.register(
            FunctionDefinition::new(
                "descendents",
                vec![FunctionParameter::required("source", CqlType::Any)],
                CqlType::List(Box::new(CqlType::Any)),
            )
            .fluent()
            .with_implementation(descendents_fn),
        );
    }
}

/// Helper to collect all descendants from a value
fn collect_descendants(value: &CqlValue, result: &mut Vec<CqlValue>) {
    match value {
        CqlValue::List(list) => {
            collect_descendants_list(&list.elements, result);
        }
        CqlValue::Tuple(tuple) => {
            for (_, v) in tuple.iter() {
                result.push(v.clone());
                collect_descendants(v, result);
            }
        }
        _ => {}
    }
}

/// Helper to collect descendants from a list
fn collect_descendants_list(elements: &[CqlValue], result: &mut Vec<CqlValue>) {
    for elem in elements {
        result.push(elem.clone());
        collect_descendants(elem, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_signature_matching() {
        let sig = OperatorSignature::binary("Add", CqlType::Integer, CqlType::Integer, CqlType::Integer);

        assert!(sig.matches(&[CqlType::Integer, CqlType::Integer]));
        assert!(!sig.matches(&[CqlType::String, CqlType::Integer]));
        assert!(!sig.matches(&[CqlType::Integer])); // Wrong arity
    }

    #[test]
    fn test_unary_registry() {
        let mut registry = UnaryOperatorRegistry::new();

        let negate: UnaryOpFn = Arc::new(|value, _ctx| {
            match value {
                CqlValue::Integer(i) => Ok(CqlValue::Integer(-i)),
                _ => Err(EvalError::invalid_operand("Negate", "expected Integer")),
            }
        });

        registry.register("Negate", CqlType::Integer, CqlType::Integer, negate);

        assert!(registry.get("Negate", &CqlType::Integer).is_some());
        assert!(registry.get("Negate", &CqlType::String).is_none());
    }

    #[test]
    fn test_binary_registry() {
        let mut registry = BinaryOperatorRegistry::new();

        let add: BinaryOpFn = Arc::new(|left, right, _ctx| {
            match (left, right) {
                (CqlValue::Integer(a), CqlValue::Integer(b)) => {
                    Ok(CqlValue::Integer(a + b))
                }
                _ => Err(EvalError::invalid_operand("Add", "expected Integer")),
            }
        });

        registry.register("Add", CqlType::Integer, CqlType::Integer, CqlType::Integer, add);

        assert!(registry.get("Add", &CqlType::Integer, &CqlType::Integer).is_some());
        assert!(registry.get("Add", &CqlType::String, &CqlType::Integer).is_none());
    }

    #[test]
    fn test_function_matching() {
        let func = FunctionDefinition::new(
            "TestFunc",
            vec![
                FunctionParameter::required("a", CqlType::Integer),
                FunctionParameter::optional("b", CqlType::String),
            ],
            CqlType::Boolean,
        );

        // One required arg
        assert!(func.matches(&[CqlType::Integer]));
        // Both args
        assert!(func.matches(&[CqlType::Integer, CqlType::String]));
        // No args (missing required)
        assert!(!func.matches(&[]));
        // Too many args
        assert!(!func.matches(&[CqlType::Integer, CqlType::String, CqlType::Boolean]));
    }
}
