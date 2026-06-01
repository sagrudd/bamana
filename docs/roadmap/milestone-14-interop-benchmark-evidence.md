# Milestone 14: Interoperability And Benchmark Evidence

Status: active as of 2026-06-01. Milestone 14 turns benchmark and
interoperability claims into governed evidence.

## Goal

Make Bamana's comparator, benchmark, and fixture evidence strong enough to
support release-facing claims without implying broad parity where it has not
been measured. This milestone should expand the `benchmark` command and
repository-local smoke hooks in lockstep with public command contracts.

## M14.1 Activation And Benchmark Evidence Audit

M14.1 activates the interoperability and benchmark evidence milestone after
Milestone 13 closeout. The activation is an audit-only step: it does not add
new benchmark profiles, new comparator claims, schemas, or command behavior.

Current public `benchmark` command profiles:

* `fastq_ingress`: containerized FASTQ.GZ-to-unmapped-BAM benchmark comparing
  Bamana against a fastcat-plus-samtools path, rendered through
  `benchmarks/bin/run_fastq_ingress_benchmark.sh`;
* `fastq_gz_enumerate`: containerized FASTQ.GZ record-count benchmark
  comparing Bamana-native enumeration against gzip decompression plus line
  counting, rendered through
  `benchmarks/bin/run_fastq_gz_enumerate_benchmark.sh`.

Current repository-local microbenchmark smoke hooks:

* `bgzf_microbench`: native BGZF throughput plus optional `check_eof` command
  smoke timing;
* `header_microbench`: native BAM header parsing/serialization plus optional
  `verify`, `header`, `reheader`, and `annotate_rg` command timings;
* `scanner_microbench`: native BAM scanner throughput plus governed
  command-timing rows for inspection, transform/ingest, index, indexed-region,
  selected-region, CSI fallback, mutation, remediation, and forensics smoke
  paths;
* `fastq_microbench`: native FASTQ/FASTQ.GZ parser and writer throughput plus
  optional `subsample_fastq` and `subsample_fastq_gz` command timings.

Current broader benchmark framework inventory:

* Nextflow entry point: `benchmarks/main.nf`;
* run parameter schema: `benchmarks/params.schema.json`;
* input manifest schema: `benchmarks/inputs/manifest.schema.json`;
* result schemas: `benchmarks/results/result.schema.json`,
  `benchmarks/results/benchmark_row.schema.json`, and the four microbenchmark
  result schemas;
* wrapper registry and matrix:
  `benchmarks/tools/tool_registry.example.json` and
  `benchmarks/tools/workflow_variant_matrix.md`;
* primary wrapper tools: Bamana, samtools, and fastcat, with sambamba, seqtk,
  and rasusa represented for explicit support/unsupported handling.

M14 starts from an evidence boundary rather than a parity claim. Existing smoke
timings are regression guardrails. Existing comparator rows are profile- and
scenario-specific. Broad comparator parity, biological equivalence, release
performance promises, CRAM comparator claims, and external-tool authority
remain out of scope until later M14 tasks pin command-specific evidence.

## M14.2 Command Comparator Evidence Matrix

M14.2 defines the current command-level evidence matrix in
`benchmarks/command_evidence_matrix.md`. The matrix classifies public command
surfaces as `public_profile_comparator`,
`scenario_matrix_comparator_scaffold`, `local_smoke`, or
`no_external_comparator_claim`.

Current public-profile comparator evidence is limited to:

* FASTQ.GZ `enumerate` through `fastq_gz_enumerate`, comparing Bamana-native
  record counting with gzip decompression plus line counting;
* FASTQ.GZ unmapped `consume` through `fastq_ingress`, comparing Bamana
  normalization with a fastcat-plus-samtools path.

Repository-local smoke evidence covers commands represented in
`bgzf_microbench`, `header_microbench`, `scanner_microbench`, and
`fastq_microbench`. Workflow-matrix comparator rows for BAM `subsample`,
`sort`, and FASTQ.GZ `subsample` remain scaffolded evidence only, not
release-facing comparator parity.

`identify`, `fastq`, `unmap`, CRAM consume behavior, unmeasured command modes,
and scaffold-only workflow rows remain `no_external_comparator_claim` surfaces
until later M14 tasks add command-specific fixtures, semantic assumptions, and
schema coverage.

