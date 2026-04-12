use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "bamana",
    author,
    version,
    about = "High-performance BAM verification, QC, inspection, and transformation toolkit.",
    long_about = None
)]
pub struct Cli {
    /// Emit pretty-printed JSON.
    #[arg(long, global = true)]
    pub json_pretty: bool,
    /// Reserved for future log suppression.
    #[arg(long, global = true)]
    pub quiet: bool,
    /// Reserved for future verbose diagnostics.
    #[arg(long, global = true, action = ArgAction::Count)]
    pub verbose: u8,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Determine the file type as quickly and deterministically as possible.
    Identify(IdentifyArgs),
    /// Perform shallow BAM verification only.
    Verify(BamPathArgs),
    /// Check for the canonical BGZF EOF marker only.
    #[command(name = "check_eof")]
    CheckEof(BamPathArgs),
    /// Extract BAM header information only.
    Header(BamPathArgs),
}

#[derive(Debug, Args)]
pub struct IdentifyArgs {
    /// Input path to inspect.
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct BamPathArgs {
    /// BAM file to inspect.
    #[arg(long = "bam")]
    pub bam: PathBuf,
}
