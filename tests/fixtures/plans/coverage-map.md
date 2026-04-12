# Fixture Coverage Map

This document describes how the planned fixture suite will move Bamana from
schema-only contract checks toward executable interop coverage.

## Commands With Clear Fixture Targets

### Broad baseline coverage

These commands should become executable quickly once
`tiny.valid.coordinate.bam` exists:

* `identify`
* `verify`
* `check_eof`
* `header`
* `check_sort`
* `check_map`
* `check_index`
* `summary`
* `validate`
* `checksum`

### Tag-focused coverage

These commands depend on the tag fixtures:

* `check_tag`
* `validate` aux traversal branches
* tag-aware `checksum` modes

### Index-focused coverage

These commands depend on the BAI fixtures:

* `check_index`
* index-backed `check_map`
* index-aware `summary`
* `index` once writer support is present

### Transform coverage

These commands depend on the transform fixture family:

* `sort`
* `merge`
* `explode`
* canonical `checksum` preservation checks

### Consume coverage

These commands depend on a small consume-specific fixture family:

* `tiny.valid.coordinate` for alignment-mode BAM ingest
* `tiny.valid.sam` for alignment-mode SAM ingest
* `tiny.valid.fastq` for unmapped FASTQ ingest
* `tiny.valid.fastq_gz` for unmapped FASTQ.GZ ingest
* `tiny.consume.mixed_alignment_raw` for strict mixed-format rejection
* `tiny.consume.directory_tree` for deterministic directory traversal

## Commands Still Primarily Backed By Spec Artifacts

Until real fixtures land, the following remain mostly schema/example-backed:

* `index` creation success paths
* `explode` runtime behavior
* merge/explode round-trip preservation

That is acceptable, but the manifest now makes the missing executable assets
explicit.

## Recommended Build-Out Order

1. `tiny.valid.coordinate.bam`
2. `tiny.valid.coordinate.bam.bai`
3. `tiny.invalid.no_eof.bam`
4. `tiny.invalid.truncated_record.bam`
5. `tiny.tags.nm_rg.bam`
6. transform family

This order gives the highest executable contract value with the fewest files.

## `consume`

Target fixture coverage:

* `tiny.valid.coordinate`: alignment-mode BAM ingest
* `tiny.valid.sam`: alignment-mode SAM ingest
* `tiny.valid.cram.explicit_ref`: alignment-mode CRAM ingest with explicit
  FASTA under `--reference-policy strict`
* `tiny.valid.cram.reference_required`: strict-policy CRAM missing-reference
  failure using the same tiny CRAM semantics as the explicit-reference success
  case when practical
* `tiny.valid.cram.compatible_refdict` +
  `tiny.valid.bam.compatible_refdict`: mixed alignment-bearing consume success
  with identical reference dictionaries
* `tiny.valid.cram.compatible_refdict` +
  `tiny.valid.bam.incompatible_refdict`: `incompatible_headers` failure
* `tiny.valid.cram.no_external_ref`: conservative no-external-reference CRAM,
  only if a deterministic fixture is actually available
* `tiny.valid.fastq`: unmapped FASTQ ingest
* `tiny.valid.fastq_gz`: unmapped FASTQ.GZ ingest
* `tiny.consume.mixed_alignment_raw`: mixed-format rejection
* `tiny.consume.directory_tree`: lexical discovery, recursive traversal, and
  unsupported-entry reporting

Each CRAM-oriented consume fixture should support:

* JSON schema validation against `consume.schema.json`
* golden-output testing for stable `reference.policy`, `reference.source_used`,
  and error-code fields
* CLI smoke coverage for representative invocations
* future interop expansion once real CRAM binaries are committed

Representative CRAM consume contract scenarios:

* explicit-reference success:
  `bamana consume --mode alignment --input tiny.valid.cram.explicit_ref.cram --reference tiny.ref.primary.fasta --reference-policy strict --out out.bam`
  Expected outcome: success, `reference.source_used = explicit_fasta`.
* strict missing-reference failure:
  `bamana consume --mode alignment --input tiny.valid.cram.explicit_ref.cram --reference-policy strict --out out.bam`
  Expected outcome: failure, `error.code = reference_required`.
* compatible header success:
  `bamana consume --mode alignment --input tiny.valid.cram.compatible_refdict.cram tiny.valid.bam.compatible_refdict.bam --reference tiny.ref.primary.fasta --reference-policy strict --out out.bam`
  Expected outcome: success, `header.reference_compatibility = compatible`.
* incompatible header failure:
  `bamana consume --mode alignment --input tiny.valid.cram.compatible_refdict.cram tiny.valid.bam.incompatible_refdict.bam --reference tiny.ref.primary.fasta --reference-policy strict --out out.bam`
  Expected outcome: failure, `error.code = incompatible_headers`.

## Duplication And Forensics Trio

The fixture plan also reserves a focused build-out path for:

* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

### `inspect_duplication`

Target fixture coverage:

* `tiny.clean.fastq`: no duplication
* `tiny.clean.bam`: no duplication
* `tiny.duplicate.fastq.whole_append`: strong whole-append detection
* `tiny.duplicate.fastq.local_block`: local block detection
* `tiny.duplicate.bam.local_block`: BAM contiguous block detection
* `tiny.invalid.fastq.truncated`: parse-failure path
* `tiny.invalid.bam.truncated_record.duplication`: parse-failure path

### `deduplicate`

Target fixture coverage:

* clean fixtures: no-op success
* duplicate FASTQ/BAM fixtures: stable dry-run removal plan plus stable applied
  clean output
* invalid fixtures: parse-failure path

### `forensic_inspect`

Target fixture coverage:

* `tiny.clean.bam`: clean success
* `tiny.forensic.bam.rg_pg_inconsistent`: header/program provenance findings
* `tiny.forensic.bam.readname_shift`: read-name regime-shift findings
* `tiny.forensic.bam.concatenated_signature`: high-confidence suspicious result
* `tiny.invalid.bam.truncated_record.duplication`: parse-failure path
