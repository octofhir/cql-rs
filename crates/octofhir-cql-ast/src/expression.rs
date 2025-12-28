//! Expression AST nodes for CQL
//!
//! This module defines all expression types in CQL, covering 50+ expression kinds.

use crate::{
    BinaryOp, BoxExpr, Identifier, IntervalOp, Literal, OptBoxExpr, QualifiedIdentifier, Query,
    Retrieve, Spanned, TemporalPrecision, TypeSpecifier, UnaryOp,
};

/// All CQL expression types
#[derive(Debug, Clone)]
pub enum Expression {
    // === Literals ===
    /// Literal value (null, boolean, number, string, date, etc.)
    Literal(Literal),

    // === Identifiers and References ===
    /// Identifier reference (variable, parameter, definition)
    IdentifierRef(IdentifierRef),
    /// Qualified identifier reference (Library.Definition)
    QualifiedIdentifierRef(QualifiedIdentifierRef),
    /// Property access (expr.property)
    Property(PropertyAccess),

    // === Operators ===
    /// Binary operation
    BinaryOp(BinaryOpExpr),
    /// Unary operation
    UnaryOp(UnaryOpExpr),
    /// Interval operation with precision
    IntervalOp(IntervalOpExpr),

    // === Type Operations ===
    /// Type cast (expr as Type)
    As(AsCastExpr),
    /// Type test (expr is Type)
    Is(IsTypeExpr),
    /// Convert expression
    Convert(ConvertExpr),
    /// Cast expression (explicit)
    Cast(CastExpr),
    /// Minimum value for type (minimum Integer, minimum DateTime, etc.)
    MinValue(MinMaxValueExpr),
    /// Maximum value for type (maximum Integer, maximum DateTime, etc.)
    MaxValue(MinMaxValueExpr),

    // === Conditionals ===
    /// If-then-else expression
    If(IfExpr),
    /// Case expression
    Case(CaseExpr),
    /// Coalesce expression (returns first non-null)
    Coalesce(CoalesceExpr),

    // === Nulls ===
    /// Is null test
    IsNull(IsNullExpr),
    /// Is false test
    IsFalse(IsFalseExpr),
    /// Is true test
    IsTrue(IsTrueExpr),

    // === Collections ===
    /// List literal
    List(ListExpr),
    /// Tuple literal
    Tuple(TupleExpr),
    /// Instance creation
    Instance(InstanceExpr),
    /// Indexer access (expr[index])
    Indexer(IndexerExpr),

    // === Intervals ===
    /// Interval literal
    Interval(IntervalExpr),
    /// Start of interval
    Start(StartExpr),
    /// End of interval
    End(EndExpr),
    /// Point from singleton interval
    PointFrom(PointFromExpr),
    /// Width of interval
    Width(WidthExpr),
    /// Size of interval
    Size(SizeExpr),

    // === Queries ===
    /// Query expression
    Query(Box<Query>),
    /// Retrieve expression
    Retrieve(Box<Retrieve>),

    // === Function Calls ===
    /// Function invocation
    FunctionRef(FunctionRefExpr),
    /// External function call
    ExternalFunctionRef(ExternalFunctionRefExpr),

    // === Aggregate Expressions ===
    /// Aggregate expression
    Aggregate(AggregateExpr),

    // === Date/Time ===
    /// Now expression (current DateTime)
    Now,
    /// Today expression (current Date)
    Today,
    /// TimeOfDay expression (current Time)
    TimeOfDay,
    /// Date expression
    Date(DateConstructorExpr),
    /// DateTime expression
    DateTime(DateTimeConstructorExpr),
    /// Time expression
    Time(TimeConstructorExpr),
    /// Duration between
    DurationBetween(DurationBetweenExpr),
    /// Difference between
    DifferenceBetween(DifferenceBetweenExpr),
    /// Date/time component extraction
    DateTimeComponent(DateTimeComponentExpr),

    // === String Operations ===
    /// Concatenate strings
    Concatenate(ConcatenateExpr),
    /// Combine strings with separator
    Combine(CombineExpr),
    /// Split string
    Split(SplitExpr),
    /// Matches regex
    Matches(MatchesExpr),
    /// Replace matches
    ReplaceMatches(ReplaceMatchesExpr),

    // === List Operations ===
    /// First element
    First(FirstExpr),
    /// Last element
    Last(LastExpr),
    /// Single element (singleton)
    Single(SingleExpr),
    /// Slice list
    Slice(SliceExpr),
    /// Index of element
    IndexOf(IndexOfExpr),

    // === Membership and Comparison ===
    /// Between expression (value between low and high)
    Between(BetweenExpr),

