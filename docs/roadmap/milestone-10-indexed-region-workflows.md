# Milestone 10: Native Indexed Region Workflows

Status: active as of 2026-05-23. M10.1 activated this milestone only after
Milestone 9 closeout evidence recorded native BAI writing, BAI validation,
typed virtual offsets, internal random-access helpers, and index-aware
`check_map`/`summary` evidence.

## Technical Goal

Turn the Milestone 9 index and random-access substrate into bounded,
user-visible indexed-region workflows without expanding into native CRAM
parsing or broad comparator parity.

Primary command and subsystem targets:

* indexed region parsing and normalization
* random-access chunk planning
* region-aware evidence paths for `check_map` and `summary`
* first public indexed-region command contract, if one is introduced
* fixture and benchmark coverage for indexed versus scan fallback behavior

## Owned Modules

Primary ownership:

* `src/bam/index.rs`
* `src/bgzf/virtual_offset.rs`
* `src/bgzf/reader.rs`
* `src/bam/scan.rs`
* `src/bam/record.rs`
* command orchestration in `src/commands/check_map.rs` and
  `src/commands/summary.rs`

Expected new or expanded support modules:

* region parsing and normalization
* chunk planning and chunk coalescing
* indexed read helpers above the native BGZF reader

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF and virtual-offset groundwork
* Milestone 3 native BAM record scanner
* Milestone 6 native inspection and validation command hardening
* Milestone 9 native BAM index writing and random-access groundwork

## Commands Enabled Or Hardened

Primary beneficiaries:

* region-aware `check_map`
* region-aware `summary`
* future indexed region selection command, if promoted into the public CLI

## M10.1 Baseline Audit

Present substrate:

* `src/bam/index.rs` owns native BAI parsing, implemented-depth validation,
  BAI bin/chunk and linear-index representation, `bai_bin_for_region`, CSI
  header detection, and adjacent sidecar classification.
* `src/bgzf/reader.rs` seeks by typed `VirtualOffset`, and `src/bam/scan.rs`
  exposes `raw_records_in_virtual_range` for bounded internal record retrieval
  between virtual offsets.
* `src/commands/check_map.rs` and `src/commands/summary.rs` already use usable
  BAI mapped/unmapped metadata as index-derived evidence and otherwise explain
  native scan fallback.
* fixture plans include valid coordinate BAM/BAI pairs, stale BAI, malformed
  BAI, mismatched-reference BAI, and CSI-header fixtures.

Known gaps:

* M10.2 defines the internal region-string grammar and interval-normalization
  model, but no public command flag consumes it yet;
* region-file input is explicitly deferred after M10.2;
* random-access chunk planning has not yet been promoted into public command
  behavior;
* overlapping-region, duplicate-region, and multi-reference semantics remain
  unspecified;
* region-aware `check_map` and `summary` payloads do not yet distinguish
  requested-region evidence from whole-file evidence;
* M10.9 now adds indexed-region benchmark smoke timing rows that compare
  lookup and random-access traversal behavior with scan fallback for the
  promoted read-only evidence commands;
* CSI large-reference behavior remains unsupported until a later scoped
  decision.

## M10.2 Region Syntax And Normalization

`src/bam/region.rs` defines the first native region syntax layer without
wiring it into public command behavior.

Supported region strings:

* `reference` requests a whole reference by exact BAM header dictionary name.
* `reference:start-end` requests a 1-based closed interval. Normalized output
  uses 0-based half-open coordinates with the original 1-based closed
  coordinates retained for reporting.
* multiple regions are accepted as an ordered list by `normalize_region_strings`
  and are preserved exactly in request order. M10.2 does not merge,
  deduplicate, sort, or overlap-resolve them.

Reference resolution:

* names are matched against the parsed BAM header reference dictionary;
* duplicate reference names are ambiguous and rejected;
* reference names containing colons are supported when the split is
  unambiguous;
* a string that is both an exact reference name and a valid
  `reference:start-end` interval is rejected as ambiguous.

Rejected region strings:

* empty strings or strings with leading/trailing whitespace;
* unknown references;
* non-numeric, zero, reversed, or out-of-range coordinates;
* zero-length whole-reference requests;
* ambiguous reference resolution.

