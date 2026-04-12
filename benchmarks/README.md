# Bamana Benchmark Framework

This directory contains the first reproducible benchmarking framework for
Bamana. The framework is intended for real large user-supplied files, including
whole human genome BAM inputs and large FASTQ.GZ collections, and is designed
to answer performance questions honestly even when Bamana does not yet win.

## Purpose

The benchmark suite exists to answer:

* how Bamana compares with established tools on common file-handling workflows
* where Bamana is currently slower or faster
* which workflow stages dominate runtime and memory
* whether Bamana is competitive in ingestion and normalization workloads
* whether Bamana can eventually beat EPI2ME fastcat in ONT-style ingestion and
  concatenation workflows

This framework is for transparent measurement, not marketing. A slower result
is still informative and should drive implementation work.

## Primary Baseline Recommendation

If one BAM-oriented baseline tool must be chosen, use `samtools`.

`samtools` is the canonical BAM comparator because it is the best-established
HTSlib-backed baseline for BAM manipulation. The framework also includes:

* `fastcat` for ONT and FASTQ ingestion or concatenation comparisons
* `sambamba` where BAM sorting and indexing comparisons are relevant
* `seqtk` for FASTQ-oriented subsampling baselines
* `rasusa` as an explicit comparator candidate for read or alignment
  downsampling, while recording semantic mismatches honestly when the current
  benchmark contract is fractional rather than coverage-based

## Current Bamana Gaps

The benchmark framework can now execute real Bamana subsampling on BAM, FASTQ,
and FASTQ.GZ inputs. Remaining current gaps are:

* Bamana executable BAM index creation is still incomplete for full
  sort-plus-index parity
* Bamana fastq-ingestion benchmarking still uses `consume`, while dedicated
  fastq subsample benchmark variants remain to be layered in
* comparator semantics still need careful review where tools are only partial
  matches for a given scenario

This is deliberate. The benchmark layer records partial or unsupported
comparisons explicitly instead of pretending the tools are directly equivalent.

## Directory Layout

* [main.nf](/Users/stephen/Projects/bamana/benchmarks/main.nf): DSL2 workflow entry point
* [nextflow.config](/Users/stephen/Projects/bamana/benchmarks/nextflow.config): default parameters and profiles
* [conf/](/Users/stephen/Projects/bamana/benchmarks/conf): local and Docker execution profiles
* [modules/](/Users/stephen/Projects/bamana/benchmarks/modules): staging, per-tool benchmark execution, aggregation, plotting
* [bin/](/Users/stephen/Projects/bamana/benchmarks/bin): timing and tool-version helpers
* [R/](/Users/stephen/Projects/bamana/benchmarks/R): aggregation and plotting scripts
* [results/](/Users/stephen/Projects/bamana/benchmarks/results): result schema and output layout notes
* [design.md](/Users/stephen/Projects/bamana/benchmarks/design.md): benchmark design and fairness policy
* [params.schema.json](/Users/stephen/Projects/bamana/benchmarks/params.schema.json): benchmark parameter schema
* [Dockerfile](/Users/stephen/Projects/bamana/benchmarks/Dockerfile): reproducible benchmark environment

## Benchmark Scenarios

The first framework defines three core scenarios:

* `mapped_bam_chain`: mapped BAM subsample then sort then index where sensible
* `unmapped_bam_chain`: unmapped BAM subsample with sort and index omitted when
  not meaningful
* `fastq_ingest_chain`: FASTQ.GZ ingestion or concatenation workflows,
  including Bamana `consume` and `fastcat`

Replication is built in via `replicate_count` and `warmup_runs`.

## Fairness Policy

The workflow does not force every tool through an identical but unnatural
execution order. Instead, it records:

* the scenario
* the exact tool-specific workflow variant
* whether the comparison is semantically `full`, `partial`, `unsupported`, or
  `roadmap_blocked`

Unsupported combinations are recorded explicitly in the result tables instead
of being silently omitted or misreported as slow.

## Result Outputs

Per-run outputs include:

* `*.result.tsv`
* `*.result.json`

Aggregated outputs include:

* `benchmark_runs.tsv`
* `benchmark_runs.json`
* `benchmark_summary.tsv`
* `benchmark_summary.json`
* `benchmark_support_matrix.tsv`
* `benchmark_failures.tsv`

Publication-ready figures include:

* wall time by tool and scenario
* throughput by tool and scenario
* memory by tool and scenario
* replicate variability plots
* support-status heatmaps

## Running Locally

Build the benchmark container:

```bash
docker build -f benchmarks/Dockerfile -t bamana-bench:latest .
```

Run the workflow with Docker:

```bash
cd benchmarks
nextflow run main.nf \
  -profile docker \
  --mapped_bams "/data/hg38.mapped.bam" \
  --unmapped_bams "/data/hg38.unmapped.bam" \
  --fastq_gzs "/data/run.fastq.gz" \
  --replicate_count 5 \
  --warmup_runs 1 \
  --subsample_fraction 0.1 \
  --subsample_seed 104729 \
  --output_dir "/workspace/benchmarks/results/latest"
```

## Installed Toolchain

The benchmark container is intentionally explicit:

* Java and Nextflow for workflow execution
* `samtools`
* `sambamba`
* `seqtk`
* `rasusa`
* `fastcat`
* `R` plus publication-oriented plotting libraries
* `jq`, `pigz`, and GNU `time` for measurement and result assembly

Use:

```bash
print_tool_versions.sh
```

to capture the installed version baseline inside the container.
