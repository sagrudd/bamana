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

## M8.1 Baseline Audit

The activation baseline found the following native paths already present:

* `sort` uses Bamana's sort engine to build coordinate or queryname orderings,
  rewrites `@HD` sort metadata, writes BAM output through the native BGZF
  writer, and can request canonical checksum verification.
* `merge` combines BAM inputs with conservative reference-dictionary
  compatibility checks, supports input-order, coordinate, and queryname output
  modes, writes through the native BGZF writer, and can request canonical
  multiset checksum verification.
* `checksum` exposes deterministic native checksum domains over serialized BAM
  header and record payloads, including raw encounter-order, canonical
  order-insensitive, header, and payload modes plus primary/mapped/tag filters.
* `explode` supports BAM, SAM, and FASTQ.GZ inputs. The FASTQ.GZ path can
  create or reuse adjacent `FASTQ.GZI` metadata for contiguous shard planning,
  and the BAM/SAM paths preserve encounter order within each shard.
* `consume` performs discovery, format classification, mixed-format policy
  enforcement, FASTQ/SAM/BAM normalization, explicit CRAM reference-policy
  handling, threaded FASTQ.GZ import, and dry-run reporting.
* JSON schemas, canonical examples, README coverage, CLI contracts, JSON-output
  notes, and fixture reservations already exist for the M8 command set.

The baseline also records the hardening gaps that later M8 tasks must resolve
or explicitly defer:

* `sort`, `merge`, `checksum`, BAM-side `explode`, and BAM-side `consume`
  still use older `BamReader::open`, `parse_bam_header_from_reader`, and
  `read_next_record_layout` paths in several places rather than consistently
  using scanner-owned raw-record or writer-bridge ownership.
* `sort`, `merge`, canonical `checksum`, and BAM/SAM sharding use in-memory
  first-slice strategies that need documented limits, benchmark
  interpretation, and later external-memory follow-up where appropriate.
* `consume --verify-checksum` remains planned in this slice and must either be
  implemented in M8 or documented as a precise deferral.
* `explode` shard guarantees need a full audit across BAM, SAM, and FASTQ.GZ,
  especially around checksum intent, index metadata, exact boundary claims,
  and uneven `FASTQ.GZI` checkpoint-aligned shard sizes.
* M8 dependency-boundary tests and command-level benchmark smoke hooks are not
  yet complete for the full command set.

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