Region-file status:

* BED-like and line-oriented region files are explicitly deferred for M10.2.
  `reject_region_file_request` returns a precise unimplemented error rather
  than silently treating a file as a region string.

## M10.3 Region Workflow Contracts And Fixtures

M10.3 freezes region-aware `check_map` and region-aware `summary` as the first
planned public command surfaces. A standalone indexed selection command remains
deferred.

Planned command flags:

* `check_map --region <REGION>` for mapping evidence scoped to one or more
  M10.2 region strings;
* `summary --region <REGION>` for operational summary evidence scoped to one
  or more M10.2 region strings;
* repeated `--region` values preserve request order and are not merged or
  deduplicated;
* region files remain deferred until a later M10 task promotes a file syntax.

Frozen payload contract:

* `region_scope.requested` records that region behavior was requested;
* `region_scope.source` is `cli_regions` for repeated `--region` values;
* `region_scope.coordinate_base` is
  `input_1_based_closed_output_0_based_half_open`;
* `region_scope.duplicate_policy` is
  `preserve_request_order_without_merging_or_deduplication`;
* `region_scope.regions` contains ordered normalized M10.2 intervals;
* `region_scope.execution` distinguishes `indexed`, `scan_fallback`, and
  `rejected`.

Fixture plan coverage must distinguish single-region indexed success,
multi-region indexed success, overlapping-region request-order behavior,
unknown-reference rejection, empty-region rejection, stale-index scan fallback,
missing-index scan fallback, and unsupported-index scan fallback.

## M10.4 Indexed Chunk Planning

`src/bam/region_plan.rs` implements the internal chunk-planning layer above the
M10.2 region parser and M9 BAI substrate.

Planning rules:

* normalized intervals are expanded into all candidate BAI bins across the BAI
  hierarchy;
* chunk collection consumes validated `BaiIndex` structures and not shallow
  sidecar detection;
* candidate chunks are sorted by typed virtual offsets and coalesced when they
  overlap or touch;
* no-hit intervals produce deterministic empty chunk plans;
* plan provenance records index path, index kind, reference name, reference
  index, requested region strings, candidate bins, candidate chunks, and
  coalesced chunks.

Structured rejection happens before indexed evidence can be claimed:

* unsupported index kinds, including CSI until scoped support is implemented;
* stale BAI sidecars;
* normalized regions whose reference index is incompatible with the BAI;
* empty region sets;
* impossible virtual-offset chunks where the start is not before the end.

## M10.5 Region-Bounded Traversal

`src/bam/region_traversal.rs` implements the internal traversal baseline above
M10.4 chunk plans. `traverse_planned_region_chunks` walks coalesced chunk
ranges with typed `VirtualOffset` values and `raw_records_in_virtual_range`,
not raw byte offsets.

Traversal rules:

* each retrieved record is parsed through the native BAM record view;
* records are filtered against normalized intervals by reference and overlap,
  because broad BAI bins and coalesced chunks may include records outside the
  requested regions;
* duplicate records from overlapping chunks or overlapping region requests are
  deduplicate by virtual-offset range;
* matched region strings are merged onto the returned record when one record
  overlaps more than one requested interval;
* no usable index remains visible as an explicit scan fallback through
  `NativeScanRequired`.

## M10.6 Region-Aware `check_map`

`check_map --region <REGION>` is public command behavior. Region requests are
normalized through M10.2, planned through M10.4 when a usable BAI is present,
and traversed through M10.5 by typed `VirtualOffset` ranges.

Command behavior:

* usable BAI sidecars produce `region_scope.execution: indexed`;
* indexed payloads include `index_path`, `chunks_traversed`,
  `raw_records_seen`, and `duplicate_records_suppressed`;
* missing, stale, unsupported, or invalid index state produces
  `region_scope.execution: scan_fallback`;
* fallback payloads include `fallback_mode: native_scan_required` and
  `scan_records_limit`;
* region-scoped counters use `region_records_examined`,
  `region_mapped_records_observed`, and
  `region_unmapped_records_observed`.

