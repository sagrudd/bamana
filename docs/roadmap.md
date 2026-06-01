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

* status: complete
* detail: [roadmap/milestone-11-indexed-region-selection.md](roadmap/milestone-11-indexed-region-selection.md)
* goal: promote selected-record region output and region-file input only after
  output semantics, header preservation, duplicate handling, index invalidation,
  and write-safety are specified
* commands enabled first: `select_region` for BGZF BAM file output from CLI
  `--region` requests
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
* M11.9 dependency and benchmark guardrails: selected-region output hot paths
  stay protected from direct production `noodles` imports, and
  `scanner_microbench --bamana-bin` emits `select_region_scan_fallback` and
  `select_region_indexed_output` smoke timings with interpretation limits for
  stdout output, public region files, replacement output indexes, comparator
  parity, native CRAM indexed queries, and biological interpretation
* M11.10 closeout: M11.1 through M11.10 completed on 2026-05-28 with full
  tests, contract tests, Sphinx HTML documentation, formatting, whitespace
  checks, binary builds, and scanner microbenchmark smoke evidence recorded

### Milestone 12: Extended Index Compatibility

* status: complete as of 2026-05-28
* detail: [roadmap/milestone-12-extended-index-compatibility.md](roadmap/milestone-12-extended-index-compatibility.md)
* goal: harden CSI, large-reference, stale-index, and cross-index compatibility
  behavior without weakening native BAM ownership
* commands enabled first: index-aware `check_index`, `check_map`, `summary`,
  and any M11 selection surface
* M12.1 activation baseline: BAI detection/parsing/writing is implemented for
  coordinate-sorted BAM; CSI is discovered and parsed at header level but
  remains detected-not-usable; `check_map`, `summary`, and `select_region`
  remain BAI-first with scan fallback; FASTQ.GZI remains a FASTQ.GZ planning
  sidecar rather than a BAM random-access index
* M12.2 CSI support-level freeze: `check_index` now reports `support_level`
  values, with BAI as `read_write`, CSI as `detect_only`, FASTQ.GZI as
  `planning_sidecar`, unknown sidecars as `unsupported`, and absent sidecars as
  `absent`; CSI is not promoted to read traversal or writing in this milestone
  slice
* M12.3/M12.4 compatibility hardening: BAI creation rejects references longer
  than 536,870,912 bases before writing output, CSI remains detect-only rather
  than a large-reference fallback, and `check_map.index` reports
  machine-readable diagnostic states for usable, absent, stale, unsupported,
  malformed, mismatched-reference, incomplete, and disabled index conditions
* M12.5 fixture extension: reserved `tiny.invalid.large_reference.bam`,
  `tiny.invalid.mismatched_reference_count.csi`, and
  `tiny.invalid.fastq_gz.bad_gzi` for BAI, CSI, and FASTQ.GZI compatibility
  failure coverage
* M12.6 read-only CSI region behavior: `check_map --region <REGION>` and
  `summary --region <REGION>` preserve CSI context in JSON while using native
  scan fallback; BAI remains the only index kind used for region traversal
* M12.7 selected-region compatibility: `select_region` now reports
  `input_index` compatibility for selected adjacent input sidecars, keeps CSI
  as detect-only scan fallback, and leaves output BAI/CSI invalidation and
  replacement-index deferrals unchanged
* M12.8 public contract refresh: schemas, examples, CLI docs, README, roadmap,
  Sphinx, and fixture docs now align around `support_level`,
  `diagnostic_status`, `index_derived`, `input_index`, and the reserved M12
  fixture outputs
* M12.9 benchmark and dependency guardrails: `scanner_microbench --bamana-bin`
  emits CSI compatibility smoke timings for detect-only support-level
  reporting, native scan fallback, and selected-region `input_index`
  compatibility, and dependency-boundary tests keep the promoted M12 index
  compatibility hot paths Bamana-native
