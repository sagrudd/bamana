# Milestone 4: Native FASTQ / FASTQ.GZ Parser

## Technical Goal

Implement and stabilize a Bamana-native FASTQ and FASTQ.GZ parser / writer
core.

## Owned Modules

Primary ownership:

* `src/fastq/mod.rs`
* future `src/fastq/reader.rs`
* future `src/fastq/writer.rs`
* future `src/fastq/gzip.rs`
* future `src/fastq/record.rs`

Compatibility shim:

* `src/ingest/fastq.rs`

## Dependencies / Prerequisites

Independent of BAM record scanning in principle, but sequenced after BAM-first
substrate work because:

* BAM-first command migration is higher priority
* FASTQ work still benefits from the same native-core patterns

## Commands Enabled Or Migrated

Primary beneficiaries:

* FASTQ-side `subsample`
* `consume`
* `inspect_duplication`
* `deduplicate`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility
* tests and oracles

Disallowed:

* external generic bioinformatics format crates as the FASTQ hot path

## Acceptance Criteria

* plain FASTQ parsing works robustly
* FASTQ.GZ parsing works robustly
* 4-line structure is validated
* sequence and quality length equality is validated
* valid FASTQ and FASTQ.GZ writing is supported
* no FASTQ hot path depends on external generic bioinformatics format crates

## Benchmark Hooks

* FASTQ parse throughput microbenchmark
* FASTQ.GZ parse throughput microbenchmark
* rerun FASTQ-side `subsample` timing
* compare ingest throughput where meaningful against earlier implementation and
  external comparators such as `fastcat` or `seqtk`

## Risks / Follow-Up

* gzip strategy and buffering need to stay honest in benchmark interpretation
* future paired-read semantics should not bloat the base parser abstraction
