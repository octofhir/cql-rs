//! Implementation of the `cql` command-line tool (feature `cli`).
//!
//! - `cql parse` prints the AST as JSON
//! - `cql translate` prints the ELM (JSON)
//! - `cql validate` reports syntax and name-resolution errors
//! - `cql eval` evaluates a library, optionally per patient against FHIR data
//! - `cql repl` evaluates expressions interactively

pub mod eval;
pub mod repl;

use crate::CompiledLibrary;
use crate::diagnostics::{CqlError, SourceLocation};
use crate::elm::AstToElmConverter;
use crate::eval::fhir::InMemoryFhirData;
use crate::eval::{CqlEngine, EvaluationContext};
use crate::parser::{ParseMode, parse_expression, parse_with_mode};
use crate::types::CqlValue;
use anyhow::{Context, Result, anyhow, bail};
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Write `content` to `output` (or stdout), followed by a newline.
pub fn write_output(content: &str, output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => fs::write(path, format!("{content}\n"))
            .with_context(|| format!("cannot write {}", path.display())),
        None => {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{content}")?;
            Ok(())
        }
    }
}

/// Serialize as JSON, pretty unless `compact`.
pub fn to_json<T: serde::Serialize>(value: &T, compact: bool) -> Result<String> {
    Ok(if compact {
        serde_json::to_string(value)?
    } else {
        serde_json::to_string_pretty(value)?
    })
}

fn read_source(file: &Path) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))
}

/// `cql parse`: print the AST.
pub fn parse(file: &Path, debug: bool, compact: bool, output: Option<&Path>) -> Result<bool> {
    let source = read_source(file)?;
    let result = parse_with_mode(&source, ParseMode::Analysis);
    if !result.errors.is_empty() {
        for error in &result.errors {
            eprintln!(
                "{}",
                render_diagnostic(file, &source, error, Severity::Error)
            );
        }
        return Ok(false);
    }
    let library = result
        .library
        .ok_or_else(|| anyhow!("parser returned no library"))?;
    let content = if debug {
        format!("{library:#?}")
    } else {
        to_json(&library, compact)?
    };
    write_output(&content, output)?;
    Ok(true)
}

/// `cql translate`: print the ELM of a library (ELM JSON, `{ "library": ... }`).
pub fn translate(
    file: &Path,
    library_paths: &[PathBuf],
    compact: bool,
    output: Option<&Path>,
) -> Result<bool> {
    let source = read_source(file)?;
    let library = match CompiledLibrary::compile_file(file, library_paths) {
        Ok(library) => library,
        Err(error) => {
            eprintln!(
                "{}",
                render_diagnostic(file, &source, &error, Severity::Error)
            );
            return Ok(false);
        }
    };
    for diagnostic in library.diagnostics() {
        eprintln!(
            "{}",
            render_diagnostic(file, &source, diagnostic, Severity::Warning)
        );
    }
    let elm = if compact {
        library.elm().to_json()?
    } else {
        library.elm().to_json_pretty()?
    };
    write_output(&elm, output)?;
    Ok(true)
}

/// `cql validate`: report errors for each file; `Ok(false)` if any were found.
pub fn validate(files: &[PathBuf], library_paths: &[PathBuf], quiet: bool) -> Result<bool> {
    if files.is_empty() {
        bail!("no files to validate");
    }
    let mut failed = 0usize;
    for file in files {
        let source = read_source(file)?;
        let parsed = parse_with_mode(&source, ParseMode::Analysis);
        let mut errors: Vec<CqlError> = parsed.errors;
        if errors.is_empty() {
            match CompiledLibrary::compile_file(file, library_paths) {
                Ok(library) => errors.extend(library.diagnostics().iter().cloned()),
                Err(error) => errors.push(error),
            }
        }
        if errors.is_empty() {
            if !quiet {
                println!("{} {}", "ok".green().bold(), file.display());
            }
        } else {
            failed += 1;
            for error in &errors {
                eprintln!(
                    "{}",
                    render_diagnostic(file, &source, error, Severity::Error)
                );
            }
            eprintln!(
                "{} {}: {} error(s)",
                "failed".red().bold(),
                file.display(),
                errors.len()
            );
        }
    }
    Ok(failed == 0)
}

