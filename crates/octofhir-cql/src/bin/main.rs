//! CQL command-line interface

use anyhow::Result;
use clap::{Parser, Subcommand};
use octofhir_cql::cli::{execute, output, repl, resolver, translate, validate};
use std::path::PathBuf;

/// CQL command-line tool
#[derive(Parser)]
#[command(name = "cql")]
#[command(author, version, about = "Clinical Quality Language (CQL) tools", long_about = None)]
struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format (json, table, pretty)
    #[arg(short = 'f', long, global = true)]
    format: Option<String>,

    /// Output file (default: stdout)
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,

    /// Color output (auto, always, never)
    #[arg(long, default_value = "auto", global = true)]
    color: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a CQL file
    Execute {
        /// CQL file to execute
        file: PathBuf,

        /// Parameters (name=value)
        #[arg(short, long = "param")]
        params: Vec<String>,

        /// Context data file (JSON)
        #[arg(short, long)]
        data: Option<PathBuf>,

        /// Library search paths
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
    },

    /// Translate CQL to ELM
    Translate {
        /// CQL file to translate
        file: PathBuf,

        /// Output format (json, xml)
        #[arg(short = 'F', long = "format", default_value = "json")]
        elm_format: String,

        /// Include annotations
        #[arg(short, long)]
        annotations: bool,

        /// Pretty-print output
        #[arg(short, long)]
        pretty: bool,

        /// Library search paths
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
    },

    /// Validate CQL syntax and semantics
    Validate {
        /// CQL files to validate
        files: Vec<PathBuf>,

        /// Strict mode (warnings as errors)
        #[arg(short, long)]
        strict: bool,

        /// Library search paths
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
    },

    /// Start interactive REPL
    Repl {
        /// Data model to use
        #[arg(short, long, default_value = "FHIR")]
        model: String,

        /// Model version
        #[arg(short = 'V', long)]
        version: Option<String>,

        /// Library search paths
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
    },
}

#[tokio::main]
async fn main() {
    human_panic::setup_panic!();

    let cli = Cli::parse();

    // Set up color output
    output::setup_colors(&cli.color);

    // Set up verbosity
    if cli.verbose {
        // Note: setting env vars is unsafe in Rust 2024 edition
        // For now, we just use the verbose flag directly in commands
    }

    let result = match cli.command {
        Commands::Execute {
            file,
            params,
            data,
            library_paths,
        } => {
            let config = execute::ExecuteConfig {
                file,
                params,
                data,
                library_paths,
                verbose: cli.verbose,
                output_format: cli.format.clone(),
                output_file: cli.output.clone(),
            };
            execute::execute(config).await
        }

        Commands::Translate {
            file,
            elm_format,
            annotations,
            pretty,
            library_paths,
        } => {
            let config = translate::TranslateConfig {
                file,
                format: elm_format,
                annotations,
                pretty,
                library_paths,
                output_file: cli.output.clone(),
            };
            translate::translate(config).await
        }

        Commands::Validate {
            files,
            strict,
            library_paths,
        } => {
            let config = validate::ValidateConfig {
                files,
                strict,
                library_paths,
                verbose: cli.verbose,
            };
            validate::validate(config).await
        }

        Commands::Repl {
            model,
            version,
            library_paths,
        } => {
            let config = repl::ReplConfig {
                model,
                version,
                library_paths,
            };
            repl::run(config).await
        }
    };

    if let Err(e) = result {
        eprintln!("{}", output::format_error(&e));
        std::process::exit(1);
    }
}
