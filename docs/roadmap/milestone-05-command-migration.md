# Milestone 5: Command Migration Off `noodles`

Status: complete as of 2026-05-20.

## Technical Goal

Use the substrate milestones to migrate the first proof commands off external
hot-path dependencies in this order:

1. `verify`
2. `header`
3. `subsample`

## Owned Modules

Primary ownership:

* `src/bgzf/`
* `src/bam/header.rs`
* `src/bam/reader.rs`
* `src/bam/write.rs`
* `src/fastq/`
* `src/sampling/`
* command orchestration in `src/commands/verify.rs`,
  `src/commands/header.rs`, and `src/commands/subsample.rs`

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF
* Milestone 2 native BAM header codec
* Milestone 3 native BAM record scanner
* Milestone 4 native FASTQ parser for full `subsample` coverage

## Why This Order

* `verify` is header-level and proves BGZF plus header ownership quickly
* `header` is the natural follow-on proof of header codec ownership
* `subsample` is the first strong end-to-end scan and transform proof across
  BAM and FASTQ

## Acceptance Criteria

* `verify` uses native BGZF and native header path only
* `header` uses native header codec only
* `subsample` uses native BAM scanning and native FASTQ parsing
* command JSON contracts remain stable
* differential tests and fixtures continue to pass
* `noodles` no longer appears in production code paths for these commands

## Benchmark Hooks

* benchmark `verify` before and after migration
* benchmark `header` before and after migration
* benchmark `subsample` before and after migration
* capture command-level deltas in the benchmark framework where possible

## M5.1 Baseline Audit

M5.1 activated this milestone after Milestone 4 closed. The baseline audit is
documentation-only and does not change command behavior.

Already-native proof paths:

* `verify` probes the path, requires a BGZF-backed BAM container, and calls
  `parse_bam_header_from_native_bgzf`;
* `header` uses the same native BGZF and BAM header parse path and returns the
  native `HeaderPayload`;
* FASTQ and FASTQ.GZ `subsample` paths use the native FASTQ reader, record, and
  writer core from Milestone 4.

Remaining migration target:

* BAM `subsample` currently uses `BamReader::open`,
  `parse_bam_header_from_reader`, `read_next_record_layout`,
  `serialize_record_layout`, and `BgzfWriter`. This path is free of direct
  `noodles` imports, but it is not yet proven through `BamScanner` or a
  scanner-owned raw-record bridge.

Contract and benchmark surfaces:

* `benchmark`, `fastq`, and `unmap` remain public contract commands protected
  by contract tests for schemas, examples, and CLI documentation;
* `verify`, `header`, and `subsample` already have JSON schemas, examples, and
  CLI documentation that M5 migration work must preserve or deliberately
  version;
* `header_microbench` can time `verify` and `header` when supplied a Bamana
  binary, and the benchmark framework contains `subsample_only` workflow
  variants for command-level subsample timing.

## M5.2 Contract Freeze

M5.2 freezes the proof-command contract baseline before deeper migration work.
No intentional command contract changes are introduced by this task.

Frozen proof-command surfaces:

* `verify`, `header`, and `subsample` each have a JSON schema plus canonical
  success and failure examples;
* `spec/cli/commands.md`, `docs/cli.md`, and `docs/json-output.md` describe
  the supported behavior and limits for the three proof commands;
* focused contract tests now fail if any proof-command schema, example, or
  governed documentation surface disappears;
* fixture manifest coverage now reserves BAM, FASTQ, FASTQ.GZ, malformed
  FASTQ, and truncated BAM inputs for `subsample` before implementation
  migration changes begin.

Reserved `subsample` fixture baseline:

* `tiny.clean.bam`
* `tiny.clean.fastq`
* `tiny.valid.fastq_gz`
* `tiny.invalid.fastq.truncated`
* `tiny.invalid.bam.truncated_record`

## M5.3 `verify` Native Migration

M5.3 confirms the `verify` proof command as a completed native migration. The
production command path remains limited to:

