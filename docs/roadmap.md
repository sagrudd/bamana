# Bamana Native-Core Implementation Roadmap

## Purpose

This roadmap turns the Bamana-native core decision into a buildable migration
sequence with milestones, acceptance criteria, command ordering, benchmark
hooks, and dependency checkpoints.

It is intentionally organized around **engine capability milestones**, not just
command count.

## Backbone Order

The backbone order for the migration is:

1. native BGZF
2. native BAM header codec
3. native BAM record scanner
4. native FASTQ / FASTQ.GZ parser
5. command migration off `noodles`, beginning with:
   * `verify`
   * `header`
   * `subsample`

This order is retained because it matches the dependency chain of the runtime:

* BGZF is the physical substrate
* BAM header parsing is the smallest high-value BAM capability above BGZF
* BAM record scanning is the first true hot-loop execution substrate
* FASTQ is required for ingest and mixed command families but does not block
  BAM-first migration
* command migration should follow substrate maturity rather than race ahead of
  it

## Milestone Index

### Milestone 1: Native BGZF Core

* detail: [roadmap/milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)
* goal: own BGZF reading, writing, block handling, EOF checks, and virtual
  offset groundwork
* commands enabled first: `check_eof`, `verify`

### Milestone 2: Native BAM Header Codec

* detail: [roadmap/milestone-02-bam-header.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-02-bam-header.md)
* goal: own BAM magic, header text, binary references, and deterministic header
  serialization
* commands enabled first: `verify`, `header`

### Milestone 3: Native BAM Record Scanner

* detail: [roadmap/milestone-03-bam-record-scan.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-03-bam-record-scan.md)
* goal: own selective BAM record iteration and lightweight field extraction
* commands enabled first: scan commands and BAM-side `subsample`

### Milestone 4: Native FASTQ / FASTQ.GZ Parser

* detail: [roadmap/milestone-04-fastq.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-04-fastq.md)
* goal: own FASTQ and FASTQ.GZ parsing and writing for ingest and transform
  paths
* commands enabled first: FASTQ-side `subsample`, `consume`,
  duplication/forensics families

### Milestone 5: Command Migration Off `noodles`

* detail: [roadmap/milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* goal: prove the substrate is real by migrating `verify`, `header`, and
  `subsample` in that order

## Later Waves

Later migration waves should follow after Milestone 5.

### Wave 2

Primary command targets:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

These commands rely heavily on native scan and selective decode quality.

### Wave 3

Primary command targets:

* `reheader`
* `annotate_rg`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

These commands depend on stable BAM header ownership, BAM record scanning, and
FASTQ support.

### Wave 4

Primary command targets:

* `sort`
* `merge`
* `explode`
* `checksum`
* `consume`

These are the larger transform and ingest families that depend on the earlier
substrate work and benefit most from benchmark-driven optimization.

CRAM remains explicitly staged later and must not derail the BAM/FASTQ native
core sequence.

## Milestone Template

Every milestone should be reviewed against the same completion template:

* functionality completed
* owned modules implemented or strengthened
* commands enabled or migrated
* tests added or updated
* benchmark hooks run or prepared
* docs updated
* `noodles` usage reduced, isolated, or checkpointed
* unresolved risks and follow-up tasks recorded

See:

* [roadmap/milestone-template.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-template.md)

## Dependency Demotion Checkpoints

### Checkpoint A

`noodles` remains available only in:

* CRAM compatibility
* tests
* fixture tooling
* oracles and compatibility checks

### Checkpoint B

For each migrated command family, `noodles` is absent from its production code
path and any old dependency is isolated behind a shim or removed.

### Checkpoint C

Once enough migration is complete, `noodles` can be feature-gated more
strictly, made optional, or moved further toward dev/test-only roles, with CRAM
compatibility remaining the only explicit exception if still needed.

## Benchmark Hooks By Stage

Architecture migration is not complete until it is measured.

Each milestone must define:

* microbenchmarks for the new substrate
* command-level benchmarks to re-run
* evidence that shows improvement or at least preserves semantics while moving
  ownership into Bamana-native code

Use the benchmark framework under
[benchmarks/](/Users/stephen/Projects/bamana/benchmarks) as the command-level
measurement layer. Microbenchmarks may remain lightweight and repository-local
until a fuller harness is added.

## Current Milestone

The currently active milestone is tracked in:

* [roadmap/current_milestone.md](/Users/stephen/Projects/bamana/docs/roadmap/current_milestone.md)
