# CRAM Source Provenance Package

This directory contains the human-auditable source-of-truth files for the first
tiny CRAM consume fixture package.

## Purpose

The package exists to support `bamana consume` contract and interop tests for:

* explicit-reference CRAM success
* strict missing-reference failure using the same CRAM fixture
* future alignment-mode compatibility checks against a BAM derived from the
  same source alignments

## Source Of Truth

The source-of-truth files are:

* [tiny.valid.cram.explicit_ref.source.sam](/Users/stephen/Projects/bamana/tests/fixtures/source/tiny.valid.cram.explicit_ref.source.sam)
* [tiny.ref.primary.fasta](/Users/stephen/Projects/bamana/tests/fixtures/source/tiny.ref.primary.fasta)

These files are intentionally tiny, synthetic, and repository-local.

They were created specifically for deterministic contract testing. The SAM and
FASTA are the auditable provenance root. Reviewers should read these files
before reviewing any regenerated BAM or CRAM artifact.

## Derived Artifacts

Derived artifacts planned from this package are:

* `tests/fixtures/bam/valid/tiny.valid.cram.explicit_ref.source.bam`
* `tests/fixtures/cram/valid/tiny.valid.cram.explicit_ref.cram`

The derived BAM and CRAM are governed outputs, not the provenance root.

## Contract Semantics

This package is designed so that:

* `consume --mode alignment --input tiny.valid.cram.explicit_ref.cram --reference tiny.ref.primary.fasta --reference-policy strict`
  should succeed once the derived CRAM exists and is wired into executable
  tests
* the same CRAM should fail under `--reference-policy strict` when the FASTA is
  withheld

Missing-reference failure is exercised by withholding the FASTA under strict
policy, not by changing the fixture itself.

## Review Guidance

When this package changes, reviewers should check:

* whether the SAM header still matches the FASTA reference dictionary exactly
* whether the aligned records are still tiny, deterministic, and easy to audit
* whether the derived BAM/CRAM paths and manifest relationships are still
  accurate
* whether any binary artifact drift came from source changes or toolchain
  changes

## Regeneration

Use the documented recipe in:

* [generate_tiny_cram_fixture.sh](/Users/stephen/Projects/bamana/tests/fixtures/source/generate_tiny_cram_fixture.sh)
* [tests/fixtures/plans/generation-strategy.md](/Users/stephen/Projects/bamana/tests/fixtures/plans/generation-strategy.md)

Important:

* exact byte-for-byte CRAM reproducibility may depend on the external CRAM
  toolchain version
* the provenance root remains the committed SAM and FASTA regardless
