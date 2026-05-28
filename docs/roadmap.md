# Bamana Native-Core Implementation Roadmap

## Purpose

This roadmap turns the Bamana-native core decision into a buildable migration
sequence with milestones, acceptance criteria, command ordering, benchmark
hooks, and dependency checkpoints.

It is intentionally organized around **engine capability milestones**, not just
command count.

Some command first slices already exist ahead of their final migration wave.
That does not change the milestone order. The roadmap tracks when the native
substrate is owned well enough for command behavior to rely on it, not merely
when a command name first appears in the CLI.

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

* status: complete
* detail: [roadmap/milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)
* goal: own BGZF reading, writing, block handling, EOF checks, and virtual
  offset groundwork
* commands enabled first: `check_eof`, `verify`

### Milestone 2: Native BAM Header Codec

* status: complete
* detail: [roadmap/milestone-02-bam-header.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-02-bam-header.md)
* goal: own BAM magic, header text, binary references, and deterministic header
  serialization
* commands enabled first: `verify`, `header`

### Milestone 3: Native BAM Record Scanner

* status: complete
* detail: [roadmap/milestone-03-bam-record-scan.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-03-bam-record-scan.md)
* goal: own selective BAM record iteration and lightweight field extraction
* commands enabled first: scan commands and BAM-side `subsample`

### Milestone 4: Native FASTQ / FASTQ.GZ Parser

* status: complete
* detail: [roadmap/milestone-04-fastq.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-04-fastq.md)
* goal: own FASTQ and FASTQ.GZ parsing and writing for ingest and transform
  paths
* commands enabled first: FASTQ-side `subsample`, `consume`,
  duplication/forensics families

### Milestone 5: Command Migration Off `noodles`

* status: complete
* detail: [roadmap/milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* goal: prove the substrate is real by migrating `verify`, `header`, and
  `subsample` in that order

### Milestone 6: Native Inspection And Validation Commands

* status: complete
* detail: [roadmap/milestone-06-inspection-validation.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-06-inspection-validation.md)
* goal: harden native inspection and validation command paths after the proof
  command migration
* commands enabled first: `check_eof`, `check_sort`, `check_map`, `summary`,
  `check_tag`, `validate`

### Milestone 7: Native Mutation, Remediation, And Forensics Commands

* status: complete
* detail: [roadmap/milestone-07-mutation-forensics.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-07-mutation-forensics.md)
* goal: harden native mutation, conservative remediation, and provenance
  inspection command paths
* commands enabled first: `reheader`, `annotate_rg`, `inspect_duplication`,
  `deduplicate`, `forensic_inspect`

### Milestone 8: Native Transform, Checksum, Explode, And Ingest Commands

* status: complete
* detail: [roadmap/milestone-08-transform-ingest.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-08-transform-ingest.md)
* goal: harden large native transform, checksum, sharding, and ingest command
  paths
* commands enabled first: `sort`, `merge`, `explode`, `checksum`, `consume`

### Milestone 9: Native BAM Index And Random Access

* status: complete
* detail: [roadmap/milestone-09-bam-index-random-access.md](roadmap/milestone-09-bam-index-random-access.md)
* goal: own BAM index writing, deeper index validation, and virtual-offset
  random-access groundwork
* commands enabled first: `index`, `check_index`, indexed `check_map`,
  indexed `summary`

### Milestone 10: Native Indexed Region Workflows

* status: complete
* detail: [roadmap/milestone-10-indexed-region-workflows.md](roadmap/milestone-10-indexed-region-workflows.md)
* goal: turn native BAM index and random-access substrates into bounded
  indexed-region workflows
* commands enabled first: region-aware `check_map`, region-aware `summary`,
  future indexed region selection command explicitly deferred

### Milestone 11: Public Indexed Region Selection And Region Files

* status: active
* detail: [roadmap/milestone-11-indexed-region-selection.md](roadmap/milestone-11-indexed-region-selection.md)
* goal: promote selected-record region output and region-file input only after
  output semantics, header preservation, duplicate handling, index invalidation,
  and write-safety are specified
* commands enabled first: planned `select_region`, after the M11 contract
  freezes its semantics
* M11.2 region-file contract: future inputs are UTF-8 line-oriented files using
  one M10-style region per non-comment line; blank lines and leading `#`
  comment lines are ignored; request order, duplicate lines, and overlapping
  intervals are preserved; BED-like and other unsupported coordinate models are
  rejected until explicitly promoted
* M11.3 output contract: future selected-record output is BAM-only, using
  BGZF-compressed BAM input and output, explicit `--out`, JSON report separation
  when binary BAM is sent to stdout, and dry-run reports with no BAM writes
* M11.4 header contract: future selected output preserves the full reference
  dictionary and textual header, appends only `@PG` provenance, downgrades
  `@HD SO` to `unknown`, removes `SS`, and reports input/output sort metadata
* M11.5 duplicate policy: future selected output emits each physical source
  record at most once in source virtual-offset order, suppressing duplicate
  physical records from repeated, overlapping, or broad-bin region discovery
* M11.6 implementation slice: `select_region` now writes BGZF BAM file output
  for CLI `--region` requests, uses native indexed traversal when a usable
  non-stale BAI exists, falls back to native scanning otherwise, preserves raw
  selected record bytes, and keeps `--out -`, `--region-file`, schemas,
  examples, and fixtures deferred to later M11 tasks
* M11.7 write-safety and index invalidation: same-path input/output rewrites
  are rejected, existing output BAM or adjacent output BAI/CSI sidecars are
  collisions unless `--force` is supplied, forced applied runs remove stale
  adjacent output index sidecars, dry runs report planned invalidation without
  deleting files, and no replacement output index is created
* M11.8 governed contract artifacts: `select_region` now has a JSON schema,
  canonical success/failure examples, JSON-output documentation, CLI contract
  text, and fixture-plan reservations for the implemented file-output surface

### Milestone 12: Extended Index Compatibility

* status: planned
* detail: [roadmap/milestone-12-extended-index-compatibility.md](roadmap/milestone-12-extended-index-compatibility.md)
* goal: harden CSI, large-reference, stale-index, and cross-index compatibility
  behavior without weakening native BAM ownership
* commands enabled first: index-aware `check_index`, `check_map`, `summary`,
  and any M11 selection surface

### Milestone 13: Native CRAM Strategy And Compatibility Boundary

* status: planned
* detail: [roadmap/milestone-13-native-cram-strategy.md](roadmap/milestone-13-native-cram-strategy.md)
* goal: decide and document the next CRAM compatibility boundary, including
  whether any native CRAM substrate is promoted or explicitly deferred
* commands enabled first: `consume` and CRAM-facing inspection paths only if
  the native/reference-policy contract is frozen

### Milestone 14: Interoperability And Benchmark Evidence

* status: planned
* detail: [roadmap/milestone-14-interop-benchmark-evidence.md](roadmap/milestone-14-interop-benchmark-evidence.md)
* goal: turn comparator, benchmark, fixture, and reproducibility evidence into
  governed claims without implying broad parity where it has not been measured
* commands enabled first: `benchmark` profiles and command-level smoke evidence
  for governed public surfaces

### Milestone 15: Release Hardening And Public Contract Freeze

* status: planned
* detail: [roadmap/milestone-15-release-hardening.md](roadmap/milestone-15-release-hardening.md)
* goal: prepare a coherent release boundary with contract stability,
  documentation completeness, packaging, CI, and operational support evidence
* commands enabled first: all public contract commands accepted into the
  release boundary

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
