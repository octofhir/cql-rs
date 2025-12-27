//! Validate command implementation

use super::{output, resolver};
use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::PathBuf;

/// Configuration for validate command
pub struct ValidateConfig {
    pub files: Vec<PathBuf>,
    pub strict: bool,
    pub library_paths: Vec<PathBuf>,
    pub verbose: bool,
}

/// Validation result for a single file
struct ValidationResult {
    file: PathBuf,
    success: bool,
    errors: Vec<DiagnosticMessage>,
    warnings: Vec<DiagnosticMessage>,
}

/// Diagnostic message with location
struct DiagnosticMessage {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

impl DiagnosticMessage {
    fn new(message: String) -> Self {
        Self {
            message,
            line: None,
            column: None,
        }
    }

    fn with_location(message: String, line: usize, column: usize) -> Self {
        Self {
            message,
            line: Some(line),
            column: Some(column),
        }
    }
}

/// Validate CQL files
pub async fn validate(config: ValidateConfig) -> Result<()> {
    if config.files.is_empty() {
        anyhow::bail!("No files specified for validation");
    }

    // Set up library resolver
    let _resolver = resolver::LibraryResolver::new(config.library_paths);

    let mut all_results = Vec::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;

    // Validate each file
    for file in &config.files {
        let result = validate_file(file, config.verbose).await?;

        total_errors += result.errors.len();
        total_warnings += result.warnings.len();

        all_results.push(result);
    }

    // Print results
    for result in &all_results {
        print_validation_result(result);
    }

    // Print summary
    println!();
    if total_errors == 0 && total_warnings == 0 {
        println!("{}", output::format_success(&format!(
            "All {} file(s) validated successfully",
            config.files.len()
        )));
        Ok(())
    } else {
        let mut summary = Vec::new();

        if total_errors > 0 {
            summary.push(format!("{} error(s)", total_errors).red().to_string());
        }

        if total_warnings > 0 {
            summary.push(format!("{} warning(s)", total_warnings).yellow().to_string());
        }

        eprintln!(
            "{} Found {}",
            "Validation failed:".red().bold(),
            summary.join(", ")
        );

        if config.strict && total_warnings > 0 {
            eprintln!("{}", "Strict mode: treating warnings as errors".yellow());
            std::process::exit(1);
        }

        if total_errors > 0 {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Validate a single file
async fn validate_file(file: &PathBuf, verbose: bool) -> Result<ValidationResult> {
    if verbose {
        eprintln!("Validating: {}", file.display());
    }

    let mut result = ValidationResult {
        file: file.clone(),
        success: true,
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    // Load the file
    let cql_content = match fs::read_to_string(file) {
        Ok(content) => content,
        Err(e) => {
            result.success = false;
            result.errors.push(DiagnosticMessage::new(
                format!("Failed to read file: {}", e)
            ));
            return Ok(result);
        }
    };

    // Parse the CQL
    match crate::parser::parse(&cql_content) {
        Ok(library) => {
            if verbose {
                if let Some(def) = &library.definition {
                    eprintln!(
                        "  Parsed library: {} version {}",
                        def.name.name.name,
                        def.version.as_ref().map(|v| v.version.as_str()).unwrap_or("(no version)")
                    );
                }
            }

            // TODO: Add semantic validation when type checker is ready
            // For now, just check for basic issues

            // Check for empty library
            if library.statements.is_empty() {
                result.warnings.push(DiagnosticMessage::new(
                    "Library contains no statements".to_string()
                ));
            }

            // Check for duplicate definition names
            let mut seen_names = std::collections::HashSet::new();
            for stmt in &library.statements {
                use crate::ast::Statement;
                let name = match &stmt.inner {
                    Statement::ExpressionDef(def) => &def.name.name,
                    Statement::FunctionDef(def) => &def.name.name,
                };
                if !seen_names.insert(name.clone()) {
                    result.warnings.push(DiagnosticMessage::new(
                        format!("Duplicate definition name: {}", name)
                    ));
                }
            }
        }
        Err(e) => {
            result.success = false;

            // Try to extract location information from error
            // This is a simplified version - proper implementation would
            // need to enhance the parser to provide span information
            let error_msg = format!("{}", e);
            result.errors.push(DiagnosticMessage::new(error_msg));
        }
    }

    Ok(result)
}

/// Print validation result for a file
fn print_validation_result(result: &ValidationResult) {
    let status = if result.success {
        "✓".green().bold()
    } else {
        "✗".red().bold()
    };

    println!("{} {}", status, result.file.display().to_string().cyan());

    // Print errors
    for error in &result.errors {
        print_diagnostic("error", error, &result.file);
    }

    // Print warnings
    for warning in &result.warnings {
        print_diagnostic("warning", warning, &result.file);
    }
}

/// Print a diagnostic message
fn print_diagnostic(level: &str, diag: &DiagnosticMessage, file: &PathBuf) {
    let level_str = match level {
        "error" => "error".red().bold(),
        "warning" => "warning".yellow().bold(),
        _ => level.normal(),
    };

    if let (Some(line), Some(col)) = (diag.line, diag.column) {
        println!(
            "  {} {}: {}",
            level_str,
            output::format_location(&file.display().to_string(), line, col),
            diag.message
        );
    } else {
        println!("  {}: {}", level_str, diag.message);
    }
}
