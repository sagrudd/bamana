# Tool Wrapper Contract

## Purpose

This document defines the contract between:

* the Nextflow benchmark workflow
* the per-tool execution path
* the timing wrapper
* the result contract

The benchmark framework must record what each comparator actually did, not just
which tool name appeared in a plot.

## Three Layers

### Tool Identity

Examples:

* `bamana`
* `samtools`
* `fastcat`

### Workflow Variant

Examples:

* `bamana_subsample_sort_partial_index`
* `samtools_view_sort_index`
* `fastcat_concat_gzip`

### Wrapper Implementation

Examples:

* a dedicated Nextflow process module
* a shell script that emits a command file
* a common timing wrapper around a generated command file

The benchmark layer must preserve all three.

## Wrapper Responsibilities

A wrapper must:

* accept the scenario and workflow-variant context explicitly
* consume one staged input path and its metadata
* construct the exact command path for the tool
* define or expose the primary output path used for the benchmark
* define how version is obtained
* classify unsupported combinations explicitly
* return a clear exit status for attempted runs
* remain compatible with the common timing wrapper

## Required Wrapper Inputs

A wrapper should be defined in terms of these concepts:

* `tool`
* `scenario`
* `workflow_variant`
* `input_file`
* `input_metrics_json`
* `output_target`
* `threads`
* `subsample_fraction`
* `subsample_seed`
* `subsample_mode`
* tool-specific execution paths such as `bamana_bin`

The current pipeline passes these through Nextflow metadata and then into
[run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh).

## Required Wrapper Outputs

A wrapper must emit or allow the framework to capture:

* the final primary output path
* tool version string
* normalized command line
* workflow variant id
* scenario id
* status semantics:
  * `success`
  * `unsupported`
  * `failed`

## Status Semantics

### `success`

The tool/scenario combination was intended to run and completed successfully.

### `unsupported`

The tool/scenario combination is not supported or not applicable.

This is not a failure.

### `failed`

The tool/scenario combination was intended to run but the execution failed.

## Version Reporting

Each wrapper must define a version command.

Examples:

* Bamana: `"${meta.bamana_bin} --version"`
* samtools: `"samtools --version"`
* fastcat: `"fastcat --version"`

The version string must be carried into the raw result and tidy row.

## Command Provenance

Each wrapper must provide or allow capture of:

* `tool`
* `workflow_variant`
* `scenario`
* `command_line`

This is required so plots and summaries can be interpreted against the actual
operation chain.

## Output Naming

Wrappers should use deterministic output-target conventions or report the final
output path explicitly.

Examples:

* mapped BAM pipeline final target: `*.sorted.bam`
* subsample-only BAM target: `*.subsampled.bam`
* fastq consume target: tool-specific output file declared by the wrapper

Intermediates may exist, but the wrapper must make it clear which file is the
benchmark target.

## Current Implementation Model

Current benchmark execution uses:

* one Nextflow module per tool
* module-local command assembly
* a shared timing wrapper:
  [run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh)

This means the wrapper implementation type is currently best described as:

* `nextflow_process` for per-tool orchestration
* `command_file_plus_timing_wrapper` for actual execution capture

## Adding a New Tool

To add a new comparator cleanly:

1. add a registry entry in the tool registry
2. define supported scenarios and workflow variants
3. implement or document the wrapper path
4. update the workflow-variant matrix
5. ensure the result layer records the correct `tool`, `tool_version`, and
   `workflow_variant`
