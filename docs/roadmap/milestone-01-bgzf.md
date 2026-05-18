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

## Command-Surface Boundary

Milestone 1 is not a command-completion milestone. It closes only when the
native BGZF substrate is owned, tested, benchmarkable, and dependency-audited.

Milestone 1 evidence:

* `check_eof` exercises native EOF-marker handling
* `verify` exercises native first-member inflation and BAM magic detection
* BAM-compatible writer paths exercise native BGZF output
* `bgzf_microbench` captures native read, write, EOF, `verify`, and
  `check_eof` timings

Downstream first slices already present in the repository, including public
contract commands such as `benchmark`, `fastq`, and `unmap`, are not themselves
proof that Milestone 1 is complete. Broader command semantics belong to later
milestones and command-migration waves.

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

Reader coverage now separates BGZF container/member handling, EOF-marker
presence, first-member BAM magic, and first-member payload validity. Truncated
headers, truncated declared payloads, and invalid gzip footer metadata retain
specific error detail for JSON command failures.

Writer coverage now verifies empty streams, deterministic EOF marker emission,
single-payload round trips, multi-block payloads, and payloads near the writer
target block boundary through the native BGZF reader.

## Benchmark Hooks

* `bgzf_microbench --profile small|medium|large` measures native BGZF read
  throughput, native BGZF write throughput, and EOF-check latency using
  deterministic generated fixtures
* `bgzf_microbench --bamana-bin <path>` also captures `verify` and `check_eof`
  command timings against the generated BAM-like BGZF fixture
* JSON output is governed by
  `benchmarks/results/bgzf_microbench.schema.json` so local and CI runs can
  archive comparable artifacts

## Risks / Follow-Up

* BAI/CSI parsing and future random-access readers should consume
  `bgzf::VirtualOffset` instead of passing packed `u64` values through new BGZF
  APIs
* block reuse and buffering strategy will matter for later throughput work

## Deferred Beyond Milestone 1

* full BAM header ownership belongs to Milestone 2
* native BAM record scanning belongs to Milestone 3
* full BAI/CSI random-access behavior remains later index work
* broad command migration off transitional compatibility surfaces belongs to
  Milestone 5 and later waves