/// Diagnostic severity for rendering.
#[derive(Clone, Copy)]
pub enum Severity {
    Error,
    Warning,
}

/// `file:line:col: error[CQLnnnn]: message`, followed by the source line and
/// a caret when the location is known.
pub fn render_diagnostic(
    file: &Path,
    source: &str,
    error: &CqlError,
    severity: Severity,
) -> String {
    if let CqlError::Multiple(errors) = error {
        return errors
            .iter()
            .map(|e| render_diagnostic(file, source, e, severity))
            .collect::<Vec<_>>()
            .join("\n");
    }
    let label = match severity {
        Severity::Error => format!("error[{}]", error.code()).red().bold(),
        Severity::Warning => format!("warning[{}]", error.code()).yellow().bold(),
    };
    let message = diagnostic_message(error);
    match error.location() {
        Some(location) => {
            let mut out = format!(
                "{}:{}:{}: {label}: {message}",
                file.display(),
                location.line,
                location.column
            );
            if let Some(snippet) = snippet(source, location) {
                out.push('\n');
                out.push_str(&snippet);
            }
            out
        }
        None => format!("{}: {label}: {message}", file.display()),
    }
}

fn diagnostic_message(error: &CqlError) -> String {
    match error {
        CqlError::Parse { message, .. }
        | CqlError::Semantic { message, .. }
        | CqlError::Evaluation { message, .. }
        | CqlError::Model { message, .. }
        | CqlError::System { message, .. } => message.clone(),
        CqlError::Multiple(errors) => format!("{} errors", errors.len()),
    }
}

fn snippet(source: &str, location: &SourceLocation) -> Option<String> {
    let line = source.lines().nth(location.line.checked_sub(1)?)?;
    let gutter = location.line.to_string();
    let pad = " ".repeat(gutter.len());
    let caret_offset: String = line
        .chars()
        .take(location.column.saturating_sub(1))
        .map(|c| if c == '\t' { '\t' } else { ' ' })
        .collect();
    let width = location.length.max(1);
    Some(format!(
        "{pad} |\n{gutter} | {line}\n{pad} | {caret_offset}{}",
        "^".repeat(width).red().bold()
    ))
}

/// Load FHIR data from JSON (resource, Bundle, array) or NDJSON files.
pub fn load_data(files: &[PathBuf]) -> Result<InMemoryFhirData> {
    let mut data = InMemoryFhirData::new();
    for file in files {
        let text = read_source(file)?;
        let is_ndjson = file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ndjson"));
        let added = if is_ndjson {
            data.add_ndjson(&text)
                .with_context(|| format!("invalid NDJSON in {}", file.display()))?
        } else {
            let json: serde_json::Value = serde_json::from_str(&text)
                .with_context(|| format!("invalid JSON in {}", file.display()))?;
            data.add_json(&json)
        };
        if added == 0 {
            eprintln!(
                "{} {} contains no FHIR resources",
                "warning:".yellow().bold(),
                file.display()
            );
        }
    }
    Ok(data)
}

/// Parse `name=<CQL expression>` parameter assignments and evaluate the
/// expressions (`"Measurement Period=Interval[@2024-01-01, @2025-01-01)"`).
pub fn parse_parameters(assignments: &[String]) -> Result<Vec<(String, CqlValue)>> {
    let engine = CqlEngine::new();
    let converter = AstToElmConverter::new();
    assignments
        .iter()
        .map(|assignment| {
            let (name, expression) = assignment.split_once('=').ok_or_else(|| {
                anyhow!("invalid parameter `{assignment}`: expected NAME=EXPRESSION")
            })?;
            let name = name.trim().trim_matches('"').to_string();
            let ast = parse_expression(expression.trim())
                .map_err(|e| anyhow!("invalid value for parameter `{name}`: {e}"))?;
            let elm = converter.convert_expression(&ast.inner);
            let value = engine
                .evaluate(&elm, &mut EvaluationContext::new())
                .map_err(|e| anyhow!("cannot evaluate parameter `{name}`: {e}"))?;
            Ok((name, value))
        })
        .collect()
}
