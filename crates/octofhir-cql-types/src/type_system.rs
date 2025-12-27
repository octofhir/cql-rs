//! CQL Type System
//!
//! This module defines the complete CQL type system including:
//! - CqlType enum representing all CQL types
//! - Type specifiers for compile-time type representations
//! - Tuple and choice type definitions
//! - Type display and comparison utilities

use serde::{Deserialize, Serialize};
use std::fmt;

/// The complete CQL type representation
///
/// This enum represents all types in the CQL type system per the CQL 1.5 specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CqlType {
    // === Special Types ===
    /// The Any type - supertype of all types
    Any,

    // === Primitive Types ===
    /// Boolean type (true/false/null)
    Boolean,
    /// 32-bit signed integer
    Integer,
    /// 64-bit signed integer
    Long,
    /// Arbitrary precision decimal
    Decimal,
    /// Unicode string
    String,

    // === Temporal Types ===
    /// Date with variable precision
    Date,
    /// DateTime with variable precision and timezone
    DateTime,
    /// Time of day with variable precision
    Time,

    // === Clinical Types ===
    /// Quantity with value and UCUM unit
    Quantity,
    /// Ratio of two quantities
    Ratio,
    /// Code from a code system
    Code,
    /// Concept (collection of equivalent codes)
    Concept,
    /// Vocabulary type (CodeSystem or ValueSet)
    Vocabulary,

    // === Collection Types ===
    /// List of elements
    #[serde(rename = "List")]
    List(Box<CqlType>),
    /// Interval between two points
    #[serde(rename = "Interval")]
    Interval(Box<CqlType>),
    /// Tuple with named elements
    #[serde(rename = "Tuple")]
    Tuple(Vec<TupleTypeElement>),

    // === Choice Type ===
    /// Choice of multiple types (union type)
    #[serde(rename = "Choice")]
    Choice(Vec<CqlType>),

    // === Named Types ===
    /// Named type reference (e.g., FHIR.Patient)
    #[serde(rename = "NamedType")]
    Named {
        /// Optional namespace (e.g., "FHIR", "System")
        namespace: Option<String>,
        /// Type name
        name: String,
    },
}

impl CqlType {
    // === Constructors ===

    /// Create a System.Any type
    pub fn any() -> Self {
        Self::Any
    }

    /// Create a list type
    pub fn list(element_type: CqlType) -> Self {
        Self::List(Box::new(element_type))
    }

    /// Create an interval type
    pub fn interval(point_type: CqlType) -> Self {
        Self::Interval(Box::new(point_type))
    }

    /// Create a tuple type
    pub fn tuple(elements: Vec<TupleTypeElement>) -> Self {
        Self::Tuple(elements)
    }

    /// Create a choice type
    pub fn choice(types: Vec<CqlType>) -> Self {
        Self::Choice(types)
    }

