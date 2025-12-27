//! ELM (Expression Logical Model) structures per HL7 ELM specification
//!
//! This module defines all ELM types for representing compiled CQL as a
//! portable, executable representation. The types match the HL7 ELM schema.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============================================================================
// Library Structure
// ============================================================================

/// ELM Library - the root element containing a compiled CQL library
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    /// Library identifier
    pub identifier: VersionedIdentifier,
    /// Schema identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_identifier: Option<VersionedIdentifier>,
    /// Using definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usings: Option<UsingDefs>,
    /// Include definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub includes: Option<IncludeDefs>,
    /// Parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ParameterDefs>,
    /// Code systems
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_systems: Option<CodeSystemDefs>,
    /// Value sets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_sets: Option<ValueSetDefs>,
    /// Codes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codes: Option<CodeDefs>,
    /// Concepts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concepts: Option<ConceptDefs>,
    /// Contexts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<ContextDefs>,
    /// Statements (expression and function definitions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statements: Option<Statements>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

impl Library {
    /// Create a new library
    pub fn new(id: impl Into<String>, version: Option<impl Into<String>>) -> Self {
        Self {
            identifier: VersionedIdentifier {
                id: id.into(),
                system: None,
                version: version.map(Into::into),
            },
            schema_identifier: Some(VersionedIdentifier {
                id: "urn:hl7-org:elm".to_string(),
                system: None,
                version: Some("r1".to_string()),
            }),
            usings: None,
            includes: None,
            parameters: None,
            code_systems: None,
            value_sets: None,
            codes: None,
            concepts: None,
            contexts: None,
            statements: None,
            annotation: None,
        }
    }
}

/// Versioned identifier for libraries and schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedIdentifier {
    /// Identifier
    pub id: String,
    /// System/namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// ============================================================================
// Definition Containers
// ============================================================================

/// Container for using definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsingDefs {
    #[serde(rename = "def")]
    pub defs: Vec<UsingDef>,
}

/// Container for include definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeDefs {
    #[serde(rename = "def")]
    pub defs: Vec<IncludeDef>,
}

/// Container for parameter definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefs {
    #[serde(rename = "def")]
    pub defs: Vec<ParameterDef>,
}

/// Container for code system definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSystemDefs {
    #[serde(rename = "def")]
    pub defs: Vec<CodeSystemDef>,
}

/// Container for value set definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSetDefs {
    #[serde(rename = "def")]
    pub defs: Vec<ValueSetDef>,
}

/// Container for code definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDefs {
    #[serde(rename = "def")]
    pub defs: Vec<CodeDef>,
}

/// Container for concept definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptDefs {
    #[serde(rename = "def")]
    pub defs: Vec<ConceptDef>,
}

/// Container for context definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDefs {
    #[serde(rename = "def")]
    pub defs: Vec<ContextDef>,
}

/// Container for statements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statements {
    #[serde(rename = "def")]
    pub defs: Vec<ExpressionDef>,
}

// ============================================================================
// Definitions
// ============================================================================

/// Using definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsingDef {
    /// Local identifier
    pub local_identifier: String,
    /// Model URI
    pub uri: String,
    /// Model version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Include definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncludeDef {
    /// Local identifier
    pub local_identifier: String,
    /// Library path
    pub path: String,
    /// Library version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterDef {
    /// Parameter name
    pub name: String,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_level: Option<AccessModifier>,
    /// Parameter type specifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_type_specifier: Option<TypeSpecifier>,
    /// Default value expression
    #[serde(rename = "default", skip_serializing_if = "Option::is_none")]
    pub default_expr: Option<Box<Expression>>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Code system definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSystemDef {
    /// Name
    pub name: String,
    /// Code system ID/URI
    pub id: String,
    /// Version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_level: Option<AccessModifier>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Value set definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetDef {
    /// Name
    pub name: String,
    /// Value set ID/URI
    pub id: String,
    /// Version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_level: Option<AccessModifier>,
    /// Code systems
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_system: Option<Vec<CodeSystemRef>>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Code definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDef {
    /// Name
    pub name: String,
    /// Code value
    pub id: String,
    /// Display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_level: Option<AccessModifier>,
    /// Code system reference
    pub code_system: CodeSystemRef,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Concept definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptDef {
    /// Name
    pub name: String,
    /// Display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_level: Option<AccessModifier>,
    /// Code references
    pub code: Vec<CodeRef>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Context definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDef {
    /// Context name
    pub name: String,
}

/// Expression definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionDef {
    /// Name
    pub name: String,
    /// Context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_level: Option<AccessModifier>,
    /// Expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<Box<Expression>>,
    /// Result type specifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type_specifier: Option<TypeSpecifier>,
    /// Annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<Vec<Annotation>>,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDef {
    /// Name
    pub name: String,
    /// Context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_level: Option<AccessModifier>,
    /// Whether fluent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fluent: Option<bool>,
    /// Whether external
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    /// Operands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operand: Option<Vec<OperandDef>>,
    /// Return type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type_specifier: Option<TypeSpecifier>,
    /// Expression (body)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<Box<Expression>>,
}

/// Operand definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperandDef {
    /// Name
    pub name: String,
    /// Operand type specifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operand_type_specifier: Option<TypeSpecifier>,
}

/// Access modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AccessModifier {
    Public,
    Private,
}

