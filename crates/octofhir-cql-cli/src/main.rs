//! CQL command-line interface

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CQL command-line tool
#[derive(Parser)]
#[command(name = "cql")]
#[command(author, version, about = "Clinical Quality Language (CQL) tools", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a CQL file and print the AST
    Parse {
        /// CQL file to parse
        file: PathBuf,
        /// Output format (ast, json)
        #[arg(short, long, default_value = "ast")]
        format: String,
    },
    /// Translate CQL to ELM
    Translate {
        /// CQL file to translate
        file: PathBuf,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format (json, xml)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Validate CQL syntax and semantics
    Validate {
        /// CQL files to validate
        files: Vec<PathBuf>,
    },
    /// Start interactive REPL
    Repl {
        /// Data model to use
        #[arg(short, long, default_value = "FHIR")]
        model: String,
        /// Model version
        #[arg(short, long)]
        version: Option<String>,
    },
}

fn main() {
    human_panic::setup_panic!();

    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { file, format } => {
            println!("Parsing {:?} with format {}", file, format);
            // TODO: Implement parsing
            println!("Parser not yet implemented");
        }
        Commands::Translate { file, output, format } => {
            println!("Translating {:?} to {:?} format {}", file, output, format);
            // TODO: Implement translation
            println!("Translation not yet implemented");
        }
        Commands::Validate { files } => {
            println!("Validating {:?}", files);
            // TODO: Implement validation
            println!("Validation not yet implemented");
        }
        Commands::Repl { model, version } => {
            println!("Starting REPL with model {} version {:?}", model, version);
            // TODO: Implement REPL
            println!("REPL not yet implemented");
        }
    }
}
