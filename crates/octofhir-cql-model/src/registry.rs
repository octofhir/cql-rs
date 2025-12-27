//! Model registry implementing ModelProvider
//!
//! This module provides a concrete implementation of ModelProvider using ModelInfo.

use crate::model_info::{ModelInfo, PropertyInfo, TypeInfo};
use crate::provider::{ModelProvider, ModelProviderError};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

/// Model registry that implements ModelProvider
#[derive(Clone)]
pub struct ModelRegistry {
    model_info: Arc<RwLock<ModelInfo>>,
}

impl ModelRegistry {
    /// Create a new model registry from ModelInfo
    pub fn new(model_info: ModelInfo) -> Self {
        Self {
            model_info: Arc::new(RwLock::new(model_info)),
        }
    }

    /// Load ModelInfo from XML string
    pub fn from_xml(xml: &str) -> Result<Self, ModelProviderError> {
        let model_info = crate::model_info::parse_xml(xml)
            .map_err(|e| ModelProviderError::ParseError(e.to_string()))?;
        Ok(Self::new(model_info))
    }

    /// Load ModelInfo from JSON string
    pub fn from_json(json: &str) -> Result<Self, ModelProviderError> {
        let model_info = crate::model_info::parse_json(json)
            .map_err(|e| ModelProviderError::ParseError(e.to_string()))?;
        Ok(Self::new(model_info))
    }

    /// Load ModelInfo from XML file at runtime
    pub fn from_xml_file(path: impl AsRef<std::path::Path>) -> Result<Self, ModelProviderError> {
        let xml = std::fs::read_to_string(path)
            .map_err(|e| ModelProviderError::IoError(e.to_string()))?;
        Self::from_xml(&xml)
    }

    /// Load ModelInfo from JSON file at runtime
    pub fn from_json_file(path: impl AsRef<std::path::Path>) -> Result<Self, ModelProviderError> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| ModelProviderError::IoError(e.to_string()))?;
        Self::from_json(&json)
    }

    /// Auto-detect and load ModelInfo from file based on extension (.xml or .json)
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ModelProviderError> {
        let path = path.as_ref();
        match path.extension().and_then(|e| e.to_str()) {
            Some("xml") => Self::from_xml_file(path),
            Some("json") => Self::from_json_file(path),
            Some(ext) => Err(ModelProviderError::ParseError(format!(
                "Unsupported file extension: .{}. Expected .xml or .json",
                ext
            ))),
            None => Err(ModelProviderError::ParseError(
                "No file extension found. Expected .xml or .json".to_string(),
            )),
        }
    }

    /// Get the model name
    pub fn model_name(&self) -> String {
        self.model_info.read().name.clone()
    }

    /// Get the model version
    pub fn model_version(&self) -> String {
        self.model_info.read().version.clone()
    }

    /// Get the model URL
    pub fn model_url(&self) -> String {
        self.model_info.read().url.clone()
    }

    /// Get all retrievable types
    pub fn get_retrievable_types(&self) -> Vec<String> {
        self.model_info
            .read()
            .get_retrievable_types()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

#[async_trait]
impl ModelProvider for ModelRegistry {
    async fn get_type(&self, type_name: &str) -> Result<Option<TypeInfo>, ModelProviderError> {
        let model = self.model_info.read();
        Ok(model.get_type(type_name).cloned())
    }

    async fn get_property_type(
        &self,
        parent: &str,
        property: &str,
    ) -> Result<Option<PropertyInfo>, ModelProviderError> {
        let model = self.model_info.read();
        Ok(model.get_property_type(parent, property).cloned())
    }

    fn is_retrievable(&self, type_name: &str) -> bool {
        let model = self.model_info.read();
        model.is_retrievable(type_name)
    }

    fn get_primary_code_path(&self, type_name: &str) -> Option<String> {
        let model = self.model_info.read();
        model.get_primary_code_path(type_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_model_registry_from_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <modelInfo name="TestModel" version="1.0.0" url="http://test.org">
            <typeInfo name="Patient" retrievable="true" primaryCodePath="code">
                <element name="id" type="String"/>
                <element name="name" type="list<String>"/>
            </typeInfo>
        </modelInfo>"#;

        let registry = ModelRegistry::from_xml(xml).unwrap();

        assert_eq!(registry.model_name(), "TestModel");
        assert_eq!(registry.model_version(), "1.0.0");
        assert!(registry.is_retrievable("Patient"));
        assert_eq!(
            registry.get_primary_code_path("Patient"),
            Some("code".to_string())
        );

        let patient_type = registry.get_type("Patient").await.unwrap().unwrap();
        assert_eq!(patient_type.name, "Patient");
        assert_eq!(patient_type.elements.len(), 2);

        let id_prop = registry
            .get_property_type("Patient", "id")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(id_prop.name, "id");
        assert_eq!(id_prop.element_type, "String");
    }

    #[tokio::test]
    async fn test_model_registry_from_json() {
        let json = r#"{
            "name": "TestModel",
            "version": "1.0.0",
            "url": "http://test.org",
            "typeInfo": [{
                "name": "Patient",
                "retrievable": true,
                "primaryCodePath": "code",
                "element": [
                    {"name": "id", "type": "String"},
                    {"name": "name", "type": "list<String>"}
                ]
            }]
        }"#;

        let registry = ModelRegistry::from_json(json).unwrap();

        assert_eq!(registry.model_name(), "TestModel");
        assert!(registry.is_retrievable("Patient"));

        let patient_type = registry.get_type("Patient").await.unwrap().unwrap();
        assert_eq!(patient_type.name, "Patient");
    }

    #[tokio::test]
    async fn test_model_registry_from_xml_file() {
        // Create a temporary XML file
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <modelInfo name="FileTest" version="2.0.0" url="http://test.org">
            <typeInfo name="Observation" retrievable="true">
                <element name="id" type="String"/>
            </typeInfo>
        </modelInfo>"#;
        temp_file.write_all(xml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Load from file
        let registry = ModelRegistry::from_xml_file(temp_file.path()).unwrap();

        assert_eq!(registry.model_name(), "FileTest");
        assert_eq!(registry.model_version(), "2.0.0");
        assert!(registry.is_retrievable("Observation"));
    }

    #[tokio::test]
    async fn test_model_registry_from_file_auto_detect() {
        // Create a temporary XML file with .xml extension
        let temp_dir = tempfile::tempdir().unwrap();
        let xml_path = temp_dir.path().join("test-model.xml");

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <modelInfo name="AutoDetectTest" version="3.0.0" url="http://test.org">
            <typeInfo name="Patient" retrievable="true">
                <element name="id" type="String"/>
            </typeInfo>
        </modelInfo>"#;
        std::fs::write(&xml_path, xml).unwrap();

        // Load with auto-detection
        let registry = ModelRegistry::from_file(&xml_path).unwrap();

        assert_eq!(registry.model_name(), "AutoDetectTest");
        assert!(registry.is_retrievable("Patient"));
    }
}
