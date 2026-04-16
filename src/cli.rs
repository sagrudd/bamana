use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::bam::checksum::{ChecksumAlgorithm, ChecksumMode};
use crate::bam::merge::MergeMode;
use crate::bam::sort::{QuerynameSubOrder, SortOrder};
use crate::forensics::deduplicate::{DeduplicateKeepPolicy, DeduplicateMode};
use crate::forensics::duplication::DuplicationIdentityMode;
use crate::forensics::forensic_inspect::ForensicScope;
use crate::ingest::{
    consume::{ConsumeMode, ConsumePlatform, ConsumeSortOrder},
    cram::ConsumeReferencePolicy,
};
use crate::sampling::{DeterministicIdentity, SubsampleMode};

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
    /// Run a containerized benchmark profile with owned tool and report setup.
    Benchmark(BenchmarkArgs),
    /// Determine the likely file type quickly and deterministically.
    Identify(IdentifyArgs),
    /// Subsample BAM or FASTQ inputs with explicit deterministic or random policy.
    Subsample(SubsampleArgs),
    /// Inspect suspicious collection-duplication and operator-error signatures.
    #[command(name = "inspect_duplication")]
    InspectDuplication(InspectDuplicationArgs),
    /// Remove suspicious collection-duplication blocks conservatively.
    Deduplicate(DeduplicateArgs),
    /// Inspect provenance anomalies and concatenation/coercion hallmarks.
    #[command(name = "forensic_inspect")]
    ForensicInspect(ForensicInspectArgs),
    /// Insert, replace, or normalize per-record RG:Z tags across BAM alignment records.
    #[command(name = "annotate_rg")]
    AnnotateRg(AnnotateRgArgs),
    /// Consume files and directories into a normalized BAM with explicit ingest semantics.
    Consume(ConsumeArgs),
    /// Rewrite a BAM as a single FASTQ.GZ stream as quickly as possible.
    Fastq(FastqArgs),
    /// Compute machine-verifiable BAM checksums over explicit checksum domains.
    Checksum(ChecksumArgs),
    /// Merge multiple BAM inputs into a single BAM output.
    Merge(MergeArgs),
    /// Mutate BAM header metadata only without touching alignment-record RG tags.
    Reheader(ReheaderArgs),
    /// Sort a BAM file into a requested output ordering.
    Sort(SortArgs),
    /// Strip reference-bound alignment state from a BAM while preserving non-mapping metadata.
    Unmap(UnmapArgs),
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BenchmarkProfile {
    #[value(name = "fastq_ingress", alias = "fastq-ingress")]
    FastqIngress,
    #[value(name = "fastq_gz_enumerate", alias = "fastq-gz-enumerate")]
    FastqGzEnumerate,
}