## M14.3 Benchmark Result Schema Extension

M14.3 extends benchmark result schemas for post-M10 command families without
changing runtime behavior or adding benchmark profiles. The raw result schema
`benchmarks/results/result.schema.json` and tidy row schema
`benchmarks/results/benchmark_row.schema.json` now allow optional
`command_family`, `evidence_level`, `evidence_source`, and `comparator_scope`
fields. Existing result rows remain valid because the fields are optional.

`benchmarks/results/scanner_microbench.schema.json` now records a post-M10
command-family taxonomy for `indexed_region`, `selected_region`,
`csi_fallback`, `remediation`, `forensics`, `transform_ingest`, `inspection`,
and `index` timing rows. `benchmarks/results/header_microbench.schema.json`
records the `header`/`mutation` split for `verify`, `header`, `reheader`, and
`annotate_rg`, and `benchmarks/results/fastq_microbench.schema.json` records
the `fastq` timing family. This schema metadata maps existing timing rows to
evidence families while preserving the M14.2 boundary: local smoke evidence is
not broad comparator parity, biological equivalence, CRAM indexed-query
evidence, CSI random-access traversal, or replacement output-index evidence.

## M14.4 Fixture Provenance Metadata

M14.4 adds reproducible fixture provenance metadata for benchmark inputs and
future comparator fixtures without materializing new fixtures or changing
runtime benchmark behavior. The governed schema is
`benchmarks/inputs/manifest.schema.json`, the example is
`benchmarks/inputs/example_manifest.json`, and the human-readable contract is
`benchmarks/inputs/fixture_provenance.md`.

The `fixture_provenance` object records source kind, source description,
source URI, `derived_from`, generation command, generation environment,
expected semantic scope, review boundary, checksum policy, and reproducibility
notes. `benchmarks/bin/validate_inputs.py` validates the block when it is
present. M14.4 does not add fixture generation scripts, benchmark profiles, or
comparator claims; later M14 tasks must attach checksums, generated artifacts,
and archived outputs before new release-facing benchmark evidence is promoted.

## M14.5 Aligned Comparator Profiles

M14.5 adds the governed aligned comparator profile catalog at
`benchmarks/comparator_profiles.json`, with schema coverage in
`benchmarks/comparator_profiles.schema.json`. The catalog currently records
only the measured public profiles whose semantics are aligned enough for
profile-specific evidence:

* `fastq_ingress`: Bamana `consume --mode unmapped` for FASTQ.GZ input versus
  `fastcat fastq | samtools import`;
* `fastq_gz_enumerate`: Bamana `enumerate --input` for FASTQ.GZ input versus
  `gzip -cd | awk` line counting.

Each profile records the benchmark runner, Bamana path, comparator path, result
artifacts, semantic equivalence assumptions, unsupported mismatch cases, and
release claim boundary. The public `benchmark` JSON payload now exposes
`semantic_equivalence_assumptions` and `unsupported_mismatch_cases` for the
selected profile. M14.5 does not promote `fastq`, `unmap`, CRAM behavior,
workflow-matrix scaffold rows, or unmeasured command modes to comparator
claims.

## M14.6 Comparator Mismatch Register

M14.6 adds `benchmarks/comparator_mismatches.md` as the unsupported comparator
register. The register documents no-claim and scaffolded mismatch surfaces so
they are visible as intentional benchmark boundaries rather than missing
implementation.

The register covers `fastq`, `unmap`, `identify`, CRAM consume behavior,
BAM/SAM alignment ingest, mixed-directory ingest, BAM and FASTQ.GZ
subsampling, `rasusa` downsampling, mapped BAM sort/index pipelines,
`select_region`, mutation commands, and forensic commands. Each surface records
an evidence status, semantic mismatch reason, current boundary, and promotion
requirement. The only public-profile exceptions remain `fastq_ingress` and
`fastq_gz_enumerate`, bounded by `benchmarks/comparator_profiles.json`.

## M14.7 Benchmark Schema Stability Checks

M14.7 adds the local benchmark schema stability harness
`benchmarks/bin/check_schema_stability.py` and the pinned inventory
`benchmarks/schema_stability_manifest.json`. The manifest lists every
`benchmarks/**/*.schema.json` file with its `$id`, contract surface, version
pointer, version value, and required stability pointers.

