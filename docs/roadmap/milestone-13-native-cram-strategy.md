# Milestone 13: Native CRAM Strategy And Compatibility Boundary

Status: active as of 2026-06-01. Milestone 13 is the first explicit
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

## Ten-Task Outline

1. M13.1 activate scope and audit current CRAM ingestion/reference-policy
   behavior.
2. M13.2 decide native CRAM promotion, compatibility-only continuation, or
   explicit deferral. Complete: compatibility-only continuation.
3. M13.3 freeze reference discovery, explicit FASTA, embedded-reference, and
   cache policy semantics.
4. M13.4 define whether CRAM indexed queries are unsupported, transitional, or
   native work.
5. M13.5 add CRAM fixtures and oracle boundaries for the chosen decision.
6. M13.6 implement only the chosen CRAM behavior with documented dependency
   boundaries.
7. M13.7 update `consume` and inspection command contracts where CRAM behavior
   changes.
8. M13.8 update schemas, examples, README, CLI docs, Sphinx docs, and roadmap
   notes.
9. M13.9 add CRAM dependency-boundary and benchmark guardrails.
10. M13.10 close the milestone with full verification and residual risk notes.

## Non-Goals

M13 does not weaken Bamana-native BAM, BGZF, FASTQ, sampling, ingest, or
forensic hot-path ownership. `noodles` remains limited to documented CRAM
compatibility, tests, fixtures, compatibility checks, and oracle validation
unless a later contract changes that explicitly.
