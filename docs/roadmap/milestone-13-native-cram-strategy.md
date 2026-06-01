# Milestone 13: Native CRAM Strategy And Compatibility Boundary

Status: complete as of 2026-06-01. Milestone 13 was the first explicit
post-M12 checkpoint for CRAM strategy. It follows the completed M12 extended
index compatibility milestone and does not reopen BAM, BGZF, FASTQ, or index
contracts.

## Goal

Decide whether Bamana promotes any native CRAM substrate or keeps CRAM as a
documented compatibility boundary. The milestone should reduce ambiguity around
reference policy, cache policy, indexed CRAM queries, and allowed production
dependency exceptions.

## M13.1 Activation And Baseline Audit

M13.1 activates the native CRAM strategy milestone without changing CLI
behavior. The current baseline is intentionally conservative:

* CRAM support is compatibility-oriented and concentrated in
  `src/ingest/cram.rs`.
* Direct production `noodles_*` imports are allowed only in that documented
  CRAM compatibility boundary. BAM, BGZF, FASTQ, sampling, ingest planning,
  indexing, and forensic hot paths remain Bamana-native.
* The default feature set includes `cram-compat`, which keeps
  `noodles-cram`, `noodles-bam`, `noodles-fasta`, and `noodles-sam` available
  for the transitional compatibility layer.
* `consume` is the only current public CRAM-facing command path. CRAM is
  accepted only in alignment mode and is normalized to BAM before downstream
  Bamana-native handling.
* The default CRAM reference policy is `strict`. Under `strict`, CRAM
  ingestion requires `--reference <fasta>` and an adjacent `.fai`.
* An explicit reference FASTA takes precedence over `--reference-cache` in the
  current slice.
* `allow-cache` and cache-backed `--reference-cache` decoding are planned but
  unimplemented.
* `allow-embedded` and `auto-conservative` may attempt decode without external
  reference material. Dry runs validate only policy shape and cannot prove
  decode success; real decode failures that require reference material are
  reported as `reference_required`.
* CRAM normalization currently decodes through the compatibility reader, writes
  a temporary BAM, and then re-enters the Bamana-native BAM reader and record
  layout path.
* CRAI handling, indexed CRAM queries, native CRAM parsing, native CRAM
  writing, cache-backed decoding, and broad comparator parity remain deferred.
* The CRAM fixture plan is partially reserved: source SAM and explicit FASTA
  provenance are present, while derived CRAM/BAM binaries and no-external-ref
  fixtures remain planned or deferred until reproducible generation is
  documented.

## M13.2 CRAM Direction Decision

M13.2 chooses **compatibility-only continuation** for Milestone 13. Bamana does
not promote a native CRAM substrate in this milestone, does not remove CRAM
ingestion from the native-core package, and does not broaden CRAM support into
general-purpose indexed CRAM querying. The chosen direction is:

* keep CRAM support limited to the existing `consume` alignment-mode
  normalization path;
* keep direct production `noodles_*` imports confined to
  `src/ingest/cram.rs`;
* keep BAM, BGZF, FASTQ, sampling, indexing, and forensic hot paths
  Bamana-native;
* keep `cram-compat` as the explicit transitional compatibility feature while
  later M13 tasks decide whether it should become non-default, narrower, or
  replaced;
* require any future native CRAM work to arrive as a separate staged
  implementation plan with fixtures, contracts, dependency guardrails, and
  benchmark evidence;
* treat CRAI/indexed CRAM queries, native CRAM parsing, native CRAM writing,
  cache-backed decoding, and broad comparator parity as deferred unless a
  later M13 task explicitly changes that contract.

This decision lets M13.3-M13.9 focus on freezing reference/cache semantics,
indexed-query position, fixtures, public contracts, and dependency guardrails
around the compatibility boundary that actually exists.

## M13.3 Reference And Cache Policy Freeze

M13.3 freezes the CRAM reference and cache semantics for the compatibility-only
Milestone 13 direction:

* `strict` is the default and safest policy. Without `--reference <fasta>`, it
  fails before decode with `reference_required`.
* `--reference <fasta>` must name a readable FASTA with an adjacent `.fai`.
  Missing FASTA or missing `.fai` fails as `reference_not_found`.
* An explicit FASTA always takes precedence over `--reference-cache`,
  regardless of the selected policy, and reports `source_used:
  explicit_fasta` plus `decode_without_external_reference: false` when CRAM
  decoding succeeds.
* `allow-embedded` permits only a no-external-reference decode attempt. Dry
  runs validate policy shape, leave `source_used` and
  `decode_without_external_reference` unknown, and do not prove decode success.
  Real decode failures that require reference material return
  `reference_required`.
* `auto-conservative` uses an explicit FASTA when one is supplied. Without
  explicit FASTA or cache, it behaves like the conservative
  no-external-reference attempt above. With `--reference-cache`, it returns
  `unimplemented` because cache-backed decoding is not implemented.
* `allow-cache` is reserved for cache-backed CRAM decoding and returns
  `unimplemented` in this slice, whether or not the cache path exists.
* `--reference-cache` is a recorded request field only until cache-backed
  decoding is implemented; it is not searched, populated, or used for fallback.

