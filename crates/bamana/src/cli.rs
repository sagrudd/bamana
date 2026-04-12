use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "bamana",
    author,
    version,
    about = "High-performance BAM verification, QC, inspection, and transformation toolkit."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Identify a file and emit a structured result.
    Identify(IdentifyArgs),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Debug, clap::Args)]
pub struct IdentifyArgs {
    /// Input path to inspect.
    pub input: PathBuf,
    /// Output format for the result.
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,
}