    /// Create a named type
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named {
            namespace: None,
            name: name.into(),
        }
    }

    /// Create a qualified named type
    pub fn qualified(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self::Named {
            namespace: Some(namespace.into()),
            name: name.into(),
        }
    }

    // === Type Properties ===

    /// Check if this is the Any type
    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }

    /// Check if this is a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Self::Boolean | Self::Integer | Self::Long | Self::Decimal | Self::String
        )
    }

    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Integer | Self::Long | Self::Decimal)
    }

    /// Check if this is a temporal type
    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::Date | Self::DateTime | Self::Time)
    }

    /// Check if this is a clinical type
    pub fn is_clinical(&self) -> bool {
        matches!(
            self,
            Self::Quantity | Self::Ratio | Self::Code | Self::Concept | Self::Vocabulary
        )
    }

    /// Check if this is a collection type
    pub fn is_collection(&self) -> bool {
        matches!(self, Self::List(_) | Self::Interval(_) | Self::Tuple(_))
    }

    /// Check if this type is ordered (supports comparison operators)
    pub fn is_ordered(&self) -> bool {
        matches!(
            self,
            Self::Integer
                | Self::Long
                | Self::Decimal
                | Self::String
                | Self::Date
                | Self::DateTime
                | Self::Time
                | Self::Quantity
        )
    }

    /// Check if this type can be used as interval point type
    pub fn is_interval_point_type(&self) -> bool {
        self.is_ordered()
    }

    /// Get the namespace for this type
    pub fn namespace(&self) -> Option<&str> {
        match self {
            Self::Named { namespace, .. } => namespace.as_deref(),
            // System types have implicit "System" namespace
            Self::Any
            | Self::Boolean
            | Self::Integer
            | Self::Long
            | Self::Decimal
            | Self::String
            | Self::Date
            | Self::DateTime
            | Self::Time
            | Self::Quantity
            | Self::Ratio
            | Self::Code
            | Self::Concept
            | Self::Vocabulary => Some("System"),
            _ => None,
        }
    }

    /// Get the simple name of this type
    pub fn name(&self) -> &str {
        match self {
            Self::Any => "Any",
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Long => "Long",
            Self::Decimal => "Decimal",
            Self::String => "String",
            Self::Date => "Date",
            Self::DateTime => "DateTime",
            Self::Time => "Time",
            Self::Quantity => "Quantity",
            Self::Ratio => "Ratio",
            Self::Code => "Code",
            Self::Concept => "Concept",
            Self::Vocabulary => "Vocabulary",
            Self::List(_) => "List",
            Self::Interval(_) => "Interval",
            Self::Tuple(_) => "Tuple",
            Self::Choice(_) => "Choice",
            Self::Named { name, .. } => name,
        }
    }

    /// Get the fully qualified name of this type
    pub fn qualified_name(&self) -> String {
        match self {
            Self::Any => "System.Any".to_string(),
            Self::Boolean => "System.Boolean".to_string(),
            Self::Integer => "System.Integer".to_string(),
            Self::Long => "System.Long".to_string(),
            Self::Decimal => "System.Decimal".to_string(),
            Self::String => "System.String".to_string(),
            Self::Date => "System.Date".to_string(),
            Self::DateTime => "System.DateTime".to_string(),
            Self::Time => "System.Time".to_string(),
            Self::Quantity => "System.Quantity".to_string(),
            Self::Ratio => "System.Ratio".to_string(),
            Self::Code => "System.Code".to_string(),
            Self::Concept => "System.Concept".to_string(),
            Self::Vocabulary => "System.Vocabulary".to_string(),
            Self::List(elem) => format!("List<{}>", elem.qualified_name()),
            Self::Interval(point) => format!("Interval<{}>", point.qualified_name()),
            Self::Tuple(elements) => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| format!("{}: {}", e.name, e.element_type.qualified_name()))
                    .collect();
                format!("Tuple {{ {} }}", elems.join(", "))
            }
            Self::Choice(types) => {
                let type_names: Vec<String> =
                    types.iter().map(|t| t.qualified_name()).collect();
                format!("Choice<{}>", type_names.join(", "))
            }
            Self::Named { namespace, name } => {
                if let Some(ns) = namespace {
                    format!("{}.{}", ns, name)
                } else {
                    name.clone()
                }
            }
        }
    }

    /// Get the element type for List types
    pub fn element_type(&self) -> Option<&CqlType> {
        match self {
            Self::List(elem) => Some(elem),
            _ => None,
        }
    }

    /// Get the point type for Interval types
    pub fn point_type(&self) -> Option<&CqlType> {
        match self {
            Self::Interval(point) => Some(point),
            _ => None,
        }
    }

    /// Get tuple elements for Tuple types
    pub fn tuple_elements(&self) -> Option<&[TupleTypeElement]> {
        match self {
            Self::Tuple(elements) => Some(elements),
            _ => None,
        }
    }

    /// Get choice types for Choice types
    pub fn choice_types(&self) -> Option<&[CqlType]> {
        match self {
            Self::Choice(types) => Some(types),
            _ => None,
        }
    }

    // === Type Relationships ===

    /// Check if this type is a subtype of another type
    ///
    /// Per CQL spec:
    /// - Any is the supertype of all types
    /// - Integer is a subtype of Long
    /// - Integer and Long are subtypes of Decimal
    /// - List<A> is a subtype of List<B> if A is a subtype of B
    /// - Interval<A> is a subtype of Interval<B> if A is a subtype of B
    pub fn is_subtype_of(&self, other: &CqlType) -> bool {
        // Any is supertype of everything
        if matches!(other, CqlType::Any) {
            return true;
        }

        // Same type is subtype of itself
        if self == other {
            return true;
        }

        // Numeric subtyping: Integer < Long < Decimal
        match (self, other) {
            (CqlType::Integer, CqlType::Long) => true,
            (CqlType::Integer, CqlType::Decimal) => true,
            (CqlType::Long, CqlType::Decimal) => true,

            // List covariance
            (CqlType::List(elem_a), CqlType::List(elem_b)) => elem_a.is_subtype_of(elem_b),

            // Interval covariance
            (CqlType::Interval(point_a), CqlType::Interval(point_b)) => {
                point_a.is_subtype_of(point_b)
            }

            // Choice type - this type is subtype if it's a subtype of any choice
            (_, CqlType::Choice(choices)) => choices.iter().any(|c| self.is_subtype_of(c)),

            // Named type matching
            (
                CqlType::Named {
                    namespace: ns1,
                    name: n1,
                },
                CqlType::Named {
                    namespace: ns2,
                    name: n2,
                },
            ) => ns1 == ns2 && n1 == n2,

            _ => false,
        }
    }

    /// Check if this type is a supertype of another type
    pub fn is_supertype_of(&self, other: &CqlType) -> bool {
        other.is_subtype_of(self)
    }

    /// Check if types are compatible (one is subtype of the other)
    pub fn is_compatible_with(&self, other: &CqlType) -> bool {
        self.is_subtype_of(other) || other.is_subtype_of(self)
    }

    /// Find the common supertype of two types
    ///
    /// Returns the least upper bound in the type hierarchy.
    pub fn common_supertype(&self, other: &CqlType) -> Option<CqlType> {
        if self == other {
            return Some(self.clone());
        }

        if self.is_subtype_of(other) {
            return Some(other.clone());
        }

        if other.is_subtype_of(self) {
            return Some(self.clone());
        }

        // Numeric types have common supertype
        match (self, other) {
            (CqlType::Integer, CqlType::Long) | (CqlType::Long, CqlType::Integer) => {
                Some(CqlType::Long)
            }
            (CqlType::Integer, CqlType::Decimal) | (CqlType::Decimal, CqlType::Integer) => {
                Some(CqlType::Decimal)
            }
            (CqlType::Long, CqlType::Decimal) | (CqlType::Decimal, CqlType::Long) => {
                Some(CqlType::Decimal)
            }

            // List types - find common element type
            (CqlType::List(elem_a), CqlType::List(elem_b)) => {
                elem_a.common_supertype(elem_b).map(CqlType::list)
            }

            // Interval types - find common point type
            (CqlType::Interval(point_a), CqlType::Interval(point_b)) => {
                point_a.common_supertype(point_b).map(CqlType::interval)
            }

            // Default to Any
            _ => Some(CqlType::Any),
        }
    }
}

