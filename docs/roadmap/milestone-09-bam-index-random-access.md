# Milestone 9: Native BAM Index And Random Access

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