// ============================================================================
// Type Specifiers
// ============================================================================

/// Type specifier
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TypeSpecifier {
    /// Named type
    #[serde(rename = "NamedTypeSpecifier")]
    Named(NamedTypeSpecifier),
    /// List type
    #[serde(rename = "ListTypeSpecifier")]
    List(ListTypeSpecifier),
    /// Interval type
    #[serde(rename = "IntervalTypeSpecifier")]
    Interval(IntervalTypeSpecifier),
    /// Tuple type
    #[serde(rename = "TupleTypeSpecifier")]
    Tuple(TupleTypeSpecifier),
    /// Choice type
    #[serde(rename = "ChoiceTypeSpecifier")]
    Choice(ChoiceTypeSpecifier),
}

/// Named type specifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedTypeSpecifier {
    /// Namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Type name
    pub name: String,
}

impl NamedTypeSpecifier {
    /// Create a system type
    pub fn system(name: impl Into<String>) -> Self {
        Self {
            namespace: Some("System".to_string()),
            name: name.into(),
        }
    }

    /// Create a FHIR type
    pub fn fhir(name: impl Into<String>) -> Self {
        Self {
            namespace: Some("FHIR".to_string()),
            name: name.into(),
        }
    }
}

/// List type specifier
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTypeSpecifier {
    /// Element type
    pub element_type: Box<TypeSpecifier>,
}

/// Interval type specifier
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntervalTypeSpecifier {
    /// Point type
    pub point_type: Box<TypeSpecifier>,
}

/// Tuple type specifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TupleTypeSpecifier {
    /// Elements
    pub element: Vec<TupleElementDefinition>,
}

/// Tuple element definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TupleElementDefinition {
    /// Name
    pub name: String,
    /// Element type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<Box<TypeSpecifier>>,
}

/// Choice type specifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceTypeSpecifier {
    /// Choice types
    pub choice: Vec<TypeSpecifier>,
}

// ============================================================================
// Base Element
// ============================================================================

/// Base element for all ELM nodes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// Locator (source position)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    /// Result type name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type_name: Option<String>,
    /// Result type specifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type_specifier: Option<TypeSpecifier>,
}

// ============================================================================
// Expressions - The main ELM expression types (~150 types)
// ============================================================================

