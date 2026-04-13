# Benchmark R Layer

This directory contains the first analysis layer for the Bamana benchmark
framework. The current slice is intentionally simple:

1. aggregate raw per-run result JSON into tidy CSV outputs
2. plot a first wall-time comparison figure from successful measured runs
3. keep unsupported and failed runs visible in the tidy data

## Input Expectations

The analysis layer is now raw-result-first.

Primary contract files:

* [../results/result.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/result.schema.json): structured raw per-run JSON contract emitted by the execution layer
* [../results/benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json): flat tidy row contract produced during aggregation
* [../results/tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md): column-level semantics and grouping rules
* [../tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json): canonical tool and workflow variant ids used by support reporting

## What `aggregate_results.R` Does

`aggregate_results.R` reads:

* one `*.result.json` file per attempted benchmark run, typically from
  `results/raw/`

For each raw result JSON it:

* parses the nested execution record
* flattens it into one tidy row
* preserves `success`, `unsupported`, and `failed` status semantics
* derives throughput fields when they are not already present but sufficient
  inputs exist

It writes:

* `results/aggregated/tidy_results.csv`
* `results/aggregated/tidy_summary.csv`

The tidy dataset keeps all attempted runs visible. The grouped summary keeps
all run counts, but performance metrics such as median wall time are computed
from successful measured runs only.

## What `plot_benchmarks.R` Does

`plot_benchmarks.R` reads:

* `results/aggregated/tidy_results.csv`
* `results/aggregated/tidy_summary.csv`

It writes:

* `results/plots/wall_time_by_tool.png`
* `results/plots/wall_time_by_tool.pdf`

The first figure is intentionally narrow:

* x-axis: tool
* y-axis: wall time in seconds
* facets: scenario
* points: successful measured replicates
* black diamonds: grouped median wall time

This figure does not attempt to show unsupported or failed runs as timing
results. Those remain visible in the tidy data and support summaries.

## Unsupported and Failed Runs

Hard rules:

* unsupported rows remain in `tidy_results.csv`
* failed rows remain in `tidy_results.csv`
* only successful measured runs contribute to wall-time medians and the first
  wall-time figure

Use the support matrix layer for capability interpretation:

* [build_support_matrix.R](/Users/stephen/Projects/bamana/benchmarks/R/build_support_matrix.R)
* [../results/support_matrix_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/support_matrix_contract.md)

## Quick Start

Assuming the minimal Nextflow benchmark slice has already produced raw result
JSON under `results/raw/`:

1. Aggregate raw results:

```bash
Rscript /Users/stephen/Projects/bamana/benchmarks/R/aggregate_results.R \
  --input-dir /path/to/results/raw \
  --output-dir /path/to/results/aggregated
```

2. Plot the first wall-time figure:

```bash
Rscript /Users/stephen/Projects/bamana/benchmarks/R/plot_benchmarks.R \
  --tidy-csv /path/to/results/aggregated/tidy_results.csv \
  --summary-csv /path/to/results/aggregated/tidy_summary.csv \
  --output-dir /path/to/results/plots
```

3. Build the capability-aware support matrix if needed:

```bash
Rscript /Users/stephen/Projects/bamana/benchmarks/R/build_support_matrix.R \
  --runs-csv /path/to/results/aggregated/tidy_results.csv \
  --outdir /path/to/results/aggregated
```

4. Inspect:

* `results/aggregated/tidy_results.csv`
* `results/aggregated/tidy_summary.csv`
* `results/plots/wall_time_by_tool.png`
* `results/aggregated/support_matrix.csv`

## Review Rule

If the tidy column contract changes, review these files together:

* [aggregate_results.R](/Users/stephen/Projects/bamana/benchmarks/R/aggregate_results.R)
* [plot_benchmarks.R](/Users/stephen/Projects/bamana/benchmarks/R/plot_benchmarks.R)
* [build_support_matrix.R](/Users/stephen/Projects/bamana/benchmarks/R/build_support_matrix.R)
* [../results/tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md)
* [../results/support_matrix_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/support_matrix_contract.md)
