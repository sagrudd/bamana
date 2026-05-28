# Current Milestone

## Milestone Status

**Milestone 6: Native Inspection And Validation Commands** is complete as of
2026-05-21. **Milestone 7: Native Mutation, Remediation, And Forensics
Commands** is complete as of 2026-05-22. **Milestone 8: Native Transform,
Checksum, Explode, And Ingest Commands** is complete as of 2026-05-23.
**Milestone 9: Native BAM Index And Random Access** is complete as of
2026-05-23. It was activated by M9.1 only after the M8 closeout commit was in
place and closed after M9.1 through M9.10 completed on 2026-05-23.
**Milestone 10: Native Indexed Region Workflows** is complete as of
2026-05-23. It was activated by M10.1 only after the Milestone 9 closeout
evidence was recorded and closed after M10.1 through M10.10 completed on
2026-05-23. **Milestone 11: Public Indexed Region Selection And Region Files**
is complete as of 2026-05-28. It was activated by M11.1 only after the
Milestone 10 closeout evidence was recorded and closed after M11.1 through
M11.10 completed on 2026-05-28.
Status: complete as of 2026-05-28 for Milestone 11.

Milestone 6 became active after **Milestone 5: Command Migration Off
`noodles`** closed on 2026-05-20, and closed after M6.1 through M6.10
completed on 2026-05-21. Milestone 7 became active only after that M6
closeout was recorded, and closed after M7.1 through M7.10 completed on
2026-05-22. Milestone 8 became active only after that M7 closeout was
recorded, and closed after M8.1 through M8.10 completed on 2026-05-23.
Milestone 9 became active only after that M8 closeout was recorded. Milestone
10 became active only after that M9 closeout was recorded and closed with
region syntax, chunk planning, random-access traversal, read-only
`check_map --region <REGION>` and `summary --region <REGION>` evidence,
dependency guardrails, and smoke benchmark evidence in place.

See:

