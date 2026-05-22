# Milestone 7: Native Mutation, Remediation, And Forensics Commands

Status: active as of 2026-05-22, after Milestone 6 closed on 2026-05-21.

## Technical Goal

Harden the next command wave on native BAM, FASTQ, scanner, and header
substrates:

* `reheader`
* `annotate_rg`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

This milestone covers commands that either mutate metadata/records, remediate
operator-error duplication signatures, or produce provenance evidence. The
scope is operational correctness and native hot-path ownership, not biological
duplicate marking or fraud detection.

## Owned Modules

Primary ownership:

* `src/bam/header.rs`
* `src/bam/reheader.rs`
* `src/bam/annotate_rg.rs`
* `src/bam/scan.rs`
* `src/bam/record.rs`
* `src/bam/tags.rs`
* `src/fastq/`
* `src/forensics/duplication.rs`
* `src/forensics/deduplicate.rs`
* `src/forensics/forensic_inspect.rs`
* command orchestration in `src/commands/reheader.rs`,
  `src/commands/annotate_rg.rs`, `src/commands/inspect_duplication.rs`,
  `src/commands/deduplicate.rs`, and `src/commands/forensic_inspect.rs`

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF
* Milestone 2 native BAM header codec
* Milestone 3 native BAM record scanner
* Milestone 4 native FASTQ / FASTQ.GZ parser
* Milestone 6 native inspection and validation command hardening

## Commands Enabled Or Hardened

Primary beneficiaries:

* `reheader`
* `annotate_rg`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

## M7.1 Baseline Audit

The activation baseline found the following native paths already present:

* `reheader` plans header-only mutations with the native BAM header codec,
  serializes replacement headers with Bamana's header serializer, rewrites BAM
  output through the native BGZF writer, and reports checksum and index
  invalidation evidence. It does not modify per-record `RG:Z` tags. M7.3 added
  direct tests for header replacement, `@RG`/`@PG`/`@CO` mutation planning,
  checksum verification, index invalidation reporting, and refusal to claim
  true in-place safety when not proven.
* `annotate_rg` uses the native BAM header codec, native aux-tag traversal,
  native record-layout serialization, and the native BGZF writer for
  record-level read-group annotation. M7.4 added direct tests for record-mode
  policies, header-policy choices, dry-run behavior, `RG`-excluded checksum
  verification, index invalidation reporting, and output `RG:Z` mutation.
* `inspect_duplication` scans BAM input through `BamScanner` and FASTQ or
  FASTQ.GZ input through the native FASTQ reader. M7.5 added direct tests for
  clean full scans, whole-file append signatures, local contiguous-block
  signatures, bounded scan caveats, malformed FASTQ parse uncertainty,
  BAM-only read-group identity semantics, and FASTQ rejection of RG identity.
* `deduplicate` supports conservative BAM, FASTQ, and FASTQ.GZ remediation.
  M7.6 moved BAM planning onto `BamScanner` with a native record-layout bridge,
  while FASTQ output continues to use the native FASTQ writer and BAM output
  continues to use Bamana's header serialization, record-layout serialization,
  and BGZF writer.
* `forensic_inspect` is BAM-first and uses `BamScanner` plus native header,
  record, aux-tag, read-name, and duplication-hallmark evidence. M7.7 added
  direct tests for read-name regime shifts, aux-tag regime shifts, bounded
  scan caveats, full-scan evidence scope, max-finding truncation, and
  non-BAM format rejection.

Hardening still required by the milestone:

* continue to distinguish `reheader` header-only behavior from `annotate_rg`
  record-level mutation;
* add command-level smoke benchmark evidence for the complete M7 command set.

M7.3 added `reheader` dry-run smoke timing to `header_microbench --bamana-bin`
as the first M7 command-level benchmark hook. M7.4 added the corresponding
`annotate_rg` dry-run smoke timing. M7.5 added `inspect_duplication` command
smoke timing to `scanner_microbench --bamana-bin`. M7.6 added `deduplicate`
dry-run command smoke timing to the same scanner benchmark hook. M7.7 added
`forensic_inspect` full-scan command smoke timing with explicit provenance
scopes.

M7.8 hardened cross-command output safety: `reheader`, `annotate_rg`, and
`deduplicate` have focused no-output-on-failure coverage for existing output
paths, `deduplicate --emit-removed-report` validates the report path before
writing the primary output, and dry-run paths remain side-effect bounded.

M7.2 froze the command schemas, canonical success/failure examples, fixture
reservations, documentation caveats, and dependency-boundary protection for the
five governed commands before later implementation hardening.

## Acceptance Criteria

* `reheader` uses the native header codec and documents header-only behavior
* `annotate_rg` uses native header/record primitives and remains clearly
  distinct from `reheader`
* `inspect_duplication` uses native BAM scanner and native FASTQ parser paths
* `deduplicate` uses native BAM/FASTQ parser and writer paths for supported
  remediation modes
* `forensic_inspect` uses native scanner/header evidence and preserves its
  provenance-inspection boundary
* command JSON contracts remain stable or are deliberately versioned
* dependency-boundary tests protect mutation, remediation, and forensics hot
  paths from direct `noodles` imports

## Benchmark Hooks

* command-level smoke timings for `reheader`
* command-level smoke timings for `annotate_rg`
* command-level smoke timings for `inspect_duplication`
* command-level smoke timings for `deduplicate`
* command-level smoke timings for `forensic_inspect`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

Disallowed:

* production mutation, remediation, or forensics hot paths through `noodles`

## Risks / Follow-Up

* `deduplicate` must remain conservative remediation, not biological duplicate
  marking
* `forensic_inspect` must remain evidence-driven provenance inspection, not a
  fraud detector
* `annotate_rg` and `reheader` must preserve their record-touching versus
  header-only distinction
