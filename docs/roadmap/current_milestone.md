# Current Milestone

## Milestone Status

**Milestone 6: Native Inspection And Validation Commands** is complete as of
2026-05-21. **Milestone 7: Native Mutation, Remediation, And Forensics
Commands** is complete as of 2026-05-22. **Milestone 8: Native Transform,
Checksum, Explode, And Ingest Commands** is complete as of 2026-05-23.
**Milestone 9: Native BAM Index And Random Access** is complete as of
2026-05-23. It was activated by M9.1 only after the M8 closeout commit was in
place and closed after M9.1 through M9.10 completed on 2026-05-23.
**Milestone 10: Native Indexed Region Workflows** is active as of 2026-05-23.
It was activated by M10.1 only after the Milestone 9 closeout evidence was
recorded.

Milestone 6 became active after **Milestone 5: Command Migration Off
`noodles`** closed on 2026-05-20, and closed after M6.1 through M6.10
completed on 2026-05-21. Milestone 7 became active only after that M6
closeout was recorded, and closed after M7.1 through M7.10 completed on
2026-05-22. Milestone 8 became active only after that M7 closeout was
recorded, and closed after M8.1 through M8.10 completed on 2026-05-23.
Milestone 9 became active only after that M8 closeout was recorded. Milestone
10 became active only after that M9 closeout was recorded.

See:

* [milestone-08-transform-ingest.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-08-transform-ingest.md)
* [milestone-09-bam-index-random-access.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-09-bam-index-random-access.md)
* [milestone-10-indexed-region-workflows.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-10-indexed-region-workflows.md)
* [milestone-07-mutation-forensics.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-07-mutation-forensics.md)
* [milestone-06-inspection-validation.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-06-inspection-validation.md)
* [milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

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

Milestone 10 is active for native indexed-region workflows. The baseline audit
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
* indexed-region benchmark rows do not yet compare random-access lookup with
  scan fallback;
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