1. shallow path probing;
2. BGZF-backed BAM container confirmation;
3. native BAM magic and header/reference-dictionary parsing through
   `parse_bam_header_from_native_bgzf`.

The task does not expand `verify` into alignment-record validation, full BAM
body validation, or BGZF EOF-marker checking. EOF-marker checks remain owned by
`check_eof`, and deeper body checks remain owned by `validate`.

Additional hardening records that a valid BGZF BAM header can verify even when
the canonical BGZF EOF marker is absent. That behavior is deliberate: missing
EOF is outside the `verify` contract.

Command-level benchmark evidence is recorded through `header_microbench` with
`--bamana-bin`, which times both `verify` and `header` against the generated
native header fixture.

## M5.4 `header` Native Migration

M5.4 confirms the `header` proof command as a completed native migration. The
production command path remains limited to:

1. shallow path probing;
2. BGZF-backed BAM container confirmation;
3. native BAM magic, header text, and binary reference-dictionary parsing
   through `parse_bam_header_from_native_bgzf`;
4. returning the native `HeaderPayload`.

The command remains header-only. It does not validate alignment records, does
not validate the complete BAM body, and does not report BGZF EOF-marker status.
Additional hardening records that malformed alignment-body bytes after an
otherwise valid header do not make `header` fail.

Command-level benchmark evidence is recorded through `header_microbench` with
`--bamana-bin`, which times the `header` command against the generated native
header fixture.

## M5.5 BAM-Side `subsample` Scanner Migration

M5.5 migrates BAM-side `subsample` traversal to `BamScanner`. The command now
opens BAM input through the scanner, reuses the scanner-owned native header for
output header serialization, applies filters and sampling decisions from
`BamRecordView`, and writes retained records from `BamRecordView::raw_record`.

This is an explicit scanner-owned raw-record bridge. It preserves output record
bytes for retained records without rebuilding every BAM record through the
older transitional `BamReader::open` plus `read_next_record_layout` loop.

Preserved semantics:

* deterministic selection remains hash-based over the configured identity;
* seeded-random selection remains reproducible for a fixed seed;
* `--mapped-only` and `--primary-only` are applied before sampling;
* retained records preserve input encounter order;
* pre-existing BAM index invalidation remains explicit in the JSON payload and
  notes;
* JSON contracts remain unchanged.

## M5.6 FASTQ-Side `subsample` M4 API Migration

M5.6 records FASTQ-side `subsample` as complete on the stable Milestone 4
FASTQ APIs. Plain FASTQ and FASTQ.GZ inputs are opened through
`open_fastq_reader`, streamed as owned `FastqRecord` values via
`read_next_fastq_record`, selected with `FastqRecord` identity bytes, and
written through `FastqWriter`.

Preserved semantics:

* deterministic selection remains hash-based over the configured raw-read
  identity;
* seeded-random selection remains reproducible for a fixed seed;
* retained FASTQ records preserve input encounter order;
* header comments, plus-line comments, sequences, and qualities are preserved
  by the M4 record model and writer;
* FASTQ.GZ output compression remains inferred from the output filename
  extension, including the staged temporary output path;
* BAM-only flags and index creation remain rejected before FASTQ streaming
  begins;
* JSON contracts remain unchanged.

## M5.7 Dependency Boundary Strengthening

M5.7 makes the proof-command dependency boundary explicit in contract tests.
The M5 proof-command set is:

* `verify`
* `header`
* `subsample`

`tests/contract/dependency_boundary.rs` now protects the command files and
their native substrate hot paths from direct `noodles` imports. The global
production dependency guard remains in place and still allows direct production
`noodles` usage only in the documented CRAM compatibility boundary:
`src/ingest/cram.rs`.

Test-only oracle policy is documented in `docs/testing-oracles.md`. Header
oracle usage remains isolated to `tests/header_oracle.rs`; BAM and FASTQ
`subsample` expectations are owned by native scanner and native FASTQ command
tests unless a future differential test is explicitly labelled as an oracle or
compatibility comparison.

