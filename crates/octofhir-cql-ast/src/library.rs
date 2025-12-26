//! Library structure AST nodes

use crate::{
    AccessModifier, BoxExpr, Expression, Identifier, OptBoxExpr, QualifiedIdentifier, Spanned,
    TypeSpecifier, VersionSpecifier,
};

/// A complete CQL library
#[derive(Debug, Clone)]
pub struct Library {
    /// Library definition (name and version)
    pub definition: Option<LibraryDefinition>,
    /// Using definitions (data models)
    pub usings: Vec<Spanned<UsingDefinition>>,
    /// Include definitions (library includes)
    pub includes: Vec<Spanned<IncludeDefinition>>,
    /// Parameter definitions
    pub parameters: Vec<Spanned<ParameterDefinition>>,
    /// Codesystem definitions
    pub codesystems: Vec<Spanned<CodesystemDefinition>>,
    /// Valueset definitions
    pub valuesets: Vec<Spanned<ValuesetDefinition>>,
    /// Code definitions
    pub codes: Vec<Spanned<CodeDefinition>>,
    /// Concept definitions
    pub concepts: Vec<Spanned<ConceptDefinition>>,
    /// Context definitions
    pub contexts: Vec<Spanned<ContextDefinition>>,
    /// Expression definitions (named expressions)
    pub statements: Vec<Spanned<Statement>>,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    /// Create a new empty library
    pub fn new() -> Self {
        Self {
            definition: None,
            usings: Vec::new(),
            includes: Vec::new(),
            parameters: Vec::new(),
            codesystems: Vec::new(),
            valuesets: Vec::new(),
            codes: Vec::new(),
            concepts: Vec::new(),
            contexts: Vec::new(),
            statements: Vec::new(),
        }
    }
}

/// Library definition (name and version)
#[derive(Debug, Clone)]
pub struct LibraryDefinition {
    /// Library name (qualified identifier)
    pub name: QualifiedIdentifier,
    /// Optional version
    pub version: Option<VersionSpecifier>,
}

/// Using definition for data model
#[derive(Debug, Clone)]
pub struct UsingDefinition {
    /// Model identifier (e.g., "FHIR")
    pub model: Identifier,
    /// Optional version
    pub version: Option<VersionSpecifier>,
}

/// Include definition for library inclusion
#[derive(Debug, Clone)]
pub struct IncludeDefinition {
    /// Library identifier
    pub library: QualifiedIdentifier,
    /// Optional version
    pub version: Option<VersionSpecifier>,
    /// Optional alias (called qualifier)
    pub alias: Option<Identifier>,
}

/// Parameter definition
#[derive(Debug, Clone)]
pub struct ParameterDefinition {
    /// Access modifier
    pub access: AccessModifier,
    /// Parameter name
    pub name: Identifier,
    /// Optional type specifier
    pub type_specifier: Option<Spanned<TypeSpecifier>>,
    /// Optional default value
    pub default: OptBoxExpr,
}

/// Codesystem definition
#[derive(Debug, Clone)]
pub struct CodesystemDefinition {
    /// Access modifier
    pub access: AccessModifier,
    /// Codesystem name
    pub name: Identifier,
    /// Codesystem URI
    pub uri: String,
    /// Optional version
    pub version: Option<VersionSpecifier>,
}

/// Valueset definition
#[derive(Debug, Clone)]
pub struct ValuesetDefinition {
    /// Access modifier
    pub access: AccessModifier,
    /// Valueset name
    pub name: Identifier,
    /// Valueset URI
    pub uri: String,
    /// Optional version
    pub version: Option<VersionSpecifier>,
    /// Optional codesystems
    pub codesystems: Vec<CodeSystemRef>,
}

/// Reference to a codesystem in valueset definition
#[derive(Debug, Clone)]
pub struct CodeSystemRef {
    /// Codesystem name
    pub name: QualifiedIdentifier,
}

/// Code definition
#[derive(Debug, Clone)]
pub struct CodeDefinition {
    /// Access modifier
    pub access: AccessModifier,
    /// Code name
    pub name: Identifier,
    /// Code value
    pub code: String,
    /// Codesystem reference
    pub codesystem: QualifiedIdentifier,
    /// Optional display string
    pub display: Option<String>,
}

/// Concept definition (group of codes)
#[derive(Debug, Clone)]
pub struct ConceptDefinition {
    /// Access modifier
    pub access: AccessModifier,
    /// Concept name
    pub name: Identifier,
    /// Codes in this concept
    pub codes: Vec<QualifiedIdentifier>,
    /// Optional display string
    pub display: Option<String>,
}

/// Context definition
#[derive(Debug, Clone)]
pub struct ContextDefinition {
    /// Context identifier (e.g., "Patient", "Practitioner")
    pub context: Identifier,
}

/// A statement in the library (expression definition or function definition)
#[derive(Debug, Clone)]
pub enum Statement {
    /// Named expression definition
    ExpressionDef(ExpressionDefinition),
    /// Function definition
    FunctionDef(FunctionDefinition),
}

/// Expression definition (named expression)
#[derive(Debug, Clone)]
pub struct ExpressionDefinition {
    /// Access modifier
    pub access: AccessModifier,
    /// Expression name
    pub name: Identifier,
    /// The expression
    pub expression: BoxExpr,
}

/// Function definition
#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    /// Access modifier
    pub access: AccessModifier,
    /// Whether this is a fluent function
    pub fluent: bool,
    /// Function name
    pub name: Identifier,
    /// Function parameters
    pub parameters: Vec<FunctionParameter>,
    /// Optional return type
    pub return_type: Option<Spanned<TypeSpecifier>>,
    /// Function body (None for external functions)
    pub body: Option<BoxExpr>,
    /// Whether this is an external function
    pub external: bool,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct FunctionParameter {
    /// Parameter name
    pub name: Identifier,
    /// Parameter type
    pub type_specifier: Spanned<TypeSpecifier>,
}
