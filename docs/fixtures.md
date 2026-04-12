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

* alignment-bearing ingest (`BAM`, `SAM`, staged `CRAM`)
* raw-read ingest (`FASTQ`, `FASTQ.GZ`)
* mixed-format rejection across those boundaries
* explicit CRAM reference-policy success and failure paths
* deterministic directory traversal, including unsupported and nested entries

These fixtures should be used to prove discovery order, classification, and
policy behavior before larger normalization or transform workflows are tested.

### CRAM Consume Fixtures

CRAM is handled more conservatively than BAM or SAM because its decode path may
depend on explicit reference material. The fixture plan therefore stays small
and purpose-built:

* one explicit-reference success fixture
* one strict missing-reference failure scenario
* one compatibility group for CRAM + BAM/SAM reference-dictionary checks

This small companion set is preferable to a large unmanaged CRAM corpus because
it keeps provenance, reference-policy assumptions, and expected `consume`
contract outcomes reviewable. Missing-reference behavior must be tested as its
own contract, not inferred from a generic parse failure, and any future
no-external-reference fixture must remain clearly marked as planned or deferred
rather than assumed to exist.

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

For the first CRAM consume package, reviewers should check:

* the plain-text source SAM before considering the derived CRAM
* the plain-text synthetic FASTA that the CRAM is expected to require
* the provenance metadata that links source SAM, FASTA, derived BAM, and
  derived CRAM
* whether any regenerated binary artifact changed because the source content
  changed or because the external CRAM toolchain changed

Binary fixture derivation should therefore be governed explicitly:

* source SAM and FASTA are the provenance root
* derived BAM and CRAM are maintainer-generated artifacts
* the derivation recipe and script should be reviewed alongside any binary
  refresh
* maintainers should not assume byte-for-byte CRAM stability across toolchain
  changes even when semantic provenance is unchanged

See:

* [tests/fixtures/README.md](/Users/stephen/Projects/bamana/tests/fixtures/README.md)
* [tests/fixtures/plans/generation-strategy.md](/Users/stephen/Projects/bamana/tests/fixtures/plans/generation-strategy.md)
* [tests/fixtures/plans/coverage-map.md](/Users/stephen/Projects/bamana/tests/fixtures/plans/coverage-map.md)