## M5.8 `subsample` Fixture And Differential Coverage

M5.8 strengthens `subsample` fixture-style evidence across BAM, FASTQ, and
FASTQ.GZ inputs.

Command tests now cover:

* deterministic BAM selection through the scanner-owned raw-record bridge;
* seeded-random BAM selection through the scanner-owned raw-record bridge;
* deterministic FASTQ selection through the Milestone 4 reader and writer;
* seeded-random FASTQ.GZ selection through the Milestone 4 reader and writer;
* retained-record encounter order and ordered record digests for BAM, FASTQ,
  and FASTQ.GZ when all records are retained.

The ordered digest checks compare output records against the source fixture
records after command execution, proving retained record bytes or FASTQ record
structure remain stable for the covered tiny inputs.

Governed failure examples now cover:

* unsupported input format;
* invalid fraction;
* invalid FASTQ filter/index combinations for BAM-only controls.

## M5.9 Proof-Command Benchmark Evidence

M5.9 records runnable proof-command benchmark hooks for the Milestone 5 command
set:

* `header_microbench --bamana-bin` emits command-level smoke timings for
  `verify` and `header`;
* `scanner_microbench --bamana-bin` emits `subsample_bam` command-level
  dry-run timing over a generated BAM fixture;
* `fastq_microbench --bamana-bin` emits `subsample_fastq` and
  `subsample_fastq_gz` command-level dry-run timings over generated FASTQ and
  FASTQ.GZ fixtures.

The benchmark schemas under `benchmarks/results/` include the new subsample
command-timing rows, so the outputs remain machine-readable and archivable.

Smoke evidence from the M5.9 implementation run:

* `target/debug/header_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana` reported `verify: 1/1` and `header: 1/1`;
* `target/debug/scanner_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana` reported `subsample_bam: 1/1`;
* `target/debug/fastq_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana` reported `subsample_fastq: 1/1` and
  `subsample_fastq_gz: 1/1`.

These command timings are smoke timings. They include process startup, CLI
parsing, path probing, command execution, and JSON emission. They are not pure
substrate microbenchmarks and should not be compared directly with in-process
BGZF, scanner, or FASTQ throughput rows.

## M5.10 Closeout

M5.10 closes Milestone 5 with the proof-command set fully recorded:

* `verify` is a native BGZF plus native BAM header proof command. It remains
  header-level verification and does not claim alignment-record validation,
  full BAM body validation, or BGZF EOF-marker checking.
* `header` is a native BAM header extraction proof command. It remains
  header-only and does not validate alignment records or full BAM body
  readability.
* `subsample` uses native BAM scanner traversal for BAM inputs and the native
  FASTQ/FASTQ.GZ reader and writer APIs for raw-read inputs.
* command JSON contracts remained stable.
* dependency-boundary tests keep direct production `noodles` usage isolated to
  CRAM compatibility while separately protecting the M5 proof-command hot
  paths.
* fixture and differential tests cover BAM, FASTQ, and FASTQ.GZ `subsample`
  selection, retained-record evidence, and encounter-order preservation.

Closeout verification completed with:

* `cargo test`;
* `cargo test --test contract`;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`;
* `header_microbench`, `scanner_microbench`, and `fastq_microbench` smoke
  profiles using `--profile small --iterations 1 --bamana-bin
  target/debug/bamana`.

The final smoke benchmark evidence reported `verify: 1/1`, `header: 1/1`,
`subsample_bam: 1/1`, `subsample_fastq: 1/1`, and
`subsample_fastq_gz: 1/1`.

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

## Risks / Follow-Up

* command migrations in later milestones must not regress JSON contract
  stability
* benchmark regressions should be treated as real signals, not postponed
* Milestone 6 should activate the inspection and validation command wave
  without broadening Milestone 5's proof-command claims
