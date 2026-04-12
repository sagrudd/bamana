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

## Consume Fixtures

The fixture suite also reserves a focused planning layer for `consume`.

Those fixtures should separate:

* alignment-mode BAM/SAM ingest
* unmapped FASTQ / FASTQ.GZ ingest
* mixed-format rejection
* directory traversal behavior

The first consume fixtures should remain tiny and should prove discovery and
policy semantics before they try to exercise larger normalization workflows.

## Adding A Fixture

When adding or changing a fixture:

1. update `manifest.json`
2. update the relevant plan files under `plans/`
3. add or update the expected outputs under `expected/`
4. document regeneration or mutation steps under `scripts/`
5. update contract or interop tests if command coverage changes

Avoid adding large datasets or opaque binary blobs without explicit review.
