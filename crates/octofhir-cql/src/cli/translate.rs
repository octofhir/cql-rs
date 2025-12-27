//! Translate command implementation

use super::{output, resolver};
use anyhow::{Context, Result};
use crate::elm::serialize::ElmSerializer;
use std::fs;
use std::path::PathBuf;

/// Configuration for translate command
pub struct TranslateConfig {
    pub file: PathBuf,
    pub format: String,
    pub annotations: bool,
    pub pretty: bool,
    pub library_paths: Vec<PathBuf>,
    pub output_file: Option<PathBuf>,
}

/// Translate CQL to ELM
pub async fn translate(config: TranslateConfig) -> Result<()> {
    // Set up library resolver
    let _resolver = resolver::LibraryResolver::new(config.library_paths);

    // Load the CQL file
    let cql_content = fs::read_to_string(&config.file)
        .with_context(|| format!("Failed to read CQL file: {}", config.file.display()))?;

    // Parse the CQL library
    let library = crate::parser::parse(&cql_content)
        .with_context(|| format!("Failed to parse CQL file: {}", config.file.display()))?;

    // Convert to ELM
    let mut converter = crate::elm::converter::AstToElmConverter::new();
    let elm_library = converter.convert_library(&library);

    // Serialize based on format
    let output_content = match config.format.to_lowercase().as_str() {
        "json" => {
            let mut serializer = crate::elm::serialize::JsonSerializer::new();
            serializer.pretty = config.pretty;
            serializer.serialize(&elm_library)
                .map_err(|e| anyhow::anyhow!("JSON serialization failed: {}", e))?
        }
        "xml" => {
            let mut serializer = crate::elm::serialize::XmlSerializer::new();
            serializer.pretty = config.pretty;
            serializer.serialize(&elm_library)
                .map_err(|e| anyhow::anyhow!("XML serialization failed: {}", e))?
        }
        other => {
            anyhow::bail!("Unsupported output format: {}. Use 'json' or 'xml'", other);
        }
    };

    // Write output
    output::write_output(&output_content, config.output_file.as_deref())?;

    Ok(())
}
