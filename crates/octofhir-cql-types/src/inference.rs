//! CQL Type Inference Engine
//!
//! This module implements type inference for CQL expressions following
//! the CQL 1.5 specification. It provides:
//! - Literal type inference
//! - Expression type inference
//! - Operator result type determination
//! - Function return type inference

use octofhir_cql_ast::{
    BinaryOp, Expression, IntervalOp, Literal, TypeSpecifier as AstTypeSpecifier, UnaryOp,
};
use thiserror::Error;

use crate::{CqlType, TupleTypeElement};

/// Type inference errors
#[derive(Debug, Clone, Error)]
pub enum TypeInferenceError {
    /// Type mismatch in operation
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    /// Incompatible operand types
    #[error("Incompatible operand types: {left} and {right} for {operation}")]
    IncompatibleOperands {
        left: String,
        right: String,
        operation: String,
    },

    /// Unknown identifier reference
    #[error("Unknown identifier: {name}")]
    UnknownIdentifier { name: String },

    /// Invalid operation for type
    #[error("Invalid operation {operation} for type {type_name}")]
    InvalidOperation { operation: String, type_name: String },

    /// Cannot infer type
    #[error("Cannot infer type for expression")]
    CannotInfer,

    /// Tuple element not found
    #[error("Tuple element '{name}' not found")]
    TupleElementNotFound { name: String },

    /// Invalid cast
    #[error("Cannot cast from {from} to {to}")]
    InvalidCast { from: String, to: String },
}

/// Type inference result
pub type InferenceResult<T> = Result<T, TypeInferenceError>;

/// Type inference engine
///
/// Provides type inference for CQL expressions.
pub struct TypeInferrer {
    /// Type environment for symbol lookup
    type_env: TypeEnvironment,
}

impl TypeInferrer {
    /// Create a new type inferrer
    pub fn new() -> Self {
        Self {
            type_env: TypeEnvironment::new(),
        }
    }

    /// Create with an existing type environment
    pub fn with_environment(type_env: TypeEnvironment) -> Self {
        Self { type_env }
    }

    /// Get a reference to the type environment
    pub fn environment(&self) -> &TypeEnvironment {
        &self.type_env
    }

    /// Get a mutable reference to the type environment
    pub fn environment_mut(&mut self) -> &mut TypeEnvironment {
        &mut self.type_env
    }

    /// Infer the type of a literal
    pub fn infer_literal(&self, literal: &Literal) -> CqlType {
        match literal {
            Literal::Null => CqlType::Any,
            Literal::Boolean(_) => CqlType::Boolean,
            Literal::Integer(_) => CqlType::Integer,
            Literal::Long(_) => CqlType::Long,
            Literal::Decimal(_) => CqlType::Decimal,
            Literal::String(_) => CqlType::String,
            Literal::Date(_) => CqlType::Date,
            Literal::DateTime(_) => CqlType::DateTime,
            Literal::Time(_) => CqlType::Time,
            Literal::Quantity(_) => CqlType::Quantity,
            Literal::Ratio(_) => CqlType::Ratio,
        }
    }

    /// Infer the type of an expression
    pub fn infer_expression(&self, expr: &Expression) -> InferenceResult<CqlType> {
        match expr {
            Expression::Literal(lit) => Ok(self.infer_literal(lit)),

            Expression::IdentifierRef(id_ref) => {
                self.type_env
                    .lookup(&id_ref.name.name)
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::UnknownIdentifier {
                        name: id_ref.name.name.clone(),
                    })
            }

            Expression::QualifiedIdentifierRef(qid_ref) => {
                let name = if let Some(q) = &qid_ref.name.qualifier {
                    format!("{}.{}", q, qid_ref.name.name.name)
                } else {
                    qid_ref.name.name.name.clone()
                };
                self.type_env
                    .lookup(&name)
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::UnknownIdentifier { name })
            }

            Expression::BinaryOp(bin_op) => {
                let left_type = self.infer_expression(&bin_op.left.inner)?;
                let right_type = self.infer_expression(&bin_op.right.inner)?;
                self.infer_binary_op(&bin_op.op, &left_type, &right_type)
            }

            Expression::UnaryOp(unary_op) => {
                let operand_type = self.infer_expression(&unary_op.operand.inner)?;
                self.infer_unary_op(&unary_op.op, &operand_type)
            }

