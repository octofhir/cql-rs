//! CQL Type Coercion Rules
//!
//! This module implements the implicit and explicit type conversion rules
//! per the CQL 1.5 specification. It provides:
//! - Implicit promotion rules (e.g., Integer -> Long -> Decimal)
//! - Type compatibility checking
//! - Conversion validation

use crate::CqlType;
use thiserror::Error;

/// Coercion errors
#[derive(Debug, Clone, Error)]
pub enum CoercionError {
    /// Cannot convert between types
    #[error("Cannot convert from {from} to {to}")]
    CannotConvert { from: String, to: String },

    /// Implicit conversion not allowed
    #[error("Implicit conversion from {from} to {to} is not allowed")]
    ImplicitNotAllowed { from: String, to: String },

    /// Precision loss would occur
    #[error("Conversion from {from} to {to} may cause precision loss")]
    PrecisionLoss { from: String, to: String },
}

/// Type coercion result
pub type CoercionResult<T> = Result<T, CoercionError>;

/// Type coercion rules per CQL 1.5 specification
///
/// CQL defines specific implicit and explicit conversion paths between types.
#[derive(Debug, Clone, Default)]
pub struct TypeCoercer;

impl TypeCoercer {
    /// Create a new type coercer
    pub fn new() -> Self {
        Self
    }

    /// Check if implicit conversion from `from` to `to` is allowed
    ///
    /// Per CQL spec, the following implicit conversions are allowed:
    /// - Integer -> Long
    /// - Integer -> Decimal
    /// - Long -> Decimal
    /// - Any null -> any type
    /// - Subtype to supertype
    /// - Code -> Concept (implicit promotion)
    /// - Date -> DateTime (at day precision)
    pub fn can_implicitly_convert(&self, from: &CqlType, to: &CqlType) -> bool {
        // Same type
        if from == to {
            return true;
        }

        // Any is compatible with everything
        if matches!(from, CqlType::Any) || matches!(to, CqlType::Any) {
            return true;
        }

        // Numeric promotions
        match (from, to) {
            // Integer can promote to Long or Decimal
            (CqlType::Integer, CqlType::Long) => true,
            (CqlType::Integer, CqlType::Decimal) => true,
            // Long can promote to Decimal
            (CqlType::Long, CqlType::Decimal) => true,

            // Code to Concept promotion
            (CqlType::Code, CqlType::Concept) => true,

            // Date to DateTime (partial precision)
            (CqlType::Date, CqlType::DateTime) => true,

            // List covariance
            (CqlType::List(elem_from), CqlType::List(elem_to)) => {
                self.can_implicitly_convert(elem_from, elem_to)
            }

            // Interval covariance
            (CqlType::Interval(point_from), CqlType::Interval(point_to)) => {
                self.can_implicitly_convert(point_from, point_to)
            }

            // Choice type - from matches any choice alternative
            (_, CqlType::Choice(choices)) => {
                choices.iter().any(|c| self.can_implicitly_convert(from, c))
            }

            // From choice - any alternative can convert to target
            (CqlType::Choice(choices), _) => {
                choices.iter().any(|c| self.can_implicitly_convert(c, to))
            }

            // Tuple subtyping (structural)
            (CqlType::Tuple(from_elems), CqlType::Tuple(to_elems)) => {
                // Target must have all elements of source with compatible types
                to_elems.iter().all(|to_elem| {
                    from_elems.iter().any(|from_elem| {
                        from_elem.name == to_elem.name
                            && self.can_implicitly_convert(
                                &from_elem.element_type,
                                &to_elem.element_type,
                            )
                    })
                })
            }

            _ => false,
        }
    }

