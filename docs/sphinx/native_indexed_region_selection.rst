Native Indexed Region Selection
===============================

Milestone 11 is active as of 2026-05-28. It follows the completed Milestone 10
read-only indexed-region evidence surface.

Scope
-----

M11 is the contract runway for selected-record indexed region output and
region-file input. M11.1 freezes the selection surface decision as a new
planned public command named ``select_region``. M11 must not overload
``check_map``, ``summary``, or ``subsample`` for selected-record output.

Activation Boundary
-------------------

M11.1 does not implement new CLI behavior. No public ``select_region`` synopsis
exists until later M11 tasks define:

* output semantics for stdout, files, and dry runs;
* header preservation and ``@HD`` sort-order behavior;
* record ordering;
* duplicate-region and overlapping-region behavior;
* region-file syntax and validation;
* index invalidation or regeneration notes;
* write-safety and collision handling.

Until those contracts are complete, ``check_map --region <REGION>`` and
``summary --region <REGION>`` remain the only public region-aware behavior, and
they remain read-only evidence surfaces.

Region-File Syntax
------------------

M11.2 freezes the future region-file syntax without making it public CLI
behavior yet. Until ``select_region`` is implemented, region-file requests must
fail as unimplemented and report that region-file parsing is specified for
M11.2 but is not yet public CLI behavior.

Accepted future region files are UTF-8 text files. LF and CRLF line endings
are accepted. The parser reads one region per non-comment line, trims
surrounding ASCII whitespace, ignores blank lines, and ignores comment lines
whose first non-whitespace character is ``#``. Inline comments are not
recognized.

Each non-comment line uses the M10 region grammar:

* ``reference`` for a whole-reference request;
* ``reference:start-end`` for an explicit interval;
* interval coordinates are 1-based closed in input;
* normalized coordinates remain 0-based half-open.

Request order is preserved. Duplicate lines and overlapping intervals are
preserved by parsing and normalization. Region-file parsing must not sort,
merge, or deduplicate request lines; selected-record duplicate emission is
reserved for the later M11 duplicate-policy task.

Malformed lines fail with the region file path and 1-based line number. Empty
files, comment-only files, unknown references, ambiguous references, zero-length
whole-reference requests, empty intervals, zero coordinates, reversed
intervals, out-of-range intervals, and non-numeric coordinates use the native
``invalid_region`` taxonomy. Unsupported coordinate models are rejected,
including BED-like ``chrom start end`` rows, 0-based half-open interval files,
comma-separated ranges, open-ended ranges, strand/name columns, and other
tabular metadata. Missing, unreadable, or non-UTF-8 files fail before any
output is written.

Inherited Substrate
-------------------

M11 inherits:

* ``src/bam/region.rs`` for bounded region string parsing and normalization;
* ``src/bam/region_plan.rs`` for validated BAI chunk planning;
* ``src/bam/region_traversal.rs`` for random-access traversal, interval
  filtering, and duplicate virtual-offset suppression;
* ``src/commands/check_map.rs`` and ``src/commands/summary.rs`` for read-only
  region evidence payloads;
* ``src/bam/write.rs`` and ``src/output_safety.rs`` as patterns for later
  selected-record output and collision safety.

Non-Goals
---------

M11 does not imply native CRAM indexed queries, CSI large-reference support,
biological interpretation, pileup/genotyping behavior, or broad external
comparator parity.
