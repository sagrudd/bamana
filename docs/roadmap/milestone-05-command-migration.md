# Milestone 5: Command Migration Off `noodles`

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

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

## Risks / Follow-Up

* command migrations must not regress JSON contract stability
* benchmark regressions should be treated as real signals, not postponed
