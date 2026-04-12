# Benchmark Status For Tomorrow

This document is the short operational summary for the next benchmark session.

## What Is Ready

The repository is in a benchmark-ready scaffold state for a first real run:

* manifest-driven dataset declaration exists
* example params files exist
* wrapper scripts exist for `bamana`, `samtools`, and `fastcat`
* the minimal Nextflow slice can expand dataset/scenario/tool/replicate runs
* one raw result JSON file is emitted per attempted run
* unsupported combinations are preserved explicitly rather than disappearing
* raw-result-first R scripts exist for tidy aggregation, first plotting, and
  support-matrix generation
* the architecture and roadmap now clearly say that Bamana's long-term hot
  path is native-core, not `noodles`-driven

## What Is Scaffolded But May Need Light Debugging Tomorrow

These parts are coherent enough to use, but should be treated as first-run
debugging targets rather than assumed production-hard:

* the minimal Nextflow slice itself
* wrapper-to-runtime handoff for local tool paths
* the first raw-to-tidy aggregation pass on real outputs
* support-matrix rendering on a real run directory
* environment-specific availability of `nextflow`, `Rscript`, `samtools`,
  `fastcat`, and the local Bamana binary

## What Is Intentionally Deferred

These items are not blockers for tomorrow's smoke test, but they are not done:

* full Bamana BAM index creation
* broad comparator coverage across every scenario
* final publication-quality multi-figure reporting
* larger replicate matrices
* deep performance optimization of the native-core migration itself

## Bamana Command Readiness Notes

Treat the benchmark-facing Bamana commands like this tomorrow:

* `subsample`
  Implemented and suitable for benchmark use on BAM, FASTQ, and FASTQ.GZ.
* `consume`
  Suitable for the current `consume --mode unmapped` FASTQ.GZ ingestion path.
* `sort`
  Suitable for the current first-slice benchmark smoke test.
* `index`
  Not yet a full benchmark-ready BAM index writer; wrappers and interpretation
  must treat full index parity as deferred.

## Recommended First Benchmark Run

Start small.

Recommended initial run:

* datasets:
  * one mapped BAM dataset
  * one FASTQ.GZ dataset
* tools:
  * `bamana`
  * `samtools`
  * `fastcat` where relevant
* scenarios:
  * `mapped_bam_pipeline`
  * `fastq_consume_pipeline`
* run shape:
  * `replicates = 1`
  * `warmup_runs = 0`
  * `include_unsupported_matrix_rows = true`

Suggested command pattern:

```bash
nextflow run benchmarks/main.nf \
  -profile local \
  -params-file benchmarks/params.examples/local.example.json
```

Then inspect, in order:

1. `results/raw/`
2. `results/metadata/raw_result_inventory.tsv`
3. `results/aggregated/tidy_results.csv`
4. `results/aggregated/support_matrix.csv`
5. `results/plots/wall_time_by_tool.png`

## Do Not Do First Tomorrow

* do not start with many datasets
* do not start with many replicates
* do not start with every comparator and every scenario
* do not start by debugging final publication plots
* do not benchmark unsupported combinations and treat them as failures
* do not assume Bamana index parity is already complete

## Definition Of A Good Tomorrow

Tomorrow is successful if the team can say:

* the wrappers planned the right commands
* raw result JSON exists for every attempted run
* unsupported rows are visible and honest
* tidy aggregation runs on the produced raw results
* the first wall-time figure and support matrix can be generated

If those conditions are met, the repository is doing its job. The next work
should then be benchmark debugging and performance iteration rather than more
framework ambiguity.
