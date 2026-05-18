# Milestone 7: Native Mutation, Remediation, And Forensics Commands

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