    // === Message ===
    /// Message expression (trace/log)
    Message(MessageExpr),

    // === Timing Expressions ===
    /// Same timing expression
    SameAs(SameAsExpr),
    /// Same or before
    SameOrBefore(SameOrBeforeExpr),
    /// Same or after
    SameOrAfter(SameOrAfterExpr),

    // === Total ===
    /// Total aggregate accumulator
    Total(TotalExpr),

    // === Iteration ===
    /// Iteration variable ($this)
    Iteration,
    /// Index variable ($index)
    Index,
    /// Total accumulator ($total)
    TotalRef,

    // === Error Recovery ===
    /// Error placeholder (for error recovery in analysis mode)
    Error,
}

// === Expression Components ===

/// Identifier reference
#[derive(Debug, Clone)]
pub struct IdentifierRef {
    /// The referenced identifier
    pub name: Identifier,
}

/// Qualified identifier reference
#[derive(Debug, Clone)]
pub struct QualifiedIdentifierRef {
    /// The qualified identifier
    pub name: QualifiedIdentifier,
}

/// Property access
#[derive(Debug, Clone)]
pub struct PropertyAccess {
    /// Source expression
    pub source: BoxExpr,
    /// Property name
    pub property: Identifier,
}

/// Binary operation expression
#[derive(Debug, Clone)]
pub struct BinaryOpExpr {
    /// Left operand
    pub left: BoxExpr,
    /// Operator
    pub op: BinaryOp,
    /// Right operand
    pub right: BoxExpr,
}

/// Unary operation expression
#[derive(Debug, Clone)]
pub struct UnaryOpExpr {
    /// Operator
    pub op: UnaryOp,
    /// Operand
    pub operand: BoxExpr,
}

/// Interval operation with optional precision
#[derive(Debug, Clone)]
pub struct IntervalOpExpr {
    /// Left operand
    pub left: BoxExpr,
    /// Operator
    pub op: IntervalOp,
    /// Right operand
    pub right: BoxExpr,
    /// Optional precision
    pub precision: Option<TemporalPrecision>,
}

/// As cast expression
#[derive(Debug, Clone)]
pub struct AsCastExpr {
    /// Expression to cast
    pub operand: BoxExpr,
    /// Target type
    pub as_type: Spanned<TypeSpecifier>,
    /// Whether to use strict casting
    pub strict: bool,
}

/// Is type test expression
#[derive(Debug, Clone)]
pub struct IsTypeExpr {
    /// Expression to test
    pub operand: BoxExpr,
    /// Type to test against
    pub is_type: Spanned<TypeSpecifier>,
}

/// Convert expression
#[derive(Debug, Clone)]
pub struct ConvertExpr {
    /// Expression to convert
    pub operand: BoxExpr,
    /// Target type
    pub to_type: Spanned<TypeSpecifier>,
}

/// Cast expression
#[derive(Debug, Clone)]
pub struct CastExpr {
    /// Expression to cast
    pub operand: BoxExpr,
    /// Target type
    pub as_type: Spanned<TypeSpecifier>,
}

/// MinValue/MaxValue expression
#[derive(Debug, Clone)]
pub struct MinMaxValueExpr {
    /// Target type name (e.g., "Integer", "DateTime")
    pub value_type: Identifier,
}

/// If expression
#[derive(Debug, Clone)]
pub struct IfExpr {
    /// Condition
    pub condition: BoxExpr,
    /// Then branch
    pub then_expr: BoxExpr,
    /// Else branch
    pub else_expr: BoxExpr,
}

/// Case expression
#[derive(Debug, Clone)]
pub struct CaseExpr {
    /// Comparand (for simple case)
    pub comparand: OptBoxExpr,
    /// Case items
    pub items: Vec<CaseItem>,
    /// Else expression
    pub else_expr: OptBoxExpr,
}

/// Case item (when-then pair)
#[derive(Debug, Clone)]
pub struct CaseItem {
    /// When condition
    pub when: BoxExpr,
    /// Then result
    pub then: BoxExpr,
}

/// Coalesce expression
#[derive(Debug, Clone)]
pub struct CoalesceExpr {
    /// Operands (returns first non-null)
    pub operands: Vec<Spanned<Expression>>,
}

/// Is null expression
#[derive(Debug, Clone)]
pub struct IsNullExpr {
    pub operand: BoxExpr,
}

/// Is false expression
#[derive(Debug, Clone)]
pub struct IsFalseExpr {
    pub operand: BoxExpr,
}

