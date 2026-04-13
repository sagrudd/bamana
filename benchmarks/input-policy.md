# Benchmark Input Policy

## Purpose

This document defines how large user-supplied benchmark inputs are classified,
declared, validated, staged, and tracked for the Bamana benchmarking
framework.

The repository does not commit whole-genome BAM or large FASTQ.GZ benchmark
inputs. Instead, benchmark users supply those files locally and describe them
through a dataset-centric manifest plus a run-centric Nextflow params file.

Manifest versus params:

* manifest: what datasets exist and how they should be interpreted
* params: which dataset ids, tools, scenarios, and execution settings apply to
  one benchmark run

## Core Policy

The benchmark framework treats source inputs and derived inputs as different
objects.

### Tier A: Source Benchmark Inputs

These are the original user-supplied files. Examples:

* mapped whole-genome BAM
* unmapped BAM
* gzipped FASTQ collection

Tier A rules:

* source inputs are external to the repository
* source inputs are treated as read-only
* cleanup routines must never delete source inputs
* source inputs must be identified by a stable `source_input_id`
* source inputs should be described in an input manifest when runs are meant to
  be repeatable or shareable across operators

### Tier B: Derived Benchmark Scenario Inputs

These are materialized benchmark-local artifacts derived from Tier A. Examples:

* deterministic subsampled BAM
* seeded-random subsampled FASTQ.GZ
* normalized scenario-specific working inputs

Tier B rules:

* derived inputs may be retained for reuse across replicates
* derived inputs must encode the source input identity and materialization
  policy in their metadata
* derived inputs are benchmark artifacts, not source data
* derived inputs may be deleted by cleanup policies, but only under the rules
  in [cleanup.md](/Users/stephen/Projects/bamana/benchmarks/cleanup.md)

## Taxonomy

Supported first-slice source input categories:

* `mapped_bam`
* `unmapped_bam`
* `fastq_gz`

Future categories are documented but not yet primary:

* `cram`
* `folder_ont_collection`
* `multi_fastq_collection`
* `mixed_file_set`

## Manifest Requirement

Preferred benchmark practice is to supply a manifest file via:

```bash
nextflow run benchmarks/main.nf -params-file /abs/path/to/benchmark-run.json
```

The manifest is the governed source of truth for:

* input id
* source path
* source classification
* mapped state
* expected sort order
* index availability
* storage context
* sensitivity level
* allowed benchmark scenarios
* staging policy hints

The params file is the governed source of truth for:

* `dataset_ids`
* `tools`
* `scenarios`
* `replicates`
* `warmup_runs`
* `subsample_fraction`
* `subsample_seed`
* `subsample_mode`
* `output_dir`

The schema is defined in
[inputs/manifest.schema.json](/Users/stephen/Projects/bamana/benchmarks/inputs/manifest.schema.json).

The run-parameter schema is defined in
[params.schema.json](/Users/stephen/Projects/bamana/benchmarks/params.schema.json).

## Read-Only Source Rule

The benchmark framework must never mutate or delete user-supplied source data.

This includes:

* source BAMs
* source FASTQ.GZ files
* pre-existing BAM indices
* any directory tree outside the benchmark output or work areas

If a benchmark requires normalization, subsampling, or scenario-specific
materialization, that work must produce derived inputs under benchmark-managed
paths rather than editing the source file in place.

## Fairness Principle

The primary benchmark interpretation is tool runtime on already-available
inputs. One-off staging or copying must therefore not be mixed silently into
the timed region.

Default policy:

* source validation and benchmark-managed staging occur before timed execution
* timed execution should measure the target tool command path
* staging mode, storage context, and scenario materialization are recorded in
  per-run metadata

If staging is intentionally included in the timed path, that must be declared
explicitly through `include_staging_in_timing` and interpreted as an end-to-end
operational benchmark rather than a pure tool-runtime benchmark.

## Input Validation Expectations

Before benchmarking begins, the framework should validate or classify:

* file exists
* file is readable
* declared input category matches the intended scenario
* BAM versus FASTQ.GZ classification is coherent
* mapped versus unmapped designation is coherent for BAM scenarios
* index presence is declared honestly where relevant

The helper
[bin/validate_inputs.py](/Users/stephen/Projects/bamana/benchmarks/bin/validate_inputs.py)
is included as a lightweight manifest validator and local-preflight tool.

## Provenance Requirements

Each timed benchmark row should record enough information to answer:

* what was the original source input?
* which staged or derived file was actually benchmarked?
* how was it staged?
* was staging included in the timed result?
* was the file reused across replicates?
* what scenario materialization path was used?

The per-run schema therefore records fields including:

* `source_input_id`
* `source_input_path`
* `source_input_type`
* `staged_input_id`
* `staged_input_path`
* `staging_mode`
* `scenario_materialization`
* `storage_context`
* `subsample_fraction`
* `subsample_seed`
* `subsample_mode`

## Large-Input Governance

For regulated or operationally demanding environments:

* keep manifests in version control when permissible
* keep source paths external to the repository
* record source ownership and sensitivity level in the manifest
* prefer deterministic or seeded subsampling for repeatable derived inputs
* use profile-specific storage context declarations such as `local_ssd` or
  `shared_filesystem`
