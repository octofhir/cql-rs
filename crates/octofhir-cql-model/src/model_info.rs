//! ModelInfo abstraction for data model definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ModelInfo structure describing a data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name (e.g., "FHIR")
    pub name: String,
    /// Model version
    pub version: String,
    /// Model URL
    pub url: String,
    /// Target qualifier (namespace)
    pub target_qualifier: Option<String>,
    /// Patient class name
    pub patient_class_name: Option<String>,
    /// Patient birth date property
    pub patient_birth_date_property_name: Option<String>,
    /// Type definitions
    pub type_infos: HashMap<String, TypeInfo>,
}

impl ModelInfo {
    /// Create a new ModelInfo
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            url: String::new(),
            target_qualifier: None,
            patient_class_name: None,
            patient_birth_date_property_name: None,
            type_infos: HashMap::new(),
        }
    }

    /// Get type info by name
    pub fn get_type(&self, name: &str) -> Option<&TypeInfo> {
        self.type_infos.get(name)
    }

    /// Check if model contains type
    pub fn has_type(&self, name: &str) -> bool {
        self.type_infos.contains_key(name)
    }
}

/// Type information for a model type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Type name
    pub name: String,
    /// Namespace
    pub namespace: Option<String>,
    /// Base type name
    pub base_type: Option<String>,
    /// Whether this is retrievable
    pub retrievable: bool,
    /// Primary code path for terminology filtering
    pub primary_code_path: Option<String>,
    /// Element definitions
    pub elements: Vec<ElementInfo>,
}

impl TypeInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            base_type: None,
            retrievable: false,
            primary_code_path: None,
            elements: Vec::new(),
        }
    }

    /// Get element by name
    pub fn get_element(&self, name: &str) -> Option<&ElementInfo> {
        self.elements.iter().find(|e| e.name == name)
    }
}

/// Element information within a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    /// Element name
    pub name: String,
    /// Element type
    pub element_type: String,
    /// Whether element is a list
    pub is_list: bool,
    /// Target mapping
    pub target: Option<String>,
}

impl ElementInfo {
    pub fn new(name: impl Into<String>, element_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            element_type: element_type.into(),
            is_list: false,
            target: None,
        }
    }
}
