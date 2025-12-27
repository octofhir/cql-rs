//! Output formatting utilities

use anyhow::{Context, Result};
use colored::*;
use serde_json::Value;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

#[cfg(feature = "cli")]
use tabled::{
    settings::Style,
    Table, Tabled,
};

/// Output format options
#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Json,
    JsonPretty,
    Table,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => Self::Json,
            "pretty" | "json-pretty" => Self::JsonPretty,
            "table" => Self::Table,
            _ => Self::JsonPretty, // default
        }
    }
}

/// Set up color output based on user preference
pub fn setup_colors(mode: &str) {
    match mode.to_lowercase().as_str() {
        "always" => colored::control::set_override(true),
        "never" => colored::control::set_override(false),
        "auto" | _ => {
            // Auto-detect based on terminal
            if atty::is(atty::Stream::Stdout) {
                colored::control::set_override(true);
            } else {
                colored::control::set_override(false);
            }
        }
    }
}

/// Format an error for display
pub fn format_error(error: &anyhow::Error) -> String {
    format!("{} {}", "Error:".red().bold(), error)
}

/// Format a warning for display
pub fn format_warning(warning: &str) -> String {
    format!("{} {}", "Warning:".yellow().bold(), warning)
}

/// Format a success message for display
pub fn format_success(message: &str) -> String {
    format!("{} {}", "Success:".green().bold(), message)
}

/// Format diagnostic information (file:line:col)
pub fn format_location(file: &str, line: usize, col: usize) -> String {
    format!("{}:{}:{}", file.cyan(), line, col)
}

/// Write output to a file or stdout
pub fn write_output(content: &str, output_file: Option<&Path>) -> Result<()> {
    if let Some(path) = output_file {
        let mut file = File::create(path)
            .with_context(|| format!("Failed to create output file: {}", path.display()))?;
        file.write_all(content.as_bytes())
            .with_context(|| format!("Failed to write to output file: {}", path.display()))?;
        eprintln!(
            "{}",
            format_success(&format!("Output written to {}", path.display()))
        );
    } else {
        println!("{}", content);
    }
    Ok(())
}

/// Format JSON value for output
pub fn format_json(value: &Value, pretty: bool) -> Result<String> {
    if pretty {
        serde_json::to_string_pretty(value)
            .context("Failed to serialize JSON")
    } else {
        serde_json::to_string(value)
            .context("Failed to serialize JSON")
    }
}

/// Format value as table (if possible)
#[cfg(feature = "cli")]
pub fn format_as_table(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                return Some("(empty list)".to_string());
            }

            // Try to format as table if items are objects with same keys
            if let Some(Value::Object(first)) = items.first() {
                let keys: Vec<_> = first.keys().cloned().collect();

                // Check if all objects have the same keys
                let all_same = items.iter().all(|item| {
                    if let Value::Object(obj) = item {
                        obj.keys().len() == keys.len() &&
                        obj.keys().all(|k| keys.contains(k))
                    } else {
                        false
                    }
                });

                if all_same {
                    #[derive(Tabled)]
                    struct Row {
                        #[tabled(rename = "Field")]
                        field: String,
                        #[tabled(rename = "Value")]
                        value: String,
                    }

                    let rows: Vec<Row> = items
                        .iter()
                        .flat_map(|item| {
                            if let Value::Object(obj) = item {
                                keys.iter().map(|k| Row {
                                    field: k.clone(),
                                    value: format_value(obj.get(k).unwrap()),
                                }).collect()
                            } else {
                                vec![]
                            }
                        })
                        .collect();

                    let table = Table::new(rows).with(Style::modern()).to_string();
                    return Some(table);
                }
            }

            // Simple list
            let items_str: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(i, v)| format!("  {}. {}", i + 1, format_value(v)))
                .collect();
            Some(items_str.join("\n"))
        }
        Value::Object(obj) => {
            #[derive(Tabled)]
            struct KeyValue {
                #[tabled(rename = "Key")]
                key: String,
                #[tabled(rename = "Value")]
                value: String,
            }

            let rows: Vec<KeyValue> = obj
                .iter()
                .map(|(k, v)| KeyValue {
                    key: k.clone(),
                    value: format_value(v),
                })
                .collect();

            let table = Table::new(rows).with(Style::modern()).to_string();
            Some(table)
        }
        _ => None,
    }
}

/// Format a simple value for display
fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(arr) => format!("[{} items]", arr.len()),
        Value::Object(obj) => format!("{{}} with {} fields", obj.len()),
    }
}

/// Print output in the specified format
pub fn print_output(
    value: &Value,
    format: OutputFormat,
    output_file: Option<&Path>,
) -> Result<()> {
    let content = match format {
        OutputFormat::Json => format_json(value, false)?,
        OutputFormat::JsonPretty => format_json(value, true)?,
        #[cfg(feature = "cli")]
        OutputFormat::Table => {
            format_as_table(value).unwrap_or_else(|| format_json(value, true).unwrap())
        }
        #[cfg(not(feature = "cli"))]
        OutputFormat::Table => format_json(value, true)?,
    };

    write_output(&content, output_file)
}

// Add this to check if we're in a TTY
mod atty {
    pub enum Stream {
        Stdout,
    }

    pub fn is(_stream: Stream) -> bool {
        // Simple check - can be enhanced with proper atty crate
        std::env::var("TERM").is_ok()
    }
}
