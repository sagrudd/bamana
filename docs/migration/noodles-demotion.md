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

## Phased Replacement Plan

### Phase 1

* establish Bamana-native top-level `bgzf` and `fastq` modules
* make the crate architecture explicit through `src/lib.rs`
* convert old paths into compatibility shims where needed
* document hot-path ownership and dependency policy

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

## Transitional Code Rule

Any remaining production `noodles` usage must:

* live in a clearly labeled compatibility boundary
* carry a comment that the usage must not expand
* be documented here
