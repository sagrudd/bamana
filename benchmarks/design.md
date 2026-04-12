# Benchmark Design

## 1. Goals

The benchmark framework exists to support both internal optimization work and
publication-ready reporting. It is intended to answer:

* how long Bamana takes on common BAM and FASTQ.GZ workflows
* whether runtime is dominated by subsampling, sorting, indexing, or ingestion
* whether Bamana is already competitive in any workflow slice
* where Bamana is clearly slower and therefore needs targeted engineering work
* whether Bamana eventually beats `fastcat` in ingestion or concatenation space
* whether Bamana becomes faster and easier to use than `samtools` for the
  operations it chooses to own

The framework is not designed to manufacture a favorable result. It is designed
to surface trustworthy evidence.

## 2. Scenario Definitions

### Scenario A: `mapped_bam_pipeline`

Input:

* mapped BAM

Primary operation chain:

1. subsample
2. sort
3. index

Notes:

* tools are allowed to use their natural best-order workflow so long as the
  chosen order is documented
* this scenario is the canonical BAM throughput baseline

### Scenario B: `unmapped_bam_pipeline`

Input:

* unmapped BAM

Primary operation chain:

1. subsample
2. optional normalization step if the tool requires it
3. omit sort and index unless they are both valid and meaningful

Notes:

* the first framework treats this as a subsample-only comparison

### Scenario C: `fastq_consume_pipeline`

Input:

* FASTQ.GZ

Primary operation chain:

1. consume, concatenate, or normalize according to the tool's native role
2. optional subsample if that is part of the selected comparator workflow
3. sort or index only when the workflow actually produces sorted indexable BAM

Notes:

* Bamana uses `consume --mode unmapped`
* `fastcat` is intentionally benchmarked as a partial ingestion comparator
* `seqtk` is benchmarked as a partial FASTQ subsampling comparator
* `samtools` and `sambamba` are recorded as unsupported for this scenario in
  the first iteration

## 3. Comparator Set

### Canonical BAM Baseline

* `samtools`

Rationale:

* best-established HTSlib-backed baseline
* canonical point of comparison for BAM-oriented workflows

### Additional BAM Comparator

* `sambamba`

Rationale:

* commonly used for BAM sorting and indexing comparisons
* useful to distinguish Bamana versus HTSlib from Bamana versus alternative
  multi-threaded BAM tooling

### FASTQ and Subsampling Comparators

* `seqtk`
* `rasusa`

Rationale:

* `seqtk` is a widely used FASTQ-oriented sampling baseline
* `rasusa` is explicitly relevant for read and alignment downsampling, but its
  semantics are often coverage- or count-based rather than purely fractional

### ONT Ingestion Comparator

* `EPI2ME fastcat`

Rationale:

* directly relevant to the project's ingestion and concatenation performance
  target
* one explicit project goal is to beat `fastcat` where Bamana chooses to own
  similar operator workflows

## 4. Fairness Policy

The benchmark framework follows four fairness rules.

### 4.1 Do Not Force Invalid Workflows

If a tool does not support a scenario or operation meaningfully, record it as:

* `unsupported`
* `roadmap_blocked`
* `partial`

Do not force an invalid comparison and then describe it as a speed result.

### 4.2 Use Sensible Native Order

Per-tool workflows may use their natural best-order path, for example:

* subsample then sort then index
* concatenate only for `fastcat`

The exact workflow path must still be recorded in `workflow_variant`.

### 4.3 Keep Semantic Differences Explicit

The result schema records `semantic_equivalence`:

* `full`
* `partial`
* `unsupported`
* `roadmap_blocked`

This prevents partial ingestion or subsampling workflows from being treated as
identical to BAM normalization workflows when they are not.

### 4.4 Record Failures

Unsupported, failed, and blocked runs are preserved in the result tables.
Failures are data.

## 5. Supported and Unsupported Matrix