impl fmt::Display for CqlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.qualified_name())
    }
}

impl Default for CqlType {
    fn default() -> Self {
        Self::Any
    }
}

/// Element of a tuple type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TupleTypeElement {
    /// Element name
    pub name: String,
    /// Element type
    pub element_type: CqlType,
}

impl TupleTypeElement {
    /// Create a new tuple type element
    pub fn new(name: impl Into<String>, element_type: CqlType) -> Self {
        Self {
            name: name.into(),
            element_type,
        }
    }
}

/// Type specifier for ELM representation
///
/// This is the serializable form of type information used in ELM output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TypeSpecifier {
    /// Named type specifier
    #[serde(rename = "NamedTypeSpecifier")]
    Named(NamedTypeSpecifier),
    /// List type specifier
    #[serde(rename = "ListTypeSpecifier")]
    List(ListTypeSpecifier),
    /// Interval type specifier
    #[serde(rename = "IntervalTypeSpecifier")]
    Interval(IntervalTypeSpecifier),
    /// Tuple type specifier
    #[serde(rename = "TupleTypeSpecifier")]
    Tuple(TupleTypeSpecifier),
    /// Choice type specifier
    #[serde(rename = "ChoiceTypeSpecifier")]
    Choice(ChoiceTypeSpecifier),
}

