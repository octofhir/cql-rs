//! Query expression AST nodes for CQL

use crate::{BoxExpr, Expression, Identifier, OptBoxExpr, Spanned, TypeSpecifier};

/// A query expression
#[derive(Debug, Clone)]
pub struct Query {
    /// Query sources
    pub sources: Vec<QuerySource>,
    /// Let clauses
    pub lets: Vec<LetClause>,
    /// Relationship clauses (with/without)
    pub relationships: Vec<RelationshipClause>,
    /// Where clause (filter)
    pub where_clause: OptBoxExpr,
    /// Return clause
    pub return_clause: Option<ReturnClause>,
    /// Aggregate clause
    pub aggregate_clause: Option<AggregateClause>,
    /// Sort clause
    pub sort_clause: Option<SortClause>,
}

impl Query {
    /// Create a new query with a single source
    pub fn new(source: QuerySource) -> Self {
        Self {
            sources: vec![source],
            lets: Vec::new(),
            relationships: Vec::new(),
            where_clause: None,
            return_clause: None,
            aggregate_clause: None,
            sort_clause: None,
        }
    }

    /// Create a new multi-source query
    pub fn multi(sources: Vec<QuerySource>) -> Self {
        Self {
            sources,
            lets: Vec::new(),
            relationships: Vec::new(),
            where_clause: None,
            return_clause: None,
            aggregate_clause: None,
            sort_clause: None,
        }
    }
}

/// A source in a query (from clause)
#[derive(Debug, Clone)]
pub struct QuerySource {
    /// The source expression (retrieve or other expression)
    pub expression: BoxExpr,
    /// Alias for the source
    pub alias: Identifier,
}

impl QuerySource {
    pub fn new(expression: BoxExpr, alias: impl Into<Identifier>) -> Self {
        Self {
            expression,
            alias: alias.into(),
        }
    }
}

/// Let clause for defining intermediate values
#[derive(Debug, Clone)]
pub struct LetClause {
    /// Variable name
    pub identifier: Identifier,
    /// Value expression
    pub expression: BoxExpr,
}

impl LetClause {
    pub fn new(identifier: impl Into<Identifier>, expression: BoxExpr) -> Self {
        Self {
            identifier: identifier.into(),
            expression,
        }
    }
}

/// Relationship clause (with/without)
#[derive(Debug, Clone)]
pub struct RelationshipClause {
    /// Type of relationship
    pub kind: RelationshipKind,
    /// Source for the relationship
    pub source: QuerySource,
    /// Such that condition
    pub such_that: OptBoxExpr,
}

/// Kind of relationship clause
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipKind {
    /// With clause (inclusion)
    With,
    /// Without clause (exclusion)
    Without,
}

/// Return clause
#[derive(Debug, Clone)]
pub struct ReturnClause {
    /// Whether to return distinct values
    pub distinct: bool,
    /// Whether to return all values (instead of distinct)
    pub all: bool,
    /// The return expression
    pub expression: BoxExpr,
}

impl ReturnClause {
    pub fn new(expression: BoxExpr) -> Self {
        Self {
            distinct: false,
            all: false,
            expression,
        }
    }

    pub fn distinct(expression: BoxExpr) -> Self {
        Self {
            distinct: true,
            all: false,
            expression,
        }
    }

    pub fn all(expression: BoxExpr) -> Self {
        Self {
            distinct: false,
            all: true,
            expression,
        }
    }
}

/// Aggregate clause
#[derive(Debug, Clone)]
pub struct AggregateClause {
    /// Whether distinct
    pub distinct: bool,
    /// Aggregate identifier
    pub identifier: Identifier,
    /// Starting value
    pub starting: OptBoxExpr,
    /// Let clauses within aggregate
    pub lets: Vec<LetClause>,
    /// Aggregate expression
    pub expression: BoxExpr,
}

/// Sort clause
#[derive(Debug, Clone)]
pub struct SortClause {
    /// Sort items
    pub items: Vec<SortItem>,
}

impl SortClause {
    pub fn new(items: Vec<SortItem>) -> Self {
        Self { items }
    }

    pub fn single(item: SortItem) -> Self {
        Self { items: vec![item] }
    }
}

/// Sort item
#[derive(Debug, Clone)]
pub struct SortItem {
    /// Sort expression (None for sorting by the query result itself)
    pub expression: OptBoxExpr,
    /// Sort direction
    pub direction: SortDirection,
}

impl SortItem {
    pub fn new(expression: OptBoxExpr, direction: SortDirection) -> Self {
        Self {
            expression,
            direction,
        }
    }

    pub fn ascending(expression: OptBoxExpr) -> Self {
        Self::new(expression, SortDirection::Ascending)
    }

    pub fn descending(expression: OptBoxExpr) -> Self {
        Self::new(expression, SortDirection::Descending)
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    /// Ascending order (default)
    #[default]
    Ascending,
    /// Ascending order (explicit)
    Asc,
    /// Descending order
    Descending,
    /// Descending order (explicit)
    Desc,
}

impl SortDirection {
    /// Check if this is ascending
    pub const fn is_ascending(&self) -> bool {
        matches!(self, Self::Ascending | Self::Asc)
    }

    /// Check if this is descending
    pub const fn is_descending(&self) -> bool {
        matches!(self, Self::Descending | Self::Desc)
    }
}

/// Retrieve expression for data retrieval
#[derive(Debug, Clone)]
pub struct Retrieve {
    /// Data type to retrieve (e.g., "Condition", "Observation")
    pub data_type: Spanned<TypeSpecifier>,
    /// Optional template id
    pub template_id: Option<String>,
    /// Code path for terminology filtering
    pub code_path: Option<String>,
    /// Code comparator
    pub code_comparator: Option<CodeComparator>,
    /// Codes to filter by
    pub codes: OptBoxExpr,
    /// Date path for temporal filtering
    pub date_path: Option<String>,
    /// Date range for temporal filtering
    pub date_range: OptBoxExpr,
    /// Context identifier (optional override)
    pub context: Option<Identifier>,
}

impl Retrieve {
    pub fn new(data_type: Spanned<TypeSpecifier>) -> Self {
        Self {
            data_type,
            template_id: None,
            code_path: None,
            code_comparator: None,
            codes: None,
            date_path: None,
            date_range: None,
            context: None,
        }
    }
}

/// Code comparator for retrieve expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeComparator {
    /// in (code is in valueset)
    In,
    /// = (exact match)
    Equal,
    /// ~ (equivalent)
    Equivalent,
}

impl CodeComparator {
    pub const fn symbol(&self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Equal => "=",
            Self::Equivalent => "~",
        }
    }
}
