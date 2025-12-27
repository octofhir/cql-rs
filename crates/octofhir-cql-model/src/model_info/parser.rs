//! ModelInfo parser for XML and JSON formats
//!
//! Parses HL7 ModelInfo files (XML and JSON) into ModelInfo structures.
//! Reference: http://cql.hl7.org/07-physicalrepresentation.html#modelinfo

use super::types::{ModelInfo, TypeInfo, PropertyInfo};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value as JsonValue;

/// Error type for ModelInfo parsing
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("XML parse error: {0}")]
    XmlError(#[from] quick_xml::Error),

    #[error("XML attribute error: {0}")]
    AttrError(#[from] quick_xml::events::attributes::AttrError),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid ModelInfo structure: {0}")]
    InvalidStructure(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Parse ModelInfo from XML format
pub fn parse_xml(xml_content: &str) -> Result<ModelInfo, ParseError> {
    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);

    let mut model_info = ModelInfo::new("", "");
    let mut current_type: Option<TypeInfo> = None;
    let mut current_property: Option<PropertyInfo> = None;
    let mut current_element = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match current_element.as_str() {
                    "modelInfo" => {
                        // Parse modelInfo attributes
                        for attr in e.attributes() {
                            let attr = attr?;
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let value = String::from_utf8_lossy(&attr.value);

                            match key.as_ref() {
                                "name" => model_info.name = value.to_string(),
                                "version" => model_info.version = value.to_string(),
                                "url" => model_info.url = value.to_string(),
                                "targetQualifier" => model_info.target_qualifier = Some(value.to_string()),
                                "patientClassName" => model_info.patient_class_name = Some(value.to_string()),
                                "patientBirthDatePropertyName" => {
                                    model_info.patient_birth_date_property_name = Some(value.to_string())
                                }
                                _ => {}
                            }
                        }
                    }
                    "typeInfo" | "classInfo" | "simpleTypeInfo" | "profileInfo" => {
                        let mut type_info = TypeInfo::new("");

                        for attr in e.attributes() {
                            let attr = attr?;
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let value = String::from_utf8_lossy(&attr.value);

                            match key.as_ref() {
                                "name" => type_info.name = value.to_string(),
                                "namespace" => type_info.namespace = Some(value.to_string()),
                                "baseType" => type_info.base_type = Some(value.to_string()),
                                "retrievable" => {
                                    type_info.retrievable = value.as_ref() == "true"
                                }
                                "primaryCodePath" => {
                                    type_info.primary_code_path = Some(value.to_string())
                                }
                                _ => {}
                            }
                        }

                        current_type = Some(type_info);
                    }
                    "element" => {
                        if current_type.is_some() {
                            let mut property = PropertyInfo::new("", "");

                            for attr in e.attributes() {
                                let attr = attr?;
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);

                                match key.as_ref() {
                                    "name" => property.name = value.to_string(),
                                    "type" | "elementType" => property.element_type = value.to_string(),
                                    "target" => property.target = Some(value.to_string()),
                                    _ => {}
                                }
                            }

                            // Check if it's a list type
                            if property.element_type.starts_with("list<") || property.element_type.starts_with("List<") {
                                property.is_list = true;
                                // Extract inner type from list<T>
                                if let Some(inner) = property.element_type.strip_prefix("list<").or_else(|| property.element_type.strip_prefix("List<")) {
                                    if let Some(inner) = inner.strip_suffix('>') {
                                        property.element_type = inner.to_string();
                                    }
                                }
                            }

                            current_property = Some(property);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                // Handle self-closing tags like <element name="id" type="String"/>
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag_name.as_str() {
                    "element" => {
                        if current_type.is_some() {
                            let mut property = PropertyInfo::new("", "");

                            for attr in e.attributes() {
                                let attr = attr?;
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);

                                match key.as_ref() {
                                    "name" => property.name = value.to_string(),
                                    "type" | "elementType" => property.element_type = value.to_string(),
                                    "target" => property.target = Some(value.to_string()),
                                    _ => {}
                                }
                            }

                            // Check if it's a list type
                            if property.element_type.starts_with("list<") || property.element_type.starts_with("List<") {
                                property.is_list = true;
                                // Extract inner type from list<T>
                                if let Some(inner) = property.element_type.strip_prefix("list<").or_else(|| property.element_type.strip_prefix("List<")) {
                                    if let Some(inner) = inner.strip_suffix('>') {
                                        property.element_type = inner.to_string();
                                    }
                                }
                            }

                            // Add property directly since it's a self-closing tag
                            if let Some(ref mut type_info) = current_type {
                                type_info.elements.push(property);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag_name.as_str() {
                    "typeInfo" | "classInfo" | "simpleTypeInfo" | "profileInfo" => {
                        if let Some(type_info) = current_type.take() {
                            model_info.type_infos.insert(type_info.name.clone(), type_info);
                        }
                    }
                    "element" => {
                        if let Some(property) = current_property.take() {
                            if let Some(ref mut type_info) = current_type {
                                type_info.elements.push(property);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::XmlError(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(model_info)
}

/// Parse ModelInfo from JSON format
pub fn parse_json(json_content: &str) -> Result<ModelInfo, ParseError> {
    let json: JsonValue = serde_json::from_str(json_content)?;

    let mut model_info = ModelInfo::new(
        json["name"].as_str().unwrap_or(""),
        json["version"].as_str().unwrap_or(""),
    );

    model_info.url = json["url"].as_str().unwrap_or("").to_string();
    model_info.target_qualifier = json["targetQualifier"].as_str().map(String::from);
    model_info.patient_class_name = json["patientClassName"].as_str().map(String::from);
    model_info.patient_birth_date_property_name =
        json["patientBirthDatePropertyName"].as_str().map(String::from);

    // Parse type infos
    if let Some(type_infos) = json["typeInfo"].as_array() {
        for type_json in type_infos {
            let type_info = parse_type_info_json(type_json)?;
            model_info.type_infos.insert(type_info.name.clone(), type_info);
        }
    }

    Ok(model_info)
}

fn parse_type_info_json(json: &JsonValue) -> Result<TypeInfo, ParseError> {
    let mut type_info = TypeInfo::new(
        json["name"].as_str().ok_or_else(||
            ParseError::InvalidStructure("Missing type name".to_string())
        )?
    );

    type_info.namespace = json["namespace"].as_str().map(String::from);
    type_info.base_type = json["baseType"].as_str().map(String::from);
    type_info.retrievable = json["retrievable"].as_bool().unwrap_or(false);
    type_info.primary_code_path = json["primaryCodePath"].as_str().map(String::from);

    // Parse elements
    if let Some(elements) = json["element"].as_array() {
        for elem_json in elements {
            let property = parse_property_json(elem_json)?;
            type_info.elements.push(property);
        }
    }

    Ok(type_info)
}

fn parse_property_json(json: &JsonValue) -> Result<PropertyInfo, ParseError> {
    let name = json["name"].as_str().ok_or_else(||
        ParseError::InvalidStructure("Missing element name".to_string())
    )?;

    let mut element_type = json["type"]
        .as_str()
        .or_else(|| json["elementType"].as_str())
        .ok_or_else(|| ParseError::InvalidStructure("Missing element type".to_string()))?
        .to_string();

    let mut is_list = false;

    // Check if it's a list type
    if element_type.starts_with("list<") || element_type.starts_with("List<") {
        is_list = true;
        // Extract inner type from list<T>
        if let Some(inner) = element_type.strip_prefix("list<").or_else(|| element_type.strip_prefix("List<")) {
            if let Some(inner) = inner.strip_suffix('>') {
                element_type = inner.to_string();
            }
        }
    }

    let mut property = PropertyInfo::new(name, element_type);
    property.is_list = is_list;
    property.target = json["target"].as_str().map(String::from);

    Ok(property)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <modelInfo name="TestModel" version="1.0.0" url="http://test.org">
            <typeInfo name="Patient" retrievable="true" primaryCodePath="code">
                <element name="id" type="String"/>
                <element name="name" type="list<String>"/>
            </typeInfo>
        </modelInfo>"#;

        let result = parse_xml(xml);
        assert!(result.is_ok());

        let model = result.unwrap();
        assert_eq!(model.name, "TestModel");
        assert_eq!(model.version, "1.0.0");
        assert!(model.type_infos.contains_key("Patient"));

        let patient_type = model.type_infos.get("Patient").unwrap();
        assert_eq!(patient_type.retrievable, true);
        assert_eq!(patient_type.primary_code_path, Some("code".to_string()));
        assert_eq!(patient_type.elements.len(), 2);

        let name_prop = &patient_type.elements[1];
        assert_eq!(name_prop.name, "name");
        assert_eq!(name_prop.is_list, true);
        assert_eq!(name_prop.element_type, "String");
    }

    #[test]
    fn test_parse_simple_json() {
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

        let result = parse_json(json);
        assert!(result.is_ok());

        let model = result.unwrap();
        assert_eq!(model.name, "TestModel");
        assert_eq!(model.version, "1.0.0");
        assert!(model.type_infos.contains_key("Patient"));
    }
}