impl TypeSpecifier {
    /// Convert to CqlType
    pub fn to_cql_type(&self) -> CqlType {
        match self {
            Self::Named(n) => {
                // Check for system types
                if n.namespace.as_deref() == Some("System") || n.namespace.is_none() {
                    match n.name.as_str() {
                        "Any" => CqlType::Any,
                        "Boolean" => CqlType::Boolean,
                        "Integer" => CqlType::Integer,
                        "Long" => CqlType::Long,
                        "Decimal" => CqlType::Decimal,
                        "String" => CqlType::String,
                        "Date" => CqlType::Date,
                        "DateTime" => CqlType::DateTime,
                        "Time" => CqlType::Time,
                        "Quantity" => CqlType::Quantity,
                        "Ratio" => CqlType::Ratio,
                        "Code" => CqlType::Code,
                        "Concept" => CqlType::Concept,
                        "Vocabulary" => CqlType::Vocabulary,
                        _ => CqlType::Named {
                            namespace: n.namespace.clone(),
                            name: n.name.clone(),
                        },
                    }
                } else {
                    CqlType::Named {
                        namespace: n.namespace.clone(),
                        name: n.name.clone(),
                    }
                }
            }
            Self::List(l) => CqlType::list(l.element_type.to_cql_type()),
            Self::Interval(i) => CqlType::interval(i.point_type.to_cql_type()),
            Self::Tuple(t) => CqlType::tuple(
                t.elements
                    .iter()
                    .map(|e| TupleTypeElement {
                        name: e.name.clone(),
                        element_type: e.element_type.to_cql_type(),
                    })
                    .collect(),
            ),
            Self::Choice(c) => {
                CqlType::choice(c.types.iter().map(|t| t.to_cql_type()).collect())
            }
        }
    }

    /// Create from CqlType
    pub fn from_cql_type(cql_type: &CqlType) -> Self {
        match cql_type {
            CqlType::Any => Self::Named(NamedTypeSpecifier::system("Any")),
            CqlType::Boolean => Self::Named(NamedTypeSpecifier::system("Boolean")),
            CqlType::Integer => Self::Named(NamedTypeSpecifier::system("Integer")),
            CqlType::Long => Self::Named(NamedTypeSpecifier::system("Long")),
            CqlType::Decimal => Self::Named(NamedTypeSpecifier::system("Decimal")),
            CqlType::String => Self::Named(NamedTypeSpecifier::system("String")),
            CqlType::Date => Self::Named(NamedTypeSpecifier::system("Date")),
            CqlType::DateTime => Self::Named(NamedTypeSpecifier::system("DateTime")),
            CqlType::Time => Self::Named(NamedTypeSpecifier::system("Time")),
            CqlType::Quantity => Self::Named(NamedTypeSpecifier::system("Quantity")),
            CqlType::Ratio => Self::Named(NamedTypeSpecifier::system("Ratio")),
            CqlType::Code => Self::Named(NamedTypeSpecifier::system("Code")),
            CqlType::Concept => Self::Named(NamedTypeSpecifier::system("Concept")),
            CqlType::Vocabulary => Self::Named(NamedTypeSpecifier::system("Vocabulary")),
            CqlType::List(elem) => Self::List(ListTypeSpecifier {
                element_type: Box::new(Self::from_cql_type(elem)),
            }),
            CqlType::Interval(point) => Self::Interval(IntervalTypeSpecifier {
                point_type: Box::new(Self::from_cql_type(point)),
            }),
            CqlType::Tuple(elements) => Self::Tuple(TupleTypeSpecifier {
                elements: elements
                    .iter()
                    .map(|e| TupleElementDefinition {
                        name: e.name.clone(),
                        element_type: Self::from_cql_type(&e.element_type),
                    })
                    .collect(),
            }),
            CqlType::Choice(types) => Self::Choice(ChoiceTypeSpecifier {
                types: types.iter().map(Self::from_cql_type).collect(),
            }),
            CqlType::Named { namespace, name } => Self::Named(NamedTypeSpecifier {
                namespace: namespace.clone(),
                name: name.clone(),
            }),
        }
    }
}

