//! CQL System types

use serde::{Deserialize, Serialize};

/// CQL System types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemType {
    /// Any type (top type)
    Any,
    /// Boolean type
    Boolean,
    /// Integer type (32-bit signed)
    Integer,
    /// Long type (64-bit signed)
    Long,
    /// Decimal type (arbitrary precision)
    Decimal,
    /// String type
    String,
    /// Date type
    Date,
    /// DateTime type
    DateTime,
    /// Time type
    Time,
    /// Quantity type
    Quantity,
    /// Ratio type
    Ratio,
    /// Code type
    Code,
    /// Concept type
    Concept,
    /// Vocabulary type (codesystem/valueset)
    Vocabulary,
}

impl SystemType {
    /// Get the full qualified name
    pub const fn qualified_name(&self) -> &'static str {
        match self {
            Self::Any => "System.Any",
            Self::Boolean => "System.Boolean",
            Self::Integer => "System.Integer",
            Self::Long => "System.Long",
            Self::Decimal => "System.Decimal",
            Self::String => "System.String",
            Self::Date => "System.Date",
            Self::DateTime => "System.DateTime",
            Self::Time => "System.Time",
            Self::Quantity => "System.Quantity",
            Self::Ratio => "System.Ratio",
            Self::Code => "System.Code",
            Self::Concept => "System.Concept",
            Self::Vocabulary => "System.Vocabulary",
        }
    }

    /// Get the simple name
    pub const fn name(&self) -> &'static str {
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
        }
    }

    /// Check if this type is numeric
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Integer | Self::Long | Self::Decimal)
    }

    /// Check if this type is temporal
    pub const fn is_temporal(&self) -> bool {
        matches!(self, Self::Date | Self::DateTime | Self::Time)
    }

    /// Check if this type is ordered (supports comparison)
    pub const fn is_ordered(&self) -> bool {
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
}