            Expression::IntervalOp(interval_op) => {
                let _left_type = self.infer_expression(&interval_op.left.inner)?;
                let _right_type = self.infer_expression(&interval_op.right.inner)?;
                self.infer_interval_op(&interval_op.op)
            }

            Expression::If(if_expr) => {
                let then_type = self.infer_expression(&if_expr.then_expr.inner)?;
                let else_type = self.infer_expression(&if_expr.else_expr.inner)?;
                then_type.common_supertype(&else_type).ok_or_else(|| {
                    TypeInferenceError::IncompatibleOperands {
                        left: then_type.qualified_name(),
                        right: else_type.qualified_name(),
                        operation: "if-then-else".to_string(),
                    }
                })
            }

            Expression::Case(case_expr) => {
                if let Some(first_item) = case_expr.items.first() {
                    let first_type = self.infer_expression(&first_item.then.inner)?;
                    if let Some(else_expr) = &case_expr.else_expr {
                        let else_type = self.infer_expression(&else_expr.inner)?;
                        first_type.common_supertype(&else_type).ok_or_else(|| {
                            TypeInferenceError::IncompatibleOperands {
                                left: first_type.qualified_name(),
                                right: else_type.qualified_name(),
                                operation: "case else".to_string(),
                            }
                        })
                    } else {
                        Ok(first_type)
                    }
                } else if let Some(else_expr) = &case_expr.else_expr {
                    self.infer_expression(&else_expr.inner)
                } else {
                    Ok(CqlType::Any)
                }
            }

            Expression::Coalesce(coalesce) => {
                if coalesce.operands.is_empty() {
                    return Ok(CqlType::Any);
                }
                let first_type = self.infer_expression(&coalesce.operands[0].inner)?;
                Ok(first_type)
            }

            Expression::List(list_expr) => {
                if list_expr.elements.is_empty() {
                    if let Some(elem_type) = &list_expr.element_type {
                        Ok(CqlType::list(self.ast_type_to_cql_type(&elem_type.inner)))
                    } else {
                        Ok(CqlType::list(CqlType::Any))
                    }
                } else {
                    let first_type = self.infer_expression(&list_expr.elements[0].inner)?;
                    let mut result_type = first_type;
                    for elem in list_expr.elements.iter().skip(1) {
                        let elem_type = self.infer_expression(&elem.inner)?;
                        result_type = result_type
                            .common_supertype(&elem_type)
                            .unwrap_or(CqlType::Any);
                    }
                    Ok(CqlType::list(result_type))
                }
            }

            Expression::Interval(interval_expr) => {
                let point_type = if let Some(low) = &interval_expr.low {
                    self.infer_expression(&low.inner)?
                } else if let Some(high) = &interval_expr.high {
                    self.infer_expression(&high.inner)?
                } else {
                    CqlType::Any
                };
                Ok(CqlType::interval(point_type))
            }

            Expression::Tuple(tuple_expr) => {
                let elements: Result<Vec<TupleTypeElement>, _> = tuple_expr
                    .elements
                    .iter()
                    .map(|elem| {
                        let elem_type = self.infer_expression(&elem.value.inner)?;
                        Ok(TupleTypeElement::new(elem.name.name.clone(), elem_type))
                    })
                    .collect();
                Ok(CqlType::tuple(elements?))
            }

            Expression::Property(prop) => {
                let source_type = self.infer_expression(&prop.source.inner)?;
                self.infer_property_type(&source_type, &prop.property.name)
            }

            Expression::Indexer(indexer) => {
                let source_type = self.infer_expression(&indexer.source.inner)?;
                match source_type {
                    CqlType::List(elem_type) => Ok(*elem_type),
                    CqlType::String => Ok(CqlType::String),
                    _ => Err(TypeInferenceError::InvalidOperation {
                        operation: "indexer".to_string(),
                        type_name: source_type.qualified_name(),
                    }),
                }
            }

            Expression::As(as_expr) => Ok(self.ast_type_to_cql_type(&as_expr.as_type.inner)),

            Expression::Is(_) => Ok(CqlType::Boolean),

            Expression::Convert(convert_expr) => {
                Ok(self.ast_type_to_cql_type(&convert_expr.to_type.inner))
            }

            Expression::Cast(cast_expr) => Ok(self.ast_type_to_cql_type(&cast_expr.as_type.inner)),

            Expression::IsNull(_) | Expression::IsTrue(_) | Expression::IsFalse(_) => {
                Ok(CqlType::Boolean)
            }

