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

`consume` is the ingestion gateway into Bamana. It is the command that accepts
files and directories containing supported upstream formats and normalizes them
into a single BAM according to an explicit ingest mode. The current staged
implementation supports alignment-mode BAM/SAM/CRAM normalization and unmapped
FASTQ/FASTQ.GZ normalization, with deterministic directory discovery and dry-run
support. CRAM is available only in alignment mode and is governed by an
explicit `--reference-policy`; Bamana does not silently guess CRAM reference
behavior. Its detailed contract is documented in
[spec/cli/commands.md](/Users/stephen/Projects/bamana/spec/cli/commands.md).
