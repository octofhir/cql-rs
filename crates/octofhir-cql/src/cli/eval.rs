//! `cql eval`: evaluate a library's definitions.

use super::{Severity, load_data, parse_parameters, render_diagnostic, to_json, write_output};
use crate::CompiledLibrary;
use crate::eval::fhir::InMemoryFhirData;
use crate::eval::{DataProvider, EvaluationContext, TerminologyProvider};
use crate::model::fhir::json::cql_to_json;
use crate::types::CqlValue;
use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, FixedOffset};
use colored::Colorize;
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Options of `cql eval`.
pub struct EvalOptions {
    pub file: PathBuf,
    pub data: Vec<PathBuf>,
    pub params: Vec<String>,
    pub expressions: Vec<String>,
    pub patients: Vec<String>,
    pub library_paths: Vec<PathBuf>,
    pub now: Option<String>,
    pub compact: bool,
    pub output: Option<PathBuf>,
}

/// Evaluate and print the results as JSON. `Ok(false)` if the library does
/// not compile or evaluation fails.
pub fn run(options: &EvalOptions) -> Result<bool> {
    let source = std::fs::read_to_string(&options.file)
        .with_context(|| format!("cannot read {}", options.file.display()))?;
    let library = match CompiledLibrary::compile_file(&options.file, &options.library_paths) {
        Ok(library) => library,
        Err(error) => {
            eprintln!(
                "{}",
                render_diagnostic(&options.file, &source, &error, Severity::Error)
            );
            return Ok(false);
        }
    };
    if !library.diagnostics().is_empty() {
        for diagnostic in library.diagnostics() {
            eprintln!(
                "{}",
                render_diagnostic(&options.file, &source, diagnostic, Severity::Error)
            );
        }
        return Ok(false);
    }

    let data = Arc::new(load_data(&options.data)?);
    if !options.data.is_empty() {
        warn_missing_value_sets(&library, &data);
    }
    let parameters = parse_parameters(&options.params)?;
    let now = options.now.as_deref().map(parse_now).transpose()?;

    let patient_context = library
        .elm()
        .contexts
        .as_ref()
        .is_some_and(|c| c.defs.iter().any(|d| d.name == "Patient"));

    let evaluate = |context: Option<CqlValue>| -> Result<Value> {
        let mut ctx = match now {
            Some(now) => EvaluationContext::with_timestamp(now),
            None => EvaluationContext::new(),
        }
        .with_data_provider(data.clone() as Arc<dyn DataProvider>)
        .with_terminology_provider(data.clone() as Arc<dyn TerminologyProvider>);
        if let Some(patient) = context {
            ctx = ctx.with_context("Patient", patient);
        }
        library.register(&mut ctx);
        for (name, value) in &parameters {
            ctx.set_parameter(name.clone(), value.clone());
        }
        evaluate_selected(&library, &options.expressions, &mut ctx)
    };

    let result = if patient_context && !data.resources("Patient").is_empty() {
        let mut by_patient = Map::new();
        for patient in select_patients(&data, &options.patients)? {
            let id = patient_id(&patient).unwrap_or_default().to_string();
            let results =
                evaluate(Some(patient)).with_context(|| format!("evaluating patient `{id}`"))?;
            by_patient.insert(id, results);
        }
        Value::Object(by_patient)
    } else {
        if !options.patients.is_empty() {
            bail!(
                "--patient requires a library with `context Patient` and Patient resources in --data"
            );
        }
        evaluate(None)?
    };

    write_output(
        &to_json(&result, options.compact)?,
        options.output.as_deref(),
    )?;
    Ok(true)
}

/// Evaluate the requested definitions (all public ones when none requested).
fn evaluate_selected(
    library: &CompiledLibrary,
    expressions: &[String],
    ctx: &mut EvaluationContext,
) -> Result<Value> {
    let mut out = Map::new();
    if expressions.is_empty() {
        for (name, value) in library.evaluate(ctx).map_err(|e| anyhow!("{e}"))? {
            out.insert(name, cql_to_json(&value));
        }
    } else {
        for name in expressions {
            let value = library
                .evaluate_definition(name, ctx)
                .map_err(|e| anyhow!("`{name}`: {e}"))?;
            out.insert(name.clone(), cql_to_json(&value));
        }
    }
    Ok(Value::Object(out))
}

fn patient_id(patient: &CqlValue) -> Option<&str> {
    match patient {
        CqlValue::Tuple(t) => t.get("id").and_then(CqlValue::as_string),
        _ => None,
    }
}

/// All loaded patients, or the ones named by `--patient`.
fn select_patients(data: &InMemoryFhirData, ids: &[String]) -> Result<Vec<CqlValue>> {
    let patients = data.resources("Patient");
    if ids.is_empty() {
        return Ok(patients.to_vec());
    }
    ids.iter()
        .map(|id| {
            patients
                .iter()
                .find(|p| patient_id(p) == Some(id))
                .cloned()
                .ok_or_else(|| anyhow!("no Patient with id `{id}` in --data"))
        })
        .collect()
}

/// Warn about declared value sets that the loaded data does not define.
fn warn_missing_value_sets(library: &CompiledLibrary, data: &InMemoryFhirData) {
    for id in library.value_set_ids() {
        if !data.has_value_set(&id) {
            eprintln!(
                "{} value set {id} is not in the loaded data: retrieves filtered by it return no resources and membership tests return null",
                "warning:".yellow().bold()
            );
        }
    }
}

/// `--now`: an RFC 3339 timestamp, or a date (midnight UTC).
fn parse_now(text: &str) -> Result<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(text)
        .or_else(|_| DateTime::parse_from_rfc3339(&format!("{text}T00:00:00Z")))
        .map_err(|_| anyhow!("invalid --now `{text}`: expected e.g. 2024-06-30T12:00:00Z"))
}
