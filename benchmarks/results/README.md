# Benchmark Results Layout

Generated benchmark outputs should be written under the path configured by
`--output_dir`.

Expected subdirectories:

* `input_metadata/`: per-input size and record-count metadata
* `derived_inputs/`: reusable subsampled or otherwise materialized scenario inputs when retention is enabled
* `per_run/`: one TSV, one JSON, and command logs per tool/scenario/replicate
* `summary/`: aggregated run tables and support matrices
* `figures/`: publication-ready PDF and PNG plots

Per-run rows should preserve both source and staged provenance, including:

* `source_input_id`
* `source_input_path`
* `staged_input_id`
* `staged_input_path`
* `staging_mode`
* `scenario_materialization`
* `storage_context`

Contracts:

* [result.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/result.schema.json): structured raw per-run JSON record
* [benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json): flat tidy per-run row contract
* [tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md): human-readable aggregation contract
* [../tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json): canonical `tool` and `workflow_variant` values
* [support_matrix_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/support_matrix_contract.md): support and capability reporting contract

Examples:

* [example_raw_result.json](/Users/stephen/Projects/bamana/benchmarks/results/example_raw_result.json)
* [example_raw_result.unsupported.json](/Users/stephen/Projects/bamana/benchmarks/results/example_raw_result.unsupported.json)
* [example_raw_result.failure.json](/Users/stephen/Projects/bamana/benchmarks/results/example_raw_result.failure.json)
* [example_tidy_results.csv](/Users/stephen/Projects/bamana/benchmarks/results/example_tidy_results.csv)
* [example_tidy_summary.csv](/Users/stephen/Projects/bamana/benchmarks/results/example_tidy_summary.csv)
* [example_support_matrix.csv](/Users/stephen/Projects/bamana/benchmarks/results/example_support_matrix.csv)
* [example_support_summary.csv](/Users/stephen/Projects/bamana/benchmarks/results/example_support_summary.csv)

Design rule:

* unsupported rows are not failures
* failed rows are not unsupported
* successful rows alone drive performance summaries
* unsupported and failed rows remain visible for support and reliability analysis

This directory is tracked for documentation and schema only. Large generated
benchmark outputs should not be committed.
