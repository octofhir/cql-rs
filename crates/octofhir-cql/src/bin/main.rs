//! `cql` — Clinical Quality Language command-line tool.

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use octofhir_cql::cli::{self, eval::EvalOptions, repl::ReplOptions};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

/// Parse, translate, validate and evaluate CQL libraries.
#[derive(Parser)]
#[command(name = "cql", author, version, about, long_about = None)]
struct Cli {
    /// When to color output
    #[arg(long, value_enum, default_value_t = ColorMode::Auto, global = true)]
    color: ColorMode,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a CQL file and print its syntax tree as JSON
    Parse {
        /// CQL file
        file: PathBuf,
        /// Print the Rust debug representation instead of JSON
        #[arg(long)]
        debug: bool,
        /// Single-line JSON
        #[arg(long)]
        compact: bool,
        /// Write to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Translate a CQL library to ELM (JSON)
    Translate {
        /// CQL file
        file: PathBuf,
        /// Directory to search for included libraries (repeatable; the file's
        /// own directory is always searched first)
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
        /// Single-line JSON
        #[arg(long)]
        compact: bool,
        /// Write to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Check CQL files for syntax and name-resolution errors (exit code 1 on errors)
    Validate {
        /// CQL files
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Directory to search for included libraries (repeatable)
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
        /// Print only errors
        #[arg(short, long)]
        quiet: bool,
    },

    /// Evaluate a library's definitions, optionally per patient against FHIR data
    #[command(visible_alias = "run")]
    Eval {
        /// CQL file
        file: PathBuf,
        /// FHIR data: a resource, Bundle or JSON array (.json) or NDJSON (.ndjson); repeatable
        #[arg(short, long)]
        data: Vec<PathBuf>,
        /// Parameter value as a CQL expression: NAME=EXPRESSION (repeatable)
        #[arg(short, long = "param")]
        params: Vec<String>,
        /// Evaluate only this definition (repeatable)
        #[arg(short, long = "expression")]
        expressions: Vec<String>,
        /// Evaluate only for this Patient id (repeatable)
        #[arg(long = "patient")]
        patients: Vec<String>,
        /// Directory to search for included libraries (repeatable)
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
        /// Evaluation timestamp for Now()/Today() (RFC 3339 or a date)
        #[arg(long)]
        now: Option<String>,
        /// Single-line JSON
        #[arg(long)]
        compact: bool,
        /// Write to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Evaluate expressions interactively
    Repl {
        /// FHIR data files (see `eval --data`)
        #[arg(short, long)]
        data: Vec<PathBuf>,
        /// Run in Patient context for this Patient id
        #[arg(long)]
        patient: Option<String>,
        /// Directory to search for included libraries (repeatable)
        #[arg(short = 'L', long = "library-path")]
        library_paths: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    human_panic::setup_panic!();
    let cli = Cli::parse();

    let color = match cli.color {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => std::io::stderr().is_terminal(),
    };
    colored::control::set_override(color);

    let result = match cli.command {
        Command::Parse {
            file,
            debug,
            compact,
            output,
        } => cli::parse(&file, debug, compact, output.as_deref()),
        Command::Translate {
            file,
            library_paths,
            compact,
            output,
        } => cli::translate(&file, &library_paths, compact, output.as_deref()),
        Command::Validate {
            files,
            library_paths,
            quiet,
        } => cli::validate(&files, &library_paths, quiet),
        Command::Eval {
            file,
            data,
            params,
            expressions,
            patients,
            library_paths,
            now,
            compact,
            output,
        } => cli::eval::run(&EvalOptions {
            file,
            data,
            params,
            expressions,
            patients,
            library_paths,
            now,
            compact,
            output,
        }),
        Command::Repl {
            data,
            patient,
            library_paths,
        } => cli::repl::run(&ReplOptions {
            data,
            patient,
            library_paths,
        })
        .map(|()| true),
    };

    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{} {error:#}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}
