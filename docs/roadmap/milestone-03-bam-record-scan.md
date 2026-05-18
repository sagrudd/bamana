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

## Baseline Audit

M3.1 established this baseline before introducing scanner code:

* `src/bam/records.rs` is the current central BAM record bridge.
  `read_next_record_layout` validates core record bounds, read-name length,
  NUL termination, negative sequence length, and variable-section length
  arithmetic, then materializes read name, CIGAR bytes, sequence bytes, quality
  bytes, and aux bytes into owned values.
* `LightAlignmentRecord` already exposes many required scanner fields:
  `ref_id`, `pos`, flags, MAPQ, read name, and flag-derived booleans. It does
  not expose sequence length, raw record bounds, CIGAR/sequence/quality ranges,
  or aux-region boundaries.
* `read_next_light_record` is not a true selective scan path yet. It calls
  `read_next_record_layout` first, so field-only consumers still allocate and
  read skipped CIGAR, sequence, quality, and aux sections.
* `src/bam/reader.rs` already has both a transitional gzip backend and a
  native BGZF backend. Current record-scanning consumers generally open the
  transitional `BamReader::open` path after BGZF probing; the M3 scanner should
  make native BGZF the record-iteration substrate.
* `src/bam/tags.rs` already contains bounded aux traversal, tag lookup, string
  extraction, filtered serialization, and malformed-aux reporting. These
  helpers operate on a materialized aux byte vector today and should be adapted
  to scanner-owned aux slices or ranges.
* Current record consumers divide into three migration classes:
  field-only readers (`check_sort`, `check_map`, `summary`), aux lookup readers
  (`check_tag`, parts of `validate` and `forensic_inspect`), and rich or raw
  record consumers (`subsample`, `explode`, `checksum`, `sort`, `merge`,
  `annotate_rg`, `unmap`, BAM FASTQ export, duplication/forensic identity
  scans).

The first consumer migration order should be:

1. `check_sort`, because it needs only encounter order, coordinates, read name,
   flags, and bounded sampling behavior.
2. `check_map` and `summary`, because they need the same lightweight flag,
   coordinate, MAPQ, and reference-id fields, while preserving index-preferred
   behavior where present.
3. `check_tag`, because it should exercise scanner-owned aux-region bounds and
   selected tag traversal without requiring full rich decode.
4. `validate`, `inspect_duplication`, and `forensic_inspect`, because they mix
   lightweight structural checks with aux traversal and, in selected modes,
   sequence and quality decoding.
5. BAM-side `subsample` and other raw-record writers after the scanner exposes
   raw record bytes or a lossless bridge back to `RecordLayout`, because these
   paths need deterministic identity bytes, eligibility filters, pass-through
   serialization, or record mutation.

## Benchmark Hooks

* records-per-second BAM scanner microbenchmark
* selective field extraction microbenchmark
* compare scanner throughput against the earlier implementation
* rerun `summary`, `check_sort`, and `check_map` timing after adoption

## Risks / Follow-Up

* scanner API must avoid becoming a new generic abstraction tax
* aux traversal and sequence access need careful bounds and allocation control