These rules are governed by the `consume.reference` JSON object:
`policy`, `explicit_reference_provided`, `reference_cache_provided`,
`cram_inputs_present`, optional `source_used`, and optional
`decode_without_external_reference`.

## M13.4 CRAM Indexed Query Position

M13.4 freezes CRAI and indexed CRAM queries as **unsupported/deferred** for
Milestone 13. They are not transitional behavior and not native work in this
milestone.

The contract is deliberately narrow:

* CRAI files and adjacent `.crai` sidecars are not discovered, parsed, planned,
  or used by public commands in M13.
* `consume` remains a sequential CRAM normalization path governed by
  `consume.reference`; it does not use CRAI and performs no random-access
  traversal.
* `check_map --region`, `summary --region`, and `select_region` do not accept
  CRAM region input and remain scoped to the existing BAM/BAI native
  random-access and scan-fallback contracts.
* `index` does not create CRAI. `index --format csi` and BAM index behavior
  remain separate from CRAM indexing.
* JSON outputs expose no JSON CRAI evidence and no CRAM indexed-query fields in
  this slice.
* Any future milestone that promotes indexed CRAM queries must introduce an
  explicit staged plan for fixtures, native or compatibility dependency
  boundaries, CLI contracts, schemas, examples, benchmarks, and regression
  tests before behavior changes.

## M13.5 CRAM Fixture And Oracle Boundary

M13.5 freezes the CRAM fixture set as a provenance-first plan rather than a
broad binary corpus. The fixture boundary follows the M13.2 compatibility-only
direction, the M13.3 reference/cache policy, and the M13.4 indexed-query
deferral.

Fixture status is:

* present provenance roots: `tiny.valid.cram.explicit_ref.source_sam` and
  `tiny.ref.primary`;
* planned derived alignment fixtures:
  `tiny.valid.cram.explicit_ref.source_bam`,
  `tiny.valid.cram.explicit_ref`,
  `tiny.valid.cram.reference_required`,
  `tiny.valid.cram.compatible_refdict`,
  `tiny.valid.bam.compatible_refdict`, and
  `tiny.valid.bam.incompatible_refdict`;
* deferred no-external-reference fixture:
  `tiny.valid.cram.no_external_ref`;
* no CRAI fixture in M13.5. `.crai` artifacts, indexed CRAM fixtures, and CRAM
  random-access oracle outputs are explicitly deferred.

The source SAM and FASTA remain the auditable source of truth. Derived BAM and
CRAM files must be regenerated from those sources with documented commands and
reviewed as derived artifacts, not treated as opaque authorities.

Oracle use is limited to fixture generation, fixture validation, and test-only
compatibility checks. `noodles` or external tools may help produce or compare
derived CRAM artifacts only under that explicit test/fixture boundary; they
must not define production behavior, reference-cache semantics, CRAI behavior,
indexed CRAM traversal, or native BAM/BGZF/FASTQ hot paths.

## M13.6 Implemented CRAM Boundary

M13.6 implements only the chosen compatibility behavior and adds a concrete
guard against accidental indexed-CRAM expansion:

* CRAM remains accepted only as consume-only alignment-mode normalization.
* Strict, explicit-FASTA, allow-embedded, allow-cache, and auto-conservative
  reference behavior remains the M13.3 behavior.
* Cache-backed CRAM decoding remains `unimplemented`.
* Directory discovery skips adjacent `.crai` sidecars before probing with
  reason `cram_index_sidecar_deferred`.
* A directly requested `.crai` path is rejected before probing as
  `unsupported_format`.
* CRAI bytes are not parsed, planned, or used for traversal, and no native CRAM
  parser or writer is introduced.
* Direct production `noodles_*` imports remain confined to
  `src/ingest/cram.rs`.

## M13.7 CRAM-Facing Contract Refresh

M13.7 updates the governed command contracts for the M13.6 behavior without
expanding CRAM support:

* `consume` is the only command with changed CRAM-facing behavior in this
  slice.
* `spec/jsonschema/consume.schema.json` documents
  `cram_index_sidecar_deferred` as the directory-discovery skipped-file reason
  for `.crai` sidecars and keeps direct `.crai` requests represented as
  `unsupported_format` failures before discovery payload population.
* `spec/examples/consume.success.crai_sidecar_skipped.json` is the canonical
  directory-discovery example for a skipped `.crai` sidecar beside a CRAM input.
* `spec/examples/consume.failure.crai_sidecar_direct.json` is the canonical
  direct-request failure example for a `.crai` input.
* Inspection commands and region commands do not gain CRAM behavior in M13.7:
  `check_map --region`, `summary --region`, `select_region`, `check_index`,
  and `index` keep their existing BAM/BAI/CSI contracts and expose no CRAM
  indexed-query fields.

## M13.8 Public Docs And Schema Inventory

M13.8 consolidates the public documentation, schema, and example inventory for
the M13.6-M13.7 boundary without adding runtime behavior. The governed public
artifacts are:

