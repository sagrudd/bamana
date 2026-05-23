# Milestone 10: Native Indexed Region Workflows

Status: active as of 2026-05-23. M10.1 activated this milestone only after
Milestone 9 closeout evidence recorded native BAI writing, BAI validation,
typed virtual offsets, internal random-access helpers, and index-aware
`check_map`/`summary` evidence.

## Technical Goal

Turn the Milestone 9 index and random-access substrate into bounded,
user-visible indexed-region workflows without expanding into native CRAM
parsing or broad comparator parity.

Primary command and subsystem targets:

* indexed region parsing and normalization
* random-access chunk planning
* region-aware evidence paths for `check_map` and `summary`
* first public indexed-region command contract, if one is introduced
* fixture and benchmark coverage for indexed versus scan fallback behavior

## Owned Modules

Primary ownership:

* `src/bam/index.rs`
* `src/bgzf/virtual_offset.rs`
* `src/bgzf/reader.rs`
* `src/bam/scan.rs`
* `src/bam/record.rs`
* command orchestration in `src/commands/check_map.rs` and
  `src/commands/summary.rs`

Expected new or expanded support modules:

* region parsing and normalization
* chunk planning and chunk coalescing
* indexed read helpers above the native BGZF reader

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF and virtual-offset groundwork
* Milestone 3 native BAM record scanner
* Milestone 6 native inspection and validation command hardening
* Milestone 9 native BAM index writing and random-access groundwork

## Commands Enabled Or Hardened

Primary beneficiaries:

* region-aware `check_map`
* region-aware `summary`
* future indexed region selection command, if promoted into the public CLI

## M10.1 Baseline Audit

Present substrate:

* `src/bam/index.rs` owns native BAI parsing, implemented-depth validation,
  BAI bin/chunk and linear-index representation, `bai_bin_for_region`, CSI
  header detection, and adjacent sidecar classification.
* `src/bgzf/reader.rs` seeks by typed `VirtualOffset`, and `src/bam/scan.rs`
  exposes `raw_records_in_virtual_range` for bounded internal record retrieval
  between virtual offsets.
* `src/commands/check_map.rs` and `src/commands/summary.rs` already use usable
  BAI mapped/unmapped metadata as index-derived evidence and otherwise explain
  native scan fallback.
* fixture plans include valid coordinate BAM/BAI pairs, stale BAI, malformed
  BAI, mismatched-reference BAI, and CSI-header fixtures.

Known gaps:

* no public region syntax, coordinate-base, inclusivity, or
  interval-normalization contract is frozen yet;
* no region-file contract is defined yet;
* random-access chunk planning has not yet been promoted into public command
  behavior;
* overlapping-region, duplicate-region, and multi-reference semantics remain
  unspecified;
* region-aware `check_map` and `summary` payloads do not yet distinguish
  requested-region evidence from whole-file evidence;
* indexed-region benchmark rows do not yet compare lookup behavior with scan
  fallback;
* CSI large-reference behavior remains unsupported until a later scoped
  decision.

## Acceptance Criteria

* region strings and optional region files are parsed into a documented
  normalized interval model
* indexed chunk planning uses validated BAI or scoped CSI evidence only when
  that evidence is suitable for the request
* region-aware command payloads distinguish index-derived, random-access, and
  scan-fallback evidence
* region queries reject unknown references, impossible intervals, unsupported
  index kinds, and stale or unusable sidecars precisely
* scan fallback remains explicit, native, and bounded by documented behavior
* any new public command or flag is added to JSON schemas, examples, CLI docs,
  Sphinx docs, and contract tests before being treated as stable

## Benchmark Hooks

* indexed region lookup smoke timings for tiny coordinate BAM fixtures
* indexed versus full-scan timings for `check_map`
* indexed versus full-scan timings for `summary`
* fallback-scan timings when no usable index is present

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

Disallowed:

* production indexed BAM region parsing, chunk planning, or random-access hot
  paths through `noodles`

## Risks / Follow-Up

* region syntax must be deliberately small before it is made public
* BAI cannot represent every possible coordinate space and large-reference
  behavior must not be overclaimed
* duplicate or overlapping regions can double-count records unless the payload
  contract defines deduplication semantics explicitly
* CRAM indexed queries remain a later milestone unless native CRAM support is
  explicitly promoted