#[derive(Debug, Args)]
pub struct BenchmarkArgs {
    /// Benchmark profile to execute.
    #[arg(long = "profile", value_enum)]
    pub profile: BenchmarkProfile,
    /// Input FASTQ.GZ file for FASTQ benchmark profiles.
    #[arg(long = "fastq")]
    pub fastq: PathBuf,
    /// Output BAM path for the fastq_ingress Bamana normalization result.
    #[arg(long = "bam")]
    pub bam: Option<PathBuf>,
    /// Output PDF path for the rendered benchmark report.
    #[arg(long = "report")]
    pub report: PathBuf,
    /// Requested worker thread count for benchmarked commands.
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    pub threads: usize,
    /// Container image tag to build and execute.
    #[arg(long = "container-image", default_value = "bamana-bench:latest")]
    pub container_image: String,
    /// Overwrite existing benchmark outputs and report targets.
    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct IdentifyArgs {
    /// Path to inspect.
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct SubsampleArgs {
    /// Input BAM, FASTQ, or FASTQ.GZ file to subsample.
    #[arg(long = "input")]
    pub input: PathBuf,
    /// Output path for the subsampled collection.
    #[arg(long = "out")]
    pub out: PathBuf,
    /// Requested approximate retained fraction.
    #[arg(long = "fraction")]
    pub fraction: f64,
    /// Subsampling mode.
    #[arg(long = "mode", value_enum)]
    pub mode: SubsampleMode,
    /// Seed for reproducible random subsampling; generated and reported if omitted.
    #[arg(long = "seed")]
    pub seed: Option<u64>,
    /// Deterministic identity basis for hash-based selection.
    #[arg(
        long = "identity",
        value_enum,
        default_value_t = DeterministicIdentity::FullRecord
    )]
    pub identity: DeterministicIdentity,
    /// Plan and count only; do not write output.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// Attempt to regenerate an index for BAM output when supported.
    #[arg(long = "create-index")]
    pub create_index: bool,
    /// For BAM input, consider mapped records only and drop unmapped records from output.
    #[arg(long = "mapped-only")]
    pub mapped_only: bool,
    /// For BAM input, consider primary alignments only and drop secondary/supplementary records from output.
    #[arg(long = "primary-only")]
    pub primary_only: bool,
    /// Requested worker thread count for future parallel implementations.
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    pub threads: usize,
    /// Overwrite an existing output path.
    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct InspectDuplicationArgs {
    /// Input BAM, FASTQ, or FASTQ.GZ file to inspect.
    #[arg(long = "input")]
    pub input: PathBuf,
    /// Record identity policy used for duplicate detection.
    #[arg(
        long = "identity",
        value_enum,
        default_value_t = DuplicationIdentityMode::QnameSeqQual
    )]
    pub identity: DuplicationIdentityMode,
    /// Minimum adjacent repeated block size required for a block finding.
    #[arg(long = "min-block-size", default_value_t = 50)]
    pub min_block_size: usize,
    /// Maximum records to inspect in bounded-scan mode.
    #[arg(long = "sample-records", default_value_t = 100_000)]
    pub sample_records: usize,
    /// Scan to EOF instead of stopping at the bounded record limit.
    #[arg(long = "full-scan")]
    pub full_scan: bool,
    /// Bound the number of reported findings.
    #[arg(long = "max-findings", default_value_t = 25)]
    pub max_findings: usize,
}

#[derive(Debug, Args)]
pub struct DeduplicateArgs {
    /// Input BAM, FASTQ, or FASTQ.GZ file to remediate.
    #[arg(long = "input")]
    pub input: PathBuf,
    /// Output path for the remediated collection.
    #[arg(long = "out")]
    pub out: PathBuf,
    /// Conservative remediation mode.
    #[arg(long = "mode", value_enum)]
    pub mode: DeduplicateMode,
    /// Record identity policy used for duplication planning.
    #[arg(
        long = "identity",
        value_enum,
        default_value_t = DuplicationIdentityMode::QnameSeqQual
    )]
    pub identity: DuplicationIdentityMode,
    /// Which duplicated copy to retain when a removable block is detected.
    #[arg(long = "keep", value_enum, default_value_t = DeduplicateKeepPolicy::First)]
    pub keep: DeduplicateKeepPolicy,
    /// Plan only; do not write an output file.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// Minimum adjacent repeated block size required for remediation.
    #[arg(long = "min-block-size", default_value_t = 50)]
    pub min_block_size: usize,
    /// Compute descriptive checksum provenance when supported.
    #[arg(long = "verify-checksum")]
    pub verify_checksum: bool,
    /// Emit a machine-readable removed-range report to this path.
    #[arg(long = "emit-removed-report")]
    pub emit_removed_report: Option<PathBuf>,
    /// Maximum records to inspect in bounded dry-run mode.
    #[arg(long = "sample-records", default_value_t = 100_000)]
    pub sample_records: usize,
    /// Scan to EOF during dry-run planning.
    #[arg(long = "full-scan")]
    pub full_scan: bool,
    /// Attempt to regenerate a BAM index when output is written.
    #[arg(long = "reindex")]
    pub reindex: bool,
    /// Overwrite existing output or report paths.
    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ForensicInspectArgs {
    /// Input BAM file to inspect for provenance anomalies.
    #[arg(long = "input")]
    pub input: PathBuf,
    /// Maximum records to inspect in bounded body-scan mode.
    #[arg(long = "sample-records", default_value_t = 100_000)]
    pub sample_records: usize,
    /// Scan the BAM body to EOF instead of stopping at the bounded record limit.
    #[arg(long = "full-scan")]
    pub full_scan: bool,
    /// Inspect BAM header structure and metadata.
    #[arg(long = "inspect-header")]
    pub inspect_header: bool,
    /// Inspect read-group declarations and record-level RG usage.
    #[arg(long = "inspect-rg")]
    pub inspect_rg: bool,
    /// Inspect @PG program-chain structure.
    #[arg(long = "inspect-pg")]
    pub inspect_pg: bool,
    /// Inspect read-name regime shifts and naming-style changes.
    #[arg(long = "inspect-readnames")]
    pub inspect_readnames: bool,
    /// Inspect selected auxiliary-tag usage regimes.
    #[arg(long = "inspect-tags")]
    pub inspect_tags: bool,
    /// Inspect duplicate-block and append hallmarks in record order.
    #[arg(long = "inspect-duplication")]
    pub inspect_duplication: bool,
    /// Bound the number of reported findings.
    #[arg(long = "max-findings", default_value_t = 25)]
    pub max_findings: usize,
}

