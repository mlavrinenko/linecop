use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use linecop::report::Format;

#[derive(Parser)]
#[command(about = "Patrols your code base to enforce line count limits.", version)]
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

fn main() -> ExitCode {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Init) => {
            match linecop::init::create(&cli.path) {
                Ok(path) => {
                    println!("Created {path}");
                    return ExitCode::SUCCESS;
                }
                Err(err) => {
                    eprintln!("error: {err:#}");
                    return ExitCode::FAILURE;
                }
            }
        }
        Some(Command::Schema) => {
            match linecop::schema::generate() {
                Ok(schema) => {
                    print!("{schema}");
                    return ExitCode::SUCCESS;
                }
                Err(err) => {
                    eprintln!("error: {err:#}");
                    return ExitCode::FAILURE;
                }
            }
        }
        None => {}
    }

    let config_path = cli
        .config
        .unwrap_or_else(|| cli.path.join(".linecop.yaml"));

    match linecop::run(&cli.path, &config_path, cli.quiet, cli.format) {
        Ok(true) => ExitCode::FAILURE,
        Ok(false) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
