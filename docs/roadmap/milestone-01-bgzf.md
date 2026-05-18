# Milestone 1: Native BGZF Core

## Technical Goal

Implement Bamana-native BGZF reading and writing primitives sufficient for:

* block reading
* EOF marker handling
* future virtual-offset support
* BAM-compatible output writing foundation

## Owned Modules

Primary ownership:

* `src/bgzf/mod.rs`
* `src/bgzf/reader.rs`
* `src/bgzf/writer.rs`
* `src/bgzf/block.rs`
* `src/bgzf/virtual_offset.rs`

Supporting consumers:

* `src/bam/reader.rs`
* `src/bam/write.rs`

## Dependencies / Prerequisites

None. This is the substrate milestone.

## Commands Enabled Or Benefited

Immediate:

* `check_eof`
* `verify`

Foundation for:

* BAM header parsing
* BAM record scanning
* BAM writing

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, fixtures, oracles

Disallowed:

* any production BGZF hot path implemented through `noodles`

## Acceptance Criteria

* canonical BGZF EOF marker detection works in native code
* BGZF block reading works in native code
* Bamana can stream decompressed bytes from the start of a BAM
* Bamana can write valid BGZF output suitable for BAM-compatible payloads
* tests exist for EOF detection and BGZF block parsing
* no production BGZF hot path depends on `noodles`

## Current Module Boundary

The native BGZF substrate is split by responsibility:

* `block` owns BGZF constants, gzip/BGZF header recognition, block-size
  extraction, and native member construction helpers
* `reader` owns EOF-marker checks and first-member inflation used by shallow
  BAM verification
* `writer` owns `BgzfWriter` and native BGZF member emission for BAM-compatible
  output streams
* `virtual_offset` owns the typed BGZF virtual-offset representation used by
  later BAI/CSI and random-access work

`src/bgzf/mod.rs` remains a narrow facade. Existing BAM writer call sites may
continue to import `BgzfWriter` through `bam::write` while the implementation
itself is owned by `src/bgzf/writer.rs`.

## Benchmark Hooks

* BGZF read throughput microbenchmark
* BGZF write throughput microbenchmark
* EOF-check latency microbenchmark
* rerun `verify` and `check_eof` command timings after integration

## Risks / Follow-Up

* BAI/CSI parsing and future random-access readers should consume
  `bgzf::VirtualOffset` instead of passing packed `u64` values through new BGZF
  APIs
* block reuse and buffering strategy will matter for later throughput work