    /// Check if explicit conversion from `from` to `to` is allowed
    ///
    /// Explicit conversions include all implicit conversions plus:
    /// - Any numeric to any numeric (with possible precision loss)
    /// - String to numeric types
    /// - Numeric to String
    /// - Date/Time conversions
    /// - Any to String (ToString)
    pub fn can_explicitly_convert(&self, from: &CqlType, to: &CqlType) -> bool {
        // All implicit conversions are valid explicit conversions
        if self.can_implicitly_convert(from, to) {
            return true;
        }

        match (from, to) {
            // Numeric conversions (may lose precision)
            (CqlType::Decimal, CqlType::Integer) => true,
            (CqlType::Decimal, CqlType::Long) => true,
            (CqlType::Long, CqlType::Integer) => true,

            // String to numeric
            (CqlType::String, CqlType::Boolean) => true,
            (CqlType::String, CqlType::Integer) => true,
            (CqlType::String, CqlType::Long) => true,
            (CqlType::String, CqlType::Decimal) => true,
            (CqlType::String, CqlType::Date) => true,
            (CqlType::String, CqlType::DateTime) => true,
            (CqlType::String, CqlType::Time) => true,
            (CqlType::String, CqlType::Quantity) => true,
            (CqlType::String, CqlType::Ratio) => true,

            // Numeric/type to String
            (CqlType::Boolean, CqlType::String) => true,
            (CqlType::Integer, CqlType::String) => true,
            (CqlType::Long, CqlType::String) => true,
            (CqlType::Decimal, CqlType::String) => true,
            (CqlType::Date, CqlType::String) => true,
            (CqlType::DateTime, CqlType::String) => true,
            (CqlType::Time, CqlType::String) => true,
            (CqlType::Quantity, CqlType::String) => true,
            (CqlType::Ratio, CqlType::String) => true,
            (CqlType::Code, CqlType::String) => true,
            (CqlType::Concept, CqlType::String) => true,

            // DateTime to Date (extract date portion)
            (CqlType::DateTime, CqlType::Date) => true,

            // DateTime to Time (extract time portion)
            (CqlType::DateTime, CqlType::Time) => true,

            // Integer to Boolean (0 = false, non-zero = true)
            (CqlType::Integer, CqlType::Boolean) => true,

            // Boolean to Integer (false = 0, true = 1)
            (CqlType::Boolean, CqlType::Integer) => true,

            // Quantity value extraction
            (CqlType::Quantity, CqlType::Decimal) => true,

            // List element conversion
            (CqlType::List(elem_from), CqlType::List(elem_to)) => {
                self.can_explicitly_convert(elem_from, elem_to)
            }

            // Interval point conversion
            (CqlType::Interval(point_from), CqlType::Interval(point_to)) => {
                self.can_explicitly_convert(point_from, point_to)
            }

            _ => false,
        }
    }

    /// Get the implicit promotion path from one type to another
    ///
    /// Returns the sequence of types involved in the promotion.
    pub fn get_promotion_path(&self, from: &CqlType, to: &CqlType) -> Option<Vec<CqlType>> {
        if from == to {
            return Some(vec![from.clone()]);
        }

        match (from, to) {
            (CqlType::Integer, CqlType::Long) => {
                Some(vec![CqlType::Integer, CqlType::Long])
            }
            (CqlType::Integer, CqlType::Decimal) => {
                Some(vec![CqlType::Integer, CqlType::Long, CqlType::Decimal])
            }
            (CqlType::Long, CqlType::Decimal) => {
                Some(vec![CqlType::Long, CqlType::Decimal])
            }
            (CqlType::Code, CqlType::Concept) => {
                Some(vec![CqlType::Code, CqlType::Concept])
            }
            (CqlType::Date, CqlType::DateTime) => {
                Some(vec![CqlType::Date, CqlType::DateTime])
            }
            _ => None,
        }
    }

    /// Calculate the conversion cost for implicit conversion
    ///
    /// Lower cost means more preferred conversion. Returns None if conversion is not possible.
    /// Used for function overload resolution.
    pub fn conversion_cost(&self, from: &CqlType, to: &CqlType) -> Option<u32> {
        if from == to {
            return Some(0);
        }

        if !self.can_implicitly_convert(from, to) {
            return None;
        }

        match (from, to) {
            // Numeric promotions have low cost
            (CqlType::Integer, CqlType::Long) => Some(1),
            (CqlType::Integer, CqlType::Decimal) => Some(2),
            (CqlType::Long, CqlType::Decimal) => Some(1),

            // Any type has higher cost
            (CqlType::Any, _) | (_, CqlType::Any) => Some(100),

            // Code to Concept
            (CqlType::Code, CqlType::Concept) => Some(10),

            // Date to DateTime
            (CqlType::Date, CqlType::DateTime) => Some(5),

            // List/Interval covariance - cost depends on element type
            (CqlType::List(elem_from), CqlType::List(elem_to)) => {
                self.conversion_cost(elem_from, elem_to).map(|c| c + 1)
            }
            (CqlType::Interval(point_from), CqlType::Interval(point_to)) => {
                self.conversion_cost(point_from, point_to).map(|c| c + 1)
            }

            // Default cost for other valid conversions
            _ => Some(50),
        }
    }

