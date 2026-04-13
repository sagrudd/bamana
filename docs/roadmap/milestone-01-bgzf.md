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
* future `src/bgzf/reader.rs`
* future `src/bgzf/writer.rs`
* future `src/bgzf/block.rs`
* future `src/bgzf/virtual_offset.rs`

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

## Benchmark Hooks

* BGZF read throughput microbenchmark
* BGZF write throughput microbenchmark
* EOF-check latency microbenchmark
* rerun `verify` and `check_eof` command timings after integration

## Risks / Follow-Up

* virtual offset support may need a dedicated type once indexed access expands
* block reuse and buffering strategy will matter for later throughput work
