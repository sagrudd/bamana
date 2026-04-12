# Tidy Result Contract

## Purpose

This document defines the flat, aggregation-ready row contract used between:

* raw benchmark result JSON emitted by the wrapper layer
* the R aggregation layer
* later plotting and support-matrix reporting

One raw benchmark attempt maps to one tidy result row.

The tidy row is derived during aggregation from
`results/raw/*.result.json` and is validated conceptually by
[benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json).

## Raw Versus Tidy

There are two benchmark result levels:

* raw structured JSON: governed by
  [result.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/result.schema.json)
* tidy flat rows: governed by this document and
  [benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json)

The raw JSON preserves nested execution context. The tidy row flattens that
context into plotting-friendly columns.

The canonical source for `tool`, `tool_version`, and `workflow_variant`
semantics is the benchmark tool-wrapper layer:

* [../tools/tool_wrapper_contract.md](/Users/stephen/Projects/bamana/benchmarks/tools/tool_wrapper_contract.md)
* [../tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json)
* [../tools/workflow_variant_matrix.md](/Users/stephen/Projects/bamana/benchmarks/tools/workflow_variant_matrix.md)

## Aggregation Flow

The first analysis slice follows this path:

1. wrappers emit one raw `*.result.json` file per attempted run
2. [../R/aggregate_results.R](/Users/stephen/Projects/bamana/benchmarks/R/aggregate_results.R)
   reads those raw JSON records
3. each raw record becomes one tidy row in `tidy_results.csv`
4. successful measured rows contribute to grouped performance summaries in
   `tidy_summary.csv`
5. [../R/plot_benchmarks.R](/Users/stephen/Projects/bamana/benchmarks/R/plot_benchmarks.R)
   uses the tidy outputs to generate the first wall-time figure

## Required Columns

Required tidy columns:

* `schema_version`
* `run_id`
* `timestamp_utc`
* `benchmark_framework_version`
* `scenario`
* `workflow_variant`
* `tool`
* `tool_version`
* `source_input_id`
* `source_input_type`
* `staged_input_id`
* `replicate`
* `warmup`
* `subsample_mode`
* `subsample_fraction`
* `subsample_seed`
* `threads`
* `staging_mode`
* `staging_included_in_timing`
* `status`
* `success`
* `unsupported`
* `failed`
* `exit_code`
* `wall_seconds`
* `cpu_seconds`
* `max_rss_bytes`
* `input_bytes`
* `output_bytes`
* `records_processed`
* `throughput_records_per_sec`
* `throughput_bytes_per_sec`
* `notes`

Additional retained columns are allowed when they remain stable and useful for
interpretation, for example:

* `source_input_path`
* `source_category`
* `staged_input_path`
* `input_type`
* `mapping_state`
* `input_basename`
* `expected_sort_order`
* `has_index`
* `reference_context`
* `subsample_enabled`
* `semantic_equivalence`
* `support_status`
* `staging_realization`
* `scenario_materialization`
* `storage_context`
* `container_image`
* `command_line`

## Column Semantics

### Identity and Scenario

* `run_id`: unique identifier for one attempted run
* `scenario`: stable scenario id such as `mapped_bam_pipeline`
* `workflow_variant`: exact tool-specific execution path from the tool registry
* `tool` and `tool_version`: comparator identity and discovered version as
  reported by the wrapper contract

### Input Provenance

* `source_input_id`: dataset id from the manifest or direct-path layer
* `source_input_type`: `BAM` or `FASTQ_GZ`
* `staged_input_id`: staged or localized input id consumed by the timed process

### Replication and Subsampling

* `replicate`: replicate index
* `warmup`: `true` for warmup runs, `false` for measured runs
* `subsample_mode`: `deterministic` or `random` when subsampling is part of
  the scenario
* `subsample_fraction`: requested fraction when relevant
* `subsample_seed`: seed used when relevant

### Staging

* `staging_mode`: `direct`, `copy`, `hardlink`, `symlink`, or `scratch_copy`
* `staging_included_in_timing`: whether staging overhead is intended to be part
  of measured runtime

### Status

* `status`: one of `success`, `unsupported`, `failed`, or `skipped`
* `success`: boolean convenience column
* `unsupported`: boolean convenience column
* `failed`: boolean convenience column

Hard rule:

* unsupported is not failure
* failed is not unsupported

### Performance Metrics

* `wall_seconds`, `cpu_seconds`, `max_rss_bytes`: measured runtime fields
* `input_bytes`, `output_bytes`: I/O size context
* `records_processed`: known or estimated processed record count
* `throughput_records_per_sec`: `records_processed / wall_seconds` when
  computable
* `throughput_bytes_per_sec`: `input_bytes / wall_seconds` when computable

## Null and NA Rules

### Unsupported Rows

Unsupported rows remain in the tidy dataset with:

* `status = unsupported`
* `success = false`
* `unsupported = true`
* timing and memory fields as blank or `NA` unless a negligible setup attempt
  was intentionally measured

### Failed Rows

Failed rows remain in the tidy dataset with:

* `status = failed`
* `failed = true`
* partial timing fields retained if honestly available

### Successful Rows

Successful rows should populate the main performance fields unless the metric is
genuinely unavailable.

## Summary Rules

Grouped summaries are expected to aggregate by combinations such as:

* `scenario`
* `tool`
* `workflow_variant`
* `source_input_id`
* `threads`
* `subsample_fraction`
* `subsample_seed`
* `subsample_mode`

Performance summaries must be computed from successful measured runs only.

Unsupported and failed rows must still contribute to support and reliability
tables, but not to median runtime or throughput statistics.
