# Bamana Tiny Fixture Suite

This tree is the planned home of Bamana's tiny synthetic BAM fixture suite.
Its purpose is to turn schema- and example-level contract tests into executable
interop tests against a very small number of deliberate sample files.

## Design Rules

* Keep fixtures tiny and deterministic.
* Prefer one primary purpose per fixture.
* Record provenance and regeneration strategy in `manifest.json`.
* Store expected command outputs separately from fixture binaries.
* Treat fixture additions and mutations as governed repository assets.

## Layout

* `manifest.json`: machine-readable fixture inventory and coverage metadata
* `bam/`: planned BAM and index files grouped by intent
* `source/`: human-auditable provenance roots such as tiny SAM and FASTA files
* `expected/`: expected JSON outputs grouped by command
* `plans/`: fixture taxonomy, generation strategy, and coverage planning
* `scripts/`: regeneration and mutation entrypoints

## Current Status

The repository is in the planning-and-scaffolding phase. The suite is designed
to stay intentionally small:

* baseline valid BAMs
* focused tag fixtures
* targeted malformed BAMs
* index fixtures
* transform round-trip fixtures
* clean, duplicate, forensic, and invalid fixtures for duplication-oriented and
  provenance-oriented commands

## Duplication And Forensics Fixture Classes

The fixture suite now reserves a separate planning layer for:

* clean fixtures: parseable baselines with no suspicious duplication
* duplicate fixtures: operator-error style repeated blocks or whole-append
  duplication
* forensic fixtures: parseable but suspicious provenance anomalies
* invalid fixtures: parse-failure assets that should not be conflated with
  suspicious-but-parseable files

This split matters because:

* `inspect_duplication` is about repeated-block signatures
* `deduplicate` is about remediating explicit operator-error duplication
* `forensic_inspect` is about provenance and coercion hallmarks, which may be
  present without simple duplication

### Trio Taxonomy

The trio plan is intentionally small and uses four explicit semantic classes:

* `clean`: parseable baselines with no suspicious duplicate block or provenance
  anomaly
* `duplicate`: parseable operator-error duplication cases such as whole-file
  append or local repeated block
* `forensic`: parseable but suspicious provenance/coercion cases that should
  not be confused with malformed inputs
* `invalid`: controlled parse-failure cases for negative-path contract testing

The current reserved trio fixtures are:

* `tiny.clean.fastq`
* `tiny.clean.bam`
* `tiny.duplicate.fastq.whole_append`
* `tiny.duplicate.fastq.local_block`
* `tiny.duplicate.bam.local_block`
* `tiny.forensic.bam.rg_pg_inconsistent`
* `tiny.forensic.bam.readname_shift`
* `tiny.forensic.bam.concatenated_signature`
* `tiny.invalid.fastq.truncated`
* `tiny.invalid.bam.truncated_record`

An aux-corruption variant such as `tiny.invalid.bam.bad_aux` remains optional
and outside the first tiny core set to keep the trio executable plan small.

### Expected Output Naming

Expected JSON outputs for the trio are reserved under:

* `expected/inspect_duplication/`
* `expected/deduplicate/`
* `expected/forensic_inspect/`

Naming conventions:

* `inspect_duplication.<fixture-id>.success.json`
* `inspect_duplication.<fixture-id>.failure.json`
* `deduplicate.<fixture-id>.dry_run.success.json`
* `deduplicate.<fixture-id>.applied.success.json`
* `deduplicate.<fixture-id>.noop.success.json`
* `deduplicate.<fixture-id>.failure.json`
* `forensic_inspect.<fixture-id>.success.json`
* `forensic_inspect.<fixture-id>.failure.json`

### Test Integration

The trio fixtures are planned to plug into the contract layer as follows:

* `json_contract.rs`: ensure every trio command has schema-backed success and
  failure examples and that the manifest keeps the clean/duplicate/forensic
  split explicit
* `golden_outputs.rs`: compare real command output against golden JSON for
  clean, duplicated, suspicious, and invalid cases
* `cli_contract.rs`: remain largely fixture-independent, with representative
  trio fixture ids available for smoke-style invocation examples when needed

The benchmark framework in
[benchmarks/](/Users/stephen/Projects/bamana/benchmarks) is separate from this
tiny fixture tree. Contract fixtures stay small and reviewable; performance
benchmarks are expected to run against real large BAM and FASTQ.GZ collections.

## Consume Fixtures

The fixture suite also reserves a focused planning layer for `consume`.

Those fixtures should separate:

* alignment-mode BAM/SAM/CRAM ingest
* unmapped FASTQ / FASTQ.GZ ingest
* mixed-format rejection
* explicit CRAM reference-policy outcomes
* directory traversal behavior

The first consume fixtures should remain tiny and should prove discovery and
policy semantics before they try to exercise larger normalization workflows.

### CRAM Consume Companion Set

The CRAM companion set stays intentionally small and exists specifically to
test `consume` semantics that are risky in regulated workflows:

* explicit-reference success under a strict policy
* strict missing-reference failure for a CRAM that otherwise decodes
* header-compatibility behavior when CRAM is combined with BAM/SAM inputs

The preferred CRAM plan is:

* `tiny.valid.cram.explicit_ref` as the primary CRAM success fixture
* `tiny.valid.cram.reference_required` as the strict-policy failure scenario,
  reusing the explicit-reference CRAM where practical
* `tiny.valid.cram.compatible_refdict` plus BAM/SAM companions for
  compatibility checks
* `tiny.valid.cram.no_external_ref` only if it can be generated
  deterministically and reviewed honestly

CRAM fixtures must remain explicit about whether they require a reference and
which reference source they are intended to use. The fixture plan must never
blur missing-reference failures into generic decode failures.

### Source Provenance Package

The first real CRAM consume package should be rooted in plain-text source files:

* `tests/fixtures/source/tiny.valid.cram.explicit_ref.source.sam`
* `tests/fixtures/source/tiny.ref.primary.fasta`

Those files are the auditable source of truth. Derived BAM and CRAM artifacts
should be reviewed as governed outputs derived from that source package, not as
opaque standalone binaries.

## Adding A Fixture

When adding or changing a fixture:

1. update `manifest.json`
2. update the relevant plan files under `plans/`
3. add or update the expected outputs under `expected/`
4. document regeneration or mutation steps under `scripts/`
5. update contract or interop tests if command coverage changes

Avoid adding large datasets or opaque binary blobs without explicit review.
