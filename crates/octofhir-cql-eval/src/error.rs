//! Evaluation errors for the CQL engine

use octofhir_cql_types::CqlType;
use thiserror::Error;

/// Result type for evaluation operations
pub type EvalResult<T> = Result<T, EvalError>;

/// Errors that can occur during CQL evaluation
#[derive(Debug, Error, Clone)]
pub enum EvalError {
    /// Type mismatch error
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    /// Invalid operand error
    #[error("Invalid operand for {operator}: {message}")]
    InvalidOperand { operator: String, message: String },

    /// Null operand in non-nullable context
    #[error("Null operand not allowed for {operator}")]
    NullOperand { operator: String },

    /// Division by zero
    #[error("Division by zero")]
    DivisionByZero,

    /// Arithmetic overflow
    #[error("Arithmetic overflow in {operation}")]
    Overflow { operation: String },

    /// Arithmetic underflow
    #[error("Arithmetic underflow in {operation}")]
    Underflow { operation: String },

    /// Undefined expression reference
    #[error("Undefined expression: {name}")]
    UndefinedExpression { name: String },

    /// Undefined function reference
    #[error("Undefined function: {name}")]
    UndefinedFunction { name: String },

    /// Undefined parameter
    #[error("Undefined parameter: {name}")]
    UndefinedParameter { name: String },

    /// Undefined alias (query scope)
    #[error("Undefined alias: {name}")]
    UndefinedAlias { name: String },

    /// Undefined let variable (query scope)
    #[error("Undefined let variable: {name}")]
    UndefinedLetVariable { name: String },

    /// Undefined library
    #[error("Undefined library: {name}")]
    UndefinedLibrary { name: String },

    /// Invalid property access
    #[error("Invalid property '{property}' on type {type_name}")]
    InvalidProperty { property: String, type_name: String },

    /// Index out of bounds
    #[error("Index {index} out of bounds for list of length {length}")]
    IndexOutOfBounds { index: i64, length: usize },

    /// Invalid interval (low > high)
    #[error("Invalid interval: low bound exceeds high bound")]
    InvalidInterval,

    /// Invalid regex pattern
    #[error("Invalid regex pattern: {pattern}")]
    InvalidRegex { pattern: String },

    /// Incompatible units for quantity operation
    #[error("Incompatible units: {unit1} and {unit2}")]
    IncompatibleUnits { unit1: String, unit2: String },

    /// Invalid unit
    #[error("Invalid UCUM unit: {unit}")]
    InvalidUnit { unit: String },

    /// Invalid date/time component
    #[error("Invalid {component}: {value}")]
    InvalidDateTimeComponent { component: String, value: String },

    /// Unsupported expression type
    #[error("Unsupported expression type: {expr_type}")]
    UnsupportedExpression { expr_type: String },

    /// Unsupported operator
    #[error("Unsupported operator: {operator} for types {types}")]
    UnsupportedOperator { operator: String, types: String },

    /// Value set not found
    #[error("Value set not found: {name}")]
    ValueSetNotFound { name: String },

    /// Code system not found
    #[error("Code system not found: {name}")]
    CodeSystemNotFound { name: String },

    /// Terminology service error
    #[error("Terminology service error: {message}")]
    TerminologyError { message: String },

    /// Data provider error
    #[error("Data provider error: {message}")]
    DataProviderError { message: String },

    /// Conversion error
    #[error("Cannot convert {from_type} to {to_type}")]
    ConversionError { from_type: String, to_type: String },

    /// Cast error (strict type conversion)
    #[error("Cannot cast {from_type} to {to_type}: {message}")]
    CastError {
        from_type: String,
        to_type: String,
        message: String,
    },

    /// Query evaluation error
    #[error("Query evaluation error: {message}")]
    QueryError { message: String },

    /// Maximum recursion depth exceeded
    #[error("Maximum recursion depth exceeded")]
    RecursionLimit,

    /// Evaluation timeout
    #[error("Evaluation timeout after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Internal error (should not happen)
    #[error("Internal evaluation error: {message}")]
    Internal { message: String },
}

impl EvalError {
    /// Create a type mismatch error
    pub fn type_mismatch(expected: impl Into<String>, found: impl Into<String>) -> Self {
        Self::TypeMismatch {
            expected: expected.into(),
            found: found.into(),
        }
    }

    /// Create a type mismatch error from CqlTypes
    pub fn type_mismatch_cql(expected: &CqlType, found: &CqlType) -> Self {
        Self::TypeMismatch {
            expected: expected.qualified_name(),
            found: found.qualified_name(),
        }
    }

    /// Create an invalid operand error
    pub fn invalid_operand(operator: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidOperand {
            operator: operator.into(),
            message: message.into(),
        }
    }

    /// Create a null operand error
    pub fn null_operand(operator: impl Into<String>) -> Self {
        Self::NullOperand {
            operator: operator.into(),
        }
    }

    /// Create an undefined expression error
    pub fn undefined_expression(name: impl Into<String>) -> Self {
        Self::UndefinedExpression { name: name.into() }
    }

    /// Create an undefined function error
    pub fn undefined_function(name: impl Into<String>) -> Self {
        Self::UndefinedFunction { name: name.into() }
    }

    /// Create an undefined parameter error
    pub fn undefined_parameter(name: impl Into<String>) -> Self {
        Self::UndefinedParameter { name: name.into() }
    }

    /// Create an undefined alias error
    pub fn undefined_alias(name: impl Into<String>) -> Self {
        Self::UndefinedAlias { name: name.into() }
    }

    /// Create an invalid property error
    pub fn invalid_property(property: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self::InvalidProperty {
            property: property.into(),
            type_name: type_name.into(),
        }
    }

    /// Create an unsupported expression error
    pub fn unsupported_expression(expr_type: impl Into<String>) -> Self {
        Self::UnsupportedExpression {
            expr_type: expr_type.into(),
        }
    }

    /// Create an unsupported operator error
    pub fn unsupported_operator(operator: impl Into<String>, types: impl Into<String>) -> Self {
        Self::UnsupportedOperator {
            operator: operator.into(),
            types: types.into(),
        }
    }

    /// Create a conversion error
    pub fn conversion_error(from_type: impl Into<String>, to_type: impl Into<String>) -> Self {
        Self::ConversionError {
            from_type: from_type.into(),
            to_type: to_type.into(),
        }
    }

    /// Create a cast error
    pub fn cast_error(
        from_type: impl Into<String>,
        to_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::CastError {
            from_type: from_type.into(),
            to_type: to_type.into(),
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create an overflow error
    pub fn overflow(operation: impl Into<String>) -> Self {
        Self::Overflow {
            operation: operation.into(),
        }
    }

    /// Create an underflow error
    pub fn underflow(operation: impl Into<String>) -> Self {
        Self::Underflow {
            operation: operation.into(),
        }
    }
}