The region-scoped fields are deliberately distinct from whole-file
`total_mapped_reads`, `total_unmapped_reads`, and scan `records_examined`
fields. Unknown references, empty intervals, reversed intervals, and other
unsupported region strings fail with precise `invalid_region` errors.
## M10.7 Region-Aware `summary`

`summary --region <REGION>` is public command behavior. Region requests reuse
the M10.2 parser, M10.4 chunk planner, and M10.5 traversal layer used by
region-aware `check_map`.

Command behavior:

* usable BAI sidecars produce `region_scope.execution: indexed`;
* indexed payloads include `index_path`, `chunks_traversed`,
  `raw_records_seen`, and `duplicate_records_suppressed`;
* missing, stale, unsupported, or invalid index state produces
  `region_scope.execution: scan_fallback`;
* fallback payloads include `fallback_mode: native_scan_required` and
  `scan_records_limit`;
* region-scoped `counts`, `fractions_observed`, `mapq`, `mapping`,
  `anomalies`, and optional `flag_categories` describe only requested
  intervals;
* whole-file BAI totals are intentionally omitted from region-scoped
  `index_derived`; the index is used only to find records.

Unknown references, empty intervals, reversed intervals, and other unsupported
region strings fail with precise `invalid_region` errors. Region files and
standalone indexed region selection remain deferred.

## M10.8 Indexed Selection Decision

M10.8 deliberately defers a public indexed region selection command. The
implemented M10 surface is read-only evidence from `check_map --region <REGION>`
and `summary --region <REGION>`; no command claims to select, copy, or write
BAM records by region. No public synopsis exists for selected-record output.

The deferral is based on the missing public contract for output semantics,
header preservation, record ordering, duplicate-region behavior, index
invalidation or regeneration notes, and output write-safety behavior. Future
selection work can build on the completed M10 substrate: the M10.2 region
parser, M10.4 BAI chunk planner, M10.5 region traversal, M10.6
`check_map --region <REGION>`, and M10.7 `summary --region <REGION>`.

## M10.9 Dependency And Benchmark Guardrails

M10.9 strengthens the dependency and benchmark boundary for indexed-region
workflows. The protected substrate set is region parsing, BAI chunk planning,
random-access traversal, `check_map --region <REGION>`,
`summary --region <REGION>`, and scan fallback. Production direct `noodles`
imports remain limited to the documented CRAM compatibility path.

`scanner_microbench --bamana-bin` emits smoke timing rows for
`check_map_region_scan_fallback`, `summary_region_scan_fallback`,
`check_map_region_indexed`, and `summary_region_indexed`. The scan-fallback
rows run before a generated BAI sidecar exists; the indexed rows run after
`index_bam` creates that sidecar and therefore exercise index lookup, BAI chunk
planning, random-access traversal, region filtering, command startup, and JSON
emission. These timings do not claim broad comparator parity, native CRAM
indexed queries, biological interpretation, or selected-record output.

## Acceptance Criteria

* region strings and optional region files are parsed into a documented
  normalized interval model
* indexed chunk planning uses validated BAI or scoped CSI evidence only when
  that evidence is suitable for the request
* region-aware command payloads distinguish index-derived, random-access, and
  scan-fallback evidence
* region queries reject unknown references, impossible intervals, unsupported
  index kinds, and stale or unusable sidecars precisely
* scan fallback remains explicit, native, and bounded by documented behavior
* any new public command or flag is added to JSON schemas, examples, CLI docs,
  Sphinx docs, and contract tests before being treated as stable

## Benchmark Hooks

* indexed region lookup smoke timings for tiny coordinate BAM fixtures
* indexed versus full-scan timings for `check_map`
* indexed versus full-scan timings for `summary`
* fallback-scan timings when no usable index is present

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

Disallowed:

* production indexed BAM region parsing, chunk planning, or random-access hot
  paths through `noodles`

## Risks / Follow-Up

* region syntax must be deliberately small before it is made public
* BAI cannot represent every possible coordinate space and large-reference
  behavior must not be overclaimed
* duplicate or overlapping regions can double-count records unless the payload
  contract defines deduplication semantics explicitly
* CRAM indexed queries remain a later milestone unless native CRAM support is
  explicitly promoted
