# Current Milestone

## Milestone Status

**Milestone 4: Native FASTQ / FASTQ.GZ Parser** is active as of 2026-05-18.

See:

* [milestone-04-fastq.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-04-fastq.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Why This Is Current

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. Milestone 3
completed selective native BAM record scanning and migrated the first
scanner-compatible BAM command consumers.

Milestone 4 moves the native-core sequence to FASTQ and FASTQ.GZ. The goal is
to turn the existing useful FASTQ helpers into an explicit parser/writer core
with clear module ownership, robust plain FASTQ and FASTQ.GZ validation,
stable record contracts, command-consumer migration evidence, and benchmark
hooks.

This milestone is intentionally smaller than broad ingest parity, paired-read
reconciliation, adapter trimming, biological quality interpretation, or full
comparator parity. It should make FASTQ-side command paths depend on
Bamana-native FASTQ primitives without implying that every raw-read workflow is
complete.

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

**Milestone 3: Native BAM Record Scanner**

See:

* [milestone-03-bam-record-scan.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-03-bam-record-scan.md)

The M3 closeout established:

* `BamRecordView` as the borrowed native BAM record-view contract;
* `BamScanner` as the native BGZF/header-backed BAM record iteration substrate;
* scanner-owned helpers for flags, coordinates, MAPQ, read names, sequence
  length, section ranges, skip offsets, borrowed section slices, and selected
  aux traversal;
* scanner-backed `check_sort`, `check_map`, `summary`, `check_tag`,
  `validate`, `inspect_duplication`, and `forensic_inspect` paths;
* scanner malformed-record tests, dependency-boundary protection, and
  `scanner_microbench` hooks.

Milestone 1 through 3 completion evidence remains recorded in:

* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Milestone 4 Current State

Known present pieces:

* `src/fastq/mod.rs` is the current native FASTQ parser/writer facade and
  preserves compatibility imports for existing command consumers;
* `src/fastq/record.rs` owns the `FastqRecord` data model,
  `FastqRecordView`, read-name parsing, record-line validation, plus-line
  preservation, field accessors, and FASTQ identity-byte construction;
* `src/fastq/reader.rs` owns plain/gzip reader opening, record parsing, record
  validation, and record counting;
* `src/fastq/writer.rs` owns plain/gzip FASTQ writing and finish behavior;
* `src/fastq/gzip.rs` owns extension-based gzip detection, `MultiGzDecoder`
  reader construction, and shared FASTQ thread-count resolution;
* `src/fastq/unmapped.rs` owns unmapped-BAM conversion,
  threaded FASTQ.GZ-to-BAM conversion, and selected HTS-style methylation
  header tag conversion;
* `src/fastq/gzi.rs` owns the `FASTQ.GZI` sidecar builder, reader, checkpoint
  sampler, and explode range planner;
* `src/ingest/fastq.rs` is a compatibility shim that re-exports
  `crate::fastq`;
* the current reader validates the four-line FASTQ structure, header marker,
  plus marker, sequence/quality length equality, and usable read name;
* the current gzip reader uses extension-based `.gz` detection and
  `flate2::read::MultiGzDecoder`, including concatenated gzip-member support
  and structured I/O errors for corrupt or truncated gzip streams;
* the current writer emits plain FASTQ or gzip-compressed FASTQ according to
  the output extension, preserves record header/plus-line content, emits LF
  line endings, and finalizes gzip output before returning success;
* `enumerate` uses the stable FASTQ count facade for plain FASTQ and remains
  `FASTQ.GZI`-aware for FASTQ.GZ;
* FASTQ-side `subsample` uses the stable FASTQ reader, record identity, gzip
  extension policy, and writer facade while preserving existing JSON payloads;
* `consume`, `inspect_duplication`, `deduplicate`, and FASTQ.GZ `explode`
  already consume native FASTQ helpers in some form.

Known gaps:

* the current streaming reader still returns owned `FastqRecord` values, so
  field-only streaming integration remains future work even though
  `FastqRecordView` exists for borrowed access when line storage is already
  available;
* remaining command consumers need to be audited and migrated to a stable
  Milestone 4 parser/writer API;
* FASTQ parser/writer benchmark smoke evidence has not yet been recorded.

First consumer order:

1. module split and stable record/reader/writer APIs;
2. plain FASTQ validation and FASTQ.GZ stream semantics;
3. writer round-trip guarantees;
4. FASTQ-side `subsample` and `enumerate`;
5. `consume`, `inspect_duplication`, `deduplicate`, and FASTQ.GZ `explode`;
6. FASTQ oracle, dependency-boundary, and microbenchmark closeout evidence.

## Completion Boundary

Milestone 4 completion will mean:

* plain FASTQ parsing is native, tested, and documented;
* FASTQ.GZ parsing is native, tested, and documented;
* FASTQ record validation covers four-line structure, header and plus markers,
  read-name parsing, and sequence/quality length equality;
* valid FASTQ and FASTQ.GZ writing is supported with round-trip tests;
* `FASTQ.GZI` sidecar behavior remains integrated with enumeration and
  shard-planning consumers;
* selected FASTQ command consumers use the stable native parser/writer APIs;
* FASTQ microbenchmark hooks are runnable and documented;
* production FASTQ hot paths do not depend on external generic bioinformatics
  parser crates.

## Command-Surface Boundary

Milestone 4 evidence is limited to native FASTQ and FASTQ.GZ parsing, writing,
record validation, sidecar-aware enumeration/planning, and selected command
consumers that use those primitives.

Commands such as FASTQ-side `subsample`, `consume`, `inspect_duplication`,
`deduplicate`, `enumerate`, and FASTQ.GZ `explode` are beneficiaries, but each
command's broader semantics remain bounded by its existing public contract
unless an explicit M4 task updates that contract.

## What Should Not Happen

Do not treat Milestone 4 as broad ingest parity, paired-read reconciliation,
adapter trimming, biological quality interpretation, native CRAM scanning,
full comparator parity, or wholesale replacement of every command-specific
FASTQ behavior. Those remain later milestones or downstream command work unless
an explicit M4 task includes them.
