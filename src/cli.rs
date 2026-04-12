use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

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
    /// Inspect BAM index presence, type, and shallow usability.
    #[command(name = "check_index")]
    CheckIndex(CheckIndexArgs),
    /// Create a BAM index or report the deferred writer path honestly.
    Index(IndexArgs),
    /// Produce a fast operational summary of BAM characteristics.
    Summary(SummaryArgs),
    /// Perform deeper BAM structural and internal-consistency validation.
    Validate(ValidateArgs),
    /// Detect presence or absence of a BAM auxiliary tag.
    #[command(name = "check_tag")]
    CheckTag(CheckTagArgs),
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

#[derive(Debug, Args)]
pub struct CheckIndexArgs {
    /// BAM file to inspect.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Fail if no usable index is available.
    #[arg(long)]
    pub require: bool,
    /// Prefer CSI over BAI when multiple adjacent indices are present.
    #[arg(long)]
    pub prefer_csi: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum IndexFormatArg {
    Bai,
    Csi,
}

#[derive(Debug, Args)]
pub struct IndexArgs {
    /// BAM file to index.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Explicit output path for the index file.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Overwrite an existing index output path.
    #[arg(long)]
    pub force: bool,
    /// Requested output index format.
    #[arg(long, value_enum)]
    pub format: Option<IndexFormatArg>,
}

#[derive(Debug, Args)]
pub struct SummaryArgs {
    /// BAM file to summarise.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Maximum number of alignment records to inspect in bounded-scan mode.
    #[arg(long = "sample-records", default_value_t = 100_000)]
    pub sample_records: usize,
    /// Scan the full alignment stream.
    #[arg(long)]
    pub full_scan: bool,
    /// Prefer index-derived totals where a usable index exists.
    #[arg(long = "prefer-index", default_value_t = true)]
    pub prefer_index: bool,
    /// Include a MAPQ histogram keyed by integer MAPQ.
    #[arg(long = "include-mapq-hist")]
    pub include_mapq_hist: bool,
    /// Include a detailed flag-category section in the output.
    #[arg(long = "include-flags")]
    pub include_flags: bool,
}

#[derive(Debug, Args)]
pub struct CheckTagArgs {
    /// BAM auxiliary tag to inspect, for example NM or RG.
    #[arg(long = "tag")]
    pub tag: String,
    /// BAM file to inspect.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Maximum number of alignment records to inspect in bounded-scan mode.
    #[arg(long = "sample-records", default_value_t = 100_000)]
    pub sample_records: usize,
    /// Scan the full alignment stream.
    #[arg(long)]
    pub full_scan: bool,
    /// Require the tag to be present with the specified BAM aux type.
    #[arg(long = "require-type")]
    pub require_type: Option<String>,
    /// Count how many records in the actual scan scope contain the requested tag.
    #[arg(long = "count-hits")]
    pub count_hits: bool,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// BAM file to validate.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Stop collecting detailed error findings after this many errors.
    #[arg(long = "max-errors", default_value_t = 100)]
    pub max_errors: usize,
    /// Bound detailed warning finding collection.
    #[arg(long = "max-warnings", default_value_t = 100)]
    pub max_warnings: usize,
    /// Validate header-level structure only.
    #[arg(long = "header-only")]
    pub header_only: bool,
    /// Validate only the first N alignment records.
    #[arg(long = "records")]
    pub records: Option<u64>,
    /// Stop at the first error-level finding.
    #[arg(long = "fail-fast")]
    pub fail_fast: bool,
    /// Include warning-level findings in the output.
    #[arg(long = "include-warnings")]
    pub include_warnings: bool,
}
