# Benchmark Results Layout

Generated benchmark outputs should be written under the path configured by
`--output_dir`.

Expected subdirectories:

* `input_metadata/`: per-input size and record-count metadata
* `derived_inputs/`: reusable subsampled or otherwise materialized scenario inputs when retention is enabled
* `raw/`: one structured JSON result per tool/scenario/replicate attempt, plus any wrapper-side compatibility artifacts
* `logs/`: wrapper and runtime command logs
* `metadata/`: wrapper planning JSON and raw result inventory files
* `aggregated/`: tidy per-run CSV, grouped summaries, and support matrices
* `plots/`: benchmark figures such as the first wall-time comparison

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
* [bgzf_microbench.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/bgzf_microbench.schema.json): native BGZF microbenchmark JSON contract, including optional `check_eof` command smoke timing
* [header_microbench.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/header_microbench.schema.json): native BAM header microbenchmark JSON contract
* [scanner_microbench.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/scanner_microbench.schema.json): native BAM scanner microbenchmark JSON contract, including optional `summary`, `check_sort`, `check_map`, `validate`, `check_tag`, `subsample_bam`, `sort`, `merge`, `checksum`, `explode`, `consume`, `check_map_region_scan_fallback`, `summary_region_scan_fallback`, `select_region_scan_fallback`, `index_bam`, `check_index`, `check_map_indexed`, `summary_indexed`, `check_map_region_indexed`, `summary_region_indexed`, `select_region_indexed_output`, `check_index_csi_detect_only`, `check_map_region_csi_fallback`, `summary_region_csi_fallback`, `select_region_csi_fallback`, `inspect_duplication`, `deduplicate`, and `forensic_inspect` command smoke timings
* [fastq_microbench.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/fastq_microbench.schema.json): native FASTQ parser/writer microbenchmark JSON contract, including optional `subsample_fastq` and `subsample_fastq_gz` command smoke timings
* [tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md): human-readable aggregation contract
* [../tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json): canonical `tool` and `workflow_variant` values
* [support_matrix_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/support_matrix_contract.md): support and capability reporting contract
* [../command_evidence_matrix.md](/Users/stephen/Projects/bamana/benchmarks/command_evidence_matrix.md): M14.2 command-level comparator, smoke, and no-claim evidence matrix

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

Benchmark interpretation notes:

* substrate timings measure in-process native readers, writers, and scanners
* command timings include process startup, CLI parsing, JSON envelope emission,
  file probing, and command-specific payload construction
* `check_eof` timing is EOF-marker evidence only, not BAM header or record
  validation
* scanner command smoke timings use deterministic synthetic BAM input and do
  not exercise malformed-input paths
* scanner command smoke timings do not imply comparator parity with external
  tools
* M8 scanner command smoke timings distinguish process startup and JSON
  emission from full-record materialization, in-memory sorting cost, merge
  compatibility and merge-ordering cost, native BGZF compression cost,
  checksum-domain traversal, shard planning, ingest normalization, and CRAM
  compatibility behavior
* M9 scanner command smoke timings distinguish BAM index construction, BAI
  structural validation, index metadata-backed consumer evidence, scan fallback
  timings, random-access lookup deferral, process startup, and JSON emission
* M12 scanner command smoke timings distinguish CSI detect-only support-level
  reporting, CSI-preserving native scan fallback, and selected-region
  `input_index` compatibility from CSI bin parsing, CSI chunk planning, CSI
  random-access traversal, CSI writing, or large-reference CSI support
* M13.9 CRAM benchmark guardrails keep `scanner_microbench` scoped to
  deterministic synthetic BAM fixtures. It emits no CRAM command timing rows,
  and its `consume` row is synthetic BAM alignment ingest only. It does not
  measure CRAI parsing, CRAM region input, CRAM random-access traversal, CRAM
  indexed-query evidence, or CRAM comparator parity
* M14.2 records command-level evidence status in
  [../command_evidence_matrix.md](/Users/stephen/Projects/bamana/benchmarks/command_evidence_matrix.md).
  Only `fastq_ingress` and `fastq_gz_enumerate` are current public-profile
  comparator evidence. Repository-local command timings are local smoke
  evidence. `fastq`, `unmap`, `identify`, unmeasured command modes, and
  scaffold-only workflow-matrix rows remain no external comparator claim
  surfaces until later M14 tasks promote them with command-specific fixtures,
  semantic assumptions, and schema coverage.
* M14.3 extends result schemas without changing runtime benchmark behavior.
  `result.schema.json` and `benchmark_row.schema.json` now allow optional
  `command_family`, `evidence_level`, `evidence_source`, and
  `comparator_scope` fields. `scanner_microbench.schema.json` records the
  post-M10 command family taxonomy for `indexed_region`, `selected_region`,
  `csi_fallback`, `remediation`, `forensics`, `transform_ingest`,
  `inspection`, and `index` command timing rows. `header_microbench.schema.json`
  records the header/mutation split for `verify`, `header`, `reheader`, and
  `annotate_rg`, while `fastq_microbench.schema.json` records FASTQ command
  timing rows. Existing emitted rows remain valid because these fields are
  optional.
* M14.4 adds benchmark input fixture provenance metadata under
  `benchmarks/inputs/manifest.schema.json`. New generated, derived, selected,
  or comparator fixtures should carry `fixture_provenance` with source kind,
  source description, source URI, `derived_from`, generation command,
  generation environment, expected semantic scope, review boundary, checksum
  policy, and reproducibility notes before they are used for release-facing
  benchmark claims.

First analysis slice:

* [../R/aggregate_results.R](/Users/stephen/Projects/bamana/benchmarks/R/aggregate_results.R)
  reads `raw/*.result.json` and writes:
  * `aggregated/tidy_results.csv`
  * `aggregated/tidy_summary.csv`
* [../R/plot_benchmarks.R](/Users/stephen/Projects/bamana/benchmarks/R/plot_benchmarks.R)
  reads the aggregated CSVs and writes:
  * `plots/wall_time_by_tool.png`
  * `plots/wall_time_by_tool.pdf`

This directory is tracked for documentation and schema only. Large generated
benchmark outputs should not be committed.
