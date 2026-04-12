# Bamana Benchmark Framework

This directory contains the first reproducible benchmarking framework for
Bamana. The framework is intended for real large user-supplied files, including
whole human genome BAM inputs and large FASTQ.GZ collections, and is designed
to answer performance questions honestly even when Bamana does not yet win.

The repository does not ship those large benchmark inputs. Instead, it ships
the policy, schema, staging guidance, and example manifest needed to describe
and benchmark them reproducibly.

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
* [input-policy.md](/Users/stephen/Projects/bamana/benchmarks/input-policy.md): source-versus-derived input governance
* [staging.md](/Users/stephen/Projects/bamana/benchmarks/staging.md): staging and reuse policy
* [cleanup.md](/Users/stephen/Projects/bamana/benchmarks/cleanup.md): cleanup and retention rules
* [inputs/](/Users/stephen/Projects/bamana/benchmarks/inputs): manifest schema, example manifest, and operator guidance
* [params.schema.json](/Users/stephen/Projects/bamana/benchmarks/params.schema.json): benchmark parameter schema
* [params.examples/](/Users/stephen/Projects/bamana/benchmarks/params.examples): ready-to-edit example Nextflow params files
* [Dockerfile](/Users/stephen/Projects/bamana/benchmarks/Dockerfile): reproducible benchmark environment

## Benchmark Scenarios

The user-facing configuration layer now exposes these stable scenario ids:

* `mapped_bam_pipeline`: mapped BAM subsample then sort then index where sensible
* `unmapped_bam_pipeline`: unmapped BAM subsample with sort and index omitted
  when not meaningful
* `fastq_consume_pipeline`: FASTQ.GZ ingestion or concatenation workflows,
  including Bamana `consume` and `fastcat`
* `subsample_only`: explicit subsample benchmarking without downstream sort or
  index

Replication is built in via `replicates` and `warmup_runs`.

## Benchmark Inputs

Large inputs should be supplied by manifest whenever the benchmark needs to be
repeatable across operators or environments.

Recommended flow:

1. keep source BAM and FASTQ.GZ files outside the repository
2. describe them with a JSON manifest
3. validate the manifest locally
4. run Nextflow with `--input_manifest`

The first-slice manifest scaffold lives at:

* [inputs/manifest.schema.json](/Users/stephen/Projects/bamana/benchmarks/inputs/manifest.schema.json)
* [inputs/example_manifest.json](/Users/stephen/Projects/bamana/benchmarks/inputs/example_manifest.json)
* [inputs/README.md](/Users/stephen/Projects/bamana/benchmarks/inputs/README.md)

The run-centric Nextflow params layer lives at:

* [params.schema.json](/Users/stephen/Projects/bamana/benchmarks/params.schema.json)
* [params.examples/local.example.json](/Users/stephen/Projects/bamana/benchmarks/params.examples/local.example.json)
* [params.examples/mapped_bam.example.json](/Users/stephen/Projects/bamana/benchmarks/params.examples/mapped_bam.example.json)
* [params.examples/fastq_gz.example.json](/Users/stephen/Projects/bamana/benchmarks/params.examples/fastq_gz.example.json)

Manifest versus params:

* manifest: stable dataset metadata such as path, type, index, reference, and
  allowed scenarios
* params file: one benchmark run definition such as dataset selection,
  tools, scenarios, replicate count, seed, and output directory

## Staging and Reuse Policy

The benchmark framework distinguishes:

* Tier A source inputs: large user-supplied read-only files
* Tier B derived inputs: subsampled or normalized scenario artifacts

Default policy:

* benchmark-managed staging occurs before timed execution
* staging metadata is recorded per run
* deterministic or seeded derived inputs should be reused across replicates
* source inputs must never be deleted by cleanup

The detailed policy is in:

* [input-policy.md](/Users/stephen/Projects/bamana/benchmarks/input-policy.md)
* [staging.md](/Users/stephen/Projects/bamana/benchmarks/staging.md)
* [cleanup.md](/Users/stephen/Projects/bamana/benchmarks/cleanup.md)

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

The benchmark result layer has two levels:

* raw structured per-run JSON records
* tidy flat per-run rows and grouped summaries for aggregation and plotting

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

Contracts and examples for this layer live under
[results/](/Users/stephen/Projects/bamana/benchmarks/results).

The key rule is that unsupported and failed runs remain explicit:

* unsupported is not failure
* failed is not unsupported
* successful runs alone drive performance summaries

## Quick Start With Your Own BAM or FASTQ.GZ Inputs

1. Copy [inputs/example_manifest.json](/Users/stephen/Projects/bamana/benchmarks/inputs/example_manifest.json) and edit the dataset paths, ids, index paths, and reference context for your environment.
2. Copy one of the files under [params.examples/](/Users/stephen/Projects/bamana/benchmarks/params.examples) and edit `input_manifest`, `dataset_ids`, `output_dir`, and any tool or scenario selection.
3. Validate the manifest locally with `python bin/validate_inputs.py --manifest /abs/path/to/your-manifest.json`.
4. Run Nextflow with `-params-file /abs/path/to/your-params.json`.

## Running Locally

Build the benchmark container:

```bash
docker build -f benchmarks/Dockerfile -t bamana-bench:latest .
```

Run the workflow with Docker:

```bash
python benchmarks/bin/validate_inputs.py --manifest /abs/path/to/benchmark-inputs.json
nextflow run benchmarks/main.nf \
  -profile docker \
  -params-file "/abs/path/to/benchmark-run.json"
```

Direct path parameters remain available for ad hoc runs:

* `--mapped_bams`
* `--unmapped_bams`
* `--fastq_gzs`

Manifest-driven runs are preferred for audited or publication-oriented
benchmarks because they preserve source-input ids, staging policy, storage
context, and allowed scenario declarations.

## Dataset Resolution Flow

The benchmark workflow resolves inputs in this order:

1. load Nextflow params
2. load the input manifest from `input_manifest`
3. filter manifest datasets by `dataset_ids`
4. validate requested scenarios against each dataset's `allowed_benchmark_scenarios`
5. stage the selected datasets according to manifest or params staging policy
6. execute tool-specific workflows and record unsupported combinations

Unsupported tool or tool-scenario combinations are recorded explicitly in the
results when `include_unsupported_matrix_rows` is `true`; they are not treated
as benchmark failures.

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

## Storage Profiles

The benchmark config includes example profiles for locality-sensitive runs:

* `-profile local_ssd`
* `-profile shared_fs`
* `-profile docker`

These profiles mainly set storage-context and staging defaults. They do not
change the benchmark interpretation by themselves; the per-run metadata still
records staging mode and storage context explicitly.
