//! ELM serialization (JSON)
//!
//! Libraries are written in the HL7 ELM JSON envelope `{ "library": { ... } }`
//! and read either with or without that envelope.

use std::io::{Read, Write};

use crate::model::Library;
use serde::{Deserialize, Serialize};

/// Errors that can occur during serialization
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

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

/// The ELM JSON envelope.
#[derive(Serialize)]
struct Envelope<'a> {
    library: &'a Library,
}

#[derive(Deserialize)]
struct OwnedEnvelope {
    library: Library,
}

/// JSON serializer for ELM
///
/// Produces the HL7 ELM JSON format (`{ "library": { ... } }`).
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
        let envelope = Envelope { library };
        Ok(if self.pretty {
            serde_json::to_string_pretty(&envelope)?
        } else {
            serde_json::to_string(&envelope)?
        })
    }

    fn serialize_to_writer<W: Write>(
        &self,
        library: &Library,
        mut writer: W,
    ) -> Result<(), SerializeError> {
        let envelope = Envelope { library };
        if self.pretty {
            serde_json::to_writer_pretty(&mut writer, &envelope)?;
        } else {
            serde_json::to_writer(&mut writer, &envelope)?;
        }
        Ok(())
    }

    fn deserialize(&self, input: &str) -> Result<Library, SerializeError> {
        let value: serde_json::Value = serde_json::from_str(input)?;
        if value.get("library").is_some() {
            Ok(serde_json::from_value::<OwnedEnvelope>(value)?.library)
        } else {
            Ok(serde_json::from_value(value)?)
        }
    }

    fn deserialize_from_reader<R: Read>(&self, mut reader: R) -> Result<Library, SerializeError> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        self.deserialize(&content)
    }
}

/// Convenience functions for quick serialization
impl Library {
    /// Serialize to ELM JSON
    pub fn to_json(&self) -> Result<String, SerializeError> {
        JsonSerializer::new().serialize(self)
    }

    /// Serialize to pretty-printed ELM JSON
    pub fn to_json_pretty(&self) -> Result<String, SerializeError> {
        JsonSerializer::pretty().serialize(self)
    }

    /// Deserialize from ELM JSON (with or without the `library` envelope)
    pub fn from_json(json: &str) -> Result<Self, SerializeError> {
        JsonSerializer::new().deserialize(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_serialization_uses_envelope() {
        let library = Library::new("TestLibrary", Some("1.0.0"));
        let json: serde_json::Value =
            serde_json::from_str(&JsonSerializer::new().serialize(&library).unwrap()).unwrap();
        assert_eq!(json["library"]["identifier"]["id"], "TestLibrary");
        assert_eq!(json["library"]["identifier"]["version"], "1.0.0");
    }

    #[test]
    fn test_json_pretty_serialization() {
        let library = Library::new("TestLibrary", Some("1.0.0"));
        let json = JsonSerializer::pretty().serialize(&library).unwrap();
        assert!(json.contains('\n'));
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
    fn test_json_without_envelope() {
        let library = Library::new("Bare", None::<String>);
        let bare = serde_json::to_string(&library).unwrap();
        assert_eq!(Library::from_json(&bare).unwrap().identifier.id, "Bare");
    }
}
