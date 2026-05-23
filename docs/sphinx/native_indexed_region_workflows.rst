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

Known Gaps
----------

The M10.3 command flags are frozen as a planned contract but are not accepted
by the binary until later M10 implementation tasks wire them. Random-access
chunk planning has not yet been promoted into public command behavior.
Overlapping-region, duplicate-region, and multi-reference semantics remain
limited to preserving request order in the normalized region set.

Region-aware ``check_map`` and ``summary`` payloads do not yet distinguish
requested-region evidence from whole-file evidence. Indexed-region benchmark
rows do not yet compare random-access lookup behavior with scan fallback. CSI
large-reference behavior remains unsupported until a later scoped decision.

Command Boundary
----------------

The public contract commands ``benchmark``, ``fastq``, and ``unmap`` remain
protected while M10 work proceeds. Native CRAM parsing, CRAM indexed queries,
broad random-access APIs, pileup or genotyping semantics, and external
comparator parity remain outside M10 unless a later M10 task explicitly
promotes them.