* `spec/jsonschema/consume.schema.json`, including the M13 CRAM contract
  metadata that records consume-only compatibility normalization,
  `cram_index_sidecar_deferred`, direct `unsupported_format` rejection, and no
  CRAM indexed-query fields;
* `spec/examples/consume.success.crai_sidecar_skipped.json`, the canonical
  directory-discovery example for skipped `.crai` sidecars;
* `spec/examples/consume.failure.crai_sidecar_direct.json`, the canonical
  direct-request failure example for `.crai` inputs;
* `spec/cli/commands.md`, `README.md`, `docs/cli.md`, `docs/json-output.md`,
  this roadmap, the current-milestone note, Sphinx native CRAM notes, and the
  task map.

M13.8 explicitly leaves inspection, region, indexing, and benchmark contracts
unchanged for CRAM. It adds no CRAI parser, no CRAM region input, no CRAI
creation, no CRAM random-access evidence, no benchmark CRAM indexed-query
evidence, and no CRAM indexed-query JSON fields.

## M13.9 CRAM Dependency And Benchmark Guardrails

M13.9 strengthens the compatibility-only decision with explicit dependency and
benchmark guardrails:

* `tests/contract/dependency_boundary.rs` names `cram_consume_boundary`,
  `non_cram_indexed_surfaces`, and `cram_benchmark_guardrail` as protected M13
  surfaces.
* Direct production `noodles_*` references remain allowed only in
  `src/ingest/cram.rs`; `check_map`, `summary`, `select_region`, `check_index`,
  `index`, native index modules, region modules, and `scanner_microbench` must
  remain free of direct `noodles` imports.
* `scanner_microbench` remains synthetic BAM-only. It emits no CRAM command
  timing rows, and the existing `consume` timing row is BAM alignment ingest
  only.
* `benchmarks/results/scanner_microbench.schema.json`, benchmark result docs,
  and Sphinx benchmark docs record that M13.9 benchmark evidence does not cover
  CRAI parsing, CRAM region input, CRAM random-access traversal, CRAM
  indexed-query evidence, CRAM compatibility throughput, or CRAM comparator
  parity.

## M13.10 Closeout

Milestone 13 is complete as of 2026-06-01. M13.1 through M13.10 completed the
native CRAM strategy and compatibility-boundary checkpoint without promoting a
native CRAM substrate or expanding CRAM indexed-query behavior.

Closeout verification passed:

* `cargo test`;
* `cargo test --test contract`;
* `sphinx-build -b html docs/sphinx docs/sphinx/_build/html`;
* `cargo fmt --check`;
* `git diff --check`;
* `cargo build --bin bamana --bin scanner_microbench`;
* `target/debug/scanner_microbench --profile small --iterations 1 --bamana-bin target/debug/bamana --out /tmp/bamana-m1310-scanner-small.json`.

Scanner smoke evidence confirmed 28 command timing rows, no CRAM command
timing rows, and the M13.9 CRAM benchmark guardrail note in the emitted
result.

Residual risk remains explicit and deferred for native CRAM parsing, native
CRAM writing, CRAI parsing, CRAI creation, indexed CRAM queries, CRAM region
input, CRAM random-access traversal, cache-backed CRAM decoding, derived
CRAM/BAM compatibility fixtures, no-external-reference CRAM fixtures, CRAM
compatibility throughput claims, CRAM comparator parity, and broad external
tool parity.

## Ten-Task Outline

1. M13.1 activate scope and audit current CRAM ingestion/reference-policy
   behavior.
2. M13.2 decide native CRAM promotion, compatibility-only continuation, or
   explicit deferral. Complete: compatibility-only continuation.
3. M13.3 freeze reference discovery, explicit FASTA, embedded-reference, and
   cache policy semantics. Complete.
4. M13.4 define whether CRAM indexed queries are unsupported, transitional, or
   native work. Complete: unsupported/deferred.
5. M13.5 add CRAM fixtures and oracle boundaries for the chosen decision.
   Complete: provenance-first plan with derived fixtures planned and CRAI
   fixtures deferred.
6. M13.6 implement only the chosen CRAM behavior with documented dependency
   boundaries. Complete: CRAI sidecars are actively skipped or rejected while
   CRAM remains consume-only compatibility normalization.
7. M13.7 update `consume` and inspection command contracts where CRAM behavior
   changes. Complete: consume schema/examples refreshed; inspection and region
   command contracts remain unchanged for CRAM.
8. M13.8 update schemas, examples, README, CLI docs, Sphinx docs, and roadmap
   notes. Complete: public docs and schema inventory consolidated without
   runtime CRAM expansion.
9. M13.9 add CRAM dependency-boundary and benchmark guardrails. Complete:
   dependency tests and benchmark docs/schema now guard the compatibility-only
   CRAM boundary.
10. M13.10 close the milestone with full verification and residual risk notes.
    Complete: full verification and scanner smoke evidence passed; residual
    CRAM risk remains explicitly deferred.

## Non-Goals

M13 does not weaken Bamana-native BAM, BGZF, FASTQ, sampling, ingest, or
forensic hot-path ownership. `noodles` remains limited to documented CRAM
compatibility, tests, fixtures, compatibility checks, and oracle validation
unless a later contract changes that explicitly.
