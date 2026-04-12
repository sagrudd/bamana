# Benchmark Readiness Checklist

This checklist is the repository-facing readiness gate for tomorrow's benchmark
exercise. It is intentionally practical rather than aspirational.

## Inputs

* [x] Example manifest exists:
  [inputs/example_manifest.json](/Users/stephen/Projects/bamana/benchmarks/inputs/example_manifest.json)
* [x] Input manifest schema exists:
  [inputs/manifest.schema.json](/Users/stephen/Projects/bamana/benchmarks/inputs/manifest.schema.json)
* [x] Params schema exists:
  [params.schema.json](/Users/stephen/Projects/bamana/benchmarks/params.schema.json)
* [x] Local params example exists:
  [params.examples/local.example.json](/Users/stephen/Projects/bamana/benchmarks/params.examples/local.example.json)
* [x] Dataset id selection is documented in
  [README.md](/Users/stephen/Projects/bamana/benchmarks/README.md)
* [x] Mapped BAM, unmapped BAM, and FASTQ.GZ examples are represented in the manifest scaffold
* [x] Benchmark input staging and retention policy is documented in:
  * [input-policy.md](/Users/stephen/Projects/bamana/benchmarks/input-policy.md)
  * [staging.md](/Users/stephen/Projects/bamana/benchmarks/staging.md)
  * [cleanup.md](/Users/stephen/Projects/bamana/benchmarks/cleanup.md)

## Wrappers

* [x] Bamana wrapper exists:
  [tools/wrappers/bamana.sh](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/bamana.sh)
* [x] samtools wrapper exists:
  [tools/wrappers/samtools.sh](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/samtools.sh)
* [x] fastcat wrapper exists:
  [tools/wrappers/fastcat.sh](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/fastcat.sh)
* [x] Shared wrapper CLI contract exists:
  [tools/wrappers/wrapper_cli_contract.md](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/wrapper_cli_contract.md)
* [x] Unsupported scenarios are explicit and machine-readable
* [x] Tool version commands are captured by the wrapper layer
* [x] Command-path provenance is emitted via wrapper metadata plus command files/logs

## Execution

* [x] Minimal Nextflow pipeline exists:
  [main.nf](/Users/stephen/Projects/bamana/benchmarks/main.nf)
* [x] Wrapper execution is isolated in a dedicated module:
  [modules/benchmark_wrapper_run.nf](/Users/stephen/Projects/bamana/benchmarks/modules/benchmark_wrapper_run.nf)
* [x] Raw result JSON can be emitted via
  [bin/run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh)
* [x] Stable output structure is documented:
  [results/README.md](/Users/stephen/Projects/bamana/benchmarks/results/README.md)
* [x] Raw result inventory collection exists under the metadata layer
* [x] `unmapped_bam` is now accepted by the manifest-driven minimal pipeline, even though tomorrow's first run should stay narrower

## Analysis

* [x] Tidy aggregation script exists:
  [R/aggregate_results.R](/Users/stephen/Projects/bamana/benchmarks/R/aggregate_results.R)
* [x] Basic plotting script exists:
  [R/plot_benchmarks.R](/Users/stephen/Projects/bamana/benchmarks/R/plot_benchmarks.R)
* [x] Support matrix script exists:
  [R/build_support_matrix.R](/Users/stephen/Projects/bamana/benchmarks/R/build_support_matrix.R)
* [x] R usage is documented:
  [R/README.md](/Users/stephen/Projects/bamana/benchmarks/R/README.md)
* [ ] End-to-end local execution of the R scripts is confirmed in this repository snapshot
  Status: scaffolded and aligned, but live `Rscript` execution remains environment-dependent.

## Contracts

* [x] Raw result schema exists:
  [results/result.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/result.schema.json)
* [x] Tidy result contract exists:
  [results/tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md)
* [x] Support matrix contract exists:
  [results/support_matrix_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/support_matrix_contract.md)
* [x] Tool registry exists:
  [tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json)
* [x] Workflow-variant matrix exists:
  [tools/workflow_variant_matrix.md](/Users/stephen/Projects/bamana/benchmarks/tools/workflow_variant_matrix.md)
* [x] Wrapper support vocabulary and raw-result support vocabulary are now translated explicitly at the Nextflow execution boundary

## Architecture

* [x] Native-core direction is documented:
  [ARCHITECTURE.md](/Users/stephen/Projects/bamana/ARCHITECTURE.md)
* [x] `noodles` demotion policy is documented:
  [docs/migration/noodles-demotion.md](/Users/stephen/Projects/bamana/docs/migration/noodles-demotion.md)
* [x] Roadmap is visible:
  [ROADMAP.md](/Users/stephen/Projects/bamana/ROADMAP.md)
* [x] Current milestone is visible:
  [docs/roadmap/current_milestone.md](/Users/stephen/Projects/bamana/docs/roadmap/current_milestone.md)

## Bamana Command Readiness For Benchmarking

* [x] `subsample` is benchmark-usable for BAM, FASTQ, and FASTQ.GZ
* [x] `consume --mode unmapped` is benchmark-usable for FASTQ.GZ ingestion smoke tests
* [x] `sort` is benchmark-usable for the current first slice
* [ ] `index` is fully benchmark-usable as a real BAM index writer
  Status: intentionally not complete; wrappers and docs must treat this as partial or planned rather than ready.

## Tomorrow's Operating Posture

* [x] Repository intent is coherent enough to begin benchmark execution tomorrow
* [x] First-run guidance exists:
  [status_for_tomorrow.md](/Users/stephen/Projects/bamana/benchmarks/status_for_tomorrow.md)
* [x] The recommended first run is intentionally small and auditable

Conclusion:

* the repository is ready for benchmark smoke-test execution tomorrow
* the main remaining risk is runtime/debugging depth rather than contract ambiguity
