//! Execute command implementation

use super::{output, resolver};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Configuration for execute command
pub struct ExecuteConfig {
    pub file: PathBuf,
    pub params: Vec<String>,
    pub data: Option<PathBuf>,
    pub library_paths: Vec<PathBuf>,
    pub verbose: bool,
    pub output_format: Option<String>,
    pub output_file: Option<PathBuf>,
}

/// Execute a CQL file
pub async fn execute(config: ExecuteConfig) -> Result<()> {
    if config.verbose {
        eprintln!("Executing CQL file: {}", config.file.display());
    }

    // Set up library resolver
    let resolver = resolver::LibraryResolver::new(config.library_paths);

    if config.verbose {
        eprintln!("Library search paths:");
        for path in resolver.search_paths() {
            eprintln!("  - {}", path.display());
        }
    }

    // Load the CQL file
    let cql_content = fs::read_to_string(&config.file)
        .with_context(|| format!("Failed to read CQL file: {}", config.file.display()))?;

    if config.verbose {
        eprintln!("Loaded {} bytes from {}", cql_content.len(), config.file.display());
    }

    // Parse parameters
    let params = parse_parameters(&config.params)?;

    if config.verbose && !params.is_empty() {
        eprintln!("Parameters:");
        for (name, value) in &params {
            eprintln!("  {} = {}", name, value);
        }
    }

    // Load context data if provided
    let context_data = if let Some(data_path) = &config.data {
        let data_content = fs::read_to_string(data_path)
            .with_context(|| format!("Failed to read data file: {}", data_path.display()))?;

        let data: Value = serde_json::from_str(&data_content)
            .with_context(|| format!("Failed to parse data file: {}", data_path.display()))?;

        if config.verbose {
            eprintln!("Loaded context data from {}", data_path.display());
        }

        Some(data)
    } else {
        None
    };

    // Parse the CQL library
    let library = crate::parser::parse(&cql_content)
        .with_context(|| format!("Failed to parse CQL file: {}", config.file.display()))?;

    if config.verbose {
        if let Some(def) = &library.definition {
            eprintln!("Parsed library: {} version {}",
                def.name.name.name,
                def.version.as_ref().map(|v| v.version.as_str()).unwrap_or("(no version)")
            );
        }
    }

    // For now, create a mock result since evaluation is not yet implemented
    // TODO: Replace with actual evaluation once octofhir-cql-eval is complete
    let lib_info = if let Some(def) = &library.definition {
        json!({
            "name": def.name.name.name,
            "version": def.version.as_ref().map(|v| v.version.as_str()),
        })
    } else {
        json!({
            "name": "(unnamed)",
            "version": null,
        })
    };

    let result = json!({
        "library": lib_info,
        "parameters": params,
        "contextData": context_data,
        "results": {
            "note": "Evaluation engine not yet implemented",
            "expressions": library.statements.len(),
        }
    });

    // Format and output results
    let format = output::OutputFormat::from_str(
        config.output_format.as_deref().unwrap_or("pretty")
    );

    output::print_output(&result, format, config.output_file.as_deref())?;

    if config.verbose {
        eprintln!("{}", output::format_success("Execution completed"));
    }

    Ok(())
}

/// Parse parameter strings (name=value) into a map
fn parse_parameters(params: &[String]) -> Result<HashMap<String, Value>> {
    let mut result = HashMap::new();

    for param in params {
        let parts: Vec<&str> = param.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid parameter format: '{}'. Expected 'name=value'", param);
        }

        let name = parts[0].trim().to_string();
        let value_str = parts[1].trim();

        // Try to parse as JSON value
        let value = if value_str.starts_with('@') {
            // Date/time literal
            json!(value_str)
        } else if let Ok(num) = value_str.parse::<i64>() {
            json!(num)
        } else if let Ok(num) = value_str.parse::<f64>() {
            json!(num)
        } else if value_str == "true" || value_str == "false" {
            json!(value_str == "true")
        } else if value_str == "null" {
            Value::Null
        } else if value_str.starts_with('{') || value_str.starts_with('[') {
            // Try to parse as JSON
            serde_json::from_str(value_str)
                .unwrap_or_else(|_| json!(value_str))
        } else {
            // Treat as string
            json!(value_str)
        };

        result.insert(name, value);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_parameters() {
        let params = vec![
            "name=John".to_string(),
            "age=30".to_string(),
            "active=true".to_string(),
            "score=98.5".to_string(),
            "date=@2024-01-01".to_string(),
        ];

        let result = parse_parameters(&params).unwrap();

        assert_eq!(result.get("name"), Some(&json!("John")));
        assert_eq!(result.get("age"), Some(&json!(30)));
        assert_eq!(result.get("active"), Some(&json!(true)));
        assert_eq!(result.get("score"), Some(&json!(98.5)));
        assert_eq!(result.get("date"), Some(&json!("@2024-01-01")));
    }

    #[test]
    fn test_parse_parameters_json() {
        let params = vec![
            r#"data={"key": "value"}"#.to_string(),
            "list=[1,2,3]".to_string(),
        ];

        let result = parse_parameters(&params).unwrap();

        assert_eq!(result.get("data"), Some(&json!({"key": "value"})));
        assert_eq!(result.get("list"), Some(&json!([1, 2, 3])));
    }

    #[test]
    fn test_parse_parameters_invalid() {
        let params = vec!["invalid".to_string()];
        assert!(parse_parameters(&params).is_err());
    }
}