/// The main Expression enum containing all ELM expression types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Expression {
    // === Literals ===
    Null(NullLiteral),
    Literal(Literal),

    // === References ===
    ExpressionRef(ExpressionRef),
    FunctionRef(FunctionRef),
    ParameterRef(ParameterRef),
    ValueSetRef(ValueSetRef),
    CodeSystemRef(CodeSystemRef),
    CodeRef(CodeRef),
    ConceptRef(ConceptRef),
    OperandRef(OperandRef),
    AliasRef(AliasRef),
    QueryLetRef(QueryLetRef),
    IdentifierRef(IdentifierRef),
    Property(Property),

    // === Arithmetic ===
    Add(BinaryExpression),
    Subtract(BinaryExpression),
    Multiply(BinaryExpression),
    Divide(BinaryExpression),
    TruncatedDivide(BinaryExpression),
    Modulo(BinaryExpression),
    Ceiling(UnaryExpression),
    Floor(UnaryExpression),
    Truncate(UnaryExpression),
    Abs(UnaryExpression),
    Negate(UnaryExpression),
    Round(RoundExpression),
    Ln(UnaryExpression),
    Exp(UnaryExpression),
    Log(BinaryExpression),
    Power(BinaryExpression),
    Successor(UnaryExpression),
    Predecessor(UnaryExpression),
    MinValue(MinMaxValueExpression),
    MaxValue(MinMaxValueExpression),
    Precision(UnaryExpression),
    LowBoundary(BoundaryExpression),
    HighBoundary(BoundaryExpression),

    // === Comparison ===
    Equal(BinaryExpression),
    Equivalent(BinaryExpression),
    NotEqual(BinaryExpression),
    Less(BinaryExpression),
    Greater(BinaryExpression),
    LessOrEqual(BinaryExpression),
    GreaterOrEqual(BinaryExpression),

    // === Logical ===
    And(BinaryExpression),
    Or(BinaryExpression),
    Xor(BinaryExpression),
    Implies(BinaryExpression),
    Not(UnaryExpression),

    // === Nullological ===
    IsNull(UnaryExpression),
    IsTrue(UnaryExpression),
    IsFalse(UnaryExpression),
    Coalesce(NaryExpression),
    If(IfExpression),
    Case(CaseExpression),

    // === String ===
    Concatenate(NaryExpression),
    Combine(CombineExpression),
    Split(SplitExpression),
    SplitOnMatches(SplitOnMatchesExpression),
    Length(UnaryExpression),
    Upper(UnaryExpression),
    Lower(UnaryExpression),
    Indexer(BinaryExpression),
    PositionOf(PositionOfExpression),
    LastPositionOf(LastPositionOfExpression),
    Substring(SubstringExpression),
    StartsWith(BinaryExpression),
    EndsWith(BinaryExpression),
    Matches(BinaryExpression),
    ReplaceMatches(TernaryExpression),

    // === DateTime ===
    Now(NowExpression),
    Today(TodayExpression),
    TimeOfDay(TimeOfDayExpression),
    Date(DateExpression),
    DateTime(DateTimeExpression),
    Time(TimeExpression),
    DateFrom(UnaryExpression),
    TimeFrom(UnaryExpression),
    TimezoneFrom(UnaryExpression),
    TimezoneOffsetFrom(UnaryExpression),
    DateTimeComponentFrom(DateTimeComponentFromExpression),
    DurationBetween(DurationBetweenExpression),
    DifferenceBetween(DifferenceBetweenExpression),
    SameAs(SameAsExpression),
    SameOrBefore(SameOrBeforeExpression),
    SameOrAfter(SameOrAfterExpression),

    // === Interval ===
    Interval(IntervalExpression),
    Start(UnaryExpression),
    End(UnaryExpression),
    PointFrom(UnaryExpression),
    Width(UnaryExpression),
    Size(UnaryExpression),
    Contains(BinaryExpression),
    In(BinaryExpression),
    Includes(BinaryExpression),
    IncludedIn(BinaryExpression),
    ProperContains(BinaryExpression),
    ProperIn(BinaryExpression),
    ProperIncludes(BinaryExpression),
    ProperIncludedIn(BinaryExpression),
    Before(BinaryExpression),
    After(BinaryExpression),
    Meets(BinaryExpression),
    MeetsBefore(BinaryExpression),
    MeetsAfter(BinaryExpression),
    Overlaps(BinaryExpression),
    OverlapsBefore(BinaryExpression),
    OverlapsAfter(BinaryExpression),
    Starts(BinaryExpression),
    Ends(BinaryExpression),
    Collapse(UnaryExpression),
    Expand(ExpandExpression),
    Union(BinaryExpression),
    Intersect(BinaryExpression),
    Except(BinaryExpression),

    // === List ===
    List(ListExpression),
    Exists(UnaryExpression),
    Times(BinaryExpression),
    Filter(FilterExpression),
    First(FirstLastExpression),
    Last(FirstLastExpression),
    Slice(SliceExpression),
    IndexOf(IndexOfExpression),
    Flatten(UnaryExpression),
    Sort(SortExpression),
    ForEach(ForEachExpression),
    Repeat(RepeatExpression),
    Distinct(UnaryExpression),
    Current(CurrentExpression),
    Iteration(IterationExpression),
    Total(TotalExpression),
    SingletonFrom(UnaryExpression),

    // === Aggregate ===
    Aggregate(AggregateExpression),
    Count(AggregateExpression),
    Sum(AggregateExpression),
    Product(AggregateExpression),
    Min(AggregateExpression),
    Max(AggregateExpression),
    Avg(AggregateExpression),
    GeometricMean(AggregateExpression),
    Median(AggregateExpression),
    Mode(AggregateExpression),
    Variance(AggregateExpression),
    StdDev(AggregateExpression),
    PopulationVariance(AggregateExpression),
    PopulationStdDev(AggregateExpression),
    AllTrue(AggregateExpression),
    AnyTrue(AggregateExpression),

    // === Type Operations ===
    As(AsExpression),
    Convert(ConvertExpression),
    Is(IsExpression),
    CanConvert(CanConvertExpression),
    ToBoolean(UnaryExpression),
    ToChars(UnaryExpression),
    ToConcept(UnaryExpression),
    ToDate(UnaryExpression),
    ToDateTime(UnaryExpression),
    ToDecimal(UnaryExpression),
    ToInteger(UnaryExpression),
    ToLong(UnaryExpression),
    ToList(UnaryExpression),
    ToQuantity(UnaryExpression),
    ToRatio(UnaryExpression),
    ToString(UnaryExpression),
    ToTime(UnaryExpression),
    ConvertsToBoolean(UnaryExpression),
    ConvertsToDate(UnaryExpression),
    ConvertsToDateTime(UnaryExpression),
    ConvertsToDecimal(UnaryExpression),
    ConvertsToInteger(UnaryExpression),
    ConvertsToLong(UnaryExpression),
    ConvertsToQuantity(UnaryExpression),
    ConvertsToRatio(UnaryExpression),
    ConvertsToString(UnaryExpression),
    ConvertsToTime(UnaryExpression),

    // === Clinical ===
    Code(CodeLiteralExpression),
    Concept(ConceptLiteralExpression),
    Quantity(QuantityExpression),
    Ratio(RatioExpression),
    InCodeSystem(InCodeSystemExpression),
    InValueSet(InValueSetExpression),
    CalculateAge(CalculateAgeExpression),
    CalculateAgeAt(CalculateAgeAtExpression),

    // === Query ===
    Query(Query),
    Retrieve(Retrieve),

    // === Tuple ===
    Tuple(TupleExpression),
    Instance(InstanceExpression),

    // === Message ===
    Message(MessageExpression),
}

