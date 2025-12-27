//! ELM Serialization
//!
//! This module provides JSON and XML serialization for ELM libraries,
//! compatible with the HL7 ELM specification.

use std::io::{Read, Write};

use crate::model::Library;

/// Errors that can occur during serialization
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// XML serialization error
    #[error("XML error: {0}")]
    Xml(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait for ELM serializers
pub trait ElmSerializer {
    /// Serialize a library to a string
    fn serialize(&self, library: &Library) -> Result<String, SerializeError>;

    /// Serialize a library to a writer
    fn serialize_to_writer<W: Write>(
        &self,
        library: &Library,
        writer: W,
    ) -> Result<(), SerializeError>;

    /// Deserialize a library from a string
    fn deserialize(&self, input: &str) -> Result<Library, SerializeError>;

    /// Deserialize a library from a reader
    fn deserialize_from_reader<R: Read>(&self, reader: R) -> Result<Library, SerializeError>;
}

/// JSON serializer for ELM
///
/// Produces output compatible with the HL7 ELM JSON format.
#[derive(Debug, Default, Clone)]
pub struct JsonSerializer {
    /// Whether to produce pretty-printed output
    pub pretty: bool,
}

impl JsonSerializer {
    /// Create a new JSON serializer
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new JSON serializer with pretty-printing enabled
    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

impl ElmSerializer for JsonSerializer {
    fn serialize(&self, library: &Library) -> Result<String, SerializeError> {
        let mut result = if self.pretty {
            serde_json::to_string_pretty(library)?
        } else {
            serde_json::to_string(library)?
        };

        // Wrap in the standard ELM JSON envelope if needed
        if !result.contains("\"library\"") {
            let wrapped = format!(
                r#"{{"library": {}}}"#,
                result
            );
            result = wrapped;
        }

        Ok(result)
    }

    fn serialize_to_writer<W: Write>(
        &self,
        library: &Library,
        mut writer: W,
    ) -> Result<(), SerializeError> {
        let json = self.serialize(library)?;
        writer.write_all(json.as_bytes())?;
        Ok(())
    }

    fn deserialize(&self, input: &str) -> Result<Library, SerializeError> {
        // Try to parse directly first
        if let Ok(lib) = serde_json::from_str::<Library>(input) {
            return Ok(lib);
        }

        // Try to parse as wrapped format { "library": ... }
        #[derive(serde::Deserialize)]
        struct Wrapper {
            library: Library,
        }

        let wrapper: Wrapper = serde_json::from_str(input)?;
        Ok(wrapper.library)
    }

    fn deserialize_from_reader<R: Read>(&self, mut reader: R) -> Result<Library, SerializeError> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        self.deserialize(&content)
    }
}

/// XML serializer for ELM
///
/// Produces output compatible with the HL7 ELM XML format.
#[derive(Debug, Default, Clone)]
pub struct XmlSerializer {
    /// Whether to produce pretty-printed output
    pub pretty: bool,
}

impl XmlSerializer {
    /// Create a new XML serializer
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new XML serializer with pretty-printing enabled
    pub fn pretty() -> Self {
        Self { pretty: true }
    }

    /// Convert a Library to XML string
    fn library_to_xml(&self, library: &Library) -> Result<String, SerializeError> {
        use std::fmt::Write;

        let mut xml = String::new();

        // XML declaration
        writeln!(
            xml,
            r#"<?xml version="1.0" encoding="UTF-8"?>"#
        )
        .map_err(|e| SerializeError::Xml(e.to_string()))?;

        // Library element with namespaces
        write!(
            xml,
            r#"<library xmlns="urn:hl7-org:elm:r1" xmlns:t="urn:hl7-org:elm-types:r1" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#
        )
        .map_err(|e| SerializeError::Xml(e.to_string()))?;

        // Local identifier
        write!(xml, r#" localId="1""#)
            .map_err(|e| SerializeError::Xml(e.to_string()))?;

        writeln!(xml, ">").map_err(|e| SerializeError::Xml(e.to_string()))?;

        // Identifier
        self.write_element(&mut xml, 1, "identifier", |xml| {
            write!(
                xml,
                r#" id="{}""#,
                self.escape_xml(&library.identifier.id)
            )?;
            if let Some(system) = &library.identifier.system {
                write!(xml, r#" system="{}""#, self.escape_xml(system))?;
            }
            if let Some(version) = &library.identifier.version {
                write!(xml, r#" version="{}""#, self.escape_xml(version))?;
            }
            Ok(())
        })?;

        // Schema identifier
        if let Some(schema) = &library.schema_identifier {
            self.write_element(&mut xml, 1, "schemaIdentifier", |xml| {
                write!(xml, r#" id="{}""#, self.escape_xml(&schema.id))?;
                if let Some(version) = &schema.version {
                    write!(xml, r#" version="{}""#, self.escape_xml(version))?;
                }
                Ok(())
            })?;
        }

        // Usings
        if let Some(usings) = &library.usings {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<usings>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for using in &usings.defs {
                self.write_element(&mut xml, 2, "def", |xml| {
                    write!(
                        xml,
                        r#" localIdentifier="{}""#,
                        self.escape_xml(&using.local_identifier)
                    )?;
                    write!(xml, r#" uri="{}""#, self.escape_xml(&using.uri))?;
                    if let Some(version) = &using.version {
                        write!(xml, r#" version="{}""#, self.escape_xml(version))?;
                    }
                    Ok(())
                })?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</usings>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // Includes
        if let Some(includes) = &library.includes {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<includes>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for include in &includes.defs {
                self.write_element(&mut xml, 2, "def", |xml| {
                    write!(
                        xml,
                        r#" localIdentifier="{}""#,
                        self.escape_xml(&include.local_identifier)
                    )?;
                    write!(xml, r#" path="{}""#, self.escape_xml(&include.path))?;
                    if let Some(version) = &include.version {
                        write!(xml, r#" version="{}""#, self.escape_xml(version))?;
                    }
                    Ok(())
                })?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</includes>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // Parameters
        if let Some(params) = &library.parameters {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<parameters>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for param in &params.defs {
                self.write_indent(&mut xml, 2)?;
                write!(xml, r#"<def name="{}""#, self.escape_xml(&param.name))
                    .map_err(|e| SerializeError::Xml(e.to_string()))?;
                if let Some(access) = &param.access_level {
                    write!(xml, r#" accessLevel="{:?}""#, access)
                        .map_err(|e| SerializeError::Xml(e.to_string()))?;
                }
                writeln!(xml, "/>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</parameters>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // CodeSystems
        if let Some(codesystems) = &library.code_systems {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<codeSystems>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for cs in &codesystems.defs {
                self.write_element(&mut xml, 2, "def", |xml| {
                    write!(xml, r#" name="{}""#, self.escape_xml(&cs.name))?;
                    write!(xml, r#" id="{}""#, self.escape_xml(&cs.id))?;
                    if let Some(version) = &cs.version {
                        write!(xml, r#" version="{}""#, self.escape_xml(version))?;
                    }
                    if let Some(access) = &cs.access_level {
                        write!(xml, r#" accessLevel="{:?}""#, access)?;
                    }
                    Ok(())
                })?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</codeSystems>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // ValueSets
        if let Some(valuesets) = &library.value_sets {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<valueSets>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for vs in &valuesets.defs {
                self.write_element(&mut xml, 2, "def", |xml| {
                    write!(xml, r#" name="{}""#, self.escape_xml(&vs.name))?;
                    write!(xml, r#" id="{}""#, self.escape_xml(&vs.id))?;
                    if let Some(version) = &vs.version {
                        write!(xml, r#" version="{}""#, self.escape_xml(version))?;
                    }
                    if let Some(access) = &vs.access_level {
                        write!(xml, r#" accessLevel="{:?}""#, access)?;
                    }
                    Ok(())
                })?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</valueSets>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // Codes
        if let Some(codes) = &library.codes {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<codes>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for code in &codes.defs {
                self.write_indent(&mut xml, 2)?;
                write!(xml, r#"<def name="{}" id="{}""#,
                    self.escape_xml(&code.name),
                    self.escape_xml(&code.id)
                ).map_err(|e| SerializeError::Xml(e.to_string()))?;
                if let Some(display) = &code.display {
                    write!(xml, r#" display="{}""#, self.escape_xml(display))
                        .map_err(|e| SerializeError::Xml(e.to_string()))?;
                }
                writeln!(xml, ">").map_err(|e| SerializeError::Xml(e.to_string()))?;
                self.write_indent(&mut xml, 3)?;
                writeln!(
                    xml,
                    r#"<codeSystem name="{}"/>"#,
                    self.escape_xml(&code.code_system.name)
                )
                .map_err(|e| SerializeError::Xml(e.to_string()))?;
                self.write_indent(&mut xml, 2)?;
                writeln!(xml, "</def>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</codes>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // Concepts
        if let Some(concepts) = &library.concepts {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<concepts>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for concept in &concepts.defs {
                self.write_indent(&mut xml, 2)?;
                write!(xml, r#"<def name="{}""#, self.escape_xml(&concept.name))
                    .map_err(|e| SerializeError::Xml(e.to_string()))?;
                if let Some(display) = &concept.display {
                    write!(xml, r#" display="{}""#, self.escape_xml(display))
                        .map_err(|e| SerializeError::Xml(e.to_string()))?;
                }
                writeln!(xml, ">").map_err(|e| SerializeError::Xml(e.to_string()))?;
                for code in &concept.code {
                    self.write_indent(&mut xml, 3)?;
                    writeln!(xml, r#"<code name="{}"/>"#, self.escape_xml(&code.name))
                        .map_err(|e| SerializeError::Xml(e.to_string()))?;
                }
                self.write_indent(&mut xml, 2)?;
                writeln!(xml, "</def>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</concepts>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // Contexts
        if let Some(contexts) = &library.contexts {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<contexts>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for ctx in &contexts.defs {
                self.write_element(&mut xml, 2, "def", |xml| {
                    write!(xml, r#" name="{}""#, self.escape_xml(&ctx.name))?;
                    Ok(())
                })?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</contexts>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // Statements
        if let Some(statements) = &library.statements {
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "<statements>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            for def in &statements.defs {
                self.write_indent(&mut xml, 2)?;
                write!(xml, r#"<def name="{}""#, self.escape_xml(&def.name))
                    .map_err(|e| SerializeError::Xml(e.to_string()))?;
                if let Some(context) = &def.context {
                    write!(xml, r#" context="{}""#, self.escape_xml(context))
                        .map_err(|e| SerializeError::Xml(e.to_string()))?;
                }
                if let Some(access) = &def.access_level {
                    write!(xml, r#" accessLevel="{:?}""#, access)
                        .map_err(|e| SerializeError::Xml(e.to_string()))?;
                }
                writeln!(xml, ">").map_err(|e| SerializeError::Xml(e.to_string()))?;
                // Expression would be serialized here - simplified for now
                if def.expression.is_some() {
                    self.write_indent(&mut xml, 3)?;
                    writeln!(xml, "<expression xsi:type=\"Null\"/>")
                        .map_err(|e| SerializeError::Xml(e.to_string()))?;
                }
                self.write_indent(&mut xml, 2)?;
                writeln!(xml, "</def>").map_err(|e| SerializeError::Xml(e.to_string()))?;
            }
            self.write_indent(&mut xml, 1)?;
            writeln!(xml, "</statements>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        }

        // Close library
        writeln!(xml, "</library>").map_err(|e| SerializeError::Xml(e.to_string()))?;

        Ok(xml)
    }

    fn write_indent(&self, xml: &mut String, level: usize) -> Result<(), SerializeError> {
        use std::fmt::Write;
        if self.pretty {
            for _ in 0..level {
                write!(xml, "   ").map_err(|e| SerializeError::Xml(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn write_element<F>(&self, xml: &mut String, level: usize, name: &str, attrs: F) -> Result<(), SerializeError>
    where
        F: FnOnce(&mut String) -> std::fmt::Result,
    {
        use std::fmt::Write;
        self.write_indent(xml, level)?;
        write!(xml, "<{}", name).map_err(|e| SerializeError::Xml(e.to_string()))?;
        attrs(xml).map_err(|e| SerializeError::Xml(e.to_string()))?;
        writeln!(xml, "/>").map_err(|e| SerializeError::Xml(e.to_string()))?;
        Ok(())
    }

    fn escape_xml(&self, s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

impl ElmSerializer for XmlSerializer {
    fn serialize(&self, library: &Library) -> Result<String, SerializeError> {
        self.library_to_xml(library)
    }

    fn serialize_to_writer<W: Write>(
        &self,
        library: &Library,
        mut writer: W,
    ) -> Result<(), SerializeError> {
        let xml = self.serialize(library)?;
        writer.write_all(xml.as_bytes())?;
        Ok(())
    }

    fn deserialize(&self, _input: &str) -> Result<Library, SerializeError> {
        // XML deserialization is more complex and would require quick-xml parsing
        // For now, we provide a stub implementation
        Err(SerializeError::Xml(
            "XML deserialization not yet implemented".to_string(),
        ))
    }

    fn deserialize_from_reader<R: Read>(&self, _reader: R) -> Result<Library, SerializeError> {
        Err(SerializeError::Xml(
            "XML deserialization not yet implemented".to_string(),
        ))
    }
}

/// Convenience functions for quick serialization
impl Library {
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, SerializeError> {
        JsonSerializer::new().serialize(self)
    }

    /// Serialize to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, SerializeError> {
        JsonSerializer::pretty().serialize(self)
    }

    /// Serialize to XML string
    pub fn to_xml(&self) -> Result<String, SerializeError> {
        XmlSerializer::new().serialize(self)
    }

    /// Serialize to pretty-printed XML string
    pub fn to_xml_pretty(&self) -> Result<String, SerializeError> {
        XmlSerializer::pretty().serialize(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, SerializeError> {
        JsonSerializer::new().deserialize(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_serialization() {
        let library = Library::new("TestLibrary", Some("1.0.0"));
        let json_serializer = JsonSerializer::new();

        let json = json_serializer.serialize(&library).unwrap();
        assert!(json.contains("TestLibrary"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_json_pretty_serialization() {
        let library = Library::new("TestLibrary", Some("1.0.0"));
        let json_serializer = JsonSerializer::pretty();

        let json = json_serializer.serialize(&library).unwrap();
        // Pretty format has newlines
        assert!(json.contains('\n'));
    }

    #[test]
    fn test_xml_serialization() {
        let library = Library::new("TestLibrary", Some("1.0.0"));
        let xml_serializer = XmlSerializer::pretty();

        let xml = xml_serializer.serialize(&library).unwrap();
        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<library"));
        assert!(xml.contains("TestLibrary"));
        assert!(xml.contains("1.0.0"));
    }

    #[test]
    fn test_library_convenience_methods() {
        let library = Library::new("TestLibrary", Some("1.0.0"));

        let json = library.to_json().unwrap();
        assert!(json.contains("TestLibrary"));

        let xml = library.to_xml().unwrap();
        assert!(xml.contains("<library"));
    }

    #[test]
    fn test_json_roundtrip() {
        let library = Library::new("TestLibrary", Some("1.0.0"));

        let json = library.to_json_pretty().unwrap();
        let parsed = Library::from_json(&json).unwrap();

        assert_eq!(parsed.identifier.id, "TestLibrary");
        assert_eq!(parsed.identifier.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_xml_escape() {
        let serializer = XmlSerializer::new();
        assert_eq!(serializer.escape_xml("a < b"), "a &lt; b");
        assert_eq!(serializer.escape_xml("a & b"), "a &amp; b");
        assert_eq!(serializer.escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
