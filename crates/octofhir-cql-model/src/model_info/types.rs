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

    /// Get property type, traversing base types if necessary
    pub fn get_property_type(&self, parent_type: &str, property_name: &str) -> Option<&PropertyInfo> {
        let mut current_type = self.get_type(parent_type)?;

        // Try to find property in current type
        loop {
            if let Some(prop) = current_type.get_property(property_name) {
                return Some(prop);
            }

            // If not found, check base type
            if let Some(ref base_type_name) = current_type.base_type {
                current_type = self.get_type(base_type_name)?;
            } else {
                break;
            }
        }

        None
    }

    /// Check if a type is retrievable
    pub fn is_retrievable(&self, type_name: &str) -> bool {
        self.get_type(type_name)
            .map(|t| t.retrievable)
            .unwrap_or(false)
    }

    /// Get primary code path for a type
    pub fn get_primary_code_path(&self, type_name: &str) -> Option<String> {
        self.get_type(type_name)
            .and_then(|t| t.primary_code_path.clone())
    }

    /// Check if one type is derived from another (considers inheritance)
    pub fn is_derived_from(&self, child_type: &str, parent_type: &str) -> bool {
        if child_type == parent_type {
            return true;
        }

        let mut current_type = match self.get_type(child_type) {
            Some(t) => t,
            None => return false,
        };

        loop {
            if let Some(ref base_type_name) = current_type.base_type {
                if base_type_name == parent_type {
                    return true;
                }
                current_type = match self.get_type(base_type_name) {
                    Some(t) => t,
                    None => return false,
                };
            } else {
                break;
            }
        }

        false
    }

    /// Get all retrievable types
    pub fn get_retrievable_types(&self) -> Vec<&str> {
        self.type_infos
            .values()
            .filter(|t| t.retrievable)
            .map(|t| t.name.as_str())
            .collect()
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
    /// Property definitions
    pub elements: Vec<PropertyInfo>,
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

    /// Get property by name
    pub fn get_property(&self, name: &str) -> Option<&PropertyInfo> {
        self.elements.iter().find(|e| e.name == name)
    }
}

/// Property information within a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    /// Property name
    pub name: String,
    /// Property type
    pub element_type: String,
    /// Whether property is a list
    pub is_list: bool,
    /// Target mapping (for FHIR path expressions)
    pub target: Option<String>,
}

impl PropertyInfo {
    pub fn new(name: impl Into<String>, element_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            element_type: element_type.into(),
            is_list: false,
            target: None,
        }
    }
}
