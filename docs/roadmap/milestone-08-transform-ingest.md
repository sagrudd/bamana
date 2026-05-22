# Milestone 8: Native Transform, Checksum, Explode, And Ingest Commands

Status: active as of 2026-05-22, after Milestone 7 closed on 2026-05-22.

## Technical Goal

Harden Bamana's largest transform and ingest command wave on native
substrates:

* `sort`
* `merge`
* `explode`
* `checksum`
* `consume`

This milestone covers ordering, merging, sharding, checksum domains, and
multi-format ingestion. The scope is explicit operational behavior,
native-hot-path ownership, and benchmark honesty, not full external-tool
parity or native CRAM ownership.

## Owned Modules

Primary ownership:

* `src/bam/sort.rs`
* `src/bam/merge.rs`
* `src/bam/checksum.rs`
* `src/bam/header.rs`
* `src/bam/scan.rs`
* `src/bam/record.rs`
* `src/bam/write.rs`
* `src/fastq/`
* `src/fastq/gzi.rs`
* `src/ingest/consume.rs`
* command orchestration in `src/commands/sort.rs`,
  `src/commands/merge.rs`, `src/commands/explode.rs`,
  `src/commands/checksum.rs`, and `src/commands/consume.rs`

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF
* Milestone 2 native BAM header codec
* Milestone 3 native BAM record scanner
* Milestone 4 native FASTQ / FASTQ.GZ parser
* Milestone 5 proof-command migration
* Milestone 6 native inspection and validation command hardening
* Milestone 7 native mutation, remediation, and forensics hardening

## Commands Enabled Or Hardened

Primary beneficiaries:

* `sort`
* `merge`
* `explode`
* `checksum`
* `consume`

## Acceptance Criteria

* `sort` uses native BAM parsing, ordering, header rewriting, writing, and
  optional checksum verification
* `merge` uses native BAM parsing, header compatibility checks, ordering, and
  writing
* `checksum` uses deterministic native header and record serialization domains
* `explode` uses native BAM/SAM/FASTQ.GZ planning and writing paths with
  explicit shard guarantees
* `consume` uses native discovery, policy enforcement, FASTQ/SAM/BAM ingest,
  and documented CRAM compatibility boundaries
* command JSON contracts remain stable or are deliberately versioned
* dependency-boundary tests protect transform and ingest hot paths from direct
  `noodles` imports outside documented CRAM compatibility

## Benchmark Hooks

* command-level smoke timings for `sort`
* command-level smoke timings for `merge`
* command-level smoke timings for `explode`
* command-level smoke timings for `checksum`
* command-level smoke timings for `consume`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

Disallowed:

* production BAM/FASTQ transform, checksum, explode, or ingest hot paths
  through `noodles`

## Risks / Follow-Up

* large in-memory first-slice strategies may need later external-memory or
  chunked implementations
* checksum domains must remain explicit and deterministic
* `consume` must keep CRAM compatibility clearly isolated from native BAM and
  FASTQ hot paths