/// Is true expression
#[derive(Debug, Clone)]
pub struct IsTrueExpr {
    pub operand: BoxExpr,
}

/// List expression
#[derive(Debug, Clone)]
pub struct ListExpr {
    /// Optional element type
    pub element_type: Option<Spanned<TypeSpecifier>>,
    /// List elements
    pub elements: Vec<Spanned<Expression>>,
}

/// Tuple expression
#[derive(Debug, Clone)]
pub struct TupleExpr {
    /// Tuple elements
    pub elements: Vec<TupleElement>,
}

/// Tuple element
#[derive(Debug, Clone)]
pub struct TupleElement {
    /// Element name
    pub name: Identifier,
    /// Element value
    pub value: BoxExpr,
}

/// Instance creation expression
#[derive(Debug, Clone)]
pub struct InstanceExpr {
    /// Type to instantiate
    pub class_type: Spanned<TypeSpecifier>,
    /// Instance elements
    pub elements: Vec<InstanceElement>,
}

/// Instance element
#[derive(Debug, Clone)]
pub struct InstanceElement {
    /// Element name
    pub name: Identifier,
    /// Element value
    pub value: BoxExpr,
}

/// Indexer expression
#[derive(Debug, Clone)]
pub struct IndexerExpr {
    /// Source expression (list or string)
    pub source: BoxExpr,
    /// Index expression
    pub index: BoxExpr,
}

/// Interval expression
#[derive(Debug, Clone)]
pub struct IntervalExpr {
    /// Low bound
    pub low: OptBoxExpr,
    /// Whether low is closed (inclusive)
    pub low_closed: bool,
    /// High bound
    pub high: OptBoxExpr,
    /// Whether high is closed (inclusive)
    pub high_closed: bool,
}

/// Start of interval
#[derive(Debug, Clone)]
pub struct StartExpr {
    pub operand: BoxExpr,
}

/// End of interval
#[derive(Debug, Clone)]
pub struct EndExpr {
    pub operand: BoxExpr,
}

/// Point from singleton interval
#[derive(Debug, Clone)]
pub struct PointFromExpr {
    pub operand: BoxExpr,
}

/// Width of interval
#[derive(Debug, Clone)]
pub struct WidthExpr {
    pub operand: BoxExpr,
}

/// Size of interval
#[derive(Debug, Clone)]
pub struct SizeExpr {
    pub operand: BoxExpr,
}

/// Function reference expression
#[derive(Debug, Clone)]
pub struct FunctionRefExpr {
    /// Optional library qualifier
    pub library: Option<Identifier>,
    /// Function name
    pub name: Identifier,
    /// Function arguments
    pub arguments: Vec<Spanned<Expression>>,
}

/// External function reference
#[derive(Debug, Clone)]
pub struct ExternalFunctionRefExpr {
    /// External function name
    pub name: Identifier,
    /// Function arguments
    pub arguments: Vec<Spanned<Expression>>,
}

/// Aggregate expression
#[derive(Debug, Clone)]
pub struct AggregateExpr {
    /// Source expression
    pub source: BoxExpr,
    /// Iteration variable name
    pub iteration: Option<Identifier>,
    /// Aggregate function expression
    pub expression: BoxExpr,
    /// Starting value
    pub starting: OptBoxExpr,
}

/// Date constructor
#[derive(Debug, Clone)]
pub struct DateConstructorExpr {
    pub year: BoxExpr,
    pub month: OptBoxExpr,
    pub day: OptBoxExpr,
}

/// DateTime constructor
#[derive(Debug, Clone)]
pub struct DateTimeConstructorExpr {
    pub year: BoxExpr,
    pub month: OptBoxExpr,
    pub day: OptBoxExpr,
    pub hour: OptBoxExpr,
    pub minute: OptBoxExpr,
    pub second: OptBoxExpr,
    pub millisecond: OptBoxExpr,
    pub timezone_offset: OptBoxExpr,
}

/// Time constructor
#[derive(Debug, Clone)]
pub struct TimeConstructorExpr {
    pub hour: BoxExpr,
    pub minute: OptBoxExpr,
    pub second: OptBoxExpr,
    pub millisecond: OptBoxExpr,
}

/// Duration between expression
#[derive(Debug, Clone)]
pub struct DurationBetweenExpr {
    /// Precision for duration
    pub precision: TemporalPrecision,
    /// Start point
    pub low: BoxExpr,
    /// End point
    pub high: BoxExpr,
}

/// Difference between expression
#[derive(Debug, Clone)]
pub struct DifferenceBetweenExpr {
    /// Precision for difference
    pub precision: TemporalPrecision,
    /// Start point
    pub low: BoxExpr,
    /// End point
    pub high: BoxExpr,
}

