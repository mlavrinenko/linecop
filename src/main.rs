use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

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

    /// Path to the config file (default: <PATH>/.linecop.yaml).
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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a starter .linecop.yaml in the target directory.
    Init,
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
        Some(Command::Init) => return run_subcommand(linecop::init::create(&cli.path)),
        Some(Command::Schema) => return run_subcommand(linecop::schema::generate()),
        None => {}
    }

    let config_path = cli.config.unwrap_or_else(|| cli.path.join(".linecop.yaml"));

    match linecop::run(&cli.path, &config_path, cli.quiet, cli.format) {
        Ok(true) => ExitCode::FAILURE,
        Ok(false) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