            Expression::Start(start_expr) => {
                let interval_type = self.infer_expression(&start_expr.operand.inner)?;
                interval_type
                    .point_type()
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::InvalidOperation {
                        operation: "start".to_string(),
                        type_name: interval_type.qualified_name(),
                    })
            }

            Expression::End(end_expr) => {
                let interval_type = self.infer_expression(&end_expr.operand.inner)?;
                interval_type
                    .point_type()
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::InvalidOperation {
                        operation: "end".to_string(),
                        type_name: interval_type.qualified_name(),
                    })
            }

            Expression::Width(_) | Expression::Size(_) => Ok(CqlType::Integer),

            Expression::PointFrom(point_from) => {
                let interval_type = self.infer_expression(&point_from.operand.inner)?;
                interval_type
                    .point_type()
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::InvalidOperation {
                        operation: "point from".to_string(),
                        type_name: interval_type.qualified_name(),
                    })
            }

            Expression::Now => Ok(CqlType::DateTime),
            Expression::Today => Ok(CqlType::Date),
            Expression::TimeOfDay => Ok(CqlType::Time),

            Expression::Date(_) => Ok(CqlType::Date),
            Expression::DateTime(_) => Ok(CqlType::DateTime),
            Expression::Time(_) => Ok(CqlType::Time),

            Expression::DurationBetween(_) | Expression::DifferenceBetween(_) => {
                Ok(CqlType::Integer)
            }

            Expression::DateTimeComponent(dtc) => {
                use octofhir_cql_ast::DateTimeComponent;
                match dtc.component {
                    DateTimeComponent::Date => Ok(CqlType::Date),
                    DateTimeComponent::Time => Ok(CqlType::Time),
                    _ => Ok(CqlType::Integer),
                }
            }

            Expression::Concatenate(_) => Ok(CqlType::String),
            Expression::Combine(_) => Ok(CqlType::String),
            Expression::Split(_) => Ok(CqlType::list(CqlType::String)),
            Expression::Matches(_) => Ok(CqlType::Boolean),
            Expression::ReplaceMatches(_) => Ok(CqlType::String),

            Expression::First(first) => {
                let source_type = self.infer_expression(&first.source.inner)?;
                source_type
                    .element_type()
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::InvalidOperation {
                        operation: "first".to_string(),
                        type_name: source_type.qualified_name(),
                    })
            }

            Expression::Last(last) => {
                let source_type = self.infer_expression(&last.source.inner)?;
                source_type
                    .element_type()
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::InvalidOperation {
                        operation: "last".to_string(),
                        type_name: source_type.qualified_name(),
                    })
            }

            Expression::Single(single) => {
                let source_type = self.infer_expression(&single.source.inner)?;
                source_type
                    .element_type()
                    .cloned()
                    .ok_or_else(|| TypeInferenceError::InvalidOperation {
                        operation: "single".to_string(),
                        type_name: source_type.qualified_name(),
                    })
            }

            Expression::Slice(slice) => {
                let source_type = self.infer_expression(&slice.source.inner)?;
                match &source_type {
                    CqlType::List(_) => Ok(source_type),
                    _ => Err(TypeInferenceError::InvalidOperation {
                        operation: "slice".to_string(),
                        type_name: source_type.qualified_name(),
                    }),
                }
            }

            Expression::IndexOf(_) => Ok(CqlType::Integer),

            Expression::Between(_) => Ok(CqlType::Boolean),

            Expression::Message(msg) => self.infer_expression(&msg.source.inner),

            Expression::SameAs(_) | Expression::SameOrBefore(_) | Expression::SameOrAfter(_) => {
                Ok(CqlType::Boolean)
            }

            Expression::FunctionRef(_) => Ok(CqlType::Any),

            Expression::ExternalFunctionRef(_) => Ok(CqlType::Any),

            Expression::Query(query) => {
                if let Some(ret) = &query.return_clause {
                    let return_type = self.infer_expression(&ret.expression.inner)?;
                    Ok(CqlType::list(return_type))
                } else {
                    Ok(CqlType::list(CqlType::Any))
                }
            }

            Expression::Retrieve(retrieve) => {
                // Retrieve returns list of the data type
                let type_name = self.ast_type_to_string(&retrieve.data_type.inner);
                Ok(CqlType::list(CqlType::Named {
                    namespace: None,
                    name: type_name,
                }))
            }

            Expression::Aggregate(_) => Ok(CqlType::Any),

            Expression::Instance(instance) => {
                Ok(self.ast_type_to_cql_type(&instance.class_type.inner))
            }

            Expression::Total(_) => Ok(CqlType::Any),
            Expression::Iteration | Expression::Index | Expression::TotalRef => Ok(CqlType::Any),

            Expression::Error => Ok(CqlType::Any),
        }
    }

    /// Infer result type for binary operations
    pub fn infer_binary_op(
        &self,
        op: &BinaryOp,
        left: &CqlType,
        right: &CqlType,
    ) -> InferenceResult<CqlType> {
        match op {
            // Comparison operators -> Boolean
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Equivalent
            | BinaryOp::NotEquivalent
            | BinaryOp::Less
            | BinaryOp::LessOrEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterOrEqual => Ok(CqlType::Boolean),

            // Logical operators -> Boolean
            BinaryOp::And | BinaryOp::Or | BinaryOp::Xor | BinaryOp::Implies => {
                Ok(CqlType::Boolean)
            }

            // Arithmetic operators
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                self.infer_arithmetic_result(left, right, op)
            }

            BinaryOp::Modulo | BinaryOp::TruncatedDivide => {
                if left.is_numeric() && right.is_numeric() {
                    left.common_supertype(right).ok_or_else(|| {
                        TypeInferenceError::IncompatibleOperands {
                            left: left.qualified_name(),
                            right: right.qualified_name(),
                            operation: format!("{:?}", op),
                        }
                    })
                } else {
                    Err(TypeInferenceError::InvalidOperation {
                        operation: format!("{:?}", op),
                        type_name: left.qualified_name(),
                    })
                }
            }

            BinaryOp::Power => Ok(CqlType::Decimal),

            // String operations
            BinaryOp::Concatenate => Ok(CqlType::String),

            // List operations
            BinaryOp::In | BinaryOp::Contains => Ok(CqlType::Boolean),

            BinaryOp::Union => {
                match (left, right) {
                    (CqlType::List(elem1), CqlType::List(elem2)) => {
                        let common = elem1.common_supertype(elem2).unwrap_or(CqlType::Any);
                        Ok(CqlType::list(common))
                    }
                    _ => Err(TypeInferenceError::IncompatibleOperands {
                        left: left.qualified_name(),
                        right: right.qualified_name(),
                        operation: "union".to_string(),
                    }),
                }
            }

            // Type operators handled separately
            BinaryOp::Is | BinaryOp::As => Ok(CqlType::Any),
        }
    }

    /// Infer result type for unary operations
    pub fn infer_unary_op(&self, op: &UnaryOp, operand: &CqlType) -> InferenceResult<CqlType> {
        match op {
            UnaryOp::Not => Ok(CqlType::Boolean),
            UnaryOp::Plus => Ok(operand.clone()),
            UnaryOp::Negate => Ok(operand.clone()),
            UnaryOp::Exists => Ok(CqlType::Boolean),
            UnaryOp::Distinct => Ok(operand.clone()),
            UnaryOp::Flatten => {
                if let CqlType::List(inner) = operand {
                    if let CqlType::List(elem) = inner.as_ref() {
                        Ok(CqlType::list(elem.as_ref().clone()))
                    } else {
                        Ok(operand.clone())
                    }
                } else {
                    Ok(operand.clone())
                }
            }
            UnaryOp::Collapse => Ok(operand.clone()),
            UnaryOp::SingletonFrom => {
                if let CqlType::List(elem) = operand {
                    Ok(elem.as_ref().clone())
                } else {
                    Ok(operand.clone())
                }
            }
        }
    }

    /// Infer result type for interval operations
    pub fn infer_interval_op(&self, op: &IntervalOp) -> InferenceResult<CqlType> {
        // All interval comparison operations return Boolean
        match op {
            IntervalOp::ProperlyIncludes
            | IntervalOp::ProperlyIncludedIn
            | IntervalOp::Includes
            | IntervalOp::IncludedIn
            | IntervalOp::Before
            | IntervalOp::After
            | IntervalOp::Meets
            | IntervalOp::MeetsBefore
            | IntervalOp::MeetsAfter
            | IntervalOp::Overlaps
            | IntervalOp::OverlapsBefore
            | IntervalOp::OverlapsAfter
            | IntervalOp::Starts
            | IntervalOp::Ends
            | IntervalOp::During
            | IntervalOp::SameAs
            | IntervalOp::SameOrBefore
            | IntervalOp::SameOrAfter => Ok(CqlType::Boolean),
        }
    }

    /// Infer arithmetic result type
    fn infer_arithmetic_result(
        &self,
        left: &CqlType,
        right: &CqlType,
        op: &BinaryOp,
    ) -> InferenceResult<CqlType> {
        match (left, right) {
            // Numeric + Numeric
            (l, r) if l.is_numeric() && r.is_numeric() => {
                l.common_supertype(r).ok_or_else(|| {
                    TypeInferenceError::IncompatibleOperands {
                        left: l.qualified_name(),
                        right: r.qualified_name(),
                        operation: format!("{:?}", op),
                    }
                })
            }

            // Quantity operations
            (CqlType::Quantity, CqlType::Quantity) => Ok(CqlType::Quantity),
            (CqlType::Quantity, r) if r.is_numeric() => Ok(CqlType::Quantity),
            (l, CqlType::Quantity) if l.is_numeric() => Ok(CqlType::Quantity),

            // Date/Time + Quantity (duration)
            (CqlType::Date, CqlType::Quantity) | (CqlType::Quantity, CqlType::Date) => {
                Ok(CqlType::Date)
            }
            (CqlType::DateTime, CqlType::Quantity) | (CqlType::Quantity, CqlType::DateTime) => {
                Ok(CqlType::DateTime)
            }
            (CqlType::Time, CqlType::Quantity) | (CqlType::Quantity, CqlType::Time) => {
                Ok(CqlType::Time)
            }

            _ => Err(TypeInferenceError::IncompatibleOperands {
                left: left.qualified_name(),
                right: right.qualified_name(),
                operation: format!("{:?}", op),
            }),
        }
    }

    /// Infer property access type
    fn infer_property_type(&self, source: &CqlType, property: &str) -> InferenceResult<CqlType> {
        match source {
            CqlType::Tuple(elements) => {
                elements
                    .iter()
                    .find(|e| e.name == property)
                    .map(|e| e.element_type.clone())
                    .ok_or_else(|| TypeInferenceError::TupleElementNotFound {
                        name: property.to_string(),
                    })
            }

            CqlType::Quantity => match property {
                "value" => Ok(CqlType::Decimal),
                "unit" => Ok(CqlType::String),
                _ => Err(TypeInferenceError::TupleElementNotFound {
                    name: property.to_string(),
                }),
            },

            CqlType::Code => match property {
                "code" | "system" | "version" | "display" => Ok(CqlType::String),
                _ => Err(TypeInferenceError::TupleElementNotFound {
                    name: property.to_string(),
                }),
            },

            CqlType::Concept => match property {
                "codes" => Ok(CqlType::list(CqlType::Code)),
                "display" => Ok(CqlType::String),
                _ => Err(TypeInferenceError::TupleElementNotFound {
                    name: property.to_string(),
                }),
            },

            CqlType::Ratio => match property {
                "numerator" | "denominator" => Ok(CqlType::Quantity),
                _ => Err(TypeInferenceError::TupleElementNotFound {
                    name: property.to_string(),
                }),
            },

            // For named types, we'd need type metadata
            CqlType::Named { .. } => Ok(CqlType::Any),

            // List property access returns list of property type
            CqlType::List(elem) => {
                let prop_type = self.infer_property_type(elem, property)?;
                Ok(CqlType::list(prop_type))
            }

            _ => Err(TypeInferenceError::InvalidOperation {
                operation: format!("property access .{}", property),
                type_name: source.qualified_name(),
            }),
        }
    }

    /// Convert AST TypeSpecifier to string
    fn ast_type_to_string(&self, ast_type: &AstTypeSpecifier) -> String {
        match ast_type {
            AstTypeSpecifier::Named(named) => {
                if let Some(ns) = &named.namespace {
                    format!("{}.{}", ns, named.name)
                } else {
                    named.name.clone()
                }
            }
            AstTypeSpecifier::List(list) => {
                format!("List<{}>", self.ast_type_to_string(&list.element_type))
            }
            AstTypeSpecifier::Interval(interval) => {
                format!("Interval<{}>", self.ast_type_to_string(&interval.point_type))
            }
            AstTypeSpecifier::Tuple(_) => "Tuple".to_string(),
            AstTypeSpecifier::Choice(_) => "Choice".to_string(),
        }
    }

    /// Convert AST TypeSpecifier to CqlType
    pub fn ast_type_to_cql_type(&self, ast_type: &AstTypeSpecifier) -> CqlType {
        match ast_type {
            AstTypeSpecifier::Named(named) => {
                let name = &named.name;
                let namespace = named.namespace.as_deref();

                // Check for system types
                match (namespace, name.as_str()) {
                    (Some("System") | None, "Any") => CqlType::Any,
                    (Some("System") | None, "Boolean") => CqlType::Boolean,
                    (Some("System") | None, "Integer") => CqlType::Integer,
                    (Some("System") | None, "Long") => CqlType::Long,
                    (Some("System") | None, "Decimal") => CqlType::Decimal,
                    (Some("System") | None, "String") => CqlType::String,
                    (Some("System") | None, "Date") => CqlType::Date,
                    (Some("System") | None, "DateTime") => CqlType::DateTime,
                    (Some("System") | None, "Time") => CqlType::Time,
                    (Some("System") | None, "Quantity") => CqlType::Quantity,
                    (Some("System") | None, "Ratio") => CqlType::Ratio,
                    (Some("System") | None, "Code") => CqlType::Code,
                    (Some("System") | None, "Concept") => CqlType::Concept,
                    (Some("System") | None, "Vocabulary") => CqlType::Vocabulary,
                    _ => CqlType::Named {
                        namespace: namespace.map(|s| s.to_string()),
                        name: name.clone(),
                    },
                }
            }
            AstTypeSpecifier::List(list) => {
                CqlType::list(self.ast_type_to_cql_type(&list.element_type))
            }
            AstTypeSpecifier::Interval(interval) => {
                CqlType::interval(self.ast_type_to_cql_type(&interval.point_type))
            }
            AstTypeSpecifier::Tuple(tuple) => {
                let elements = tuple
                    .elements
                    .iter()
                    .map(|e| {
                        let elem_type = e
                            .element_type
                            .as_ref()
                            .map(|t| self.ast_type_to_cql_type(t))
                            .unwrap_or(CqlType::Any);
                        TupleTypeElement::new(e.name.name.clone(), elem_type)
                    })
                    .collect();
                CqlType::tuple(elements)
            }
            AstTypeSpecifier::Choice(choice) => {
                let types = choice
                    .types
                    .iter()
                    .map(|t| self.ast_type_to_cql_type(t))
                    .collect();
                CqlType::choice(types)
            }
        }
    }
}

