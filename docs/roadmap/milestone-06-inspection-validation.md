# Milestone 6: Native Inspection And Validation Commands

Status: complete as of 2026-05-21.

## Technical Goal

Use the native BGZF, BAM header, BAM scanner, FASTQ, and proof-command
substrates to harden the first operational inspection and validation command
wave:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

## Owned Modules

Primary ownership:

* `src/bgzf/`
* `src/bam/header.rs`
* `src/bam/scan.rs`
* `src/bam/record.rs`
* `src/bam/tags.rs`
* `src/bam/index.rs`
* command orchestration in `src/commands/check_eof.rs`,
  `src/commands/check_sort.rs`, `src/commands/check_map.rs`,
  `src/commands/summary.rs`, `src/commands/check_tag.rs`, and
  `src/commands/validate.rs`

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF
* Milestone 2 native BAM header codec
* Milestone 3 native BAM record scanner
* Milestone 5 proof-command migration

Milestone 4 FASTQ work is not a direct dependency for BAM inspection, but the
project should keep dependency-boundary and benchmark discipline consistent
across both native cores.

## Commands Enabled Or Hardened

Primary beneficiaries:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

## Acceptance Criteria

* `check_eof` uses the native BGZF EOF-marker path and documents its narrow
  boundary
* `check_sort`, `check_map`, `summary`, `check_tag`, and `validate` use native
  BAM scanner/header primitives for their scanner-compatible paths
* command JSON contracts remain stable or are deliberately versioned
* fixture and malformed-input coverage proves bounded versus full-scan
  behavior where relevant
* dependency-boundary tests protect the inspection and validation hot paths
  from direct `noodles` imports

## Benchmark Hooks

* command-level smoke timing for `check_eof` through `bgzf_microbench
  --bamana-bin`
* command-level smoke timings for `check_sort`, `check_map`, `summary`,
  `check_tag`, and `validate` through `scanner_microbench --bamana-bin`
* benchmark interpretation notes must distinguish substrate timing from
  command timing and must not imply comparator parity with external tools

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

Disallowed:

* production BAM inspection or validation hot paths through `noodles`

## Risks / Follow-Up

* `validate` must not overclaim full biological or reference-level correctness
* `check_map` must keep index-derived and scan-derived evidence distinct
* bounded scans must continue to state when absence or validity is not proven

## M6.1 Baseline Audit

M6.1 activates this milestone after Milestone 5 closeout. The baseline audit is
documentation-only and does not change command behavior.

Already-native or native-backed command paths:

* `check_eof` probes the input as BAM/BGZF and uses the native BGZF EOF-marker
  detection path;
* `check_sort` opens BAM input through `BamScanner` and uses
  `BamRecordView`-derived fields for coordinate and queryname ordering
  evidence;
* `check_map` keeps usable BAI-derived summaries as the preferred evidence
  source and falls back to `BamScanner` record traversal when an index is not
  usable or not requested;
* `summary` opens BAM input through `BamScanner`, uses native header metadata,
  can incorporate BAI-derived totals, and otherwise reports bounded or full
  scanner evidence;
* `check_tag` uses `BamScanner` with native aux traversal helpers for selected
  tag lookup and type filtering;
* `validate` runs through the native validation substrate built on header
  parsing, `BamScanner`, and `BamRecordView`.

Baseline gaps for later M6 tasks:

* contracts and examples need a fresh milestone-level freeze for all six
  commands;
* bounded-scan absence claims and full-scan claims need focused fixture
  evidence;
* `check_map` and `summary` need continued care around index-derived versus
  scan-derived evidence;
* `validate` needs explicit hardening around structural-only validation
  claims;
* command-level smoke benchmark evidence is still needed for the full M6
  command set;
* dependency-boundary tests should name the six M6 command files as one
  protected milestone set.

## M6.2 Contract Freeze

M6.2 freezes the inspection and validation command contract baseline. No
intentional command behavior or JSON shape changes are introduced by this task.

Frozen command surfaces:

