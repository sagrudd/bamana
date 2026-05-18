# Current Milestone

## Milestone Status

**Milestone 3: Native BAM Record Scanner** is complete as of 2026-05-18.

See:

* [milestone-03-bam-record-scan.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-03-bam-record-scan.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Why This Is Current

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. Milestone 3
completed the next dependency layer: selective native BAM record scanning that
iterates alignment records, exposes lightweight field views, safely skips
unneeded variable sections, and supports selected aux-tag traversal without
full generic decode.

This milestone is intentionally smaller than full BAM semantic validation,
BAI/CSI random access, native CRAM scanning, or broad command parity. It makes
the first record-scanning command paths depend on Bamana-native BGZF, native
BAM headers, and a shared native scanner without implying that every BAM
operation has migrated.

## Previously Completed Milestones

**Milestone 1: Native BGZF Core**

See:

* [milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)

The M1 closeout established:

* native BGZF block reading and EOF-marker handling;
* BAM-compatible native BGZF writing;
* virtual-offset groundwork for later random-access work;
* BGZF microbenchmark hooks;
* dependency guardrails that keep production `noodles` usage isolated to CRAM
  compatibility.

**Milestone 2: Native BAM Header Codec**

See:

* [milestone-02-bam-header.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-02-bam-header.md)

The M2 closeout established:

* native BAM magic, `l_text`, SAM-style textual header, and binary reference
  dictionary parsing;
* deterministic BAM header serialization helpers;
* production `verify` and `header` paths backed by native BGZF plus native BAM
  header parsing;
* malformed-header tests and test-only header oracle boundaries;
* header microbenchmark hooks with machine-readable output;
* dependency-boundary checks protecting production native header code from
  direct `noodles` imports.

Milestone 1 and 2 completion evidence remains recorded in:

* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Milestone 3 Completed State

Known present pieces:

* native BGZF reading can feed BAM payload bytes;
* native BAM header parsing can position readers at the first alignment record;
* `src/bam/record.rs` defines `BamRecordView`, a borrowed lightweight record
  view over one complete length-prefixed BAM record;
* `BamRecordView` exposes core fields, sequence length, raw record bytes, and
  stable ranges for core, read name, CIGAR, sequence, qualities, and aux
  regions without allocating skipped sections;
* `BamRecordView::to_record_layout` preserves the bridge back to the existing
  owned `RecordLayout` type for richer consumers that still need
  materialization or lossless serialization;
* `src/bam/scan.rs` defines `BamScanner`, which opens BAM input through the
  native BGZF backend, parses the native BAM header once, and iterates complete
  raw alignment records into `BamRecordView`;
* `BamRecordView` centralizes selective helpers for flags, coordinates, MAPQ,
  read name, sequence length, section ranges, borrowed section slices, section
  presence, and skip offsets;
* `src/bam/tags.rs` provides record-view aux helpers for bounded traversal,
  selected tag lookup, tag counting, tag-key collection, and string tag
  extraction over `BamRecordView::aux_bytes`;
* production `check_sort` uses `BamScanner` and scanner-owned field helpers for
  record traversal while preserving its existing bounded and strict scan
  behavior;
* production `check_map` preserves its index-preferred behavior and uses
  `BamScanner` for scan fallback record traversal;
* production `summary` uses `BamScanner` for bounded and full record scans and
  observes scanner-owned record views directly;
* production `check_tag` uses `BamScanner` plus record-view aux helpers for
  selected tag lookup;
* production `validate` uses `BamScanner` and `BamRecordView` for record-level
  structural checks that fit the lightweight view;
* `inspect_duplication` uses `BamScanner`, borrowed sequence/quality sections,
  and record-view RG extraction for BAM body scans;
* `forensic_inspect` uses `BamScanner` for BAM body evidence across read-group,
  read-name regime, aux-tag regime, and duplication-hallmark checks;
* native scanner malformed-record tests cover scanner-owned expected failures
  without external parser dependency;
* dependency-boundary tests explicitly protect the scanner substrate and
  migrated record hot paths from direct `noodles` imports;
* `scanner_microbench` provides records-per-second and selective
  field-extraction microbenchmarks with machine-readable JSON output;
* `src/bam/records.rs` contains the current central record bridge through
  `read_next_record_layout`, which performs bounded layout checks and
  materializes read name, CIGAR, sequence, quality, and aux sections;
* `LightAlignmentRecord` already exposes field-only record information for
  several commands, but it is derived from the fully materialized
  `RecordLayout` path rather than a true selective scanner;
* `src/bam/tags.rs` contains bounded aux traversal and tag lookup over
  materialized aux bytes;
* remaining record-facing transform paths include BAM-side `subsample` and
  writer-heavy workflows that still need owned record materialization;
* dependency-boundary tests already prohibit production `noodles` usage outside
  the CRAM compatibility exception.

Known gaps:

* remaining BAM-side transform consumers have not yet been migrated onto a
  shared scanner where command behavior needs owned records or writer-heavy
  serialization;
* richer decode and lossless serialization paths still use `RecordLayout` where
  command behavior needs owned sequence, quality, aux, or whole-record bytes.

Completed evidence:

* `cargo test` passed with 180 library tests, 18 contract tests, 2
  header-oracle integration tests, binary tests, and doc tests;
* `cargo test --test contract` passed with 18 contract tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed;
* `cargo run --bin scanner_microbench -- --profile small --iterations 1`
  passed, and the JSON smoke check verified the small profile, one iteration,
  1,024 generated records, and the expected result schema.

First consumer order:

1. `check_sort` complete;
2. `check_map`, `summary`, and `check_tag` complete;
3. `validate`, `inspect_duplication`, and `forensic_inspect` complete;
4. scanner malformed-record tests, dependency boundaries, and microbenchmarks
   complete;
5. BAM-side `subsample` and other raw-record writers after scanner-owned raw
   record access or lossless `RecordLayout` bridging is available.

## Completion Boundary

Milestone 3 completion means:

* BAM alignment records can be iterated natively without a full generic decode;
* lightweight record views expose at least `refID`, `pos`, flags, MAPQ, read
  name, sequence length, and aux-region boundaries;
* scanner helpers can skip CIGAR, sequence, quality, and aux payloads when a
  consumer does not need them;
* selected aux tags can be traversed safely without rich materialization;
* selected first command consumers use the shared scanner for record traversal;
* scanner microbenchmark hooks are runnable and documented;
* production BAM record hot-path scanning does not depend on `noodles`.

## Command-Surface Boundary

Milestone 3 evidence is limited to native BAM record scanning and the selected
command paths that consume scanner-owned record views.

Commands such as `check_sort`, `check_map`, `summary`, `check_tag`,
`validate`, `inspect_duplication`, `forensic_inspect`, and BAM-side
`subsample` are beneficiaries, but each command's broader semantics remain
bounded by its existing public contract unless an explicit M3 task updates that
contract.

## What Should Not Happen

Do not treat Milestone 3 as full BAM semantic validation, BAI/CSI random
access, native CRAM scanning, broad command parity, biological interpretation,
or wholesale replacement of every rich record conversion path. Those remain
later milestones or downstream command work unless an explicit M3 task includes
them.
