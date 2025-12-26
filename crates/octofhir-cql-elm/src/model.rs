//! ELM model structures (placeholder for Phase 2)

use serde::{Deserialize, Serialize};

/// ELM Library representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElmLibrary {
    /// Library identifier
    pub identifier: LibraryIdentifier,
    /// Schema identifier
    #[serde(rename = "schemaIdentifier")]
    pub schema_identifier: Option<String>,
    /// Using definitions
    pub usings: Option<Vec<UsingDef>>,
    /// Include definitions
    pub includes: Option<Vec<IncludeDef>>,
    /// Parameter definitions
    pub parameters: Option<Vec<ParameterDef>>,
    /// Code systems
    #[serde(rename = "codeSystems")]
    pub code_systems: Option<Vec<CodeSystemDef>>,
    /// Value sets
    #[serde(rename = "valueSets")]
    pub value_sets: Option<Vec<ValueSetDef>>,
    /// Codes
    pub codes: Option<Vec<CodeDef>>,
    /// Concepts
    pub concepts: Option<Vec<ConceptDef>>,
    /// Contexts
    pub contexts: Option<Vec<ContextDef>>,
    /// Statements (expression definitions)
    pub statements: Option<Vec<ExpressionDef>>,
}

/// Library identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryIdentifier {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Using definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsingDef {
    #[serde(rename = "localIdentifier")]
    pub local_identifier: String,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Include definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeDef {
    #[serde(rename = "localIdentifier")]
    pub local_identifier: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    #[serde(rename = "accessLevel")]
    pub access_level: Option<String>,
    #[serde(rename = "parameterTypeSpecifier")]
    pub parameter_type_specifier: Option<serde_json::Value>,
    #[serde(rename = "default")]
    pub default_expr: Option<serde_json::Value>,
}

/// Code system definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSystemDef {
    pub name: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "accessLevel")]
    pub access_level: Option<String>,
}

/// Value set definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSetDef {
    pub name: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "accessLevel")]
    pub access_level: Option<String>,
}

/// Code definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDef {
    pub name: String,
    pub id: String,
    #[serde(rename = "codeSystem")]
    pub code_system: CodeSystemRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(rename = "accessLevel")]
    pub access_level: Option<String>,
}

/// Code system reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSystemRef {
    pub name: String,
}

/// Concept definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptDef {
    pub name: String,
    pub codes: Vec<CodeRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(rename = "accessLevel")]
    pub access_level: Option<String>,
}

/// Code reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRef {
    pub name: String,
}

/// Context definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDef {
    pub name: String,
}

/// Expression definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionDef {
    pub name: String,
    pub context: Option<String>,
    #[serde(rename = "accessLevel")]
    pub access_level: Option<String>,
    pub expression: Option<serde_json::Value>,
}