* `check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
  `validate` each have a JSON schema plus canonical success and failure
  examples;
* `spec/cli/commands.md`, `docs/cli.md`, `docs/json-output.md`, README, and
  Sphinx documentation describe the evidence scope and caveats for all six
  commands;
* contract tests fail if any M6 command schema, success example, failure
  example, CLI documentation, or JSON-output documentation disappears;
* fixture planning reserves missing EOF, sorted and unsorted BAMs,
  mapped/unmapped evidence, absent tags, malformed aux payloads, BAI-derived
  mapping and summary evidence, and structural validation failures.

M6 hardening work must preserve the distinction between bounded evidence and
full-file claims. Bounded scan output may describe only examined records.
Full-file absence, full-file summary, or full validation claims require a
complete scan that reaches EOF cleanly.

## M6.3 `check_eof` Native BGZF Boundary

M6.3 hardens `check_eof` as the native BGZF EOF-marker command. The production
command path remains limited to:

1. shallow path probing;
2. BGZF-backed BAM container confirmation;
3. canonical BGZF EOF-marker tail detection through `bgzf::has_bgzf_eof`.

The task does not expand `check_eof` into BAM magic parsing, BAM header
parsing, alignment-record validation, or auxiliary-field validation. A BGZF
stream with invalid BAM header bytes can pass `check_eof` when the canonical
EOF marker is present. That behavior is deliberate: `check_eof` proves only
tail EOF-marker evidence. Use `verify` for native BAM header checks and
`validate` for deeper structural validation.

Additional hardening records present EOF, missing EOF, too-short tail, non-BGZF
BAM input, and invalid-BAM-payload-with-present-EOF cases. Command-level smoke
timing remains available through `bgzf_microbench --bamana-bin`, which emits a
`check_eof` command timing row.

## M6.4 `check_sort` Scanner Evidence

M6.4 hardens `check_sort` as a native scanner-backed ordering-evidence command.
The production command path remains:

1. shallow path probing;
2. BGZF-backed BAM container confirmation;
3. native BAM header parsing through `BamScanner::open`;
4. record-order evidence from `BamRecordView` coordinates, flags, and read
   names.

The task preserves the distinction between bounded and strict evidence.
Bounded mode reports only the sampled window. `--strict` continues sequential
inspection until EOF or a stronger conclusion, but it still does not perform
full BAM structural validation. Specialized sort modes are preserved from the
header and reported with limited observed confirmation when full specialized
confirmation is outside the current slice.

Additional hardening records coordinate sort, queryname sort, specialized
template-coordinate sort, unknown declared sort order, bounded-scan caveats,
and strict violation detection after a bounded sample window. Command-level
smoke timing is available through `scanner_microbench --bamana-bin`, which now
emits a `check_sort` command timing row.

## M6.5 `check_map` Index And Scanner Evidence

M6.5 hardens `check_map` as an index-preferred mapping-evidence command with a
native scanner fallback. The production command path remains:

1. shallow BAM/BGZF probing;
2. native BAM header opening through `BamScanner::open`;
3. optional adjacent-index discovery and BAI metadata parsing;
4. index-derived payload construction when complete per-reference metadata is
   available;
5. native scanner traversal through `BamRecordView` flag and reference fields
   when index evidence is unavailable or explicitly disabled.

Index-derived output reports `evidence_source=index`, `index.used=true`, and no
`summary.records_examined` value because alignment records were not scanned for
the result. Scan-derived output reports `evidence_source=scan`,
`summary.records_examined`, observed mapped/unmapped counts, per-reference
observations, and inconsistent-record observations. Missing, incomplete,
unsupported, or malformed indexes remain visible in the `index` object and
`semantic_note` while still allowing scanner fallback where the BAM stream is
readable.

Additional hardening records usable BAI summaries, missing indexes, incomplete
BAI metadata, invalid BAI fallback, explicit index disabling, bounded-scan
caveats, and full-scan observation after a bounded window would have missed a
mapped record. Command-level smoke timing is available through
`scanner_microbench --bamana-bin`, which now emits a `check_map` command timing
row.

## M6.6 `summary` Scanner Evidence

M6.6 hardens `summary` as a native operational-overview command. The production
command path remains:

1. shallow BAM/BGZF probing;
2. native BAM header opening through `BamScanner::open`;
3. optional adjacent-index discovery and BAI metadata parsing;
4. scanner traversal through `SummaryAccumulator` and `BamRecordView` for
   record counts, mapping observations, MAPQ, flags, and anomalies;
5. payload construction that keeps index-derived totals separate from
   scan-derived operational counts.

Bounded output reports `mode=bounded_scan`, `evidence.full_file_scanned=false`,
`fractions_observed`, and no `counts.records_total_known` value. Full-scan
output reports `mode=full_scan`, `evidence.full_file_scanned=true`,
`counts.records_total_known`, and full-file fractions only after EOF is reached
cleanly. Header-only BAM bodies are handled as valid zero-record operational
evidence, and malformed alignment records return an indeterminate failure
payload rather than a misleading partial summary.

Additional hardening records header-only summaries, BAI-assisted summaries,
bounded summaries, full-scan summaries, malformed record failures, and
index-derived totals kept separate from scanner counts. Command-level smoke
timing remains available through `scanner_microbench --bamana-bin`, which emits
a `summary` command timing row.

## M6.7 `check_tag` Aux Traversal Evidence

M6.7 hardens `check_tag` as a native scanner-owned auxiliary traversal command.
The production command path remains:

1. shallow BAM/BGZF probing;
2. native BAM header opening through `BamScanner::open`;
3. record traversal through borrowed `BamRecordView` values;
4. auxiliary traversal through `record_aux_contains_tag`;
5. result construction that distinguishes observed presence, bounded
   non-observation, complete-scan absence, and indeterminate traversal.

The task preserves the command's evidence boundary. `records_with_tag` counts
records with at least one matching tag rather than duplicate occurrences inside
one record. `--require-type` filters matches by BAM auxiliary type code.
Malformed auxiliary payloads and unsupported B-array shapes remain structured
`tag_parse_uncertainty` failures with an indeterminate payload, not successful
absence claims.

Additional hardening records present tags, bounded absent tags, complete-scan
absent tags, type-filter mismatches, duplicate tag records, malformed aux
payloads, and unsupported B-array shapes. Command-level smoke timing remains
available through `scanner_microbench --bamana-bin`, which emits a `check_tag`
command timing row.

## M6.8 `validate` Structural Validation Boundary

M6.8 hardens `validate` as a native structural and internal-consistency
validation command. The production command path remains:

1. shallow BAM/BGZF probing;
2. native scanner opening through `BamScanner::open`;
3. header validation against native header metadata;
4. scanner-compatible record validation through borrowed `BamRecordView`
   fields;
5. native auxiliary traversal for duplicate-tag and malformed-aux structural
   findings;
6. severity-coded payload construction for header-only, bounded-record, and
   full validation scopes.

The task preserves the command boundary. Header-only validation does not imply
alignment-record validity. Bounded validation reports only the examined record
prefix. Full validation reports `summary.full_file_examined=true` only after
scanner traversal reaches EOF cleanly. Finding counters count all observed
error, warning, and info severities even when stored findings are limited by
`--max-errors` or `--max-warnings`.

Additional hardening records clean minimal BAM validation, header-only scope,
bounded-record scope, malformed record structure, malformed auxiliary
structure, severity-coded findings, and configured finding-list limits.
Command-level smoke timing remains available through
`scanner_microbench --bamana-bin`, which emits a `validate` command timing row.

## M6.9 Dependency Boundary And Benchmark Guardrails

M6.9 formalizes the milestone-level guardrails for the six-command inspection
and validation wave. Contract tests explicitly name `check_eof`, `check_sort`,
`check_map`, `summary`, `check_tag`, and `validate` as the M6 protected command
set, and their production command/substrate paths must remain free of direct
`noodles` imports.

Benchmark schema guards now cover the M6 command timing rows. `check_eof` is
timed by `bgzf_microbench --bamana-bin`; `check_sort`, `check_map`, `summary`,
`check_tag`, and `validate` are timed by `scanner_microbench --bamana-bin`.
These rows are command smoke timings. They include process startup, CLI
parsing, probing, JSON emission, and command payload construction. Scanner
command timings use deterministic synthetic BAM input and do not exercise
malformed-input behavior or adjacent-index evidence unless a future benchmark
profile explicitly adds those fixtures.

M6 benchmark notes are interpretation guardrails, not performance claims. They
distinguish native substrate timings from command timings, bounded/full scan
choices from index-derived evidence, and smoke-runnable commands from
comparator parity against external tools.

## M6.10 Closeout

Milestone 6 closed on 2026-05-21 after M6.1 through M6.10 completed. The
closeout confirms that Bamana's first operational BAM inspection and validation
wave is governed, documented, scanner-backed where applicable, and protected by
dependency-boundary tests.

Closed command surfaces:

* `check_eof` remains a narrow native BGZF EOF-marker inspection command;
* `check_sort` reports bounded or strict scanner-derived ordering evidence;
* `check_map` preserves index-preferred mapping evidence while keeping scanner
  fallback evidence distinct;
* `summary` reports operational header/index/scanner evidence without claiming
  full BAM validation;
* `check_tag` reports scanner-owned auxiliary traversal evidence and keeps
  bounded non-observation distinct from full-scan absence;
* `validate` reports structural and internal-consistency checks without
  claiming biological correctness, external reference concordance, or complete
  optional-field semantic validation.

Closeout verification:

* `cargo test`
* `cargo test --test contract`
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`
* `cargo build --bin bamana --bin bgzf_microbench --bin scanner_microbench`
* `bgzf_microbench --profile small --iterations 1 --bamana-bin target/debug/bamana`
  reported `verify:1/1, check_eof:1/1`
* `scanner_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana` reported `summary:1/1, check_sort:1/1, check_map:1/1,
  validate:1/1, check_tag:1/1, subsample_bam:1/1`

Production direct `noodles` usage remains isolated to documented CRAM
compatibility. Tests, fixtures, compatibility checks, and oracle-style
validation remain allowed surfaces.
