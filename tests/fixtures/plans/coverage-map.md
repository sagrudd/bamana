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

### Milestone 6 inspection and validation coverage

The M6 fixture freeze reserves explicit targets for:

* missing EOF evidence: `tiny.invalid.no_eof`
* sorted evidence: `tiny.valid.coordinate`, `tiny.valid.queryname`
* unsorted evidence: `tiny.invalid.unsorted_coordinate`
* mapped and unmapped evidence: `tiny.valid.coordinate`, `tiny.valid.unmapped`
* index-derived mapping and summary evidence: `tiny.valid.coordinate.bai`
* observed and absent tag evidence: `tiny.tags.nm_rg`,
  `tiny.tags.absent_requested`
* malformed aux evidence: `tiny.invalid.bad_aux`
* structural validation failures: `tiny.invalid.truncated_record`,
  `tiny.invalid.header_mismatch`

Bounded-scan fixtures should prove only examined-record claims. Full-scan
fixtures may prove absence or full-file summaries only when the command reaches
EOF cleanly.

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

M9.2 freezes the planned index fixture taxonomy and M9.5 promotes BAM BAI
writing for the coordinate-sorted BAM source:

* valid BAI: `tiny.valid.coordinate.bai`
* malformed BAI: `tiny.invalid.bad_bai`
* mismatched BAI reference count:
  `tiny.invalid.mismatched_reference_count.bai`
* stale BAI timestamp heuristic: `tiny.valid.coordinate.stale_bai`
* CSI header detection with unsupported fallback:
  `tiny.valid.coordinate.csi_header`
* malformed CSI: `tiny.invalid.bad_csi`
* coordinate-sorted BAM source: `tiny.valid.coordinate`
* unsorted BAM index rejection: `tiny.invalid.unsorted_coordinate`
* FASTQ.GZ source and FASTQ.GZI sidecar:
  `tiny.valid.fastq_gz`, `tiny.valid.fastq_gz.gzi`

M9.5 native BAM BAI writing means the coordinate-sorted BAM `index` fixture may
assert `output_index.created = true` for BAI output. BAM CSI and unsupported
failure fixtures must still assert `output_index.created = false`. FASTQ.GZI
success fixtures may also assert `output_index.created = true`.

M12.5 extends the index fixture taxonomy without materializing the binary
assets yet:

* large-reference BAI rejection:
  `tiny.invalid.large_reference.bam`, expected to fail `index --format bai`
  before any output sidecar is written;
* CSI reference-count mismatch:
  `tiny.invalid.mismatched_reference_count.csi`, expected to fail
  `check_index` and drive `check_map`/`summary` scan fallback with
  mismatched-reference diagnostics;
* malformed FASTQ.GZI:
  `tiny.invalid.fastq_gz.bad_gzi`, expected to exercise FASTQ.GZ
  planner-sidecar fallback or failure behavior for `enumerate`, `explode`, and
  `consume`.

M12 fixtures must preserve the M12.2 support-level contract and the M12.4
diagnostic vocabulary: `usable`, `absent`, `stale`, `unsupported`,
`malformed`, `mismatched_reference`, `incomplete`, and `disabled`.

### M10 indexed-region workflow coverage

M10.3 freezes region-aware `check_map` and region-aware `summary` as the first
planned public surfaces. Region fixture coverage is scenario-based over the
existing tiny coordinate BAM/BAI family until implementation tasks materialize
additional files:

* single-region indexed success: `tiny.valid.coordinate` with
  `tiny.valid.coordinate.bai`, `--region chr1:1-10`;
* multi-region indexed success: `tiny.valid.coordinate` with
  `tiny.valid.coordinate.bai`, repeated `--region` values;
* overlapping-region request-order behavior: repeated overlapping `--region`
  values preserve request order and are not merged or deduplicated;
* unknown-reference rejection: `--region missing_reference:1-10`;
* empty-region rejection: empty or whitespace-only region request;
* stale-index scan fallback: `tiny.valid.coordinate.stale_bai`;
* missing-index scan fallback: `tiny.valid.coordinate` without a usable
  adjacent sidecar;
* unsupported-index scan fallback: `tiny.valid.coordinate.csi_header`.

Region-aware payload fixtures must include `region_scope` and distinguish
`indexed`, `scan_fallback`, and `rejected` execution outcomes. Region-scoped
`check_map` evidence must not claim whole-file mapping state, and
region-scoped `summary` metrics must not claim full-file totals.

M10.8 deliberately does not promote a public indexed region selection command.
No region fixture should claim selected-record output until a later task
defines output semantics, header preservation, record ordering,
duplicate-region behavior, index invalidation or regeneration rules, and
output write-safety behavior. The current fixture plan covers read-only
evidence from `check_map --region` and `summary --region` only.

### M11 selected-region output coverage

M11.8 reserves `select_region` fixture coverage for the implemented
file-output surface:

* indexed selected-output success: `tiny.valid.coordinate` with
  `tiny.valid.coordinate.bai`, repeated `--region` values, and
  `expected/select_region/tiny.valid.coordinate.indexed.success.json`;
* scan fallback success: `tiny.valid.coordinate` without a usable adjacent BAI
  or with `tiny.valid.coordinate.stale_bai`;
* duplicate/overlap suppression: overlapping CLI `--region` values must still
  emit each physical source record at most once in source virtual-offset order;
* output-index sidecar collision: a pre-existing selected-output BAI/CSI
  sidecar must fail with `output_exists` unless `--force` is supplied;
* forced sidecar removal: a forced applied run removes stale adjacent output
  BAI/CSI sidecars and reports `output.index_invalidation`;
* same-path rejection: using the input BAM path as `--out` fails even with
  `--force`.

The current fixture plan does not claim binary stdout output, public
`--region-file`, report sidecar routing, replacement output index creation, or
broad external comparator parity for `select_region`.

### Transform coverage

These commands depend on the transform fixture family:

* `sort`
* `merge`
* `explode`
* canonical `checksum` preservation checks
* M7 `reheader` header-only mutation checks
* M7 `annotate_rg` record-level RG annotation checks

### Consume coverage

These commands depend on a small consume-specific fixture family:

* `tiny.valid.coordinate` for alignment-mode BAM ingest
* `tiny.valid.sam` for alignment-mode SAM ingest
* `tiny.valid.fastq` for unmapped FASTQ ingest
* `tiny.valid.fastq_gz` for unmapped FASTQ.GZ ingest and FASTQ.GZI source
  coverage
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

Each M7 `reheader` fixture should support:

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

Each M7 `annotate_rg` fixture should support:

* JSON schema validation against `annotate_rg.schema.json`
* golden-output testing for request mode, header policy, and record summary
  fields
* RG-excluded checksum validation when the command asserts that only RG
  annotation changed

## Duplication And Forensics Trio

The fixture plan also reserves a focused build-out path for:

* `subsample`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

### `subsample`

Target fixture coverage:

* `tiny.clean.fastq`: deterministic repeatability and seeded-random repeatability
* `tiny.clean.bam`: BAM subsampling success with header preservation
* `tiny.invalid.fastq.truncated`: parse-failure path
* `tiny.invalid.bam.truncated_record`: parse-failure path

Contract assertions to reserve explicitly:

* deterministic mode remains repeatable for identical input, fraction, and identity basis
* seeded random mode remains reproducible for a given seed
* retained encounter order is preserved by default
* BAM index invalidation remains explicit after written output
* invalid fraction errors remain machine-readable and distinct from parse failures

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
