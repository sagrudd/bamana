# Milestone 6: Native Inspection And Validation Commands

## Technical Goal

Use the native BGZF, BAM header, BAM scanner, FASTQ, and proof-command
substrates to harden the first operational inspection and validation command
wave:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

## Owned Modules

Primary ownership:

* `src/bgzf/`
* `src/bam/header.rs`
* `src/bam/scan.rs`
* `src/bam/record.rs`
* `src/bam/tags.rs`
* `src/bam/index.rs`
* command orchestration in `src/commands/check_eof.rs`,
  `src/commands/check_sort.rs`, `src/commands/check_map.rs`,
  `src/commands/summary.rs`, `src/commands/check_tag.rs`, and
  `src/commands/validate.rs`

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF
* Milestone 2 native BAM header codec
* Milestone 3 native BAM record scanner
* Milestone 5 proof-command migration

Milestone 4 FASTQ work is not a direct dependency for BAM inspection, but the
project should keep dependency-boundary and benchmark discipline consistent
across both native cores.

## Commands Enabled Or Hardened

Primary beneficiaries:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

## Acceptance Criteria

* `check_eof` uses the native BGZF EOF-marker path and documents its narrow
  boundary
* `check_sort`, `check_map`, `summary`, `check_tag`, and `validate` use native
  BAM scanner/header primitives for their scanner-compatible paths
* command JSON contracts remain stable or are deliberately versioned
* fixture and malformed-input coverage proves bounded versus full-scan
  behavior where relevant
* dependency-boundary tests protect the inspection and validation hot paths
  from direct `noodles` imports

## Benchmark Hooks

* command-level smoke timings for `check_eof`
* command-level smoke timings for `check_sort`
* command-level smoke timings for `check_map`
* command-level smoke timings for `summary`
* command-level smoke timings for `check_tag`
* command-level smoke timings for `validate`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

Disallowed:

* production BAM inspection or validation hot paths through `noodles`

## Risks / Follow-Up

* `validate` must not overclaim full biological or reference-level correctness
* `check_map` must keep index-derived and scan-derived evidence distinct
* bounded scans must continue to state when absence or validity is not proven
