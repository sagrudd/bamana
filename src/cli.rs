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
    /// Assess mapping state and reference mapping metadata.
    #[command(name = "check_map")]
    CheckMap(CheckMapArgs),
    /// Assess declared and observed BAM sort characteristics.
    #[command(name = "check_sort")]
    CheckSort(CheckSortArgs),
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

#[derive(Debug, Args)]
pub struct CheckSortArgs {
    /// BAM file to inspect.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Maximum number of alignment records to inspect in non-strict mode.
    #[arg(long = "sample-records", default_value_t = 10_000)]
    pub sample_records: usize,
    /// Continue scanning beyond the sample window until EOF or a stronger conclusion is reached.
    #[arg(long)]
    pub strict: bool,
}

#[derive(Debug, Args)]
pub struct CheckMapArgs {
    /// BAM file to inspect.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Maximum number of alignment records to inspect in scan mode before returning an assessment.
    #[arg(long = "sample-records", default_value_t = 10_000)]
    pub sample_records: usize,
    /// Scan the full alignment stream when no usable index is available.
    #[arg(long)]
    pub full_scan: bool,
    /// Prefer index-derived mapping information when a usable index exists.
    #[arg(long = "prefer-index", default_value_t = true)]
    pub prefer_index: bool,
}
