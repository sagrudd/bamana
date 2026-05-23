# Milestone 9: Native BAM Index And Random Access

Status: active as of 2026-05-23. M9 was activated by M9.1 only after
Milestone 8 closed and recorded transform, checksum, explode, and ingest output
safety.

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
  with CSI preference support, parses shallow BAI reference-count and
  pseudo-bin metadata summaries, and parses CSI headers enough to report
  detected-but-not-supported status.
* `src/bgzf/virtual_offset.rs` owns a `VirtualOffset` type with packed
  construction, bounds checks, ordering, and tests.
* `src/bgzf/reader.rs` owns sequential native BGZF member inflation, EOF-marker
  checks, and BAM-magic probing, but currently keeps compressed block positions
  internal.
* `src/bam/scan.rs` owns sequential scanner traversal over native BGZF and BAM
  header parsing, while reporting records read.
* `src/commands/index.rs` validates BAM plausibility and output path behavior,
  creates real FASTQ.GZI sidecars for FASTQ.GZ inputs, and reports BAM BAI/CSI
  writing as unimplemented rather than pretending an index was created.
* `src/commands/check_index.rs` reports adjacent index discovery, selected
  index kind, shallow BAI/CSI syntax status, timestamp-based staleness, and
  compatibility.
* `check_map` and `summary` already distinguish BAI metadata-derived evidence
  from scan-derived evidence and fall back to scanning when index metadata is
  absent or insufficient.

Outstanding M9 gaps:

* BAM `index` cannot yet write real BAI or CSI output.
* Native BGZF and BAM scanner paths do not yet expose stable virtual offsets
  for each alignment record.
* BAI binning, chunk coalescing, metadata pseudo-bin emission, linear-index
  construction, and unplaced-unmapped accounting remain unimplemented.
* `check_index` does not yet validate BAI chunks, virtual-offset ordering,
  linear-index monotonicity, reference span plausibility, or random-access
  usability.
* CSI support remains header-only detection until M9 either implements a scoped
  parser/writer contract or records precise deferral.
* `check_map` and `summary` do not yet use validated chunks for indexed
  acceleration; their index path is limited to parsed BAI metadata.
* M9 dependency-boundary and benchmark evidence still need to name BAM index
  writing, BAM index validation, virtual-offset random-access groundwork, and
  first index-aware consumers as one protected set.

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

## Benchmark Hooks

* command-level smoke timings for BAM `index`
* command-level smoke timings for `check_index`
* indexed versus scan fallback timings for `check_map`
* indexed versus scan fallback timings for `summary`

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
