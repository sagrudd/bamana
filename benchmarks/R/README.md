# Benchmark R Layer

This directory contains the aggregation and plotting layer for benchmark
results.

## Input Expectations

The R scripts currently consume the tidy flat per-run TSV outputs emitted by
the benchmark wrapper layer.

Primary contract files:

* [../results/result.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/result.schema.json): structured raw per-run JSON contract
* [../results/benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json): flat tidy row contract
* [../results/tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md): column-level semantics and aggregation rules
* [../tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json): canonical tool and workflow variant ids used in tidy rows

## What `aggregate_results.R` Reads

`aggregate_results.R` reads:

* one `*.result.tsv` file per attempted benchmark run

Each TSV row represents one attempted run and includes:

* identity fields such as `run_id`, `scenario`, `tool`, and `workflow_variant`
* input provenance fields such as `source_input_id` and `staged_input_id`
* status fields such as `status`, `success`, `unsupported`, and `failed`
* timing and memory fields where available
* throughput fields or enough information to derive them

## Unsupported and Failed Runs

The R layer must preserve the difference between:

* `status = success`
* `status = unsupported`
* `status = failed`

Rules:

* unsupported rows remain in the tidy dataset for support matrices
* failed rows remain in the tidy dataset for reliability analysis
* only successful measured runs should contribute to median runtime and
  throughput summaries

## Aggregated Outputs

The aggregation script writes:

* `benchmark_runs.tsv` and `benchmark_runs.json`
* `benchmark_summary.tsv` and `benchmark_summary.json`
* `benchmark_support_matrix.tsv` and `benchmark_support_matrix.json`
* `benchmark_failures.tsv`

`benchmark_summary.*` should aggregate successful measured runs only for
performance metrics, while still reporting:

* `n_runs`
* `n_success`
* `n_failed`
* `n_unsupported`
* `n_skipped`

## Plotting Expectations

`plot_benchmarks.R` assumes:

* success-only rows drive wall-time and performance plots
* support matrices include unsupported and failed rows
* column names stay stable and snake_case

If the tidy result contract changes, review:

* `aggregate_results.R`
* `plot_benchmarks.R`
* [../results/tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md)
