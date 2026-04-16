# Bamana CLI Contract

Bamana is a JSON-first CLI. The CLI itself is part of the governed public
surface and is intended to remain stable for workflow engines, validators, and
controlled operational environments.

The detailed command contract is maintained in:

* [spec/cli/commands.md](/Users/stephen/Projects/bamana/spec/cli/commands.md)
* [spec/cli/global-options.md](/Users/stephen/Projects/bamana/spec/cli/global-options.md)
* [spec/cli/exit-codes.md](/Users/stephen/Projects/bamana/spec/cli/exit-codes.md)

## Core Rules

* Bamana emits JSON only.
* `--json-pretty` affects formatting only.
* Command names and option names are public contract elements.
* Contract changes require schema/example/doc updates.

## Implemented And Planned Commands

The spec layer covers both:

* implemented commands already present in the CLI
* planned first-slice commands, such as `explode`, whose public contract shape
  is being stabilized before runtime implementation

This separation is deliberate: repository-facing contract design should not wait
for every implementation detail to be finished.

`subsample` is now an implemented command for BAM, FASTQ, and FASTQ.GZ inputs.
It provides seeded random Bernoulli-style subsampling and deterministic
hash-based subsampling with explicit identity semantics. The command preserves
encounter order of retained records, emits JSON only, and is designed to
support both production workflows and reproducible benchmarking.

`consume` is the ingestion gateway into Bamana. It is the command that accepts
files and directories containing supported upstream formats and normalizes them
into a single BAM according to an explicit ingest mode. The current staged
implementation supports alignment-mode BAM/SAM/CRAM normalization and unmapped
FASTQ/FASTQ.GZ normalization, with deterministic directory discovery and dry-run
support. CRAM is available only in alignment mode and is governed by an
explicit `--reference-policy`; Bamana does not silently guess CRAM reference
behavior. Its detailed contract is documented in
[spec/cli/commands.md](/Users/stephen/Projects/bamana/spec/cli/commands.md).

`annotate_rg` is the record-level read-group annotation command. It rewrites
alignment records to insert, replace, or normalize `RG:Z:` aux tags and can
explicitly require or create a matching `@RG` header line. It is intentionally
distinct from `reheader`: `annotate_rg` touches records, while `reheader`
modifies only header metadata.

`reheader` is the header-only BAM metadata mutation command. It updates the BAM
header and only the BAM header. The current slice supports full header
replacement from a SAM-style header file plus targeted `@RG`, `@PG`, and `@CO`
mutations. It plans true in-place feasibility conservatively and executes via a
rewrite path in this slice. It does not add, remove, or replace per-record
`RG:Z` tags in alignment records.

`inspect_duplication` is the collection-duplication inspection command. It
accepts a single BAM, FASTQ, or FASTQ.GZ input via `--input`, emits JSON only,
and is explicitly scoped to suspicious collection mishandling signatures such as
exact repeated records, adjacent repeated blocks, and whole-file append
patterns. It is not ordinary PCR duplicate marking, does not use BAM duplicate
flags as primary evidence, and reports findings with explicit confidence and
evidence-strength fields rather than a flat duplicate count.

`deduplicate` is the conservative remediation companion to
`inspect_duplication`. It accepts a single BAM, FASTQ, or FASTQ.GZ input plus
an explicit output path, emits JSON only, and is explicitly scoped to removing
contiguous duplicated collection blocks under a selected policy. The first slice
focuses on adjacent repeated blocks and whole-file append signatures, requires
an explicit remediation mode, and keeps molecular duplicate semantics out of
scope.

`forensic_inspect` is the provenance-inspection companion to `validate`,
`inspect_duplication`, and `deduplicate`. The first slice is BAM-first and
inspects header structure, read-group usage, program-chain anomalies, read-name
regime changes, duplicate-block hallmarks, and optional aux-tag regime shifts.
It is explicitly not a structural validator, not duplicate marking, and not a
fraud detector; it emits evidence-driven findings with conservative follow-up
recommendations.

The repository also contains a benchmark framework under
[benchmarks/](/Users/stephen/Projects/bamana/benchmarks). It uses Nextflow,
containerized toolchains, replicated benchmark runs, and R-based aggregation to
compare Bamana against `samtools`, `fastcat`, and other relevant comparators
without forcing unsupported workflows into misleading timing results.

Benchmark-profile operator documentation for the owned
`bamana benchmark --profile ...` command now lives under
[sphinx/index.rst](/Users/stephen/Projects/bamana/docs/sphinx/index.rst).