* M12.10 closeout: M12.1 through M12.10 completed on 2026-05-28 with full
  tests, contract tests, Sphinx HTML, formatting, whitespace checks, binary
  builds, and scanner smoke evidence passing; residual risk remains explicit
  for CSI bin parsing, CSI chunk planning, CSI random-access traversal, CSI
  writing, large-reference CSI support, native CRAM indexed queries, and broad
  comparator parity

### Milestone 13: Native CRAM Strategy And Compatibility Boundary

* status: complete as of 2026-06-01
* detail: [roadmap/milestone-13-native-cram-strategy.md](roadmap/milestone-13-native-cram-strategy.md)
* goal: decide and document the next CRAM compatibility boundary, including
  whether any native CRAM substrate is promoted or explicitly deferred
* commands enabled first: `consume` and CRAM-facing inspection paths only if
  the native/reference-policy contract is frozen
* M13.1 activation baseline: current CRAM behavior is conservative
  compatibility only, concentrated in `src/ingest/cram.rs`; `consume` supports
  alignment-mode CRAM normalization through the explicit `--reference-policy`
  contract, default `strict` requires `--reference <fasta>` with adjacent
  `.fai`, `allow-cache` and `--reference-cache` remain unimplemented,
  `allow-embedded` and `auto-conservative` only attempt no-external-reference
  decode, CRAI/indexed CRAM queries and native CRAM parsing/writing remain
  deferred, and direct production `noodles_*` imports remain allowed only in
  the documented CRAM compatibility boundary
* M13.2 direction decision: CRAM remains compatibility-only for Milestone 13;
  no native CRAM substrate is promoted, CRAM ingestion stays in the
  native-core package through the explicit transitional `cram-compat` feature,
  direct production `noodles_*` imports stay confined to `src/ingest/cram.rs`,
  and native CRAM parsing/writing, CRAI/indexed CRAM queries, cache-backed
  decoding, and broad comparator parity remain deferred
* M13.3 reference/cache policy freeze: `strict` requires explicit indexed
  FASTA and otherwise fails with `reference_required`; explicit FASTA takes
  precedence over `--reference-cache`; `allow-embedded` and cache-free
  `auto-conservative` only attempt no-external-reference decode; `allow-cache`
  and `auto-conservative --reference-cache` return `unimplemented`; and
  `--reference-cache` is recorded but not searched, populated, or used for
  fallback in this slice
* M13.4 CRAM indexed query position: CRAI and indexed CRAM queries are
  unsupported/deferred for Milestone 13, not transitional behavior and not
  native work; `.crai` sidecars are not discovered, parsed, planned, or used;
  `consume` remains sequential CRAM normalization with no random-access
  traversal; `check_map --region`, `summary --region`, and `select_region` do
  not accept CRAM region input; `index` does not create CRAI; JSON outputs
  expose no JSON CRAI evidence; and any future milestone must add a staged plan
  before promoting indexed CRAM queries
* M13.5 CRAM fixture/oracle boundary: the fixture plan is provenance-first,
  with `tiny.valid.cram.explicit_ref.source_sam` and `tiny.ref.primary`
  present, derived CRAM/BAM compatibility fixtures planned,
  `tiny.valid.cram.no_external_ref` deferred, and CRAI fixtures, indexed CRAM
  fixtures, and CRAM random-access oracle outputs explicitly deferred; `noodles`
  and external tools are allowed only for fixture generation, fixture
  validation, or test-only compatibility checks
* M13.6 implemented boundary: CRAM remains consume-only alignment-mode
  normalization; cache-backed decoding remains `unimplemented`; direct
  production `noodles_*` imports stay confined to `src/ingest/cram.rs`;
  directory discovery skips `.crai` sidecars before probing with
  `cram_index_sidecar_deferred`; direct `.crai` requests fail as
  `unsupported_format`; and no CRAI parsing, indexed CRAM traversal, native
  CRAM parser, or native CRAM writer is introduced
