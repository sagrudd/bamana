# Duplication And Forensics Fixture Plan

This plan covers the trio:

* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

## Semantic Split

The fixture layer must preserve four distinct states:

* `clean`: no suspicious duplication or provenance hallmarks
* `duplicate`: parseable files with operator-error style repeated blocks
* `forensic`: parseable files with suspicious provenance hallmarks
* `invalid`: parse-failure fixtures that should not be treated as meaningful
  forensic or duplication evidence

## Why The Split Matters

Operator-error duplication is not the same as molecular duplicate biology.
These fixtures are intended to test collection-level duplication signatures,
removal planning, and provenance/coercion hallmarks rather than biological
duplicate semantics.

## Minimal Planned Fixture Family

* clean:
  `tiny.clean.fastq`, `tiny.clean.bam`
* duplicate:
  `tiny.duplicate.fastq.whole_append`, `tiny.duplicate.fastq.local_block`,
  `tiny.duplicate.bam.local_block`
* forensic:
  `tiny.forensic.bam.rg_pg_inconsistent`,
  `tiny.forensic.bam.readname_shift`,
  `tiny.forensic.bam.concatenated_signature`
* invalid:
  `tiny.invalid.fastq.truncated`,
  `tiny.invalid.bam.truncated_record.duplication`

## Review Guidance

When a fixture in this family changes, reviewers should ask:

* did the fixture stay in the same semantic class?
* is the duplication signature or forensic hallmark still explicit?
* was parseability intentionally preserved or intentionally broken?
* do the expected outputs still match the command contract rather than a local
  implementation quirk?
