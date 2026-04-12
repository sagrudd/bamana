# Benchmark Results Layout

Generated benchmark outputs should be written under the path configured by
`--output_dir`.

Expected subdirectories:

* `input_metadata/`: per-input size and record-count metadata
* `per_run/`: one TSV, one JSON, and command logs per tool/scenario/replicate
* `summary/`: aggregated run tables and support matrices
* `figures/`: publication-ready PDF and PNG plots

The per-run result schema is documented in
[benchmark_row.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/benchmark_row.schema.json).

This directory is tracked for documentation and schema only. Large generated
benchmark outputs should not be committed.
