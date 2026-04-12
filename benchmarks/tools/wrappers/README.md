# Benchmark Wrapper Scripts

This directory now contains real shell-wrapper skeletons for the initial
benchmark comparator set:

* [bamana.sh](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/bamana.sh)
* [samtools.sh](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/samtools.sh)
* [fastcat.sh](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/fastcat.sh)
* [common.sh](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/common.sh)
* [wrapper_cli_contract.md](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/wrapper_cli_contract.md)

## What These Wrappers Do

These wrappers provide the stable execution-interface layer between:

* the Nextflow benchmark modules
* tool-specific command composition
* the outer timing wrapper
* the raw benchmark result contract

They are planning wrappers rather than timing wrappers.

Each wrapper:

* validates scenario and workflow-variant compatibility
* creates deterministic output paths inside the requested output directory
* writes an executable command file for the real tool invocation
* writes a command provenance log
* writes a small wrapper metadata JSON file

For Bamana specifically, wrappers must remain honest about partial command
support. In the current slice:

* `subsample` and `consume --mode unmapped` are benchmark-usable
* `sort` is benchmark-usable for first-slice smoke tests
* `index` remains deferred as a real BAM index writer

The Bamana wrapper therefore supports a partial mapped-BAM benchmark variant
and a planned full index variant that will fail honestly if invoked too early.

The outer benchmark layer still measures runtime via
[run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh).

## Supported Comparator Set

Current shell wrappers exist for:

* `bamana`
* `samtools`
* `fastcat`

The benchmark still keeps separate Nextflow modules per tool, but those
modules now delegate command planning to these scripts rather than assembling
tool commands inline.

## Unsupported Behavior

Unsupported combinations are explicit and machine-readable.

When a wrapper receives an unsupported scenario or workflow variant, it:

* emits wrapper metadata JSON with `status = unsupported`
* emits `support_status = unsupported`
* writes a no-op command file containing `true`
* exits successfully so the benchmark framework can preserve an unsupported row
  rather than a fake process failure

This is deliberate. Unsupported is not failure.

Wrapper planning failure is also distinct from benchmark execution failure:

* unsupported: wrapper emits an unsupported plan and exits `0`
* failed planning: wrapper exits non-zero before a valid plan exists
* failed execution: the outer timing wrapper captures a raw benchmark result
  with `status = failed`

## Version and Command Provenance

Each wrapper records:

* the tool id
* the scenario id
* the workflow variant id
* the tool version command
* the normalized command path
* the deterministic output target

That information is then propagated into the raw benchmark result and tidy
aggregation layers.

## Wrapper Result Files

The wrapper metadata JSON is not the same as the final benchmark raw result
JSON.

Wrapper metadata answers:

* what command path should be run?
* is this tool/scenario combination supported?
* what output path should the outer timing wrapper treat as primary?

The benchmark raw result answers:

* what actually happened at runtime?
* how long did it take?
* did it succeed, fail, or remain unsupported?

## Nextflow Calling Pattern

The expected pattern is:

1. Nextflow calls a wrapper with scenario, workflow variant, input, and output
   settings.
2. The wrapper emits:
   * `wrapper.json`
   * `command.sh`
   * `command.log`
3. Nextflow reads the wrapper JSON.
4. Nextflow passes `command.sh` to
   [run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh).

This makes the actual comparator path inspectable without duplicating command
assembly across modules.
