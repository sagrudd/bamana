# Bamana Fixture Suite

The `tests/fixtures/` tree is the repository plan for Bamana's tiny synthetic
fixture suite.

## Purpose

The fixture suite exists to support:

* contract tests against real BAM inputs
* interoperability tests against expected JSON outputs
* regression tests for shallow checks and deep validation
* transformation tests for `sort`, `merge`, `explode`, and checksum
  preservation workflows

## Philosophy

The suite should stay:

* tiny
* deterministic
* reviewable
* versioned
* easy to regenerate

The goal is not to mirror production datasets. The goal is to provide a small
set of high-value, purpose-built files that make the contract layer executable.

## Duplication And Forensics Fixtures

The repository now reserves a separate fixture-planning layer for:

* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

These commands need separate semantic classes:

* clean fixtures
* duplicate fixtures caused by operator or workflow error
* forensic fixtures that are parseable but suspicious
* invalid fixtures that fail parsing

That distinction matters. Operator-error duplication is not the same as
molecular duplicate biology, and suspicious provenance hallmarks are not the
same as structural corruption.

## Consume Fixtures

`consume` needs a separate fixture layer because it is the repository’s
ingestion gateway rather than a BAM-only downstream operation.

Its fixture plan must cover:

* alignment-bearing ingest (`BAM`, `SAM`)
* raw-read ingest (`FASTQ`, `FASTQ.GZ`)
* mixed-format rejection across those boundaries
* deterministic directory traversal, including unsupported and nested entries

These fixtures should be used to prove discovery order, classification, and
policy behavior before larger normalization or transform workflows are tested.

## Review Expectations

Fixture changes should be reviewed as governed assets.

A pull request that changes fixtures should normally include:

* a manifest update
* plan or regeneration-doc updates when the suite shape changes
* expected-output updates when command behavior changes intentionally
* a clear explanation of whether the change is semantic, corrective, or purely
  additive

## Regeneration

The preferred model is:

* hand-authored tiny SAM sources for valid fixture roots
* deterministic BAM/BAI generation
* documented mutation scripts for invalid derivatives
* expected JSON outputs captured from stable command runs

See:

* [tests/fixtures/README.md](/Users/stephen/Projects/bamana/tests/fixtures/README.md)
* [tests/fixtures/plans/generation-strategy.md](/Users/stephen/Projects/bamana/tests/fixtures/plans/generation-strategy.md)
* [tests/fixtures/plans/coverage-map.md](/Users/stephen/Projects/bamana/tests/fixtures/plans/coverage-map.md)