// ============================================================================
// Expression Components
// ============================================================================

/// Null literal
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NullLiteral {
    #[serde(flatten)]
    pub element: Element,
}

/// Literal value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Literal {
    #[serde(flatten)]
    pub element: Element,
    /// Value type
    pub value_type: String,
    /// The literal value as string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Expression reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
}

/// Function reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operand: Option<Vec<Box<Expression>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<TypeSpecifier>>,
}

/// Parameter reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
}

/// Value set reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve: Option<bool>,
}

/// Code system reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSystemRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
}

/// Code reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
}

/// Concept reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
}

/// Operand reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperandRef {
    #[serde(flatten)]
    pub element: Element,
    pub name: String,
}

/// Alias reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasRef {
    #[serde(flatten)]
    pub element: Element,
    pub name: String,
}

/// Query let reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryLetRef {
    #[serde(flatten)]
    pub element: Element,
    pub name: String,
}

/// Identifier reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifierRef {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    pub name: String,
}

/// Property access
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Box<Expression>>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

// ============================================================================
// Expression Structures
// ============================================================================

/// Unary expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnaryExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
}

/// Binary expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
}

/// Ternary expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TernaryExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
}

/// N-ary expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NaryExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
}

/// If expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IfExpression {
    #[serde(flatten)]
    pub element: Element,
    pub condition: Box<Expression>,
    pub then: Box<Expression>,
    #[serde(rename = "else")]
    pub else_clause: Box<Expression>,
}

/// Case expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparand: Option<Box<Expression>>,
    pub case_item: Vec<CaseItem>,
    #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_clause: Option<Box<Expression>>,
}

/// Case item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseItem {
    pub when: Box<Expression>,
    pub then: Box<Expression>,
}

/// Round expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<Box<Expression>>,
}

/// MinValue/MaxValue expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinMaxValueExpression {
    #[serde(flatten)]
    pub element: Element,
    pub value_type: String,
}

/// Boundary expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<Box<Expression>>,
}

// ============================================================================
// String Operations
// ============================================================================

/// Combine expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombineExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<Box<Expression>>,
}

