use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Args)]
pub struct GlobalOptions {
    /// Emit pretty-printed JSON.
    #[arg(long, global = true)]
    pub json_pretty: bool,
}

#[derive(Debug, Parser)]
#[command(
    name = "bamana",
    author,
    version,
    about = "High-performance BAM-oriented CLI for shallow inspection and verification.",
    long_about = None
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Determine the likely file type quickly and deterministically.
    Identify(IdentifyArgs),
    /// Perform shallow BAM verification only.
    Verify(BamPathArgs),
    /// Check for the canonical BGZF EOF marker only.
    #[command(name = "check_eof")]
    CheckEof(BamPathArgs),
    /// Parse the BAM header only.
    Header(BamPathArgs),
}

#[derive(Debug, Args)]
pub struct IdentifyArgs {
    /// Path to inspect.
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct BamPathArgs {
    /// BAM file to inspect.
    #[arg(long = "bam")]
    pub bam: PathBuf,
}
