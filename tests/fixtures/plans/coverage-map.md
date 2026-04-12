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
* future `reheader` header-only mutation checks
* future `annotate_rg` record-level RG annotation checks

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

Provenance root for the first explicit-reference CRAM package:

* `tests/fixtures/source/tiny.valid.cram.explicit_ref.source.sam`
* `tests/fixtures/source/tiny.ref.primary.fasta`

Those source files should support:

* explicit-reference success by pairing the derived CRAM with the committed
  FASTA
* strict missing-reference failure by withholding the same FASTA under the same
  CRAM input
* future compatibility coverage by pairing the derived CRAM with a BAM derived
  from the same source SAM

Reserved future consume golden outputs for this package:

* `tests/fixtures/expected/consume/consume.tiny.valid.cram.explicit_ref.success.json`
* `tests/fixtures/expected/consume/consume.tiny.valid.cram.explicit_ref.reference_required.failure.json`

## `reheader`

Target fixture coverage:

* `tiny.valid.coordinate.bam`: dry-run planning for header replacement, `@RG`
  add/update/remove, and explicit checksum-with-header-excluded reporting
* `tiny.valid.coordinate.bam` plus future companion index: index invalidation
  and reindex-request reporting
* `tiny.invalid.header_replacement.sam`: invalid replacement-header failure
* `tiny.valid.coordinate.bam` with missing target `@RG`: `missing_read_group`
  failure

Representative `reheader` contract scenarios:

* header-only dry-run planning:
  `bamana reheader --bam tiny.valid.coordinate.bam --add-rg ID=rg1,SM=s1,PL=ONT --dry-run --in-place`
  Expected outcome: success, `planning.in_place_feasible = false`,
  `execution.dry_run = true`.
* rewrite execution:
  `bamana reheader --bam tiny.valid.coordinate.bam --add-rg ID=rg1,SM=s1,PL=ONT --rewrite-minimized --out out.bam`
  Expected outcome: success, `execution.mode_used = rewrite-minimized`.
* missing read-group failure:
  `bamana reheader --bam tiny.valid.coordinate.bam --set-sample s1 --target-rg rg_missing --rewrite-minimized --out out.bam`
  Expected outcome: failure, `error.code = missing_read_group`.

Each future `reheader` fixture should support:

* JSON schema validation against `reheader.schema.json`
* golden-output testing for planning/execution fields and the header-only
  semantics notes
* checksum validation that excludes the header when header-only preservation is
  asserted

## `annotate_rg`

Target fixture coverage:

* `tiny.valid.coordinate.bam`: `only_missing`, `replace_existing`, and
  `fail_on_conflict` record-policy coverage
* `tiny.tags.nm_rg.bam`: existing RG-tag normalization and RG-excluded checksum
  verification
* `tiny.valid.coordinate.bam` plus future companion index: index invalidation
  and reindex-request reporting
* future header variants with and without the target `@RG`: explicit
  `require_existing` and `create_if_missing` header-policy coverage

Representative `annotate_rg` contract scenarios:

* insert missing RG tags only:
  `bamana annotate_rg --bam tiny.valid.coordinate.bam --rg-id rg001 --only-missing --create-header-rg --out out.bam`
  Expected outcome: success, missing records gain `RG:Z:rg001`, existing
  conflicting records remain unchanged.
* replace all RG tags:
  `bamana annotate_rg --bam tiny.tags.nm_rg.bam --rg-id rg001 --replace-existing --require-header-rg --verify-checksum --out out.bam`
  Expected outcome: success, all records end with `RG:Z:rg001`,
  `checksum_verification.excluded_tags = [\"RG\"]`.
* fail on conflict:
  `bamana annotate_rg --bam tiny.tags.nm_rg.bam --rg-id rg001 --fail-on-conflict --require-header-rg --out out.bam`
  Expected outcome: failure, `error.code = conflicting_read_group_tags`.

Each future `annotate_rg` fixture should support:

* JSON schema validation against `annotate_rg.schema.json`
* golden-output testing for request mode, header policy, and record summary
  fields
* RG-excluded checksum validation when the command asserts that only RG
  annotation changed

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
* `tiny.invalid.bam.truncated_record`: parse-failure path

Contract assertions to reserve explicitly:

* duplication taxonomy classification stays machine-readable and stable-minded
* `confidence` and `evidence_strength` remain separate from finding type
* adjacent repeated-block findings report deterministic 1-based record ranges
* BAM duplicate flags are not treated as primary duplication evidence
* the command remains explicit that it targets collection duplication and
  operator error, not ordinary PCR duplicate semantics

### `deduplicate`

Target fixture coverage:

* `tiny.clean.fastq`: no-op success
* `tiny.clean.bam`: no-op success
* `tiny.duplicate.fastq.whole_append`: stable dry-run whole-file-append plan
  plus stable applied clean output
* `tiny.duplicate.fastq.local_block`: adjacent repeated-block dry-run and
  applied removal coverage
* `tiny.duplicate.bam.local_block`: BAM contiguous-block removal coverage with
  header preservation and index invalidation reporting
* `tiny.invalid.fastq.truncated`: parse-failure path
* `tiny.invalid.bam.truncated_record`: parse-failure path

Contract assertions to reserve explicitly:

* dry-run planning reports deterministic 1-based keep/remove record ranges
* applied output preserves encounter order of retained records
* `keep_policy` remains explicit and auditable
* BAM duplicate flags are not treated as the primary removal basis
* existing BAM indices are reported as invalid after record removal unless a
  future slice reports successful regeneration
* the command remains explicit that it remediates collection duplication and
  operator error, not ordinary PCR duplicate semantics

### `forensic_inspect`

Target fixture coverage:

* `tiny.clean.bam`: clean success with no provenance findings
* `tiny.forensic.bam.concatenated_signature`: duplicate-block and append
  hallmark findings
* `tiny.forensic.bam.rg_pg_inconsistent`: header/program/read-group mismatch
  findings
* `tiny.forensic.bam.readname_shift`: read-name regime-shift and optional
  tag-schema-shift findings
* `tiny.invalid.bam.truncated_record`: parse-failure path

Contract assertions to reserve explicitly:

* finding categories remain machine-readable and stable-minded
* `severity`, `confidence`, `evidence_strength`, and `evidence_scope` remain
  distinct fields
* bounded scans do not overclaim whole-file conclusions
* the command remains explicit that it reports provenance anomalies and
  coercion hallmarks, not structural validity, duplicate marking, or fraud
  attribution

## Trio Contract Integration

The trio fixture layer is intended to support three distinct contract-testing
behaviors:

* `json_contract.rs`: verify that every trio semantic class has a manifest
  entry and a reserved golden-output naming path
* `golden_outputs.rs`: compare real command output with reserved JSON for clean,
  duplicate, forensic, and invalid semantics without collapsing those classes
* `cli_contract.rs`: keep representative fixture ids stable enough for smoke
  invocations and help-text examples