The harness fails when benchmark schemas are added, removed, renamed, have
unexpected `$id` values, lose version metadata, or drop required M14 metadata.
It covers benchmark params, input manifests, raw/tidy result schemas,
microbenchmark result schemas, tool registry schemas, and comparator-profile
schemas.

## M14.8 Public Documentation Refresh

M14.8 adds `benchmarks/public_evidence_guide.md` as the public documentation
index for benchmark evidence. The guide distinguishes public benchmark
profiles, repository-local smoke hooks, scaffolded comparator evidence, and
`no_external_comparator_claim` surfaces. It links the command evidence matrix,
aligned comparator profile catalog, mismatch register, and schema stability
manifest so readers can find the governed source for each evidence class.

The public contract commands remain explicit in the guide: `benchmark` owns
governed profile execution, while `fastq` and `unmap` currently remain
no-external-comparator-claim surfaces.

## M14.9 Benchmark Dependency Guardrails

M14.9 adds dependency-boundary tests for benchmark-only external tools and
oracles. `samtools`, `fastcat`, `sambamba`, `seqtk`, and `rasusa` may appear as
wrappers, comparators, fixtures, or oracle aids, and may be referenced by
governed benchmark orchestration in `src/commands/benchmark.rs`.

They must not enter Bamana-native production hot paths for BAM, BGZF, FASTQ,
sampling, ingest, index, region, mutation, remediation, or forensic behavior.
`tests/contract/dependency_boundary.rs` enforces this boundary by scanning
production Rust sources outside the benchmark runner.

## Ten-Task Outline

1. M14.1 activate scope and audit all benchmark profiles and smoke hooks.
   Complete: activation baseline records public benchmark profiles,
   repository-local smoke hooks, benchmark framework artifacts, and non-claim
   boundaries.
2. M14.2 define which public commands have comparator evidence, smoke evidence,
   or no external comparator claim.
   Complete: command-level evidence matrix added with public-profile
   comparator, local-smoke, scaffolded-comparator, and no-claim classifications.
3. M14.3 extend benchmark result schemas for post-M10 command families.
   Complete: raw, tidy, and scanner microbenchmark schemas now carry optional
   command-family and evidence-level fields plus a scanner timing taxonomy.
4. M14.4 add reproducible fixture generation and fixture provenance metadata.
   Complete: benchmark input manifests now support governed fixture provenance
   metadata and example inputs document source, generation, semantic-scope, and
   review-boundary fields.
5. M14.5 add measured comparator profiles only where semantics are aligned.
   Complete: `benchmarks/comparator_profiles.json` records `fastq_ingress`
   and `fastq_gz_enumerate` with explicit semantic assumptions, unsupported
   mismatch cases, result artifacts, and release claim boundaries, and the
   public `benchmark` JSON contract exposes those fields.
6. M14.6 document unsupported comparator cases and semantic mismatch reasons.
   Complete: `benchmarks/comparator_mismatches.md` records documented no-claim
   and scaffolded mismatch surfaces with semantic mismatch reasons, current
   boundaries, and promotion requirements.
7. M14.7 add CI or local harness checks for benchmark schema stability.
   Complete: `benchmarks/bin/check_schema_stability.py` validates the pinned
   schema inventory in `benchmarks/schema_stability_manifest.json` and is
   covered by contract tests.
8. M14.8 update benchmark docs, README, CLI docs, roadmap, and Sphinx docs.
   Complete: `benchmarks/public_evidence_guide.md` now consolidates public
   profile, smoke hook, scaffolded comparator, and no-claim interpretation,
   with README, CLI, Sphinx, roadmap, current milestone, M14 roadmap, and
   taskmap pointers.
9. M14.9 add dependency-boundary tests for benchmark-only tools and oracles.
   Complete: dependency-boundary tests keep `samtools`, `fastcat`,
   `sambamba`, `seqtk`, and `rasusa` confined to benchmark wrappers,
   comparators, fixtures, oracle aids, and governed benchmark orchestration.
10. M14.10 close the milestone with archived smoke evidence and residual risk
    notes.

## Non-Goals

M14 does not convert smoke timings into performance promises, does not claim
biological equivalence, and does not treat external tool output as authoritative
unless a command-specific oracle contract says so.
