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

## Record View Contract

M3.2 added `src/bam/record.rs` as the native lightweight record-view contract.
`BamRecordView<'a>` borrows one complete length-prefixed BAM alignment record
and exposes:

* raw record bytes;
* `refID`, `pos`, flags, MAPQ, read name, sequence length, mate fields, bin,
  and template length;
* stable ranges for the raw record, core section, read name, CIGAR bytes,
  sequence bytes, quality bytes, and aux bytes;
* borrowed section slices for consumers that need selected fields;
* `to_record_layout` as the explicit bridge to the existing owned
  `RecordLayout` materialization path.

## Native Scan Loop

M3.3 added `src/bam/scan.rs` with `BamScanner`. The scanner opens BAM inputs
through the native BGZF backend, parses the native BAM header once, and then
iterates complete raw alignment records into `BamRecordView`.

The scanner treats clean EOF as `Ok(None)`. It reports negative block sizes,
block sizes smaller than the BAM core, truncated block-size prefixes,
truncated record payloads, and record-view parse failures as structured
`AppError` values that preserve input path context.

This loop is the scanner substrate, not command migration. M3.4 and M3.5 still
need to centralize selective field helpers and scanner-owned aux traversal.
M3.6 through M3.8 still need to move command consumers from their current
record loops onto `BamScanner`.

## Selective Field Helpers

M3.4 added the scanner-facing helper surface on `BamRecordView`:

* `BamRecordFlags` and `flag_summary` centralize flag decoding and primary
  record classification;
* `BamRecordCoordinates` and `coordinates` centralize reference, position,
  mate, bin, and template-length fields;
* `mapping_quality`, `read_name`, and `sequence_len` expose common scalar
  fields directly;
* section range helpers and borrowed section slices expose CIGAR, sequence,
  quality, and aux regions without owned allocation;
* section presence helpers and `BamRecordSkipOffsets` identify the byte offsets
  a consumer can advance to when it does not need a variable section.

These helpers are the stable scanner API for common field access. Parser-local
offset arithmetic and the owned `RecordLayout` materialization bridge remain
implementation details for scanner construction or richer downstream
consumers.

## Scanner-Owned Aux Traversal

M3.5 added record-view aux helpers in `src/bam/tags.rs`. Scanner consumers can
now traverse `BamRecordView::aux_bytes`, test selected tag presence, count
matching tags, collect tag keys, and extract string tags without materializing a
full `RecordLayout`.

The helpers reuse the existing bounded aux parser, including type skipping for
scalar values, NUL-terminated strings, hex strings, and B-arrays. Malformed aux
payloads continue to fail with precise errors for truncated fields,
unterminated strings, unsupported type codes, unsupported B-array subtypes,
negative B-array counts, and array payload length overflows.

This completes the scanner aux helper substrate. `check_tag` now consumes these
helpers through `BamScanner`; read-group evidence, validation, and forensics
still need to be migrated in later Milestone 3 tasks.

## First Command Migration

M3.6 migrated `check_sort` onto `BamScanner`. The command now opens BAM input
through the native BGZF/header scanner path, builds its sort-only comparison
snapshot from `BamRecordView` and scanner-owned flag helpers, and preserves the
existing bounded scan, strict scan, specialized-sort, JSON payload, and semantic
note behavior.

M3.7 migrated `check_map`, `summary`, and `check_tag` onto `BamScanner`.
`check_map` still prefers usable index-derived evidence and uses the scanner
for scan fallback. `summary` now scans bounded and full record windows through
scanner-owned record views. `check_tag` now performs aux lookup through
record-view aux helpers. These migrations preserve existing JSON payloads and
diagnostic semantics.

M3.8 migrated validation and forensic first-slice body scans onto the scanner.
`validate` now uses `BamRecordView` fields and record-view aux traversal for
scanner-compatible structural checks. `inspect_duplication` and
`forensic_inspect` now use `BamScanner` plus borrowed record sections for
read-name, sequence, quality, RG, and aux-tag evidence. Remaining
writer-heavy transform paths and behavior that requires owned whole-record
serialization continue to use `RecordLayout` until later tasks explicitly move
them.

M3.9 added native scanner malformed-record tests, scanner differential coverage
against Bamana's owned `RecordLayout` bridge, explicit dependency-boundary
coverage for scanner and migrated record hot paths, and `scanner_microbench`.
The benchmark generates deterministic local fixtures and reports
records-per-second plus selective field-extraction throughput as JSON conforming
to `benchmarks/results/scanner_microbench.schema.json`.

## Benchmark Hooks

* `scanner_microbench --profile small --iterations 1`
* records-per-second BAM scanner microbenchmark
* selective field extraction microbenchmark
* compare scanner throughput against the earlier implementation
* rerun `summary`, `check_sort`, and `check_map` timing after adoption

## Risks / Follow-Up

* scanner API must avoid becoming a new generic abstraction tax
* aux traversal and sequence access need careful bounds and allocation control