    /// Find the best common type for a list of types
    ///
    /// Used for type inference in list literals, case expressions, etc.
    pub fn find_common_type(&self, types: &[CqlType]) -> Option<CqlType> {
        if types.is_empty() {
            return Some(CqlType::Any);
        }

        let mut result = types[0].clone();
        for ty in types.iter().skip(1) {
            result = self.find_common_pair(&result, ty)?;
        }
        Some(result)
    }

    /// Find the common type for two types
    fn find_common_pair(&self, a: &CqlType, b: &CqlType) -> Option<CqlType> {
        if a == b {
            return Some(a.clone());
        }

        if self.can_implicitly_convert(a, b) {
            return Some(b.clone());
        }

        if self.can_implicitly_convert(b, a) {
            return Some(a.clone());
        }

        // Find common supertype for numerics
        match (a, b) {
            (CqlType::Integer, CqlType::Long) | (CqlType::Long, CqlType::Integer) => {
                Some(CqlType::Long)
            }
            (CqlType::Integer, CqlType::Decimal)
            | (CqlType::Decimal, CqlType::Integer)
            | (CqlType::Long, CqlType::Decimal)
            | (CqlType::Decimal, CqlType::Long) => Some(CqlType::Decimal),

            // List common type
            (CqlType::List(elem_a), CqlType::List(elem_b)) => {
                self.find_common_pair(elem_a, elem_b).map(CqlType::list)
            }

            // Interval common type
            (CqlType::Interval(point_a), CqlType::Interval(point_b)) => {
                self.find_common_pair(point_a, point_b).map(CqlType::interval)
            }

            // Date and DateTime -> DateTime
            (CqlType::Date, CqlType::DateTime) | (CqlType::DateTime, CqlType::Date) => {
                Some(CqlType::DateTime)
            }

            // Code and Concept -> Concept
            (CqlType::Code, CqlType::Concept) | (CqlType::Concept, CqlType::Code) => {
                Some(CqlType::Concept)
            }

            // Fall back to Any
            _ => Some(CqlType::Any),
        }
    }

    /// Validate that a value can be safely converted
    ///
    /// Some conversions may fail at runtime (e.g., string "abc" to Integer).
    /// This method checks compile-time convertibility.
    pub fn validate_conversion(
        &self,
        from: &CqlType,
        to: &CqlType,
        explicit: bool,
    ) -> CoercionResult<()> {
        if explicit {
            if self.can_explicitly_convert(from, to) {
                Ok(())
            } else {
                Err(CoercionError::CannotConvert {
                    from: from.qualified_name(),
                    to: to.qualified_name(),
                })
            }
        } else if self.can_implicitly_convert(from, to) {
            Ok(())
        } else {
            Err(CoercionError::ImplicitNotAllowed {
                from: from.qualified_name(),
                to: to.qualified_name(),
            })
        }
    }

    /// Check if conversion may lose precision
    pub fn may_lose_precision(&self, from: &CqlType, to: &CqlType) -> bool {
        matches!(
            (from, to),
            (CqlType::Decimal, CqlType::Integer)
                | (CqlType::Decimal, CqlType::Long)
                | (CqlType::Long, CqlType::Integer)
                | (CqlType::DateTime, CqlType::Date)
                | (CqlType::DateTime, CqlType::Time)
        )
    }
}

/// Implicit conversion categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionCategory {
    /// No conversion needed
    Identity,
    /// Numeric widening (Integer -> Long -> Decimal)
    NumericPromotion,
    /// Type category promotion (Code -> Concept)
    TypePromotion,
    /// Temporal promotion (Date -> DateTime)
    TemporalPromotion,
    /// Collection element conversion
    CollectionCovariance,
    /// Choice type selection
    ChoiceSelection,
    /// Not convertible
    NotConvertible,
}

