use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use linecop::RunOptions;
use linecop::report::Format;

#[derive(Parser)]
#[command(
    about = "Patrols your code base to enforce line count limits.",
    version
)]
struct Cli {
    /// Root directory to scan.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Path to the config file (default: auto-detected).
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Suppress output (exit code only).
    #[arg(short, long)]
    quiet: bool,

    /// Output format.
    #[arg(long, default_value = "text", value_enum)]
    format: Format,

    /// Control color output.
    #[arg(long, value_enum, default_value = "auto")]
    color: clap::ColorChoice,

    /// Suppress the warning when no config file is found.
    #[arg(long)]
    no_config_warning: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a starter .linecop.yaml in the target directory.
    Init {
        /// Custom schema URL to embed in the config file.
        #[arg(long)]
        schema: Option<String>,

        /// Do not embed a yaml-language-server schema comment.
        #[arg(long)]
        no_schema: bool,
    },
    /// Print the JSON Schema for .linecop.yaml configuration.
    Schema,
}

fn run_subcommand(result: Result<String>) -> ExitCode {
    match result {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.color {
        clap::ColorChoice::Always => anstream::ColorChoice::Always,
        clap::ColorChoice::Never => anstream::ColorChoice::Never,
        clap::ColorChoice::Auto => anstream::ColorChoice::Auto,
    }
    .write_global();

    match cli.command {
        Some(Command::Init { schema, no_schema }) => {
            let mode = if no_schema {
                linecop::init::SchemaMode::None
            } else if let Some(url) = schema {
                linecop::init::SchemaMode::Custom(url)
            } else {
                linecop::init::SchemaMode::Default
            };
            return run_subcommand(linecop::init::create(&cli.path, &mode));
        }
        Some(Command::Schema) => return run_subcommand(linecop::schema::generate()),
        None => {}
    }

    let opts = RunOptions {
        config_path: cli.config.as_deref(),
        quiet: cli.quiet,
        format: cli.format,
        no_config_warning: cli.no_config_warning,
    };

    match linecop::run(&cli.path, &opts) {
        Ok(true) => ExitCode::FAILURE,
        Ok(false) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