* [milestone-08-transform-ingest.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-08-transform-ingest.md)
* [milestone-09-bam-index-random-access.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-09-bam-index-random-access.md)
* [milestone-10-indexed-region-workflows.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-10-indexed-region-workflows.md)
* [milestone-11-indexed-region-selection.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-11-indexed-region-selection.md)
* [milestone-12-extended-index-compatibility.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-12-extended-index-compatibility.md)
* [milestone-13-native-cram-strategy.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-13-native-cram-strategy.md)
* [milestone-14-interop-benchmark-evidence.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-14-interop-benchmark-evidence.md)
* [milestone-15-release-hardening.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-15-release-hardening.md)
* [milestone-07-mutation-forensics.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-07-mutation-forensics.md)
* [milestone-06-inspection-validation.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-06-inspection-validation.md)
* [milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

Latest closed milestone:

* **Milestone 11: Public Indexed Region Selection And Region Files** is
  complete as of 2026-05-28. M11 does not reopen Milestone 10. M11.1 freezes the
  selection surface decision as a new planned public `select_region` command,
  rather than extending `check_map`, `summary`, or `subsample`. M11 must specify
  selected-record region output and region-file input before implementation:
  output semantics, header preservation, duplicate handling, index
  invalidation, and write-safety are mandatory contract work. M11.2 freezes the
  future region-file syntax as UTF-8 line-oriented input with one M10-style
  region per non-comment line, blank lines and leading `#` comments ignored,
  request order preserved, duplicates not merged by parsing, and BED-like or
  otherwise unsupported coordinate models rejected. M11.3 freezes selected-record
  output as BAM-only: BGZF-compressed BAM input and output, explicit `--out`,
  JSON report separation when `--out -` writes binary BAM to stdout, and dry-run
  reports that write no BAM output. M11.4 freezes header behavior: preserve the
  full binary reference dictionary and textual header, append only `@PG`
  provenance, downgrade existing `@HD SO` to `unknown`, remove `SS`, and report
  input/output sort metadata. M11.5 freezes duplicate and overlapping-region
  policy: emit each physical source record at most once in source virtual-offset
  order, suppressing duplicates from repeated regions, overlapping intervals,
  adjacent chunks, and broad bins while keeping match metadata in reports.
  M11.6 implements the first runnable `select_region` slice for BGZF BAM file
  output with CLI `--region` values: usable non-stale BAI sidecars drive native
  indexed traversal, unusable index state falls back to native scanning,
  selected records preserve raw BAM record bytes, and output reports
  `source_virtual_offset_order` plus `emit_once_per_source_record`. Binary
  stdout output, public `--region-file`, schemas, examples, and fixtures remain
  later M11 work.
  M11.7 completes write-safety and index invalidation for that slice: same-path
  input/output rewrites are rejected, adjacent output BAI/CSI sidecars are
  collisions unless `--force` is supplied, forced applied runs remove those
  stale sidecars before writing selected BAM output, dry runs only report
  planned invalidation, and no replacement output index is created.
  M11.8 adds the governed schema, canonical success/failure examples,
  JSON-output documentation, CLI contract text, and fixture-plan reservations
  for the implemented `select_region` file-output surface.
  M11.9 adds dependency and benchmark guardrails: selected-region output hot
  paths remain protected from direct production `noodles` imports, and
  `scanner_microbench --bamana-bin` emits `select_region_scan_fallback` and
  `select_region_indexed_output` smoke timings that distinguish scan fallback,
  BAI chunk planning, random-access traversal, selected-record filtering,
  duplicate suppression, raw-record preservation, BGZF BAM file writing,
  temporary-output finalization, header provenance, index-invalidation
  reporting, command startup, and JSON emission without claiming
  stdout-output evidence, public region-file evidence, replacement
  output-index evidence, comparator-parity claims, native CRAM indexed-query
  support, or biological interpretation.
  M11.10 closes the milestone after full tests, contract tests, Sphinx HTML
  documentation, formatting, whitespace checks, binary builds, and
  `scanner_microbench --profile small --iterations 1 --bamana-bin` smoke
  evidence passed. The closeout leaves `select_region` governed for BGZF BAM
  file output from CLI `--region` requests, with public `--region-file`,
  `--out -`, replacement output index creation, CSI large-reference behavior,
  native CRAM indexed queries, and broad comparator parity still deferred.

Milestone 11 is closed; Milestone 12 remains planned until its activation task
is started.

## Completed Backbone

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. Milestone 3
completed selective native BAM record scanning and migrated the first
scanner-compatible BAM command consumers. Milestone 4 completed the native
FASTQ and FASTQ.GZ parser/writer core. Milestone 5 completed the proof-command
migration for `verify`, `header`, and `subsample`. Milestone 6 completed the
native inspection and validation hardening wave for `check_eof`, `check_sort`,
`check_map`, `summary`, `check_tag`, and `validate`. Milestone 7 completed
native mutation, conservative remediation, and provenance inspection for
`reheader`, `annotate_rg`, `inspect_duplication`, `deduplicate`, and
`forensic_inspect`. Milestone 8 completed the native transform, checksum,
sharding, and ingest hardening wave for `sort`, `merge`, `explode`,
`checksum`, and `consume`.

Milestone 11 completed public indexed region selection file-output governance
for `select_region`.

## Milestone 8 Closeout

Milestone 8 hardened Bamana's largest transform, checksum, explode, and ingest
command paths:

* `sort`
* `merge`
* `explode`
* `checksum`
* `consume`

Closeout evidence:

* `sort`, `merge`, `explode`, `checksum`, and `consume` have governed CLI,
  JSON-output, Sphinx, README, schema, example, fixture, and task-map coverage;
* BAM-side `sort`, `merge`, `explode`, `checksum`, and `consume` use
  scanner-backed native record loading and native writer/checksum bridges
  where applicable;
* SAM, FASTQ, and FASTQ.GZ transform/ingest behavior remains native, while
  CRAM remains a documented compatibility boundary under explicit reference
  policy;
* output-safety rules publish completed temporary outputs through final rename
  steps and keep dry-run paths side-effect bounded;
* `scanner_microbench --bamana-bin` emits command smoke timings for every M8
  command without claiming external comparator parity;
* dependency-boundary tests name the five M8 command paths and keep them free
  of direct production `noodles` imports outside documented CRAM
  compatibility.

## Milestone 6 Closeout

Milestone 6 hardened the first operational BAM inspection and validation command
wave:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

The milestone covered bounded evidence, full-scan claims, index-versus-scan
distinctions, structural validation caveats, command contracts, benchmark smoke
coverage, and dependency-boundary protection for these command paths.

Closeout evidence:

* command contracts, schemas, examples, CLI docs, JSON-output docs, README, and
  Sphinx notes cover the six M6 commands;
* bounded versus full-scan evidence is documented for absence and validity
  claims;
* index-derived and scan-derived mapping/summary evidence is distinct;
* `validate` explicitly remains structural/internal-consistency validation and
  does not claim biological correctness, external reference concordance, or
  complete optional-field semantic validation;
* `bgzf_microbench --bamana-bin` emits `check_eof` command smoke timing;
* `scanner_microbench --bamana-bin` emits `check_sort`, `check_map`,
  `summary`, `check_tag`, and `validate` command smoke timings;
* dependency-boundary tests name the six M6 command paths and keep them free of
  direct production `noodles` imports.

## Milestone 7 Closeout

Milestone 7 hardened native mutation, conservative remediation, and
provenance-inspection command paths:

* `reheader`
* `annotate_rg`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

Closeout evidence:

* command contracts, schemas, examples, CLI docs, JSON-output docs, README, and
  Sphinx notes cover the five M7 commands;
* `reheader` is documented and tested as header-only mutation, while
  `annotate_rg` is documented and tested as record-level read-group
  annotation;
* `inspect_duplication` remains collection-duplication/operator-error
  inspection, not biological duplicate marking;
* `deduplicate` remains conservative remediation over native BAM/FASTQ paths,
  not broad duplicate collapse;
* `forensic_inspect` remains evidence-driven provenance inspection, not fraud
  detection;
* `header_microbench --bamana-bin` emits `reheader` and `annotate_rg` command
  smoke timings;
* `scanner_microbench --bamana-bin` emits `inspect_duplication`,
  `deduplicate`, and `forensic_inspect` command smoke timings;
* dependency-boundary tests name the five M7 command paths and keep them free
  of direct production `noodles` imports.

## Command-Surface Boundary

Milestone 9 evidence is limited to BAM index writing, BAM index inspection,
virtual-offset random-access groundwork, and first index-aware command evidence
in `check_map` and `summary`.

Milestone 10 evidence is limited to BAM indexed-region workflows above that
native index and random-access substrate. Native CRAM parsing, CRAM indexed
queries, broad random-access APIs, external comparator parity, and FASTQ.GZI
work beyond existing sidecar behavior remain later work unless a specific M10
task explicitly includes them. The public contract commands `benchmark`,
`fastq`, and `unmap` remain protected after M10 activation.

## Milestone 9 Baseline

Milestone 9 is complete for native BAM index writing, deeper index validation,
and virtual-offset-backed random-access groundwork.

Present baseline:

* `VirtualOffset` already models packed BGZF virtual offsets with bounds and
  ordering checks;
* BGZF reader and BAM scanner paths expose typed record start/end virtual
  offsets for future BAI chunk and linear-index construction, and the BGZF
  reader can seek to typed virtual offsets for internal random-access helpers;
* `src/bam/index.rs` can build native in-memory BAI bins, chunks, linear-index
  windows, and mapped/unmapped counts from scanner-owned record traversal, and
  serialize native BAI sidecars;
* `src/bam/index.rs` already detects BAI, CSI, GZI, and unknown sidecar magic,
  discovers adjacent index candidates, validates BAI metadata, bin/chunk, and
  linear-index structure, and parses CSI headers enough to report
  detected-but-not-supported status;
* `check_index` already reports adjacent index presence, selected path, kind,
  BAI structural validity, CSI header status, staleness, compatibility, and
  apparent usability;
* `index` already creates native BAI sidecars for coordinate-sorted BAM inputs,
  creates FASTQ.GZI sidecars for FASTQ.GZ inputs, and honestly reports CSI
  writing as unimplemented;
* `check_map` and `summary` already keep index-derived evidence distinct from
  scan-derived evidence, use BAI metadata only when the selected sidecar is
  non-stale, structurally valid, and complete for mapped/unmapped metadata, and
  fall back to native scan evidence otherwise.

Post-M9 deferrals:

* BAM `index` cannot yet write real CSI sidecars;
* public commands do not yet exercise random-access chunk traversal for
  acceleration or region filtering;
* CSI support remains header-only detection until a later scoped decision.

M9.9 benchmark and dependency-boundary evidence now protects the active
index/random-access set:

* `tests/contract/dependency_boundary.rs` names `index`, `check_index`,
  indexed `check_map`, indexed `summary`, and the random-access substrate as
  Bamana-native hot paths;
* `scanner_microbench --bamana-bin` emits `index_bam`, `check_index`,
  `check_map_indexed`, and `summary_indexed` command smoke timings in addition
  to existing scan-fallback `check_map` and `summary` timings;
* benchmark notes distinguish BAI construction, BAI structural validation,
  index metadata-backed consumer evidence, scan fallback timings,
  random-access lookup deferral, process startup, JSON emission, and comparator
  non-parity.

## Milestone 9 Closeout

Milestone 9 closed after M9.1 through M9.10 completed. Closeout evidence:

* `index` creates native BAI sidecars for supported coordinate-sorted BAM
  inputs and rejects unsupported BAM index requests precisely;
* `check_index` validates BAI and CSI sidecars to the implemented depth while
  avoiding claims that every random-access offset has been exercised;
* native BGZF reader and BAM scanner paths expose typed virtual-offset capture
  and internal range retrieval for later indexed-region work;
* `check_map` and `summary` use validated BAI metadata as index-derived
  evidence only when usable and otherwise report native scan fallback evidence;
* CSI writing and public indexed-region command acceleration are explicitly
  deferred beyond M9;
* `scanner_microbench --bamana-bin` emits M9 smoke timing rows for
  `index_bam`, `check_index`, `check_map_indexed`, and `summary_indexed`;
* dependency-boundary tests protect the M9 command/substrate set from direct
  production `noodles` imports outside CRAM compatibility;
* closeout verification passed with full tests, contract tests, Sphinx, and M9
  command benchmark smoke checks.

## Milestone 10 Baseline

Milestone 10 is complete for native indexed-region workflows. The baseline audit
confirmed that M10 starts from real M9 substrate rather than from shallow
sidecar discovery.

Present substrate:

* `src/bam/index.rs` can parse and validate BAI sidecars to the implemented
  depth, detect CSI headers as unsupported, expose `bai_bin_for_region`, and
  retain BAI bin/chunk and linear-index evidence for chunk planning;
* `src/bgzf/reader.rs` can seek to typed `VirtualOffset` values, and
  `src/bam/scan.rs` exposes `raw_records_in_virtual_range` for internal
  random-access retrieval experiments;
* `src/commands/check_map.rs` and `src/commands/summary.rs` already separate
  index-derived mapped/unmapped evidence from scan-derived evidence and fall
  back when an index is stale, malformed, unsupported, or incomplete;
* fixture plans and manifests already include valid coordinate BAM/BAI pairs,
  stale BAI sidecars, malformed BAI sidecars, mismatched-reference BAI
  sidecars, and CSI-header fixtures for region-workflow expansion.

Known M10 gaps:

* M10.2 defines the internal region-string grammar and normalization model,
  but no public command flag consumes it yet;
* region-file input is explicitly deferred after M10.2;
* BAI chunk planning is not yet promoted into public command behavior;
* overlapping-region, duplicate-region, and multi-reference semantics are not
  specified;
* `check_map` and `summary` payloads do not yet distinguish requested-region
  scope from whole-file scope;
* M10.9 now adds indexed-region benchmark smoke timing rows that compare
  random-access traversal with scan fallback for the promoted read-only
  evidence commands;
* CSI large-reference behavior remains unsupported until a later scoped
  decision.

## Milestone 10 Region Syntax Baseline

M10.2 added `src/bam/region.rs` as the native region parser and normalization
layer. Supported region strings are deliberately small:

* `reference` requests a whole reference by exact BAM header dictionary name;
* `reference:start-end` requests a 1-based closed interval and normalizes it
  to 0-based half-open coordinates;
* multiple regions are represented as an ordered list and are not merged,
  deduplicated, or overlap-resolved in M10.2;
* reference names are resolved against the binary BAM header dictionary, and
  duplicate names or exact-reference-versus-interval ambiguity are rejected.

The parser rejects empty strings, leading or trailing whitespace, unknown
references, duplicate reference names, zero coordinates, reversed intervals,
non-numeric coordinates, coordinates beyond the reference length, and
zero-length whole-reference requests. BED-like and line-oriented region files
are explicitly deferred with a precise unimplemented error until a later M10
task promotes a region-file contract.

## Milestone 10 Region Workflow Contract

M10.3 freezes the first region-aware workflow contract before command wiring:

* `check_map --region <REGION>` and `summary --region <REGION>` are the first
  planned region-aware command surfaces;
* `--region` uses the M10.2 grammar and may be repeated;
* normalized regions are reported in `region_scope` with the coordinate model,
  interval semantics, duplicate policy, ordered regions, and execution mode;
* `region_scope.execution` distinguishes `indexed`, `scan_fallback`, and
  `rejected` outcomes;
* region-aware `check_map` evidence must not be confused with whole-file
  mapping evidence;
* region-aware `summary` metrics must not be confused with full-file totals;
* region files, a standalone indexed selection command, and CLI flag
  acceptance remain deferred until later M10 tasks wire and verify behavior.

## Milestone 10 Chunk Planning Baseline

M10.4 added `src/bam/region_plan.rs` as the internal indexed-region chunk
planner. It consumes M10.2 normalized regions and validated M9 `BaiIndex`
structures; it does not rely on shallow sidecar detection.

Planning behavior:

* each normalized interval expands to all candidate BAI bins across the
  hierarchy, not only the smallest alignment bin;
* candidate chunks are collected from the validated BAI reference bins and
  sorted deterministically by virtual-offset range;
* overlapping or adjacent candidate chunks are coalesced before later
  random-access traversal;
* plans retain provenance for index path, index kind, reference name,
  reference index, requested region strings, candidate bins, candidate chunks,
  and coalesced chunks.

The planner rejects unsupported index kinds such as CSI, stale BAI sidecars,
reference-count incompatibility, empty region sets, and impossible virtual
offset chunks before any later command may claim indexed-region evidence.
No-hit intervals are represented as empty chunk plans rather than parse errors.

## Milestone 10 Region Traversal Baseline

M10.5 added `src/bam/region_traversal.rs` as the internal random-access
traversal layer above M10.4 plans. It consumes planned BAI chunk ranges through
typed `VirtualOffset` values and `raw_records_in_virtual_range`; raw byte
offsets are not part of the traversal contract.

Traversal behavior:

* retrieved records are parsed natively and filtered against normalized
  intervals by reference and overlap, because broad BAI bins can return records
  outside the requested interval;
* overlapping chunks and overlapping region requests are deduplicate by
  virtual-offset range before records are returned;
* matched region strings are retained on each returned record so later command
  payloads can explain why a record was selected;
* missing or unusable index state remains visible as the explicit scan fallback
  `NativeScanRequired`.

## Milestone 10 Region-Aware Check Map

M10.6 promotes `check_map --region <REGION>` to public command behavior, and
M10.7 promotes `summary --region <REGION>` to public command behavior. Both
commands normalize repeated region strings with the M10.2 parser, use a
validated BAI sidecar and M10.4/M10.5 traversal when possible, and fall back to
native scan evidence when the index is missing, stale, unsupported, or invalid.

Payload behavior:

* `region_scope.execution` reports `indexed` or `scan_fallback`;
* indexed execution reports `index_path`, `chunks_traversed`,
  `raw_records_seen`, and `duplicate_records_suppressed`;
* fallback execution reports `fallback_mode: native_scan_required` and
  `scan_records_limit`;
* region-scoped counts are reported as `region_records_examined`,
  `region_mapped_records_observed`, and
  `region_unmapped_records_observed`, not as whole-file totals.

Unsupported region requests such as unknown references and empty intervals
fail deterministically with `invalid_region`. Region-scoped `summary` omits
whole-file BAI totals and reports requested-interval `counts`,
`fractions_observed`, `mapq`, `mapping`, `anomalies`, and optional
`flag_categories` only. Region files and standalone indexed region selection
remain deferred.

## Milestone 10 Indexed Selection Decision

M10.8 deliberately defers a public indexed region selection command. The
implemented M10 surface is read-only evidence from `check_map --region <REGION>`
and `summary --region <REGION>`; no command claims to select, copy, or write
BAM records by region. The deferral is intentional because selected-record
output needs a separate public contract for output semantics, header
preservation, record ordering, duplicate-region behavior, index invalidation or
regeneration notes, and output write-safety behavior.

The substrate for future selection work is ready enough to specify against:
M10.2 supplies normalized region parsing, M10.4 supplies validated BAI chunk
planning, M10.5 supplies region-bounded traversal, M10.6 supplies
`check_map --region <REGION>` evidence, and M10.7 supplies
`summary --region <REGION>` evidence. Region files remain deferred with the
same contract boundary.

## Milestone 10 Dependency And Benchmark Guardrails

M10.9 strengthens the dependency and benchmark boundary for indexed-region
workflows:

* `tests/contract/dependency_boundary.rs` names the M10 substrate set:
  region parsing, BAI chunk planning, random-access traversal,
  `check_map --region <REGION>`, `summary --region <REGION>`, and scan
  fallback;
* production direct `noodles` imports remain limited to the documented CRAM
  compatibility path;
* `scanner_microbench --bamana-bin` emits smoke timing rows for
  `check_map_region_scan_fallback`, `summary_region_scan_fallback`,
  `check_map_region_indexed`, and `summary_region_indexed`;
* benchmark notes distinguish index lookup, BAI chunk planning, random-access
  traversal, region filtering, scan fallback, command startup, and JSON
  emission without claiming broad comparator parity, native CRAM indexed
  queries, biological interpretation, or selected-record output.

## Milestone 10 Closeout

Milestone 10 closed after M10.1 through M10.10 completed. Closeout evidence:

* native region syntax and normalization are implemented in `src/bam/region.rs`
  for `reference` and `reference:start-end`, using 1-based closed input and
  reporting normalized 0-based half-open intervals;
* region files remain explicitly deferred and fail as unsupported region input;
* `src/bam/region_plan.rs` plans validated BAI chunks for normalized regions,
  rejects stale, unsupported, incompatible, or impossible index evidence, and
  keeps CSI large-reference behavior out of scope;
* `src/bam/region_traversal.rs` uses typed `VirtualOffset` ranges and
  `raw_records_in_virtual_range` for random-access retrieval, then filters
  records by requested interval and suppresses duplicate virtual-offset ranges;
* `check_map --region <REGION>` and `summary --region <REGION>` are promoted
  read-only public behavior with governed JSON, schema, example, CLI, README,
  Sphinx, roadmap, fixture-plan, and contract-test coverage;
* a public indexed region selection command is deliberately deferred until
  output semantics, header preservation, record ordering, duplicate-region
  behavior, index invalidation or regeneration, and write-safety are specified;
* dependency-boundary tests keep M10 BAM indexed-region hot paths free of
  direct production `noodles` imports outside documented CRAM compatibility;
* `scanner_microbench --bamana-bin` M10 smoke timings passed for
  `check_map_region_scan_fallback`, `summary_region_scan_fallback`,
  `check_map_region_indexed`, and `summary_region_indexed`;
* closeout verification passed: `cargo test`, `cargo test --test contract`,
  `cargo build --bin bamana --bin scanner_microbench`, release
  `scanner_microbench --profile small --iterations 1 --bamana-bin` smoke
  output with all four M10 rows reporting `1/1`, Sphinx HTML build,
  `cargo fmt --check`, and `git diff --check`.
