# Milestone 9: Native BAM Index And Random Access

Status: complete as of 2026-05-23. M9 was activated by M9.1 only after
Milestone 8 closed and recorded transform, checksum, explode, and ingest output
safety. It closed after M9.1 through M9.10 completed.

## Technical Goal

Implement native BAM index ownership and random-access groundwork above the
native BGZF, header, and scanner substrates.

Primary command and subsystem targets:

* `index`
* `check_index`
* BAI writing
* CSI planning or scoped CSI support
* virtual-offset-backed random-access reader primitives
* index-aware command acceleration where appropriate

## Owned Modules

Primary ownership:

* `src/bam/index.rs`
* `src/bgzf/virtual_offset.rs`
* `src/bgzf/reader.rs`
* `src/bam/scan.rs`
* `src/bam/header.rs`
* command orchestration in `src/commands/index.rs` and
  `src/commands/check_index.rs`

Supporting consumers:

* `check_map`
* `summary`
* future indexed region selection commands

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF and virtual-offset groundwork
* Milestone 2 native BAM header codec
* Milestone 3 native BAM record scanner
* Milestone 6 native inspection and validation command hardening
* Milestone 8 transform and ingest output discipline

## Commands Enabled Or Hardened

Primary beneficiaries:

* `index`
* `check_index`
* `check_map`
* `summary`

## M9.1 Baseline Audit

Present implementation pieces:

* `src/bam/index.rs` detects BAI, CSI, GZI, and unknown sidecar magic,
  discovers adjacent `.bam.bai`, `.bai`, `.bam.csi`, and `.csi` candidates
  with CSI preference support, validates BAI reference counts, bin/chunk
  shape, linear-index ordering, metadata pseudo-bin summaries, parses CSI
  headers enough to report detected-but-not-supported status, and can build
  native in-memory BAI data structures from scanner-owned record traversal and
  serialize native BAI sidecars.
* `src/bgzf/virtual_offset.rs` owns a `VirtualOffset` type with packed
  construction, bounds checks, ordering, and tests.
* `src/bgzf/reader.rs` owns sequential native BGZF member inflation, EOF-marker
  checks, BAM-magic probing, and virtual-offset reporting based on compressed
  member starts plus uncompressed in-block offsets. It can seek to typed
  virtual offsets by loading the referenced BGZF member and positioning the
  in-block cursor.
* `src/bam/scan.rs` owns sequential scanner traversal over native BGZF and BAM
  header parsing, while reporting records read and positioned record start/end
  virtual offsets for native scans. It also exposes internal raw-record range
  retrieval over typed virtual-offset bounds.
* `src/commands/index.rs` validates BAM plausibility and output path behavior,
  creates native BAI sidecars for coordinate-sorted BAM inputs, creates real
  FASTQ.GZI sidecars for FASTQ.GZ inputs, and reports CSI writing as
  unimplemented rather than pretending an index was created.
* `src/commands/check_index.rs` reports adjacent index discovery, selected
  index kind, BAI structural validity, CSI header status, timestamp-based
  staleness, compatibility, and apparent usability.
* `check_map` and `summary` already distinguish BAI metadata-derived evidence
  from scan-derived evidence. They use BAI metadata only when the selected
  sidecar is non-stale, structurally valid under the implemented checks, and
  complete for mapped/unmapped reference metadata; absent, stale, unsupported,
  malformed, or incomplete sidecars fall back to native scanning.

Deferred beyond M9:

* BAM `index` cannot yet write real CSI output.
* public commands do not yet exercise random-access chunk traversal for
  acceleration or region filtering.
* CSI support remains header-only detection until a later scoped parser/writer
  contract or long-term deferral is recorded.
* `check_map` and `summary` do not yet use validated chunks for indexed
  acceleration; their index path is limited to parsed BAI metadata.
* M9 dependency-boundary and benchmark evidence names BAM index writing, BAM
  index validation, virtual-offset random-access groundwork, and first
  index-aware consumers as one protected set.

## Acceptance Criteria

* BAM `index` can create a real BAI for supported coordinate-sorted BAM inputs
* `check_index` validates BAI structure beyond shallow magic and reference
  counts where feasible
* virtual offsets are captured from native BGZF reads and used by index-writing
  logic
* index-aware consumers distinguish index-derived evidence from scan-derived
  evidence
* CSI support is either implemented to a scoped contract or explicitly remains
  detected-but-not-supported with precise reasons