| Tool | `mapped_bam_pipeline` | `unmapped_bam_pipeline` | `fastq_consume_pipeline` |
| --- | --- | --- | --- |
| `bamana` | partial: `subsample` plus `sort`, with index still deferred | supported | supported via `consume --mode unmapped` |
| `samtools` | supported | supported | unsupported |
| `sambamba` | supported | supported | unsupported |
| `seqtk` | unsupported | unsupported | partial |
| `rasusa` | unsupported by default pending fair strategy pinning | unsupported by default pending fair strategy pinning | unsupported by default pending fair strategy pinning |
| `fastcat` | unsupported | unsupported | partial |

The audited per-tool workflow definitions now live in:

* [tools/workflow_variant_matrix.md](/Users/stephen/Projects/bamana/benchmarks/tools/workflow_variant_matrix.md)
* [tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json)

Those files are the source of truth for supported versus unsupported tool and
scenario pairings, exact `workflow_variant` ids, and wrapper entrypoints.

## 6. Input Expectations

The framework is intended for large real files supplied by the user.

Expected inputs include:

* mapped BAM suitable for sort and index benchmarking
* unmapped BAM for subsample-only benchmarking
* FASTQ.GZ collections for ingestion and concatenation comparisons

The preferred input declaration mechanism is a manifest rather than ad hoc path
lists. The manifest records:

* source input id
* source input path
* input category
* mapped state
* expected sort order
* index availability
* storage context
* staging policy
* allowed scenarios

The preferred run declaration mechanism is a params JSON file validated against
[params.schema.json](/Users/stephen/Projects/bamana/benchmarks/params.schema.json).
That params layer records:

* `input_manifest`
* `dataset_ids`
* `tools`
* `scenarios`
* `replicates`
* `warmup_runs`
* `subsample_fraction`
* `subsample_seed`
* `subsample_mode`
* `output_dir`

The pipeline computes input size and exact record counts once per input so the
benchmark layer can report throughput rather than timing only.

The repository-level policy for source and derived benchmark inputs is defined
in:

* [input-policy.md](/Users/stephen/Projects/bamana/benchmarks/input-policy.md)
* [staging.md](/Users/stephen/Projects/bamana/benchmarks/staging.md)
* [cleanup.md](/Users/stephen/Projects/bamana/benchmarks/cleanup.md)

## 7. Replication Strategy

The first framework includes:

* `warmup_runs`
* `replicates`
* seeded subsampling
* explicit `subsample_mode`

Recommended first pass:

* `warmup_runs = 1`
* `replicates = 3` or `5`
* fixed seed for deterministic mode

Deterministic mode reduces workload variance. Repeated runs still capture
system-level variance.

Replication policy for staged and derived inputs:

* source inputs are read-only and external
* staged inputs should be materialized before timing
* deterministic or seeded derived inputs should be generated once per
  source-plus-policy combination and reused across replicates
* replicate content should remain stable unless a scenario explicitly studies
  input randomness or cache effects

## 8. Measurement Outputs

Per-run rows record at least:

* scenario
* source input id and source input path
* staged input id and staged input path
* input type and input size
* staging mode and storage context
* scenario materialization
* tool and tool version
* workflow variant
* replicate id and warmup status
* subsample fraction, seed, and mode
* wall-clock time
* user and system CPU time
* max RSS
* input and output bytes
* records processed
* exit code
* success flag
* notes

## 8.0 Minimal Execution Slice

The current benchmark pipeline intentionally prioritizes raw execution capture
over final reporting.

The first executable slice performs:

* manifest resolution
* scenario and tool matrix expansion
* wrapper invocation
* raw result JSON and TSV emission
* raw result inventory generation

Aggregation, plotting, and publication reporting remain downstream consumers of
those raw result artifacts rather than mandatory steps in the first execution
path.

The first executable slice is intentionally limited to:

* mapped BAM inputs
* FASTQ.GZ inputs
* `bamana`, `samtools`, and `fastcat`
* `mapped_bam_pipeline`, `fastq_consume_pipeline`, and `subsample_only`

### 8.1 Why Workflow Variants Matter

Benchmarking is only interpretable if the framework records not just which
tool ran, but what that tool actually did.

