//! CQL operators with precedence information

use serde::{Deserialize, Serialize};

/// Binary operators in CQL with their precedence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryOp {
    // Precedence 1 (lowest) - right-associative
    /// Logical implication (A implies B)
    Implies,

    // Precedence 2
    /// Logical or
    Or,
    /// Logical exclusive or
    Xor,

    // Precedence 3
    /// Logical and
    And,

    // Precedence 4
    /// Membership test (element in collection)
    In,
    /// Containment test (collection contains element)
    Contains,

    // Precedence 5
    /// Equality
    Equal,
    /// Inequality
    NotEqual,
    /// Equivalence (null-safe equality)
    Equivalent,
    /// Non-equivalence
    NotEquivalent,

    // Precedence 6
    /// Less than
    Less,
    /// Less than or equal
    LessOrEqual,
    /// Greater than
    Greater,
    /// Greater than or equal
    GreaterOrEqual,

    // Precedence 7
    /// Union of collections
    Union,

    // Precedence 8
    /// Type test (is)
    Is,
    /// Type cast (as)
    As,

    // Precedence 9
    /// Addition
    Add,
    /// Subtraction
    Subtract,
    /// String concatenation
    Concatenate,

    // Precedence 10
    /// Multiplication
    Multiply,
    /// Division
    Divide,
    /// Integer division (truncated)
    TruncatedDivide,
    /// Modulo
    Modulo,

    // Precedence 11 (highest for binary)
    /// Power/exponentiation
    Power,
}

impl BinaryOp {
    /// Get the precedence level (1-11, higher binds tighter)
    pub const fn precedence(&self) -> u8 {
        match self {
            Self::Implies => 1,
            Self::Or | Self::Xor => 2,
            Self::And => 3,
            Self::In | Self::Contains => 4,
            Self::Equal | Self::NotEqual | Self::Equivalent | Self::NotEquivalent => 5,
            Self::Less | Self::LessOrEqual | Self::Greater | Self::GreaterOrEqual => 6,
            Self::Union => 7,
            Self::Is | Self::As => 8,
            Self::Add | Self::Subtract | Self::Concatenate => 9,
            Self::Multiply | Self::Divide | Self::TruncatedDivide | Self::Modulo => 10,
            Self::Power => 11,
        }
    }

    /// Check if operator is right-associative
    pub const fn is_right_associative(&self) -> bool {
        matches!(self, Self::Implies | Self::Power)
    }

    /// Check if this is a comparison operator
    pub const fn is_comparison(&self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::Equivalent
                | Self::NotEquivalent
                | Self::Less
                | Self::LessOrEqual
                | Self::Greater
                | Self::GreaterOrEqual
        )
    }

    /// Check if this is a logical operator
    pub const fn is_logical(&self) -> bool {
        matches!(self, Self::And | Self::Or | Self::Xor | Self::Implies)
    }

    /// Check if this is an arithmetic operator
    pub const fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            Self::Add
                | Self::Subtract
                | Self::Multiply
                | Self::Divide
                | Self::TruncatedDivide
                | Self::Modulo
                | Self::Power
        )
    }

    /// Get the operator symbol
    pub const fn symbol(&self) -> &'static str {
        match self {
            Self::Implies => "implies",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::And => "and",
            Self::In => "in",
            Self::Contains => "contains",
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::Equivalent => "~",
            Self::NotEquivalent => "!~",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Union => "|",
            Self::Is => "is",
            Self::As => "as",
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Concatenate => "&",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::TruncatedDivide => "div",
            Self::Modulo => "mod",
            Self::Power => "^",
        }
    }
}

/// Unary operators in CQL (precedence 12, highest)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnaryOp {
    /// Logical not
    Not,
    /// Unary plus
    Plus,
    /// Unary minus (negation)
    Negate,
    /// Existence check
    Exists,
    /// Distinct elements
    Distinct,
    /// Flatten nested lists
    Flatten,
    /// Collapse intervals
    Collapse,
    /// Singleton from
    SingletonFrom,
}