/// Named type specifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedTypeSpecifier {
    /// Namespace (e.g., "System", "FHIR")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Type name
    pub name: String,
}

impl NamedTypeSpecifier {
    /// Create a new named type specifier
    pub fn new(namespace: Option<String>, name: impl Into<String>) -> Self {
        Self {
            namespace,
            name: name.into(),
        }
    }

    /// Create a system type specifier
    pub fn system(name: impl Into<String>) -> Self {
        Self {
            namespace: Some("System".to_string()),
            name: name.into(),
        }
    }

    /// Create a simple (unqualified) type specifier
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            namespace: None,
            name: name.into(),
        }
    }
}

/// List type specifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTypeSpecifier {
    /// Element type
    #[serde(rename = "elementType")]
    pub element_type: Box<TypeSpecifier>,
}

/// Interval type specifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntervalTypeSpecifier {
    /// Point type
    #[serde(rename = "pointType")]
    pub point_type: Box<TypeSpecifier>,
}

/// Tuple type specifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TupleTypeSpecifier {
    /// Tuple elements
    #[serde(rename = "element")]
    pub elements: Vec<TupleElementDefinition>,
}

/// Tuple element definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TupleElementDefinition {
    /// Element name
    pub name: String,
    /// Element type
    #[serde(rename = "elementType")]
    pub element_type: TypeSpecifier,
}

/// Choice type specifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceTypeSpecifier {
    /// Choice types
    #[serde(rename = "choice")]
    pub types: Vec<TypeSpecifier>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cql_type_properties() {
        assert!(CqlType::Any.is_any());
        assert!(CqlType::Integer.is_primitive());
        assert!(CqlType::Integer.is_numeric());
        assert!(CqlType::Date.is_temporal());
        assert!(CqlType::Code.is_clinical());
        assert!(CqlType::list(CqlType::Integer).is_collection());
    }

    #[test]
    fn test_cql_type_subtyping() {
        // Integer < Long < Decimal
        assert!(CqlType::Integer.is_subtype_of(&CqlType::Long));
        assert!(CqlType::Integer.is_subtype_of(&CqlType::Decimal));
        assert!(CqlType::Long.is_subtype_of(&CqlType::Decimal));

        // Any is supertype of all
        assert!(CqlType::Integer.is_subtype_of(&CqlType::Any));
        assert!(CqlType::String.is_subtype_of(&CqlType::Any));

        // List covariance
        let list_int = CqlType::list(CqlType::Integer);
        let list_decimal = CqlType::list(CqlType::Decimal);
        assert!(list_int.is_subtype_of(&list_decimal));
    }

    #[test]
    fn test_cql_type_common_supertype() {
        // Numeric types
        assert_eq!(
            CqlType::Integer.common_supertype(&CqlType::Long),
            Some(CqlType::Long)
        );
        assert_eq!(
            CqlType::Integer.common_supertype(&CqlType::Decimal),
            Some(CqlType::Decimal)
        );

        // Same type
        assert_eq!(
            CqlType::String.common_supertype(&CqlType::String),
            Some(CqlType::String)
        );
    }

    #[test]
    fn test_cql_type_qualified_name() {
        assert_eq!(CqlType::Integer.qualified_name(), "System.Integer");
        assert_eq!(
            CqlType::list(CqlType::String).qualified_name(),
            "List<System.String>"
        );
        assert_eq!(
            CqlType::interval(CqlType::Date).qualified_name(),
            "Interval<System.Date>"
        );
    }

    #[test]
    fn test_type_specifier_conversion() {
        let cql_type = CqlType::list(CqlType::Integer);
        let specifier = TypeSpecifier::from_cql_type(&cql_type);
        let converted = specifier.to_cql_type();
        assert_eq!(cql_type, converted);
    }
}
