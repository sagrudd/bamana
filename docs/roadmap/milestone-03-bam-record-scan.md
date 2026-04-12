# Milestone 3: Native BAM Record Scanner

## Technical Goal

Implement a Bamana-native selective BAM record scanner that can:

* iterate records without full generic decode
* expose lightweight record views
* extract selected fields efficiently
* skip unneeded variable sections safely

## Owned Modules

Primary ownership:

* future `src/bam/record.rs`
* future `src/bam/fields.rs`
* future `src/bam/scan.rs`
* `src/bam/reader.rs`
* existing `src/bam/records.rs` as the current bridge into richer layouts

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF
* Milestone 2 native BAM header codec

## Commands Enabled Or Migrated

Primary beneficiaries:

* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`
* `inspect_duplication`
* `forensic_inspect`
* BAM-side `subsample`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility
* tests and oracles

Disallowed:

* hot-path BAM record iteration through `noodles`

## Acceptance Criteria

* native scan loop can iterate BAM records without full generic decode
* lightweight record views expose at least:
  * `refID`
  * `pos`
  * flags
  * MAPQ
  * read name
  * sequence length
  * aux region boundaries
* scanner can skip non-needed fields efficiently
* richer record conversion remains possible when required
* no hot-path BAM record scanning depends on `noodles`

## Benchmark Hooks

* records-per-second BAM scanner microbenchmark
* selective field extraction microbenchmark
* compare scanner throughput against the earlier implementation
* rerun `summary`, `check_sort`, and `check_map` timing after adoption

## Risks / Follow-Up

* scanner API must avoid becoming a new generic abstraction tax
* aux traversal and sequence access need careful bounds and allocation control
