Native Indexed Region Workflows
===============================

Milestone 10 is active as of 2026-05-23. M10.1 activated the milestone only
after Milestone 9 closed with native BAI writing, BAI validation, typed virtual
offsets, internal random-access helpers, and index-aware ``check_map`` and
``summary`` evidence recorded.

Scope
-----

Milestone 10 turns the native BAM index and random-access substrate into
bounded, user-visible indexed-region workflows. The intended surface includes
region syntax and normalization, optional region-file input, validated-index
chunk planning, region-aware ``check_map`` and ``summary`` evidence, and a
first public indexed-region command or flag only if one is explicitly promoted
into the CLI contract.

Present Substrate
-----------------

``src/bam/index.rs`` owns native BAI parsing, implemented-depth BAI
validation, BAI bin/chunk representation, linear-index metadata,
``bai_bin_for_region``, CSI header detection, and adjacent sidecar
classification.

``src/bgzf/reader.rs`` can seek by typed ``VirtualOffset`` values, and
``src/bam/scan.rs`` exposes ``raw_records_in_virtual_range`` for bounded
internal record retrieval between virtual offsets.

``src/commands/check_map.rs`` and ``src/commands/summary.rs`` already use
usable BAI mapped/unmapped metadata as index-derived evidence and otherwise
report native scan fallback. These paths are not yet region-scoped public
workflows.

Fixture plans include valid coordinate BAM/BAI pairs, stale BAI, malformed
BAI, mismatched-reference BAI, and CSI-header fixtures for region-workflow
expansion.

M10.2 Region Syntax
-------------------

``src/bam/region.rs`` defines the first native region parser and normalization
layer without wiring it into public command behavior.

Supported strings are deliberately small. ``reference`` requests a whole
reference by exact BAM header dictionary name. ``reference:start-end`` requests
a 1-based closed interval and normalizes it to 0-based half-open coordinates
while retaining the original 1-based closed coordinates for reporting.

Multiple regions are represented as an ordered list by
``normalize_region_strings``. M10.2 preserves request order and does not merge,
deduplicate, sort, or overlap-resolve regions.

Reference names are resolved against the parsed BAM header dictionary.
Duplicate reference names are rejected as ambiguous. Reference names containing
colons are supported when the split is unambiguous. A string that is both an
exact reference name and a valid ``reference:start-end`` interval is also
rejected as ambiguous.

The parser rejects empty strings, leading or trailing whitespace, unknown
references, non-numeric coordinates, zero coordinates, reversed intervals,
coordinates beyond the reference length, zero-length whole-reference requests,
and ambiguous reference resolution. BED-like and line-oriented region files are
explicitly deferred for M10.2 through ``reject_region_file_request``.

M10.3 Region Workflow Contract
------------------------------

M10.3 freezes region-aware ``check_map`` and region-aware ``summary`` as the
first planned public command surfaces. ``check_map --region <REGION>`` will
report mapping evidence scoped to one or more M10.2 region strings.
``summary --region <REGION>`` will report operational summary evidence scoped
to one or more M10.2 region strings. A standalone indexed selection command is
deferred.

The governed payload field is ``region_scope``. It records whether region
behavior was requested, that the source was repeated CLI regions, the
``input_1_based_closed_output_0_based_half_open`` coordinate model, the
``preserve_request_order_without_merging_or_deduplication`` duplicate policy,
the ordered normalized regions, and whether execution used indexed traversal,
scan fallback, or precise rejection.

Region-scoped ``check_map`` evidence must not be interpreted as whole-file
mapping evidence. Region-scoped ``summary`` metrics must not be interpreted as
full-file totals. Region files remain deferred until a later M10 task promotes
a file syntax.

M10.4 Chunk Planning
--------------------

``src/bam/region_plan.rs`` implements the internal indexed-region chunk
planner. It consumes M10.2 normalized regions and validated M9 ``BaiIndex``
structures; it does not rely on shallow sidecar detection.

Each interval expands to all candidate BAI bins across the hierarchy. Candidate
chunks are collected from the validated BAI reference bins, sorted by typed
virtual-offset range, and coalesced when they overlap or touch. No-hit
intervals produce deterministic empty chunk plans.

Plans retain provenance for the index path, index kind, reference name,
reference index, requested region strings, candidate bins, candidate chunks,
and coalesced chunks. Unsupported index kinds, stale BAI sidecars,
reference-count incompatibility, empty region sets, and impossible
virtual-offset chunks are rejected before any command may claim indexed-region
evidence.

M10.5 Region-Bounded Traversal
------------------------------

``src/bam/region_traversal.rs`` provides the internal traversal baseline above
the chunk planner. ``traverse_planned_region_chunks`` consumes planned
``VirtualOffset`` ranges with ``raw_records_in_virtual_range`` rather than raw
byte offsets.

Traversal filters retrieved records against normalized intervals by reference
and overlap because broad BAI bins and coalesced chunks may include records
outside the requested region. Overlapping chunks and overlapping region
requests are deduplicate by virtual-offset range, while matched region strings
are merged onto the returned record.

Fallback remains explicit. When no usable index exists, the internal payload
records ``NativeScanRequired`` as the scan fallback instead of claiming indexed
evidence.

M10.6 Region-Aware check_map
----------------------------

``check_map --region <REGION>`` is public command behavior. The command
normalizes repeated region strings, uses validated BAI chunk planning and
typed ``VirtualOffset`` traversal when possible, and falls back to native scan
evidence when the index is missing, stale, unsupported, or invalid.

Indexed payloads report ``region_scope.execution: indexed`` together with
``index_path``, ``chunks_traversed``, ``raw_records_seen``, and
``duplicate_records_suppressed``. Fallback payloads report
``region_scope.execution: scan_fallback``, ``fallback_mode:
native_scan_required``, and ``scan_records_limit``.

Region-scoped counts use ``region_records_examined``,
``region_mapped_records_observed``, and
``region_unmapped_records_observed``. They are not whole-file totals.
Unsupported region requests fail with precise ``invalid_region`` errors.
M10.7 Region-Aware summary
--------------------------

``summary --region <REGION>`` is public command behavior. The command reuses
the region parser, validated BAI chunk planning, and typed ``VirtualOffset``
traversal used by region-aware ``check_map``.

Indexed payloads report ``region_scope.execution: indexed`` together with
``index_path``, ``chunks_traversed``, ``raw_records_seen``, and
``duplicate_records_suppressed``. Fallback payloads report
``region_scope.execution: scan_fallback``, ``fallback_mode:
native_scan_required``, and ``scan_records_limit``.

Region-scoped ``counts``, ``fractions_observed``, ``mapq``, ``mapping``,
``anomalies``, and optional ``flag_categories`` describe only requested
intervals. Whole-file BAI totals are intentionally omitted from region-scoped
``index_derived`` because the index is used only to find records. Unsupported
region requests fail with precise ``invalid_region`` errors. Region files and
standalone indexed region selection remain deferred.

Known Gaps
----------

Indexed-region benchmark rows do not yet compare random-access lookup behavior
with scan fallback. CSI large-reference behavior remains unsupported until a
later scoped decision.

Command Boundary
----------------

The public contract commands ``benchmark``, ``fastq``, and ``unmap`` remain
protected while M10 work proceeds. Native CRAM parsing, CRAM indexed queries,
broad random-access APIs, pileup or genotyping semantics, and external
comparator parity remain outside M10 unless a later M10 task explicitly
promotes them.
