# Milestone 15: Release Hardening And Public Contract Freeze

Status: active as of 2026-06-01. Milestone 15 is the release-boundary
hardening milestone for the public command surface accepted up to that point.

## Goal

Prepare Bamana for a coherent release boundary: stable CLI contracts, complete
documentation, reproducible verification, packaging checks, and explicit
operational support guarantees.

## M15.1 Activation And Release Scope

M15.1 activates release hardening after Milestone 14 closeout. This is a
scope-freeze step: it does not add commands, change command behavior, change
JSON schemas, or promote benchmark/comparator claims. Later M15 tasks must
either freeze, document, or explicitly exclude each release-facing surface.

The release boundary accepts the currently implemented and documented CLI
commands as the starting inventory:

* `benchmark`
* `identify`
* `enumerate`
* `subsample`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`
* `annotate_rg`
* `consume`
* `explode`
* `fastq`
* `checksum`
* `merge`
* `reheader`
* `sort`
* `select_region`
* `unmap`
* `verify`
* `check_eof`
* `header`
* `check_map`
* `check_index`
* `index`
* `summary`
* `validate`
* `check_tag`
* `check_sort`

`benchmark`, `fastq`, and `unmap` remain named public contract commands and
must retain governed schemas, examples, CLI docs, Sphinx docs, contract tests,
and benchmark/no-claim notes where applicable.

Acceptance into the M15 release boundary means the command is eligible for
release hardening only as currently specified. It does not imply broad
external-tool comparator parity, release performance promises, biological
equivalence, CRAM indexed-query behavior, CSI writing, native CRAM parsing or
writing, public `select_region --region-file`, binary stdout output for
selected records, replacement output-index creation, or unmeasured benchmark
claims for `fastq`, `unmap`, or `identify`.

M15.1 records the release boundary in README, CLI docs, Sphinx docs, roadmap,
taskmap, and contract tests so later M15 tasks can audit completeness without
silently expanding scope.
Acceptance is limited to current schemas, examples, CLI docs, Sphinx docs,
roadmap notes, and contract tests.

## Ten-Task Outline

1. M15.1 activate release scope and list public commands accepted into the
   release boundary. Complete: current CLI command inventory recorded with
   public contract commands and deferred surfaces explicitly named.
2. M15.2 freeze CLI option semantics and JSON output compatibility rules.
3. M15.3 audit every schema, example, fixture, benchmark note, and Sphinx page.
4. M15.4 define deprecation, compatibility, and versioning policy.
5. M15.5 harden installation, packaging, binary naming, and release artifact
   checks.
6. M15.6 add CI verification for formatting, tests, contracts, Sphinx, schemas,
   examples, and smoke benchmarks.
7. M15.7 audit operational failure modes and error-message quality.
8. M15.8 update README, user docs, technical docs, roadmap, and release notes.
9. M15.9 add final dependency-boundary and license/supply-chain checks.
10. M15.10 close the release hardening milestone with archived verification
    evidence and explicit known limitations.

## Non-Goals

M15 does not require completing every aspirational Bamana feature. It freezes a
coherent release boundary around the behavior actually implemented, documented,
tested, and benchmarked.
