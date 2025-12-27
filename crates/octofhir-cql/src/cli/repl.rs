//! REPL implementation

use super::{output, resolver};
use anyhow::{Context, Result};
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for REPL
pub struct ReplConfig {
    pub model: String,
    pub version: Option<String>,
    pub library_paths: Vec<PathBuf>,
}

/// REPL state
struct ReplState {
    /// Library resolver
    resolver: resolver::LibraryResolver,
    /// Defined expressions
    definitions: HashMap<String, String>,
    /// Data model
    model: String,
    /// Model version
    version: Option<String>,
}

impl ReplState {
    fn new(config: ReplConfig) -> Self {
        Self {
            resolver: resolver::LibraryResolver::new(config.library_paths),
            definitions: HashMap::new(),
            model: config.model,
            version: config.version,
        }
    }
}

/// Run the interactive REPL
pub async fn run(config: ReplConfig) -> Result<()> {
    println!("{}", "CQL Interactive REPL".cyan().bold());
    println!("Type {} for help, {} to quit", ":help".green(), ":quit".green());
    println!("Model: {} {}", config.model, config.version.as_deref().unwrap_or("(default)"));
    println!();

    let mut state = ReplState::new(config);

    // Set up rustyline editor
    let mut rl = DefaultEditor::new()?;

    // Load history if it exists
    let history_file = dirs::home_dir()
        .map(|mut path| {
            path.push(".cql_history");
            path
        });

    if let Some(ref path) = history_file {
        let _ = rl.load_history(path);
    }

    loop {
        let readline = rl.readline("cql> ");

        match readline {
            Ok(line) => {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                // Handle commands
                if line.starts_with(':') {
                    match handle_command(line, &mut state).await {
                        Ok(false) => break, // :quit
                        Ok(true) => continue,
                        Err(e) => {
                            eprintln!("{}", output::format_error(&e));
                            continue;
                        }
                    }
                }

                // Handle define statements
                if line.starts_with("define ") {
                    match handle_define(line, &mut state) {
                        Ok(_) => {
                            println!("{}", output::format_success("Definition added"));
                        }
                        Err(e) => {
                            eprintln!("{}", output::format_error(&e));
                        }
                    }
                    continue;
                }

                // Evaluate expression
                match evaluate_expression(line, &state).await {
                    Ok(result) => {
                        println!("{}", result.green());
                    }
                    Err(e) => {
                        eprintln!("{}", output::format_error(&e));
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    if let Some(ref path) = history_file {
        let _ = rl.save_history(path);
    }

    println!("Goodbye!");
    Ok(())
}

/// Handle REPL commands (starting with :)
async fn handle_command(command: &str, state: &mut ReplState) -> Result<bool> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    match parts[0] {
        ":help" | ":h" => {
            print_help();
            Ok(true)
        }
        ":quit" | ":q" | ":exit" => {
            Ok(false)
        }
        ":clear" | ":c" => {
            state.definitions.clear();
            println!("{}", output::format_success("All definitions cleared"));
            Ok(true)
        }
        ":load" | ":l" => {
            if parts.len() < 2 {
                anyhow::bail!("Usage: :load <file.cql>");
            }
            let path = PathBuf::from(parts[1]);
            handle_load(&path, state).await?;
            Ok(true)
        }
        ":type" | ":t" => {
            if parts.len() < 2 {
                anyhow::bail!("Usage: :type <expression>");
            }
            let expr = parts[1..].join(" ");
            handle_type(&expr, state)?;
            Ok(true)
        }
        ":list" | ":ls" => {
            handle_list(state);
            Ok(true)
        }
        ":paths" => {
            println!("Library search paths:");
            for path in state.resolver.search_paths() {
                println!("  {}", path.display());
            }
            Ok(true)
        }
        other => {
            anyhow::bail!("Unknown command: {}. Type :help for help", other);
        }
    }
}

/// Handle define statements
fn handle_define(line: &str, state: &mut ReplState) -> Result<()> {
    // Parse: define Name: expression
    let without_define = line.strip_prefix("define ").unwrap();

    let parts: Vec<&str> = without_define.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid define syntax. Expected: define Name: expression");
    }

    let name = parts[0].trim().to_string();
    let expr = parts[1].trim().to_string();

    // TODO: Validate the expression
    // For now, just store it
    state.definitions.insert(name.clone(), expr);

    Ok(())
}

/// Evaluate an expression
async fn evaluate_expression(expr: &str, _state: &ReplState) -> Result<String> {
    // TODO: Implement actual evaluation
    // For now, just parse and return a placeholder

    // Try to parse as a simple literal
    if let Ok(num) = expr.parse::<i64>() {
        return Ok(num.to_string());
    }

    if let Ok(num) = expr.parse::<f64>() {
        return Ok(num.to_string());
    }

    if expr == "true" || expr == "false" {
        return Ok(expr.to_string());
    }

    if expr == "null" {
        return Ok("null".to_string());
    }

    // For complex expressions, show that evaluation is not yet implemented
    Ok(format!("(evaluation not yet implemented: {})", expr))
}

/// Load a library file
async fn handle_load(path: &PathBuf, state: &mut ReplState) -> Result<()> {
    let content = state.resolver.resolve_path(path)
        .with_context(|| format!("Failed to load library: {}", path.display()))?;

    // Parse the library
    let library = crate::parser::parse(&content)
        .with_context(|| "Failed to parse library")?;

    // Add all definitions to state
    use crate::ast::Statement;
    for stmt in &library.statements {
        let name = match &stmt.inner {
            Statement::ExpressionDef(def) => def.name.name.clone(),
            Statement::FunctionDef(def) => def.name.name.clone(),
        };
        state.definitions.insert(
            name,
            format!("(from {})", path.display())
        );
    }

    let lib_name = library.definition
        .as_ref()
        .map(|d| d.name.name.name.clone())
        .unwrap_or_else(|| "(unnamed)".to_string());
    let lib_version = library.definition
        .as_ref()
        .and_then(|d| d.version.as_ref())
        .map(|v| v.version.clone())
        .unwrap_or_else(|| "(no version)".to_string());

    println!(
        "{}",
        output::format_success(&format!(
            "Loaded library: {} version {} ({} definitions)",
            lib_name,
            lib_version,
            library.statements.len()
        ))
    );

    Ok(())
}

/// Show type of an expression
fn handle_type(expr: &str, _state: &ReplState) -> Result<()> {
    // TODO: Implement type inference
    // For now, show a placeholder
    println!("{}: {}", expr.cyan(), "(type inference not yet implemented)".yellow());
    Ok(())
}

/// List all definitions
fn handle_list(state: &ReplState) {
    if state.definitions.is_empty() {
        println!("No definitions");
        return;
    }

    println!("Definitions:");
    for (name, expr) in &state.definitions {
        println!("  {} = {}", name.cyan(), expr);
    }
}

/// Print help message
fn print_help() {
    println!("{}", "CQL REPL Commands:".bold());
    println!();
    println!("  {}  Show this help message", ":help, :h".green());
    println!("  {}  Quit the REPL", ":quit, :q, :exit".green());
    println!("  {}  Clear all definitions", ":clear, :c".green());
    println!("  {}  Load a library file", ":load <file>, :l <file>".green());
    println!("  {}  Show type of expression", ":type <expr>, :t <expr>".green());
    println!("  {}  List all definitions", ":list, :ls".green());
    println!("  {}  Show library search paths", ":paths".green());
    println!();
    println!("{}", "Expression Evaluation:".bold());
    println!();
    println!("  {}  Define a named expression", "define Name: expression".cyan());
    println!("  {}  Evaluate an expression", "expression".cyan());
    println!();
    println!("{}", "Examples:".bold());
    println!();
    println!("  {}", "define X: 1 + 2".cyan());
    println!("  {}", "X * 3".cyan());
    println!("  {}", ":type [1, 2, 3]".cyan());
    println!("  {}", ":load MyLibrary.cql".cyan());
}

// Helper to get home directory
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}