impl ForensicInspectArgs {
    pub fn resolved_scopes(&self) -> ForensicScope {
        let specific_requested = self.inspect_header
            || self.inspect_rg
            || self.inspect_pg
            || self.inspect_readnames
            || self.inspect_tags
            || self.inspect_duplication;

        if specific_requested {
            ForensicScope {
                header: self.inspect_header,
                read_groups: self.inspect_rg,
                program_chain: self.inspect_pg,
                read_names: self.inspect_readnames,
                tags: self.inspect_tags,
                duplication_hallmarks: self.inspect_duplication,
            }
        } else {
            ForensicScope {
                header: true,
                read_groups: true,
                program_chain: true,
                read_names: true,
                tags: false,
                duplication_hallmarks: true,
            }
        }
    }
}

#[derive(Debug, Args)]
pub struct AnnotateRgArgs {
    /// Input BAM file to annotate.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Requested read-group ID for per-record RG:Z tagging.
    #[arg(long = "rg-id")]
    pub rg_id: String,
    /// Output BAM path.
    #[arg(long = "out")]
    pub out: Option<PathBuf>,
    /// Insert RG tags only into records that currently lack them.
    #[arg(long = "only-missing")]
    pub only_missing: bool,
    /// Replace every existing RG tag with the requested ID.
    #[arg(long = "replace-existing")]
    pub replace_existing: bool,
    /// Fail if any record already carries an RG tag that differs from --rg-id.
    #[arg(long = "fail-on-conflict")]
    pub fail_on_conflict: bool,
    /// Require the BAM header to already contain a matching @RG line.
    #[arg(long = "require-header-rg")]
    pub require_header_rg: bool,
    /// Create a minimal matching @RG line when the BAM header lacks one.
    #[arg(long = "create-header-rg")]
    pub create_header_rg: bool,
    /// Add a new @RG line using comma-separated KEY=VALUE fields.
    #[arg(long = "add-header-rg")]
    pub add_header_rg: Option<String>,
    /// Update the existing @RG line using comma-separated KEY=VALUE fields.
    #[arg(long = "set-header-rg")]
    pub set_header_rg: Option<String>,
    /// Attempt to regenerate an index after annotation.
    #[arg(long = "reindex")]
    pub reindex: bool,
    /// Verify checksum preservation with RG excluded from the checksum domain.
    #[arg(long = "verify-checksum")]
    pub verify_checksum: bool,
    /// Requested worker thread count for future parallel implementations.
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    pub threads: usize,
    /// Overwrite an existing output file.
    #[arg(long = "force")]
    pub force: bool,
    /// Plan the mutation without writing output.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ConsumeArgs {
    /// One or more files and/or directories to ingest.
    #[arg(long = "input", alias = "in", required = true, num_args = 1..)]
    pub input: Vec<PathBuf>,
    /// Output BAM path.
    #[arg(long = "out")]
    pub out: PathBuf,
    /// Ingestion mode. Mixed alignment-bearing and raw-read inputs are rejected by default.
    #[arg(long = "mode", value_enum)]
    pub mode: ConsumeMode,
    /// Descend into input directories recursively.
    #[arg(long = "recursive")]
    pub recursive: bool,
    /// Requested worker thread count for future parallel implementations.
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    pub threads: usize,
    /// Overwrite an existing output file.
    #[arg(long = "force")]
    pub force: bool,
    /// Output sort policy after ingestion.
    #[arg(long = "sort", value_enum, default_value_t = ConsumeSortOrder::None)]
    pub sort: ConsumeSortOrder,
    /// Attempt to create an index when the output order is suitable.
    #[arg(long = "create-index")]
    pub create_index: bool,
    /// Verify checksum preservation after ingestion when semantically meaningful.
    #[arg(long = "verify-checksum")]
    pub verify_checksum: bool,
    /// Discover, classify, and plan ingestion without writing a BAM.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// Explicit reference FASTA for CRAM decoding. Stage 2 currently expects an indexed FASTA with an adjacent .fai.
    #[arg(long = "reference")]
    pub reference: Option<PathBuf>,
    /// Explicit reference cache path for future CRAM decoding support.
    #[arg(long = "reference-cache")]
    pub reference_cache: Option<PathBuf>,
    /// Conservative CRAM reference-resolution policy.
    #[arg(
        long = "reference-policy",
        value_enum,
        default_value_t = ConsumeReferencePolicy::Strict
    )]
    pub reference_policy: ConsumeReferencePolicy,
    /// Optional sample name for synthetic unmapped BAM headers.
    #[arg(long = "sample")]
    pub sample: Option<String>,
    /// Optional read-group identifier for synthetic unmapped BAM headers.
    #[arg(long = "read-group")]
    pub read_group: Option<String>,
    /// Optional sequencing platform for synthetic unmapped BAM headers.
    #[arg(long = "platform", value_enum)]
    pub platform: Option<ConsumePlatform>,
    /// Future include filter applied to discovered paths.
    #[arg(long = "include-glob")]
    pub include_glob: Vec<String>,
    /// Future exclude filter applied to discovered paths.
    #[arg(long = "exclude-glob")]
    pub exclude_glob: Vec<String>,
}

