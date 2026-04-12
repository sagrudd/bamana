# Benchmark Cleanup and Retention Policy

## Purpose

This document defines which benchmark artifacts are ephemeral, which are
retained, and what cleanup routines are allowed to delete.

## Safety Rule

User-supplied benchmark source data is always read-only and never deleted by
benchmark cleanup routines.

Protected items include:

* source BAMs
* source FASTQ.GZ files
* source directories
* source indices such as BAI files
* any path outside the benchmark-managed output and work locations

## Artifact Classes

### Source Inputs

Tier A source inputs are external and protected.

### Derived Inputs

Tier B derived scenario inputs may be retained or deleted depending on policy.
Default recommendation:

* retain deterministic and seeded derived inputs for reuse across replicates
  and reruns
* name them deterministically
* keep them under a benchmark-managed directory such as
  `results/latest/derived_inputs/`

### Per-Run Work Artifacts

Per-run stdout, stderr, and timing files are ephemeral by default but should be
retained on failure for debugging.

### Aggregated Outputs

Summary tables, JSON outputs, and figures are generally retained.

## Cleanup Policy Values

The benchmark config exposes `cleanup_policy`. Recommended meanings:

### `retain_derived_retain_failed`

Default and safest first policy.

* keep derived scenario inputs
* keep failed work artifacts
* allow ephemeral successful work directories to be cleaned by normal Nextflow
  lifecycle management

### `retain_all`

Debug-oriented mode.

* keep derived inputs
* keep successful and failed work artifacts
* useful during framework development or comparator debugging

### `delete_ephemeral_keep_failures`

Operational cleanup mode.

* delete transient successful work products when safe
* keep failed run artifacts
* retain only the derived scenario inputs required for reuse

### `delete_ephemeral_delete_failures`

Aggressive cleanup mode.

* intended only when debugging retention is not required
* still must never delete source data

## Failure Handling

Failed runs should preserve enough data to diagnose:

* the staged input identity
* the command line
* stdout and stderr
* timing files if present
* per-run result JSON or TSV

Do not delete failed run artifacts automatically unless an operator explicitly
chooses a cleanup policy that allows it.

## Source Protection Guidance

Any helper script or cleanup command must enforce at least these rules:

* never traverse upward from benchmark-managed directories in search of files
  to delete
* never delete any path declared as `source_input_path`
* never delete files outside `output_dir`, Nextflow work directories, or
  explicitly designated derived-input directories

## Retention Recommendation for Publications

For publication or cross-team performance reviews, retain:

* manifest used for the benchmark
* per-run JSON rows
* aggregated summary tables
* generated figures
* deterministic or seeded derived inputs when re-use is important for audit
