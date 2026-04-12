# Wrapper CLI Contract

This document defines the common shell-wrapper interface used by the benchmark
framework for the initial comparator set.

These wrappers are planning wrappers. They do not own wall-time measurement.
Instead, they:

* validate scenario and workflow-variant requests
* classify supported versus unsupported combinations
* materialize deterministic command files
* emit wrapper metadata JSON
* hand off executable command files to the outer timing wrapper

The outer timing and result-emission layer remains
[run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh).

## Common CLI Shape

Each wrapper accepts the same core arguments:

```bash
<tool-wrapper>.sh \
  --scenario <scenario_id> \
  --workflow-variant <variant_id> \
  --input <path> \
  --output-dir <dir> \
  --result-output <json> \
  --command-file <path> \
  --command-log <txt> \
  [--threads <N>] \
  [--subsample-fraction <f>] \
  [--subsample-seed <int>] \
  [--subsample-mode <random|deterministic>] \
  [--sort-order <none|coordinate|queryname>] \
  [--create-index] \
  [--timing-output <path>]
```

Tool-specific binary overrides are also allowed:

* `bamana.sh`: `--bamana-bin <path>`
* `samtools.sh`: `--samtools-bin <path>`
* `fastcat.sh`: `--fastcat-bin <path>`

## Required Arguments

Required for all wrappers:

* `--scenario`
* `--workflow-variant`
* `--input`
* `--output-dir`
* `--result-output`
* `--command-file`
* `--command-log`

## Optional Shared Arguments

Shared optional arguments:

* `--threads`
* `--subsample-fraction`
* `--subsample-seed`
* `--subsample-mode`
* `--sort-order`
* `--create-index`
* `--timing-output`

`--timing-output` is currently reserved for outer-wrapper integration and is
recorded in notes only. The shell wrappers themselves do not measure runtime.

## Wrapper-Emitted JSON

Each wrapper writes JSON to `--result-output` with these fields:

* `wrapper_contract_version`
* `tool`
* `scenario`
* `workflow_variant`
* `status`
* `support_status`
* `semantic_equivalence`
* `tool_version_command`
* `command`
* `command_file`
* `command_log`
* `output_dir`
* `output_paths`
* `timing_wrapper_compatible`
* `notes`

### Wrapper Planning Status

Wrapper status is a planning-layer status:

* `success`: the wrapper accepted the request and generated an executable
  command file
* `unsupported`: the tool does not support the requested scenario or workflow
  variant
* `failed`: the wrapper could not plan the command path due to invalid inputs
  or internal wrapper error

This is separate from benchmark execution status in the raw result schema.

## Output Artifacts

Each wrapper creates:

* one metadata JSON file at `--result-output`
* one executable shell script at `--command-file`
* one human-readable command provenance log at `--command-log`

Supported wrappers write the real command path to `--command-file`.

Unsupported wrappers write a no-op command file containing `true` so the outer
benchmark layer can classify the row as unsupported without inventing a fake
tool failure.

## Output Naming

Wrappers derive output names deterministically from:

* input basename
* workflow variant id
* operation stage

Examples:

* `sample.bamana_subsample_only.subsampled.bam`
* `sample.samtools_view_sort_index.sorted.bam`
* `sample.fastcat_concat_gzip.fastq.gz`

## Exit Behavior

Wrapper exit behavior:

* `0` for supported plans and explicit unsupported classifications
* non-zero only for wrapper planning failures such as missing required
  arguments or unrecoverable wrapper logic errors

## Nextflow Integration

The Nextflow modules for the initial comparator set use this flow:

1. invoke the wrapper
2. read wrapper metadata JSON
3. pass the generated command file to `run_benchmark.sh`
4. let `run_benchmark.sh` own timing, stdout/stderr capture, and benchmark
   raw/tidy result emission

This preserves one stable CLI contract for wrappers while keeping the benchmark
result layer centralized.
