# Noodles Demotion Plan

## Purpose

This document defines what "demotion" means for `noodles` in Bamana.

Demotion does not necessarily mean immediate repository removal. It means
`noodles` is no longer allowed to define the primary engine for hot-path
execution.

## Current Known Production Usage

Current direct `noodles` usage is concentrated in:

* `src/ingest/cram.rs`

That code supports conservative CRAM ingestion and reference-policy handling.

## Acceptable Temporary Retention

The current CRAM compatibility slice may remain temporarily because:

* CRAM is not the primary target of this architecture reset
* the project is focusing first on BAM, BGZF, and FASTQ ownership
* conservative compatibility is preferable to pretending native CRAM support
  exists already

## Hot-Path Usages That Must Not Return

The following usages must not be introduced or reintroduced:

* production BAM reader or writer built on `noodles-bam`
* production BGZF hot loops delegated to external format crates
* FASTQ hot loops built around external parser abstractions
* sort, merge, sampling, duplication, or provenance transforms driven by
  `noodles` record iteration

## Milestone-Driven Replacement Plan

The canonical roadmap now lives in:

* [../roadmap.md](/Users/stephen/Projects/bamana/docs/roadmap.md)

The migration sequence is intentionally engine-first:

1. native BGZF
2. native BAM header codec
3. native BAM record scanner
4. native FASTQ / FASTQ.GZ parser
5. command migration beginning with `verify`, `header`, and `subsample`

This order is retained because command migration without native substrate
ownership would only move architectural ambiguity around rather than remove it.

## Phased Replacement Plan

### Phase 1

* establish Bamana-native top-level `bgzf` and `fastq` modules
* make the crate architecture explicit through `src/lib.rs`
* convert old paths into compatibility shims where needed
* document hot-path ownership and dependency policy
* establish the roadmap and milestone acceptance template

### Phase 2

* strengthen BGZF reader and writer ownership
* harden BAM header parsing and serialization
* expand selective BAM record scanning primitives
* consolidate BAM write paths around Bamana-native code only

### Phase 3

* optimize `subsample`, `summary`, `check_sort`, `check_map`, and checksum hot
  loops around native scan primitives
* continue moving transform logic toward shared native reader/writer kernels

### Phase 4

* complete native ownership for sort, merge, explode, duplication, and
  provenance transform families
* reduce compatibility shims further

### Phase 5

* re-evaluate CRAM compatibility design
* decide whether CRAM remains explicitly compatibility-oriented or receives a
  separate Bamana-native staged implementation

M13.2 records that CRAM remains compatibility-only for Milestone 13. That
decision does not make `noodles` a general production dependency: direct
production `noodles_*` imports remain confined to `src/ingest/cram.rs`, and any
future native CRAM work must arrive as a separate staged implementation plan
with fixtures, contracts, dependency guardrails, and benchmark evidence.

## Priority Order

Recommended migration priority:

1. BGZF reader and writer ownership
2. BAM header parser and serializer
3. BAM record scanner and selective decoder
4. FASTQ and FASTQ.GZ parser and writer
5. hot-loop scan commands
6. transform engines
7. metadata rewrite commands
8. remaining compatibility fallbacks

## Dependency Checkpoints

### Checkpoint A

`noodles` remains available only in:

* CRAM compatibility
* tests
* fixture tooling
* differential or oracle checks

### Checkpoint B

For each migrated command family, `noodles` is absent from the production code
path and any old dependency is isolated behind an explicit shim or removed.

### Checkpoint C

Once enough command families are migrated, `noodles` should be further
feature-gated, made optional, or moved closer to dev/test-only roles, while
preserving any still-needed CRAM compatibility boundary.

## Transitional Code Rule

Any remaining production `noodles` usage must:

* live in a clearly labeled compatibility boundary
* carry a comment that the usage must not expand
* be documented here
* pass `tests/contract/dependency_boundary.rs`, which currently allows direct
  production `noodles_*` references only in `src/ingest/cram.rs`

## Removal criteria

The `src/ingest/cram.rs` exception is temporary. It can be removed, narrowed,
or redesigned when:

* native CRAM support is staged as its own implementation effort;
* CRAM compatibility is moved behind a narrower optional feature boundary;
* CRAM ingestion is no longer included in the native-core package.

Until one of those outcomes is chosen, the compatibility slice must not expand
outside CRAM ingestion and reference-policy handling.

For Milestone 13, M13.2 chooses compatibility-only continuation: keep the
current CRAM compatibility boundary, do not promote native CRAM parsing or
writing, and defer any narrowing of `cram-compat` to later M13 tasks.

M13.9 strengthens the boundary rather than widening it. The dependency
contract now names `cram_consume_boundary`, `non_cram_indexed_surfaces`, and
`cram_benchmark_guardrail` as protected M13 surfaces. Direct production
`noodles_*` usage must remain confined to `src/ingest/cram.rs`; indexed-region
commands, index commands, region planning/traversal modules, and
`scanner_microbench` must not import `noodles` directly. Benchmark rows remain
synthetic BAM smoke timings and must not be cited as CRAM indexed-query,
CRAM compatibility-throughput, or CRAM comparator-parity evidence.