* command JSON contracts remain stable or are deliberately versioned

## M9.3 Virtual-Offset Plumbing

M9.3 exposes virtual offsets without changing command JSON behavior. The native
BGZF reader now tracks each compressed member's file start and end offsets and
combines them with the current uncompressed in-block cursor through the
`VirtualOffset` type. The native BAM scanner exposes
`next_record_with_virtual_offsets`, returning each parsed alignment record with
start and end virtual offsets.

These offsets are the substrate for later BAI/CSI work:

* BAI chunks will use record start/end virtual offsets to describe candidate
  compressed spans for mapped records;
* BAI linear-index windows will use record start virtual offsets for the
  earliest record overlapping each window;
* CSI support, if promoted, will consume the same typed offsets rather than raw
  integers;
* offsets at the end of a BGZF member are normalized to the next compressed
  block start with in-block offset zero, preserving BGZF virtual-offset
  packing semantics.

## M9.4 Native BAI Builder

M9.4 adds native in-memory BAI construction without changing the public
`index` command behavior. `build_bai_index_from_bam` scans records through
`BamScanner::next_record_with_virtual_offsets`, validates coordinate order for
mapped records, computes BAI bins from reference-consuming CIGAR span, merges
adjacent or overlapping chunks per reference/bin, populates 16kb linear-index
windows, and accounts for mapped, reference-associated unmapped, and unplaced
unmapped reads.

M9.5 now serializes this builder output into BAI bytes, emits metadata
pseudo-bins, and makes `bamana index` report BAM sidecar creation only after
the final output path is published.

## M9.7 Random-Access Groundwork

M9.7 adds typed virtual-offset seek and chunk traversal helpers without changing
public command behavior. `NativeBgzfReader::seek_virtual_offset` consumes a
`VirtualOffset`, seeks to the compressed member, inflates it, and positions the
reader at the requested uncompressed in-block offset. `BamScanner` exposes
`seek_virtual_offset` and `raw_records_in_virtual_range` so internal consumers
can read positioned raw BAM records from validated BAI chunk ranges.

Tests prove valid virtual-offset seeks, invalid in-block offsets, manual
virtual-offset range traversal, BAI chunk traversal, and empty-range rejection.

M9.8 routes first consumer evidence through the hardened index usability rules.
`check_map` and `summary` now test generated BAI sidecars produced by Bamana's
writer, reject timestamp-stale sidecars for index evidence, and keep scan
fallbacks native and explicit in payload notes. This remains metadata evidence,
not random-access chunk acceleration.

## Benchmark Hooks

* command-level smoke timings for BAM `index`
* command-level smoke timings for `check_index`
* indexed versus scan fallback timings for `check_map`
* indexed versus scan fallback timings for `summary`

M9.9 formalizes these hooks in `scanner_microbench --bamana-bin` as
`index_bam`, `check_index`, `check_map_indexed`, and `summary_indexed` timing
rows. The existing `check_map` and `summary` rows remain scan-fallback
timings because they run before the synthetic BAI sidecar is created. The M9
interpretation notes distinguish BAM index construction, BAI structural
validation, index metadata-backed consumer evidence, scan fallback timings,
random-access lookup deferral, process startup, JSON emission, and comparator
non-parity.

## M9.10 Closeout

Milestone 9 completed the native BAM index and random-access groundwork slice.
The closeout records:

* native BAI sidecar creation for supported coordinate-sorted BAM inputs;
* hardened `check_index` BAI/CSI validation to the implemented structural
  depth;
* typed virtual-offset capture and internal BGZF/BAM random-access helpers;
* first index-aware `check_map` and `summary` consumer evidence using validated
  BAI metadata with native scan fallback;
* M9 dependency-boundary protection for index writing, index validation,
  indexed consumers, and the random-access substrate;
* M9 command smoke benchmark rows for `index_bam`, `check_index`,
  `check_map_indexed`, and `summary_indexed`;
* full tests, contract tests, Sphinx, and M9 benchmark smoke checks passing.

CSI writing, public indexed-region command acceleration, and broad
random-access APIs remain deferred beyond M9.

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

Disallowed:

* production BAM index writing, validation, or random-access hot paths through
  `noodles`

## Risks / Follow-Up

* BAI is coordinate-indexed and must reject or caveat unsuitable sort orders
* CSI may be required for large references and should not be faked
* random access must be built on correct BGZF virtual offsets, not byte offsets