/// Split expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitExpression {
    #[serde(flatten)]
    pub element: Element,
    pub string_to_split: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<Box<Expression>>,
}

/// SplitOnMatches expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitOnMatchesExpression {
    #[serde(flatten)]
    pub element: Element,
    pub string_to_split: Box<Expression>,
    pub separator_pattern: Box<Expression>,
}

/// PositionOf expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionOfExpression {
    #[serde(flatten)]
    pub element: Element,
    pub pattern: Box<Expression>,
    pub string: Box<Expression>,
}

/// LastPositionOf expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastPositionOfExpression {
    #[serde(flatten)]
    pub element: Element,
    pub pattern: Box<Expression>,
    pub string: Box<Expression>,
}

/// Substring expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubstringExpression {
    #[serde(flatten)]
    pub element: Element,
    pub string_to_sub: Box<Expression>,
    pub start_index: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<Box<Expression>>,
}

// ============================================================================
// DateTime Operations
// ============================================================================

/// Now expression
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NowExpression {
    #[serde(flatten)]
    pub element: Element,
}

/// Today expression
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TodayExpression {
    #[serde(flatten)]
    pub element: Element,
}

/// TimeOfDay expression
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimeOfDayExpression {
    #[serde(flatten)]
    pub element: Element,
}

/// Date expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateExpression {
    #[serde(flatten)]
    pub element: Element,
    pub year: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<Box<Expression>>,
}

/// DateTime expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeExpression {
    #[serde(flatten)]
    pub element: Element,
    pub year: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hour: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minute: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub millisecond: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone_offset: Option<Box<Expression>>,
}

/// Time expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeExpression {
    #[serde(flatten)]
    pub element: Element,
    pub hour: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minute: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub millisecond: Option<Box<Expression>>,
}

/// DateTimeComponentFrom expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeComponentFromExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    pub precision: DateTimePrecision,
}

/// DateTime precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DateTimePrecision {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

/// DurationBetween expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurationBetweenExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
    pub precision: DateTimePrecision,
}

/// DifferenceBetween expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferenceBetweenExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
    pub precision: DateTimePrecision,
}

/// SameAs expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SameAsExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<DateTimePrecision>,
}

/// SameOrBefore expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SameOrBeforeExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<DateTimePrecision>,
}

/// SameOrAfter expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SameOrAfterExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<DateTimePrecision>,
}

// ============================================================================
// Interval Operations
// ============================================================================

/// Interval expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntervalExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_closed_expression: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_closed_expression: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_closed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_closed: Option<bool>,
}

/// Expand expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpandExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per: Option<Box<Expression>>,
}

// ============================================================================
// List Operations
// ============================================================================

/// List expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_specifier: Option<TypeSpecifier>,
    #[serde(rename = "element", skip_serializing_if = "Option::is_none")]
    pub elements: Option<Vec<Box<Expression>>>,
}

/// Filter expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    pub condition: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// First/Last expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstLastExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
}

/// Slice expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    pub start_index: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<Box<Expression>>,
}

/// IndexOf expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexOfExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    #[serde(rename = "element")]
    pub element_to_find: Box<Expression>,
}

/// Sort expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    pub by: Vec<SortByItem>,
}

/// Sort by item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortByItem {
    pub direction: SortDirection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "ascending")]
    Ascending,
    #[serde(rename = "desc")]
    Desc,
    #[serde(rename = "descending")]
    Descending,
}

/// ForEach expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForEachExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    #[serde(rename = "element")]
    pub element_expr: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Repeat expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeatExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    #[serde(rename = "element")]
    pub element_expr: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Current expression ($this)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurrentExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Iteration expression ($index)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IterationExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Total expression ($total)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TotalExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

// ============================================================================
// Aggregate Operations
// ============================================================================

/// Aggregate expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iteration: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

// ============================================================================
// Type Operations
// ============================================================================

/// As expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_type_specifier: Option<TypeSpecifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Convert expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_type_specifier: Option<TypeSpecifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_type: Option<String>,
}

/// Is expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IsExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_type_specifier: Option<TypeSpecifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_type: Option<String>,
}

/// CanConvert expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanConvertExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_type_specifier: Option<TypeSpecifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_type: Option<String>,
}

// ============================================================================
// Clinical Operations
// ============================================================================

