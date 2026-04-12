# Workflow Variant Matrix

This matrix defines which tool participates in which scenario, what workflow
variant id is used, which wrapper plans the command path, and whether the
combination is supported.

## Why Workflow Variants Matter

Two tools can both appear to perform “subsample then sort then index” while
still using materially different command paths. The benchmark framework must
therefore record:

* tool identity
* scenario id
* workflow variant id
* wrapper implementation
* fairness notes

Without this, benchmark rows are not auditable.

## Scenario: `mapped_bam_pipeline`

Typical chain:

1. subsample
2. sort
3. index

| Tool | Wrapper | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- | --- |
| `bamana` | `wrappers/bamana.sh` | `bamana_subsample_sort_partial_index` | supported (partial) | Bamana `subsample` -> Bamana `sort` | Index creation remains deferred, so semantic equivalence is partial rather than full. The wrapper also scaffolds `bamana_subsample_sort_index` for future full-path benchmarking. |
| `samtools` | `wrappers/samtools.sh` | `samtools_view_sort_index` | supported | `samtools view -s` -> `samtools sort` -> `samtools index` | Canonical BAM baseline. |
| `sambamba` | module-local | `sambamba_view_sort_index` | supported | `sambamba view -s` -> `sambamba sort` -> `sambamba index` | Additional BAM comparator with broadly comparable ordering. |
| `seqtk` | module-local | `unsupported` | unsupported | n/a | FASTQ-only comparator in this first framework. |
| `rasusa` | module-local | `rasusa_strategy_required` | unsupported | n/a | Deferred until a fair fractional-versus-coverage strategy is pinned. |
| `fastcat` | `wrappers/fastcat.sh` | `unsupported` | unsupported | n/a | Not a BAM sort/index comparator. |

## Scenario: `unmapped_bam_pipeline`

Typical chain:

1. subsample
2. omit sort and index unless meaningful

| Tool | Wrapper | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- | --- |
| `bamana` | `wrappers/bamana.sh` | `bamana_subsample_only` | supported | Bamana `subsample` | Native BAM subsampling path. |
| `samtools` | `wrappers/samtools.sh` | `samtools_view_subsample_only` | supported | `samtools view -s` | Natural BAM-only comparator path. The wrapper also accepts `samtools_subsample_only` as an explicit alias. |
| `sambamba` | module-local | `sambamba_view_subsample_only` | supported | `sambamba view -s` | Comparable BAM-only subsampling path. |
| `seqtk` | module-local | `unsupported` | unsupported | n/a | Not a BAM comparator. |
| `rasusa` | module-local | `rasusa_strategy_required` | unsupported | n/a | Deferred pending fair strategy definition. |
| `fastcat` | `wrappers/fastcat.sh` | `unsupported` | unsupported | n/a | Not applicable to BAM. |

## Scenario: `fastq_consume_pipeline`

Typical chain:

1. consume, concatenate, or normalize FASTQ.GZ input
2. optional downstream output transformation when the tool’s role supports it

| Tool | Wrapper | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- | --- |
| `bamana` | `wrappers/bamana.sh` | `bamana_consume_unmapped_bam` | supported | Bamana `consume --mode unmapped` | Produces BAM-oriented normalization output. The wrapper also scaffolds `bamana_consume_only`, `bamana_consume_sort`, and `bamana_consume_sort_index` for later scenario expansion. |
| `samtools` | `wrappers/samtools.sh` | `unsupported` | unsupported | n/a | Not a suitable FASTQ consume comparator in this first contract. |
| `sambamba` | module-local | `unsupported` | unsupported | n/a | Not a suitable FASTQ consume comparator in this first contract. |
| `seqtk` | module-local | `seqtk_sample_gzip` | supported (partial) | `seqtk sample` -> `gzip` | FASTQ-only partial comparator; does not normalize into BAM. |
| `rasusa` | module-local | `rasusa_strategy_required` | unsupported | n/a | Deferred pending fair strategy definition. |
| `fastcat` | `wrappers/fastcat.sh` | `fastcat_concat_gzip` | supported (partial) | `fastcat` -> `gzip` | ONT-oriented ingestion and concatenation baseline. The wrapper also accepts `fastcat_consume_only` and `fastcat_fastq_concat_only` aliases for explicit ingestion-space benchmarking. |

## Scenario: `subsample_only`

Direct subsampling comparison on a supported input type.

| Tool | Wrapper | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- | --- |
| `bamana` | `wrappers/bamana.sh` | `bamana_subsample_only` | supported | Bamana `subsample` | Supports BAM and FASTQ.GZ in the current benchmark framework. |
| `samtools` | `wrappers/samtools.sh` | `samtools_view_subsample_only` | supported for BAM | `samtools view -s` | BAM-only comparator path. |
| `sambamba` | module-local | `sambamba_view_subsample_only` | supported for BAM | `sambamba view -s` | BAM-only comparator path. |
| `seqtk` | module-local | `seqtk_sample_gzip` | supported for FASTQ.GZ | `seqtk sample` -> `gzip` | FASTQ-only comparator path. |
| `rasusa` | module-local | `rasusa_strategy_required` | unsupported | n/a | Deferred pending fair strategy definition. |
| `fastcat` | `wrappers/fastcat.sh` | `unsupported` | unsupported | n/a | Not a subsampling comparator. |

## Fairness Notes

* `samtools` is the canonical BAM baseline and should anchor BAM-oriented
  comparisons.
* `fastcat` is included because one explicit project goal is to beat it in
  FASTQ ingestion and concatenation space, not because it is a BAM toolkit.
* `seqtk` is retained as a FASTQ-oriented partial comparator, especially for
  subsample-only and FASTQ consume-adjacent analyses.
* `rasusa` remains explicit in the matrix so unsupported rows are visible
  rather than silently omitted.
* Wrapper scripts preserve comparator-native command paths rather than forcing
  every tool into a single synthetic operation graph.