impl UnaryOp {
    /// Get the precedence level (always 12 for unary)
    pub const fn precedence(&self) -> u8 {
        12
    }

    /// Get the operator symbol
    pub const fn symbol(&self) -> &'static str {
        match self {
            Self::Not => "not",
            Self::Plus => "+",
            Self::Negate => "-",
            Self::Exists => "exists",
            Self::Distinct => "distinct",
            Self::Flatten => "flatten",
            Self::Collapse => "collapse",
            Self::SingletonFrom => "singleton from",
        }
    }
}

/// Interval operators for specialized interval operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntervalOp {
    /// Properly includes (interval properly includes point/interval)
    ProperlyIncludes,
    /// Properly included in (point/interval is properly included in interval)
    ProperlyIncludedIn,
    /// Includes (interval includes point/interval)
    Includes,
    /// Included in (point/interval is included in interval)
    IncludedIn,
    /// Before (point/interval is before interval)
    Before,
    /// After (point/interval is after interval)
    After,
    /// Meets (intervals meet)
    Meets,
    /// Meets before (first interval meets second before)
    MeetsBefore,
    /// Meets after (first interval meets second after)
    MeetsAfter,
    /// Overlaps (intervals overlap)
    Overlaps,
    /// Overlaps before
    OverlapsBefore,
    /// Overlaps after
    OverlapsAfter,
    /// Starts (first interval starts second)
    Starts,
    /// Ends (first interval ends second)
    Ends,
    /// During (first interval is during second)
    During,
    /// Same as (intervals are the same)
    SameAs,
    /// Same or before
    SameOrBefore,
    /// Same or after
    SameOrAfter,
}

impl IntervalOp {
    /// Get the operator keyword
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::ProperlyIncludes => "properly includes",
            Self::ProperlyIncludedIn => "properly included in",
            Self::Includes => "includes",
            Self::IncludedIn => "included in",
            Self::Before => "before",
            Self::After => "after",
            Self::Meets => "meets",
            Self::MeetsBefore => "meets before",
            Self::MeetsAfter => "meets after",
            Self::Overlaps => "overlaps",
            Self::OverlapsBefore => "overlaps before",
            Self::OverlapsAfter => "overlaps after",
            Self::Starts => "starts",
            Self::Ends => "ends",
            Self::During => "during",
            Self::SameAs => "same as",
            Self::SameOrBefore => "same or before",
            Self::SameOrAfter => "same or after",
        }
    }
}

/// Date/time precision for temporal operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemporalPrecision {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

impl TemporalPrecision {
    /// Parse from keyword string
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "year" | "years" => Some(Self::Year),
            "month" | "months" => Some(Self::Month),
            "week" | "weeks" => Some(Self::Week),
            "day" | "days" => Some(Self::Day),
            "hour" | "hours" => Some(Self::Hour),
            "minute" | "minutes" => Some(Self::Minute),
            "second" | "seconds" => Some(Self::Second),
            "millisecond" | "milliseconds" => Some(Self::Millisecond),
            _ => None,
        }
    }

    /// Get the keyword
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::Year => "year",
            Self::Month => "month",
            Self::Week => "week",
            Self::Day => "day",
            Self::Hour => "hour",
            Self::Minute => "minute",
            Self::Second => "second",
            Self::Millisecond => "millisecond",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence_order() {
        assert!(BinaryOp::Power.precedence() > BinaryOp::Multiply.precedence());
        assert!(BinaryOp::Multiply.precedence() > BinaryOp::Add.precedence());
        assert!(BinaryOp::Add.precedence() > BinaryOp::Equal.precedence());
        assert!(BinaryOp::Equal.precedence() > BinaryOp::And.precedence());
        assert!(BinaryOp::And.precedence() > BinaryOp::Or.precedence());
        assert!(BinaryOp::Or.precedence() > BinaryOp::Implies.precedence());
    }

    #[test]
    fn test_right_associative() {
        assert!(BinaryOp::Implies.is_right_associative());
        assert!(BinaryOp::Power.is_right_associative());
        assert!(!BinaryOp::Add.is_right_associative());
    }
}