/// Date/time component extraction
#[derive(Debug, Clone)]
pub struct DateTimeComponentExpr {
    /// Source date/time expression
    pub source: BoxExpr,
    /// Component to extract
    pub component: DateTimeComponent,
}

/// Date/time component types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimeComponent {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
    TimezoneOffset,
    Date,
    Time,
}

/// Concatenate expression
#[derive(Debug, Clone)]
pub struct ConcatenateExpr {
    pub operands: Vec<Spanned<Expression>>,
}

/// Combine expression
#[derive(Debug, Clone)]
pub struct CombineExpr {
    pub source: BoxExpr,
    pub separator: OptBoxExpr,
}

/// Split expression
#[derive(Debug, Clone)]
pub struct SplitExpr {
    pub source: BoxExpr,
    pub separator: BoxExpr,
}

/// Matches expression
#[derive(Debug, Clone)]
pub struct MatchesExpr {
    pub source: BoxExpr,
    pub pattern: BoxExpr,
}

/// Replace matches expression
#[derive(Debug, Clone)]
pub struct ReplaceMatchesExpr {
    pub source: BoxExpr,
    pub pattern: BoxExpr,
    pub replacement: BoxExpr,
}

/// First expression
#[derive(Debug, Clone)]
pub struct FirstExpr {
    pub source: BoxExpr,
}

/// Last expression
#[derive(Debug, Clone)]
pub struct LastExpr {
    pub source: BoxExpr,
}

/// Single expression
#[derive(Debug, Clone)]
pub struct SingleExpr {
    pub source: BoxExpr,
}

/// Slice expression
#[derive(Debug, Clone)]
pub struct SliceExpr {
    pub source: BoxExpr,
    pub start_index: BoxExpr,
    pub end_index: OptBoxExpr,
}

/// Index of expression
#[derive(Debug, Clone)]
pub struct IndexOfExpr {
    pub source: BoxExpr,
    pub element: BoxExpr,
}

/// Between expression
#[derive(Debug, Clone)]
pub struct BetweenExpr {
    pub operand: BoxExpr,
    pub low: BoxExpr,
    pub high: BoxExpr,
}

/// Message expression
#[derive(Debug, Clone)]
pub struct MessageExpr {
    pub source: BoxExpr,
    pub condition: BoxExpr,
    pub code: BoxExpr,
    pub severity: BoxExpr,
    pub message: BoxExpr,
}

/// Same as expression
#[derive(Debug, Clone)]
pub struct SameAsExpr {
    pub left: BoxExpr,
    pub right: BoxExpr,
    pub precision: Option<TemporalPrecision>,
}

/// Same or before expression
#[derive(Debug, Clone)]
pub struct SameOrBeforeExpr {
    pub left: BoxExpr,
    pub right: BoxExpr,
    pub precision: Option<TemporalPrecision>,
}

/// Same or after expression
#[derive(Debug, Clone)]
pub struct SameOrAfterExpr {
    pub left: BoxExpr,
    pub right: BoxExpr,
    pub precision: Option<TemporalPrecision>,
}

/// Total expression (aggregate accumulator)
#[derive(Debug, Clone)]
pub struct TotalExpr {
    pub result_type: Option<Spanned<TypeSpecifier>>,
}

// Helper constructors for Expression
impl Expression {
    /// Create a literal expression
    pub fn literal(lit: Literal) -> Self {
        Self::Literal(lit)
    }

    /// Create an identifier reference
    pub fn identifier(name: impl Into<Identifier>) -> Self {
        Self::IdentifierRef(IdentifierRef { name: name.into() })
    }

    /// Create a null literal
    pub fn null() -> Self {
        Self::Literal(Literal::Null)
    }

    /// Create a boolean literal
    pub fn boolean(value: bool) -> Self {
        Self::Literal(Literal::Boolean(value))
    }

    /// Create an integer literal
    pub fn integer(value: i32) -> Self {
        Self::Literal(Literal::Integer(value))
    }

    /// Create a string literal
    pub fn string(value: impl Into<String>) -> Self {
        Self::Literal(Literal::String(value.into()))
    }

    /// Create a binary operation
    pub fn binary(left: BoxExpr, op: BinaryOp, right: BoxExpr) -> Self {
        Self::BinaryOp(BinaryOpExpr { left, op, right })
    }

    /// Create a unary operation
    pub fn unary(op: UnaryOp, operand: BoxExpr) -> Self {
        Self::UnaryOp(UnaryOpExpr { op, operand })
    }
}
