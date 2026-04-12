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

The per-run result schema is documented in
[benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json).

This directory is tracked for documentation and schema only. Large generated
benchmark outputs should not be committed.