* M13.7 CRAM-facing contract refresh: `consume` is the only command with
  changed CRAM-facing behavior; the consume schema documents
  `cram_index_sidecar_deferred`, canonical examples cover directory `.crai`
  sidecar skip and direct `.crai` rejection, and inspection/region commands
  keep existing BAM/BAI/CSI contracts without CRAM indexed-query fields
* M13.8 public docs and schema inventory: `spec/jsonschema/consume.schema.json`,
  the two CRAI consume examples, CLI contracts, README, CLI docs, JSON-output
  docs, Sphinx notes, roadmap notes, current-milestone notes, and taskmap now
  record the same CRAM boundary; this slice adds no CRAI parser, CRAM region
  input, CRAI creation, CRAM random-access evidence, benchmark CRAM
  indexed-query evidence, or CRAM indexed-query JSON fields
* M13.9 CRAM dependency and benchmark guardrails: protected surfaces are
  `cram_consume_boundary`, `non_cram_indexed_surfaces`, and
  `cram_benchmark_guardrail`; direct production `noodles_*` usage remains
  confined to `src/ingest/cram.rs`; `scanner_microbench` stays synthetic
  BAM-only, emits no CRAM command timing rows, and is not evidence for CRAI
  parsing, CRAM region input, CRAM random-access traversal, CRAM indexed-query
  behavior, CRAM compatibility throughput, or CRAM comparator parity
* M13.10 closeout: M13.1 through M13.10 completed on 2026-06-01 with full
  tests, contract tests, Sphinx, formatting checks, diff checks, binary builds,
  and scanner smoke evidence passing; the scanner smoke emitted 28 command
  timing rows, no CRAM command timing rows, and the M13.9 CRAM benchmark
  guardrail note; residual risk remains explicit for native CRAM
  parsing/writing, CRAI parsing/creation, indexed CRAM queries, CRAM region
  input, CRAM random-access traversal, cache-backed decoding, derived CRAM/BAM
  fixtures, no-external-reference CRAM fixtures, CRAM compatibility throughput
  claims, CRAM comparator parity, and broad external tool parity

### Milestone 14: Interoperability And Benchmark Evidence

* status: complete as of 2026-06-01
* detail: [roadmap/milestone-14-interop-benchmark-evidence.md](roadmap/milestone-14-interop-benchmark-evidence.md)
* goal: turn comparator, benchmark, fixture, and reproducibility evidence into
  governed claims without implying broad parity where it has not been measured
* commands enabled first: `benchmark` profiles and command-level smoke evidence
  for governed public surfaces
* M14.1 activation baseline: public `benchmark` profiles are
  `fastq_ingress` and `fastq_gz_enumerate`; repository-local smoke hooks are
  `bgzf_microbench`, `header_microbench`, `scanner_microbench`, and
  `fastq_microbench`; Nextflow benchmark artifacts, result schemas, wrapper
  registry, and support matrix docs are present; existing smoke timings are
  regression guardrails and existing comparator rows are profile-specific, not
  broad comparator parity or biological-equivalence claims
* M14.2 evidence matrix: `benchmarks/command_evidence_matrix.md` classifies
  public command surfaces as public-profile comparator, local smoke,
  scenario-matrix comparator scaffold, or no external comparator claim; only
  `fastq_gz_enumerate` for FASTQ.GZ `enumerate` and `fastq_ingress` for
  FASTQ.GZ unmapped `consume` are current public-profile comparator evidence
* M14.3 schema extension: raw and tidy benchmark result schemas now allow
  optional `command_family`, `evidence_level`, `evidence_source`, and
  `comparator_scope` fields, and the scanner microbenchmark schema records
  post-M10 families for indexed-region, selected-region, CSI fallback,
  remediation, forensics, transform/ingest, inspection, and index timing rows;
  header and FASTQ microbenchmark schemas record mutation and FASTQ command
  timing families without changing runtime behavior
