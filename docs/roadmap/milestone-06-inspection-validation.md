# Milestone 6: Native Inspection And Validation Commands

Status: active as of 2026-05-21.

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

* command-level smoke timings for `check_eof`
* command-level smoke timings for `check_sort`
* command-level smoke timings for `check_map`
* command-level smoke timings for `summary`
* command-level smoke timings for `check_tag`
* command-level smoke timings for `validate`

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
