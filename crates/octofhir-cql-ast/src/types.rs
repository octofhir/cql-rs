//! Type specifier AST nodes for CQL

use crate::{Identifier, QualifiedIdentifier};

/// Type specifier in CQL
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSpecifier {
    /// Named type (e.g., "Integer", "FHIR.Patient")
    Named(NamedTypeSpecifier),
    /// List type (e.g., "List<Integer>")
    List(ListTypeSpecifier),
    /// Interval type (e.g., "Interval<Integer>")
    Interval(IntervalTypeSpecifier),
    /// Tuple type (e.g., "Tuple { name String, age Integer }")
    Tuple(TupleTypeSpecifier),
    /// Choice type (e.g., "Choice<Integer, String>")
    Choice(ChoiceTypeSpecifier),
}

impl TypeSpecifier {
    /// Create a named type specifier
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(NamedTypeSpecifier::simple(name))
    }

    /// Create a qualified named type specifier
    pub fn qualified(qualifier: impl Into<String>, name: impl Into<String>) -> Self {
        Self::Named(NamedTypeSpecifier::qualified(qualifier, name))
    }

    /// Create a list type specifier
    pub fn list(element_type: TypeSpecifier) -> Self {
        Self::List(ListTypeSpecifier::new(element_type))
    }

    /// Create an interval type specifier
    pub fn interval(point_type: TypeSpecifier) -> Self {
        Self::Interval(IntervalTypeSpecifier::new(point_type))
    }
}

/// Named type specifier
#[derive(Debug, Clone, PartialEq)]
pub struct NamedTypeSpecifier {
    /// Optional namespace/model qualifier
    pub namespace: Option<String>,
    /// Type name
    pub name: String,
}

impl NamedTypeSpecifier {
    /// Create a simple named type
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            namespace: None,
            name: name.into(),
        }
    }

    /// Create a qualified named type
    pub fn qualified(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: Some(namespace.into()),
            name: name.into(),
        }
    }

    /// Get the full name including namespace
    pub fn full_name(&self) -> String {
        if let Some(ns) = &self.namespace {
            format!("{}.{}", ns, self.name)
        } else {
            self.name.clone()
        }
    }
}

impl From<&str> for NamedTypeSpecifier {
    fn from(s: &str) -> Self {
        if let Some((ns, name)) = s.split_once('.') {
            Self::qualified(ns, name)
        } else {
            Self::simple(s)
        }
    }
}

/// List type specifier
#[derive(Debug, Clone, PartialEq)]
pub struct ListTypeSpecifier {
    /// Element type
    pub element_type: Box<TypeSpecifier>,
}

impl ListTypeSpecifier {
    pub fn new(element_type: TypeSpecifier) -> Self {
        Self {
            element_type: Box::new(element_type),
        }
    }
}

/// Interval type specifier
#[derive(Debug, Clone, PartialEq)]
pub struct IntervalTypeSpecifier {
    /// Point type (must be ordered)
    pub point_type: Box<TypeSpecifier>,
}

impl IntervalTypeSpecifier {
    pub fn new(point_type: TypeSpecifier) -> Self {
        Self {
            point_type: Box::new(point_type),
        }
    }
}

/// Tuple type specifier
#[derive(Debug, Clone, PartialEq)]
pub struct TupleTypeSpecifier {
    /// Tuple elements
    pub elements: Vec<TupleElementDefinition>,
}

impl TupleTypeSpecifier {
    pub fn new(elements: Vec<TupleElementDefinition>) -> Self {
        Self { elements }
    }
}

/// Tuple element definition
#[derive(Debug, Clone, PartialEq)]
pub struct TupleElementDefinition {
    /// Element name
    pub name: Identifier,
    /// Element type (optional - can be inferred)
    pub element_type: Option<Box<TypeSpecifier>>,
}

impl TupleElementDefinition {
    pub fn new(name: impl Into<Identifier>, element_type: Option<TypeSpecifier>) -> Self {
        Self {
            name: name.into(),
            element_type: element_type.map(Box::new),
        }
    }
}

/// Choice type specifier (union of types)
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceTypeSpecifier {
    /// Choice types
    pub types: Vec<TypeSpecifier>,
}

impl ChoiceTypeSpecifier {
    pub fn new(types: Vec<TypeSpecifier>) -> Self {
        Self { types }
    }
}

// Common CQL types as constants
impl TypeSpecifier {
    /// System.Any type
    pub fn any() -> Self {
        Self::qualified("System", "Any")
    }

    /// System.Boolean type
    pub fn boolean() -> Self {
        Self::qualified("System", "Boolean")
    }

    /// System.Integer type
    pub fn integer() -> Self {
        Self::qualified("System", "Integer")
    }

    /// System.Long type
    pub fn long() -> Self {
        Self::qualified("System", "Long")
    }

    /// System.Decimal type
    pub fn decimal() -> Self {
        Self::qualified("System", "Decimal")
    }

    /// System.String type
    pub fn string() -> Self {
        Self::qualified("System", "String")
    }

    /// System.Date type
    pub fn date() -> Self {
        Self::qualified("System", "Date")
    }

    /// System.DateTime type
    pub fn datetime() -> Self {
        Self::qualified("System", "DateTime")
    }

    /// System.Time type
    pub fn time() -> Self {
        Self::qualified("System", "Time")
    }

    /// System.Quantity type
    pub fn quantity() -> Self {
        Self::qualified("System", "Quantity")
    }

    /// System.Ratio type
    pub fn ratio() -> Self {
        Self::qualified("System", "Ratio")
    }

    /// System.Code type
    pub fn code() -> Self {
        Self::qualified("System", "Code")
    }

    /// System.Concept type
    pub fn concept() -> Self {
        Self::qualified("System", "Concept")
    }

    /// System.Vocabulary type
    pub fn vocabulary() -> Self {
        Self::qualified("System", "Vocabulary")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_type_full_name() {
        let simple = NamedTypeSpecifier::simple("Integer");
        assert_eq!(simple.full_name(), "Integer");

        let qualified = NamedTypeSpecifier::qualified("FHIR", "Patient");
        assert_eq!(qualified.full_name(), "FHIR.Patient");
    }

    #[test]
    fn test_type_specifier_from_str() {
        let simple: NamedTypeSpecifier = "Integer".into();
        assert!(simple.namespace.is_none());
        assert_eq!(simple.name, "Integer");

        let qualified: NamedTypeSpecifier = "FHIR.Patient".into();
        assert_eq!(qualified.namespace, Some("FHIR".to_string()));
        assert_eq!(qualified.name, "Patient");
    }
}
