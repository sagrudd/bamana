# Benchmark Staging Policy

## Purpose

This document defines how benchmark inputs are materialized for execution and
how those materialization decisions interact with fairness and replication.

## Default Interpretation

The primary benchmark interpretation is:

* validate inputs
* stage or localize inputs before the timed command
* time the tool execution itself

Staging is therefore excluded from the primary timing path unless a scenario
explicitly opts into end-to-end operational timing.

The params layer may override a manifest staging policy through
`staging_override`, but the override should be used conservatively and recorded
in run metadata.

## Supported Staging Modes

The benchmark layer recognizes the following staging modes.

### `direct`

Use the source-localized file without copying bytes in the benchmark-managed
layer.

Operational meaning:

* preferred when the workflow already executes close to the source file
* benchmark-managed staging should not duplicate large data unnecessarily
* in the current Nextflow scaffold this may still appear as a lightweight
  wrapper path because Nextflow localizes task inputs

### `symlink`

Create a symbolic-link wrapper to the source-localized input.

Operational meaning:

* preserves source immutability
* avoids large copies
* good default on shared filesystems when tools can read through symlinks

### `hardlink`

Create a hardlink when the filesystem allows it, with copy fallback when it
does not.

Operational meaning:

* useful on local filesystems and SSD-backed scratch
* avoids byte copies in the common case
* must record the actual realization because cross-filesystem hardlinks may
  fall back to copies

### `copy`

Copy the source input into the benchmark-local staging area before timing.

Operational meaning:

* safest when link semantics are undesirable
* expensive for very large files
* appropriate when the benchmark policy explicitly wants isolated local copies

### `scratch_copy`

Copy the source input to benchmark-local scratch or local SSD storage before
timing.

Operational meaning:

* recommended when storage locality dominates performance
* benchmark timing should still exclude the copy step in the primary scenario
* storage context must be recorded clearly

### `stream`

Reserved for workflows where the tool consumes a stream-like source path or
pipe-oriented input.

Operational meaning:

* should be used only when the comparator workflow is genuinely stream-native
* do not silently use `stream` for tools that actually require pre-materialized
  files

## Replicate Reuse Policy

Replicates should vary primarily in execution timing, not input content.

Default policy:

* the same staged or derived scenario input should be reused across replicates
* reuse must be recorded through `reuse_materialized_inputs = true`
* warmup runs should use the same content as measured replicates unless the
  scenario explicitly studies cache effects or content randomness

## Source Versus Derived Materialization

### Source Materialization

The source file itself may be staged for execution according to the selected
staging mode. This is still Tier A input.

### Derived Materialization

Subsampled or normalized scenario inputs are Tier B benchmark artifacts and
should be named deterministically. Recommended naming components:

* source input id
* scenario
* fraction
* seed
* mode
* format suffix

Examples:

* `human_wgs_mapped.subsample.f0_1.seed12345.random.bam`
* `human_fastq.subsample.f0_25.seed12345.random.fastq.gz`
* `human_wgs_mapped.subsample.f0_1.deterministic.bam`

## Scenario Materialization Policy

The framework should record whether a tool was run on:

* the original source input
* a shared derived scenario input reused across tools
* a tool-specific derived input when semantic equivalence requires it

Comparator fairness rule:

* prefer one shared source or derived scenario input across tools
* avoid letting each tool subsample or transform the benchmark content
  independently unless the benchmark is explicitly about tool-native sampling

## Cache and Locality Guidance

Storage behavior can dominate benchmark outcomes.

This first policy layer therefore requires operators to record:

* `storage_context`
* `staging_mode`
* whether the same staged artifact was reused across replicates
* whether staging was included in timing

The policy does not attempt to eliminate all cache effects in the first
version. Instead it requires them to be documented and, where possible,
normalized operationally:

* use the same locality profile across compared tools
* prefer stable staging and artifact reuse across replicates
* treat cold-cache versus warm-cache studies as explicit alternate scenarios

## Nextflow Integration

The `STAGE_INPUT` process is the benchmark-managed staging boundary.

It is responsible for:

* classifying the source input
* materializing the staged path according to `staging_mode`
* measuring source input size and coarse record counts
* emitting benchmark metadata for downstream timed processes

Downstream timed tool processes consume the staged path and the staging
metadata, and the benchmark result rows preserve both the source and staged
identities.

Dataset resolution flow:

1. load `input_manifest`
2. load `dataset_ids` from the params file
3. filter the manifest to the selected dataset ids
4. validate scenario compatibility through `allowed_benchmark_scenarios`
5. apply manifest staging policy or `staging_override`
6. emit staged input metadata for downstream timed processes