impl TypeCoercer {
    /// Categorize the conversion between two types
    pub fn categorize_conversion(&self, from: &CqlType, to: &CqlType) -> ConversionCategory {
        if from == to {
            return ConversionCategory::Identity;
        }

        match (from, to) {
            (CqlType::Integer, CqlType::Long)
            | (CqlType::Integer, CqlType::Decimal)
            | (CqlType::Long, CqlType::Decimal) => ConversionCategory::NumericPromotion,

            (CqlType::Code, CqlType::Concept) => ConversionCategory::TypePromotion,

            (CqlType::Date, CqlType::DateTime) => ConversionCategory::TemporalPromotion,

            (CqlType::List(_), CqlType::List(_))
            | (CqlType::Interval(_), CqlType::Interval(_)) => {
                ConversionCategory::CollectionCovariance
            }

            (_, CqlType::Choice(_)) => ConversionCategory::ChoiceSelection,

            _ if self.can_implicitly_convert(from, to) => ConversionCategory::TypePromotion,

            _ => ConversionCategory::NotConvertible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implicit_numeric_promotion() {
        let coercer = TypeCoercer::new();

        // Integer -> Long
        assert!(coercer.can_implicitly_convert(&CqlType::Integer, &CqlType::Long));

        // Integer -> Decimal
        assert!(coercer.can_implicitly_convert(&CqlType::Integer, &CqlType::Decimal));

        // Long -> Decimal
        assert!(coercer.can_implicitly_convert(&CqlType::Long, &CqlType::Decimal));

        // Reverse should not be implicit
        assert!(!coercer.can_implicitly_convert(&CqlType::Long, &CqlType::Integer));
        assert!(!coercer.can_implicitly_convert(&CqlType::Decimal, &CqlType::Long));
    }

    #[test]
    fn test_explicit_conversions() {
        let coercer = TypeCoercer::new();

        // Decimal -> Integer (explicit only)
        assert!(!coercer.can_implicitly_convert(&CqlType::Decimal, &CqlType::Integer));
        assert!(coercer.can_explicitly_convert(&CqlType::Decimal, &CqlType::Integer));

        // String -> Integer
        assert!(!coercer.can_implicitly_convert(&CqlType::String, &CqlType::Integer));
        assert!(coercer.can_explicitly_convert(&CqlType::String, &CqlType::Integer));
    }

    #[test]
    fn test_code_to_concept() {
        let coercer = TypeCoercer::new();

        assert!(coercer.can_implicitly_convert(&CqlType::Code, &CqlType::Concept));
        assert!(!coercer.can_implicitly_convert(&CqlType::Concept, &CqlType::Code));
    }

    #[test]
    fn test_list_covariance() {
        let coercer = TypeCoercer::new();

        let list_int = CqlType::list(CqlType::Integer);
        let list_decimal = CqlType::list(CqlType::Decimal);

        assert!(coercer.can_implicitly_convert(&list_int, &list_decimal));
        assert!(!coercer.can_implicitly_convert(&list_decimal, &list_int));
    }

    #[test]
    fn test_conversion_cost() {
        let coercer = TypeCoercer::new();

        // Same type = 0 cost
        assert_eq!(
            coercer.conversion_cost(&CqlType::Integer, &CqlType::Integer),
            Some(0)
        );

        // Integer -> Long = low cost
        assert_eq!(
            coercer.conversion_cost(&CqlType::Integer, &CqlType::Long),
            Some(1)
        );

        // Integer -> Decimal = slightly higher cost
        assert_eq!(
            coercer.conversion_cost(&CqlType::Integer, &CqlType::Decimal),
            Some(2)
        );

        // Incompatible = None
        assert_eq!(
            coercer.conversion_cost(&CqlType::String, &CqlType::Integer),
            None
        );
    }

    #[test]
    fn test_find_common_type() {
        let coercer = TypeCoercer::new();

        // Integer and Long -> Long
        let common = coercer.find_common_type(&[CqlType::Integer, CqlType::Long]);
        assert_eq!(common, Some(CqlType::Long));

        // Integer, Long, Decimal -> Decimal
        let common =
            coercer.find_common_type(&[CqlType::Integer, CqlType::Long, CqlType::Decimal]);
        assert_eq!(common, Some(CqlType::Decimal));

        // Empty -> Any
        let common = coercer.find_common_type(&[]);
        assert_eq!(common, Some(CqlType::Any));
    }

    #[test]
    fn test_precision_loss() {
        let coercer = TypeCoercer::new();

        assert!(coercer.may_lose_precision(&CqlType::Decimal, &CqlType::Integer));
        assert!(coercer.may_lose_precision(&CqlType::Long, &CqlType::Integer));
        assert!(!coercer.may_lose_precision(&CqlType::Integer, &CqlType::Long));
    }
}