* M14.4 fixture provenance: benchmark input manifests now support
  `fixture_provenance` metadata for source kind, generation command,
  generation environment, expected semantic scope, review boundary, checksum
  policy, and reproducibility notes, with example manifest entries and local
  validator checks in place
* M14.5 aligned comparator profiles: `benchmarks/comparator_profiles.json`,
  governed by `benchmarks/comparator_profiles.schema.json`, records
  `fastq_ingress` and `fastq_gz_enumerate` as measured public comparator
  profiles with explicit semantic equivalence assumptions, unsupported
  mismatch cases, result artifacts, and release claim boundaries
* M14.6 comparator mismatch register: `benchmarks/comparator_mismatches.md`
  records documented no-claim and scaffolded mismatch surfaces with semantic
  mismatch reasons, current benchmark boundaries, and promotion requirements
* M14.7 schema stability checks: `benchmarks/bin/check_schema_stability.py`
  validates `benchmarks/schema_stability_manifest.json` so every benchmark
  schema path, `$id`, version pointer, and required stability metadata is
  checked locally and by contract tests
* M14.8 documentation refresh: `benchmarks/public_evidence_guide.md`
  consolidates public profile, smoke hook, scaffolded comparator, and
  no-external-comparator-claim interpretation across README, CLI docs,
  roadmap, Sphinx, and benchmark docs
* M14.9 dependency guardrails: `samtools`, `fastcat`, `sambamba`, `seqtk`, and
  `rasusa` remain benchmark-only external tools for wrappers, comparators,
  fixtures, or oracle aids and are kept out of Bamana-native production hot
  paths by dependency-boundary tests
* M14.10 closeout: M14.1 through M14.10 completed on 2026-06-01 with full
  Rust, contract, documentation, schema-stability, manifest-validation, and
  four-hook benchmark smoke verification. Archived smoke evidence was written
  to `/tmp/bamana-m1410-bgzf-small.json`,
  `/tmp/bamana-m1410-header-small.json`,
  `/tmp/bamana-m1410-scanner-small.json`, and
  `/tmp/bamana-m1410-fastq-small.json`; those smoke outputs recorded 2 BGZF,
  4 header, 28 scanner, and 4 FASTQ command timing rows. Residual risk remains
  explicit for broad comparator parity, biological equivalence, release
  performance promises, CRAM comparator claims, external-tool authority,
  scaffold-only workflow rows, unmeasured `fastq`, `unmap`, and `identify`
  comparator claims, and future command-specific fixture generation.

### Milestone 15: Release Hardening And Public Contract Freeze

* status: active as of 2026-06-01
* detail: [roadmap/milestone-15-release-hardening.md](roadmap/milestone-15-release-hardening.md)
* goal: prepare a coherent release boundary with contract stability,
  documentation completeness, packaging, CI, and operational support evidence
* commands enabled first: all public contract commands accepted into the
  release boundary
* M15.1 activation baseline: Milestone 15 is active only after the Milestone 14
  closeout commit. The release boundary accepts the currently implemented and
  documented CLI commands: `benchmark`, `identify`, `enumerate`, `subsample`,
  `inspect_duplication`, `deduplicate`, `forensic_inspect`, `annotate_rg`,
  `consume`, `explode`, `fastq`, `checksum`, `merge`, `reheader`, `sort`,
  `select_region`, `unmap`, `verify`, `check_eof`, `header`, `check_map`,
  `check_index`, `index`, `summary`, `validate`, `check_tag`, and
  `check_sort`. `benchmark`, `fastq`, and `unmap` remain named public contract
  commands. Acceptance is limited to the behavior already represented in
  schemas, examples, CLI docs, Sphinx docs, roadmap notes, and contract tests;
  deferred surfaces remain outside the release boundary until later M15 tasks
  freeze or explicitly exclude them.

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
