//! `cql repl`: evaluate CQL expressions interactively.
//!
//! Each input is either a `define` (kept for the rest of the session) or an
//! expression, which is evaluated together with the definitions entered so
//! far. With `--data` and `--patient` the session runs in `Patient` context.

use super::load_data;
use crate::eval::fhir::InMemoryFhirData;
use crate::eval::{DataProvider, TerminologyProvider};
use crate::model::fhir::json::cql_to_json;
use crate::types::CqlValue;
use crate::{CompiledLibrary, DirectoryLibrarySource};
use anyhow::{Result, anyhow};
use colored::Colorize;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::path::PathBuf;
use std::sync::Arc;

/// Name of the definition that holds the expression being evaluated.
const RESULT: &str = "__repl_result";

/// Options of `cql repl`.
pub struct ReplOptions {
    pub data: Vec<PathBuf>,
    pub patient: Option<String>,
    pub library_paths: Vec<PathBuf>,
}

/// An interactive session: accumulated definitions plus the data context.
pub struct Session {
    definitions: Vec<String>,
    data: Arc<InMemoryFhirData>,
    patient: Option<CqlValue>,
    libraries: DirectoryLibrarySource,
}

impl Session {
    /// Create a session.
    pub fn new(options: &ReplOptions) -> Result<Self> {
        let data = Arc::new(load_data(&options.data)?);
        let patient = match &options.patient {
            Some(id) => Some(
                data.resources("Patient")
                    .iter()
                    .find(|p| matches!(p, CqlValue::Tuple(t) if t.get("id").and_then(CqlValue::as_string) == Some(id)))
                    .cloned()
                    .ok_or_else(|| anyhow!("no Patient with id `{id}` in --data"))?,
            ),
            None => None,
        };
        Ok(Self {
            definitions: Vec::new(),
            data,
            patient,
            libraries: DirectoryLibrarySource::new(options.library_paths.clone()),
        })
    }

    fn source(&self, extra: Option<&str>) -> String {
        let mut source = String::from("library Repl\nusing FHIR version '4.0.1'\n");
        if self.patient.is_some() {
            source.push_str("context Patient\n");
        }
        for definition in self.definitions.iter().map(String::as_str).chain(extra) {
            source.push_str(definition);
            source.push('\n');
        }
        source
    }

    fn compile(&self, extra: Option<&str>) -> Result<CompiledLibrary> {
        let library = CompiledLibrary::compile_with(&self.source(extra), &self.libraries)
            .map_err(|e| anyhow!("{e}"))?;
        if let Some(diagnostic) = library.diagnostics().first() {
            return Err(anyhow!("{diagnostic}"));
        }
        Ok(library)
    }

    /// Handle one line of input; returns the text to print.
    pub fn eval_line(&mut self, line: &str) -> Result<String> {
        let line = line.trim();
        if line.starts_with("define ") || line.starts_with("include ") {
            let statement = line.to_string();
            self.definitions.push(statement);
            if let Err(e) = self.compile(None) {
                self.definitions.pop();
                return Err(e);
            }
            return Ok("ok".to_string());
        }

        let library = self.compile(Some(&format!("define \"{RESULT}\": {line}")))?;
        let mut ctx = library
            .context()
            .with_data_provider(self.data.clone() as Arc<dyn DataProvider>)
            .with_terminology_provider(self.data.clone() as Arc<dyn TerminologyProvider>);
        if let Some(patient) = &self.patient {
            ctx = ctx.with_context("Patient", patient.clone());
        }
        library.register(&mut ctx);
        let value = library
            .evaluate_definition(RESULT, &mut ctx)
            .map_err(|e| anyhow!("{e}"))?;
        Ok(serde_json::to_string(&cql_to_json(&value))?)
    }
}

/// Run the interactive loop on the terminal.
pub fn run(options: &ReplOptions) -> Result<()> {
    let mut session = Session::new(options)?;
    let mut editor = DefaultEditor::new()?;
    println!(
        "CQL REPL. Enter an expression or a `define`; {} for help, {} to quit.",
        ":help".green(),
        ":quit".green()
    );
    loop {
        match editor.readline("cql> ") {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let _ = editor.add_history_entry(line);
                match line {
                    ":quit" | ":q" | ":exit" => break,
                    ":help" | ":h" => {
                        println!("  <expression>          evaluate an expression");
                        println!("  define Name: <expr>   add a definition to the session");
                        println!("  :defs                 list the session definitions");
                        println!("  :clear                remove all definitions");
                        println!("  :quit                 leave");
                    }
                    ":defs" => {
                        for definition in &session.definitions {
                            println!("  {definition}");
                        }
                    }
                    ":clear" => session.definitions.clear(),
                    _ => match session.eval_line(line) {
                        Ok(output) => println!("{output}"),
                        Err(e) => eprintln!("{} {e}", "error:".red().bold()),
                    },
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