/// Code literal expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLiteralExpression {
    #[serde(flatten)]
    pub element: Element,
    pub system: CodeSystemRef,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Concept literal expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptLiteralExpression {
    #[serde(flatten)]
    pub element: Element,
    pub code: Vec<CodeLiteralExpression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// Quantity expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantityExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// Ratio expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatioExpression {
    #[serde(flatten)]
    pub element: Element,
    pub numerator: Box<QuantityExpression>,
    pub denominator: Box<QuantityExpression>,
}

/// InCodeSystem expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InCodeSystemExpression {
    #[serde(flatten)]
    pub element: Element,
    pub code: Box<Expression>,
    pub codesystem: CodeSystemRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codesystem_expression: Option<Box<Expression>>,
}

/// InValueSet expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InValueSetExpression {
    #[serde(flatten)]
    pub element: Element,
    pub code: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valueset: Option<ValueSetRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valueset_expression: Option<Box<Expression>>,
}

/// CalculateAge expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculateAgeExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Box<Expression>,
    pub precision: DateTimePrecision,
}

/// CalculateAgeAt expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculateAgeAtExpression {
    #[serde(flatten)]
    pub element: Element,
    pub operand: Vec<Box<Expression>>,
    pub precision: DateTimePrecision,
}

// ============================================================================
// Query
// ============================================================================

/// Query expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    #[serde(flatten)]
    pub element: Element,
    pub source: Vec<AliasedQuerySource>,
    #[serde(rename = "let", skip_serializing_if = "Option::is_none")]
    pub let_clause: Option<Vec<LetClause>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<Vec<RelationshipClause>>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_clause: Option<Box<Expression>>,
    #[serde(rename = "return", skip_serializing_if = "Option::is_none")]
    pub return_clause: Option<ReturnClause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<AggregateClause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortClause>,
}

/// Aliased query source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasedQuerySource {
    pub expression: Box<Expression>,
    pub alias: String,
}

/// Let clause
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LetClause {
    pub identifier: String,
    pub expression: Box<Expression>,
}

/// Relationship clause
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelationshipClause {
    With(WithClause),
    Without(WithoutClause),
}

/// With clause
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithClause {
    pub expression: Box<Expression>,
    pub alias: String,
    pub such_that: Box<Expression>,
}

/// Without clause
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithoutClause {
    pub expression: Box<Expression>,
    pub alias: String,
    pub such_that: Box<Expression>,
}

/// Return clause
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnClause {
    pub expression: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct: Option<bool>,
}

/// Aggregate clause
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateClause {
    pub identifier: String,
    pub expression: Box<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct: Option<bool>,
}

/// Sort clause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortClause {
    pub by: Vec<SortByItem>,
}

/// Retrieve expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Retrieve {
    #[serde(flatten)]
    pub element: Element,
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_expression: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codes: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range: Option<Box<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<IncludeElement>>,
}

/// Include element
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncludeElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_path: Option<String>,
    pub related_data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_reverse: Option<bool>,
}

// ============================================================================
// Tuple/Instance
// ============================================================================

/// Tuple expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TupleExpression {
    #[serde(flatten)]
    pub element: Element,
    #[serde(rename = "element", skip_serializing_if = "Option::is_none")]
    pub elements: Option<Vec<TupleElementExpression>>,
}

/// Tuple element expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TupleElementExpression {
    pub name: String,
    pub value: Box<Expression>,
}

/// Instance expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceExpression {
    #[serde(flatten)]
    pub element: Element,
    pub class_type: String,
    #[serde(rename = "element", skip_serializing_if = "Option::is_none")]
    pub elements: Option<Vec<InstanceElementExpression>>,
}

/// Instance element expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceElementExpression {
    pub name: String,
    pub value: Box<Expression>,
}

// ============================================================================
// Message
// ============================================================================

/// Message expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageExpression {
    #[serde(flatten)]
    pub element: Element,
    pub source: Box<Expression>,
    pub condition: Box<Expression>,
    pub code: Box<Expression>,
    pub severity: Box<Expression>,
    pub message: Box<Expression>,
}

// ============================================================================
// Annotations
// ============================================================================

/// Annotation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub annotation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<NarrativeElement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<Vec<Tag>>,
}

/// Narrative element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<Vec<NarrativeElement>>,
    #[serde(rename = "$value", skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub value: String,
}