#[derive(Debug, Args)]
pub struct FastqArgs {
    /// Input BAM file to export as FASTQ.GZ.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Output FASTQ.GZ path. Defaults to <input>.fastq.gz.
    #[arg(long = "out")]
    pub out: Option<PathBuf>,
    /// Maximum worker threads for parallel decode/compression. Defaults to all available cores.
    #[arg(short = 'j', long = "threads", default_value_t = 0)]
    pub threads: usize,
    /// Overwrite an existing output file.
    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ChecksumArgs {
    /// BAM file to checksum.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Checksum mode to compute.
    #[arg(long = "mode", value_enum, default_value_t = ChecksumMode::All)]
    pub mode: ChecksumMode,
    /// Checksum algorithm.
    #[arg(long = "algorithm", value_enum, default_value_t = ChecksumAlgorithm::Sha256)]
    pub algorithm: ChecksumAlgorithm,
    /// Include the deterministic header serialization in payload mode.
    #[arg(long = "include-header")]
    pub include_header: bool,
    /// Exclude specified auxiliary tags from canonical payload hashing.
    #[arg(long = "exclude-tags", value_delimiter = ',')]
    pub exclude_tags: Vec<String>,
    /// Hash only primary alignments.
    #[arg(long = "only-primary")]
    pub only_primary: bool,
    /// Hash only mapped alignments.
    #[arg(long = "mapped-only")]
    pub mapped_only: bool,
}

#[derive(Debug, Args)]
pub struct MergeArgs {
    /// Input BAM files to merge.
    #[arg(long = "bam", required = true, num_args = 1..)]
    pub bam: Vec<PathBuf>,
    /// Output BAM path.
    #[arg(long = "out")]
    pub out: PathBuf,
    /// Shorthand for --order coordinate.
    #[arg(long = "sort")]
    pub sort: bool,
    /// Requested output merge mode.
    #[arg(long = "order", value_enum)]
    pub order: Option<MergeMode>,
    /// Queryname sub-order, only meaningful for queryname merge output.
    #[arg(long = "queryname-suborder", value_enum)]
    pub queryname_suborder: Option<QuerynameSubOrder>,
    /// Attempt to create an index for coordinate-sorted output.
    #[arg(long = "create-index")]
    pub create_index: bool,
    /// Verify canonical multiset checksum preservation across the merge.
    #[arg(long = "verify-checksum")]
    pub verify_checksum: bool,
    /// Requested worker thread count for future parallel implementations.
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    pub threads: usize,
    /// Overwrite an existing output file.
    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReheaderPlatform {
    Ont,
    Illumina,
    Pacbio,
    Unknown,
}

#[derive(Debug, Args)]
pub struct ReheaderArgs {
    /// Input BAM file to mutate.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Replace the full BAM header from a SAM-style header file.
    #[arg(long = "header")]
    pub header: Option<PathBuf>,
    /// Add a new @RG record using comma-separated KEY=VALUE fields. ID is required.
    #[arg(long = "add-rg")]
    pub add_rg: Vec<String>,
    /// Update an existing @RG record using comma-separated KEY=VALUE fields.
    #[arg(long = "set-rg")]
    pub set_rg: Vec<String>,
    /// Remove an @RG record by ID.
    #[arg(long = "remove-rg")]
    pub remove_rg: Vec<String>,
    /// Set SM on the targeted @RG record.
    #[arg(long = "set-sample")]
    pub set_sample: Option<String>,
    /// Set PL on the targeted @RG record.
    #[arg(long = "set-platform", value_enum)]
    pub set_platform: Option<ReheaderPlatform>,
    /// Target @RG ID for focused mutations such as --set-sample and --set-platform.
    #[arg(long = "target-rg")]
    pub target_rg: Option<String>,
    /// Add or update a @PG record using comma-separated KEY=VALUE fields. ID is required.
    #[arg(long = "set-pg")]
    pub set_pg: Vec<String>,
    /// Append an @CO line to the BAM header.
    #[arg(long = "add-comment")]
    pub add_comment: Vec<String>,
    /// Request true in-place header modification only if provably safe.
    #[arg(long = "in-place")]
    pub in_place: bool,
    /// Permit a rewrite-minimized fallback when true in-place modification is not feasible.
    #[arg(long = "rewrite-minimized")]
    pub rewrite_minimized: bool,
    /// Request an explicit safe rewrite mode.
    #[arg(long = "safe-rewrite")]
    pub safe_rewrite: bool,
    /// Plan the requested header mutation without writing output.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// Overwrite an existing output BAM.
    #[arg(long = "force")]
    pub force: bool,
    /// Output BAM path for rewrite modes.
    #[arg(long = "out")]
    pub out: Option<PathBuf>,
    /// Attempt to regenerate an index after reheader.
    #[arg(long = "reindex")]
    pub reindex: bool,
    /// Verify that alignment-record content was preserved while excluding header bytes.
    #[arg(long = "verify-checksum")]
    pub verify_checksum: bool,
}

#[derive(Debug, Args)]
pub struct SortArgs {
    /// Input BAM file to sort.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Output BAM path.
    #[arg(long = "out")]
    pub out: PathBuf,
    /// Requested sort order.
    #[arg(long = "order", value_enum, default_value_t = SortOrder::Coordinate)]
    pub order: SortOrder,
    /// Queryname sub-order, only meaningful when --order queryname is selected.
    #[arg(long = "queryname-suborder", value_enum)]
    pub queryname_suborder: Option<QuerynameSubOrder>,
    /// Requested worker thread count for future parallel implementations.
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    pub threads: usize,
    /// Target memory budget for future external-sort support.
    #[arg(long = "memory-limit")]
    pub memory_limit: Option<u64>,
    /// Attempt to create an index when coordinate output is produced.
    #[arg(long = "create-index")]
    pub create_index: bool,
    /// Compute canonical checksums for input and output after sorting.
    #[arg(long = "verify-checksum")]
    pub verify_checksum: bool,
    /// Overwrite an existing output file.
    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct UnmapArgs {
    /// Input BAM file to rewrite as unmapped.
    #[arg(long = "bam")]
    pub bam: PathBuf,
    /// Output BAM path. Defaults to <input>.unmapped.bam.
    #[arg(long = "out")]
    pub out: Option<PathBuf>,
    /// Plan the rewrite and report intended changes without writing output.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// Requested worker thread count for future parallel implementations.
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    pub threads: usize,
    /// Overwrite an existing output BAM.
    #[arg(long = "force")]
    pub force: bool,
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
    #[arg(long = "strict")]
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
    #[arg(long = "full-scan")]
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
    #[arg(long = "require")]
    pub require: bool,
    /// Prefer CSI over BAI when multiple adjacent indices are present.
    #[arg(long = "prefer-csi")]
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
    #[arg(long = "out")]
    pub out: Option<PathBuf>,
    /// Overwrite an existing index output path.
    #[arg(long = "force")]
    pub force: bool,
    /// Requested output index format.
    #[arg(long = "format", value_enum)]
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
    #[arg(long = "full-scan")]
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
    #[arg(long = "full-scan")]
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