The benchmark layer therefore separates:

* tool identity: `bamana`, `samtools`, `fastcat`, and other comparators
* workflow variant: the exact operation chain for a tool in one scenario, such
  as `bamana_subsample_sort_partial_index` or `fastcat_concat_gzip`
* wrapper implementation: the Nextflow module or shell wrapper that invoked the
  comparator and captured version, command-path provenance, and outputs

This matters because two tools can both participate in a scenario while still
following materially different native command paths. Those paths must remain
auditable in result tables and publication figures.

The wrapper and registry contract for this layer lives in:

* [tools/tool_wrapper_contract.md](/Users/stephen/Projects/bamana/benchmarks/tools/tool_wrapper_contract.md)
* [tools/tool_registry.schema.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.schema.json)
* [tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json)

These fields are intended to make benchmark results interpretable when large
input locality or staging policy differs across environments.

The schema is defined in
[benchmarks/results/benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json).

The structured raw wrapper output is defined in
[benchmarks/results/result.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/result.schema.json).

Contract rule:

* raw JSON preserves nested execution context
* tidy rows flatten that context for aggregation and plotting
* contract changes should update the wrapper, aggregation script, plotting
  script, and example result artifacts together

## 9. Aggregation and Plotting Outputs

The R layer generates:

* `aggregated/tidy_results.csv` derived from raw `*.result.json`
* `aggregated/tidy_summary.csv` grouped from successful measured runs
* support-matrix CSV outputs
* benchmark figures under `plots/`

The first analysis slice is intentionally narrow. It currently produces:

* one wall-time-by-tool figure faceted by scenario
* successful measured replicate points
* grouped median markers

Interpretation rules:

* raw JSON remains the source of truth for one attempted run
* tidy rows keep successful, unsupported, and failed runs visible
* success-only performance summaries must not silently erase failed or
  unsupported attempts from the underlying dataset
* the first wall-time figure excludes unsupported and failed rows from timing
  display, but those rows remain visible in tidy data and support summaries

## 9.1 Why The Support Matrix Exists

Timing figures alone are not sufficient for honest interpretation.

A missing timing bar can mean:

* the combination is explicitly unsupported
* the combination is supported in principle but was not attempted in one run
  set
* the combination was attempted but failed

The support matrix separates intended capability from observed outcome so those
cases remain visible in review and publication artifacts.

The support-matrix layer is defined in:

* [results/support_matrix_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/support_matrix_contract.md)
* [R/build_support_matrix.R](/Users/stephen/Projects/bamana/benchmarks/R/build_support_matrix.R)

## 10. Bamana Subsample Requirement

The benchmark framework now requires and uses the Bamana command:

`bamana subsample --input <file> --out <output> --fraction <f> [--seed <int>] [--mode <random|deterministic>]`

Benchmark expectations for this command:

* support BAM and FASTQ.GZ at minimum
* support `random` and `deterministic` modes
* make seeded comparison possible
* expose semantics clearly enough that comparator mismatches can be documented

The framework still records semantic mismatches honestly when comparator tools
are coverage-based, count-based, or otherwise not directly equivalent to
Bamana's fraction-based modes.

## 11. Known Limitations of the First Iteration

The first benchmark slice is intentionally honest about current limits:

* Bamana executable indexing is still incomplete for full mapped-BAM chains
* `rasusa` is recorded explicitly but defaulted to unsupported until the
  fractional versus coverage strategy is pinned fairly
* `fastcat` is a partial comparator for ingestion space, not a BAM sort/index
  baseline
* the first plotting layer emphasizes end-to-end workflow variants rather than
  detailed per-operation flame-style breakdowns

These are acceptable first-iteration limitations so long as they remain
documented.
### Scenario D: `subsample_only`

Input:

* mapped BAM
* unmapped BAM
* FASTQ.GZ where the comparator actually supports direct read subsampling

Primary operation chain:

1. subsample only

Notes:

* this scenario is intended for explicit subsampling comparisons without sort,
  index, or consume stages
* unsupported combinations are recorded explicitly rather than forced