impl Default for TypeInferrer {
    fn default() -> Self {
        Self::new()
    }
}

/// Type environment for tracking variable and symbol types
#[derive(Debug, Clone, Default)]
pub struct TypeEnvironment {
    /// Symbol table mapping names to types
    symbols: indexmap::IndexMap<String, CqlType>,
    /// Parent environment (for scoping)
    parent: Option<Box<TypeEnvironment>>,
}

impl TypeEnvironment {
    /// Create a new empty environment
    pub fn new() -> Self {
        Self {
            symbols: indexmap::IndexMap::new(),
            parent: None,
        }
    }

    /// Create a child environment
    pub fn child(&self) -> Self {
        Self {
            symbols: indexmap::IndexMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Define a symbol
    pub fn define(&mut self, name: impl Into<String>, ty: CqlType) {
        self.symbols.insert(name.into(), ty);
    }

    /// Look up a symbol
    pub fn lookup(&self, name: &str) -> Option<&CqlType> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    /// Check if a symbol is defined
    pub fn is_defined(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Get all locally defined symbols
    pub fn local_symbols(&self) -> impl Iterator<Item = (&String, &CqlType)> {
        self.symbols.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_literal() {
        let inferrer = TypeInferrer::new();

        assert_eq!(inferrer.infer_literal(&Literal::Boolean(true)), CqlType::Boolean);
        assert_eq!(inferrer.infer_literal(&Literal::Integer(42)), CqlType::Integer);
        assert_eq!(
            inferrer.infer_literal(&Literal::String("hello".to_string())),
            CqlType::String
        );
    }

    #[test]
    fn test_type_environment() {
        let mut env = TypeEnvironment::new();
        env.define("x", CqlType::Integer);
        env.define("y", CqlType::String);

        assert_eq!(env.lookup("x"), Some(&CqlType::Integer));
        assert_eq!(env.lookup("y"), Some(&CqlType::String));
        assert_eq!(env.lookup("z"), None);

        let child = env.child();
        assert_eq!(child.lookup("x"), Some(&CqlType::Integer));
    }
}
