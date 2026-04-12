# Workflow Variant Matrix

This matrix defines which tool participates in which scenario, what workflow
variant id is used, and whether the combination is supported.

## Why Workflow Variants Matter

Two tools can both appear to perform “subsample then sort then index” while
still using materially different command paths. The benchmark framework must
therefore record:

* tool identity
* scenario id
* workflow variant id
* fairness notes

Without this, benchmark rows are not auditable.

## Scenario: `mapped_bam_pipeline`

Typical chain:

1. subsample
2. sort
3. index

| Tool | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- |
| `bamana` | `bamana_subsample_sort_partial_index` | supported (partial) | Bamana `subsample` -> Bamana `sort` | Index creation remains deferred, so semantic equivalence is partial rather than full. |
| `samtools` | `samtools_view_sort_index` | supported | `samtools view -s` -> `samtools sort` -> `samtools index` | Canonical BAM baseline. |
| `sambamba` | `sambamba_view_sort_index` | supported | `sambamba view -s` -> `sambamba sort` -> `sambamba index` | Additional BAM comparator with broadly comparable ordering. |
| `seqtk` | `unsupported` | unsupported | n/a | FASTQ-only comparator in this first framework. |
| `rasusa` | `rasusa_strategy_required` | unsupported | n/a | Deferred until a fair fractional-versus-coverage strategy is pinned. |
| `fastcat` | `unsupported` | unsupported | n/a | Not a BAM sort/index comparator. |

## Scenario: `unmapped_bam_pipeline`

Typical chain:

1. subsample
2. omit sort and index unless meaningful

| Tool | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- |
| `bamana` | `bamana_subsample_only` | supported | Bamana `subsample` | Native BAM subsampling path. |
| `samtools` | `samtools_view_subsample_only` | supported | `samtools view -s` | Natural BAM-only comparator path. |
| `sambamba` | `sambamba_view_subsample_only` | supported | `sambamba view -s` | Comparable BAM-only subsampling path. |
| `seqtk` | `unsupported` | unsupported | n/a | Not a BAM comparator. |
| `rasusa` | `rasusa_strategy_required` | unsupported | n/a | Deferred pending fair strategy definition. |
| `fastcat` | `unsupported` | unsupported | n/a | Not applicable to BAM. |

## Scenario: `fastq_consume_pipeline`

Typical chain:

1. consume, concatenate, or normalize FASTQ.GZ input
2. optional downstream output transformation when the tool’s role supports it

| Tool | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- |
| `bamana` | `bamana_consume_unmapped_bam` | supported | Bamana `consume --mode unmapped` | Produces BAM-oriented normalization output. |
| `samtools` | `unsupported` | unsupported | n/a | Not a suitable FASTQ consume comparator in this first contract. |
| `sambamba` | `unsupported` | unsupported | n/a | Not a suitable FASTQ consume comparator in this first contract. |
| `seqtk` | `seqtk_sample_gzip` | supported (partial) | `seqtk sample` -> `gzip` | FASTQ-only partial comparator; does not normalize into BAM. |
| `rasusa` | `rasusa_strategy_required` | unsupported | n/a | Deferred pending fair strategy definition. |
| `fastcat` | `fastcat_concat_gzip` | supported (partial) | `fastcat` -> `gzip` | ONT-oriented ingestion and concatenation baseline. |

## Scenario: `subsample_only`

Direct subsampling comparison on a supported input type.

| Tool | Workflow Variant | Status | Operation Ordering | Notes |
| --- | --- | --- | --- | --- |
| `bamana` | `bamana_subsample_only` | supported | Bamana `subsample` | Supports BAM and FASTQ.GZ in the current benchmark framework. |
| `samtools` | `samtools_view_subsample_only` | supported for BAM | `samtools view -s` | BAM-only comparator path. |
| `sambamba` | `sambamba_view_subsample_only` | supported for BAM | `sambamba view -s` | BAM-only comparator path. |
| `seqtk` | `seqtk_sample_gzip` | supported for FASTQ.GZ | `seqtk sample` -> `gzip` | FASTQ-only comparator path. |
| `rasusa` | `rasusa_strategy_required` | unsupported | n/a | Deferred pending fair strategy definition. |
| `fastcat` | `unsupported` | unsupported | n/a | Not a subsampling comparator. |

## Fairness Notes

* `samtools` is the canonical BAM baseline and should anchor BAM-oriented
  comparisons.
* `fastcat` is included because one explicit project goal is to beat it in
  FASTQ ingestion and concatenation space, not because it is a BAM toolkit.
* `seqtk` is retained as a FASTQ-oriented partial comparator, especially for
  subsample-only and FASTQ consume-adjacent analyses.
* `rasusa` remains explicit in the matrix so unsupported rows are visible
  rather than silently omitted.
