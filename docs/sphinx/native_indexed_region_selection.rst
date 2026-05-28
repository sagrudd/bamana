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

Selected-Record Output
----------------------

M11.3 freezes selected-record output semantics without making
``select_region`` runnable. The planned invocation shape is::

   bamana select_region --bam <input.bam> (--region <REGION> ... | --region-file <regions.txt>) --out <output.bam|-> [--report <report.json>] [--dry-run] [--force]

The data stream is BAM-only. Input must be BGZF-compressed BAM. SAM, CRAM,
FASTQ, FASTQ.GZ, FASTA, and unknown inputs are rejected before any output is
written. Output is BGZF-compressed BAM for file output and stdout output. M11.3
does not introduce SAM, CRAM, FASTQ, FASTQ.GZ, text, uncompressed BAM, or
alternate compression output modes.

Applied runs require an explicit ``--out``. File output writes the selected BAM
to the requested path and emits a JSON report to stdout. ``--report
<report.json>`` additionally writes the same report to a file. ``--out -``
writes BGZF BAM to stdout; because stdout is then binary, it requires
``--report <report.json>`` and must reject ``--report -`` or an omitted report
path. Human diagnostics must use stderr.

Dry runs never write BAM output and never create or replace a report sidecar
unless ``--report <report.json>`` is explicitly supplied. Dry-run reports must
include ``dry_run: true``, ``output_created: false``, the requested output
target, region source, intended execution mode, and planned rejection or
fallback state. Dry-run stdout is JSON unless a report file is supplied and a
later CLI contract deliberately suppresses stdout.

The future JSON report must identify ``command: "select_region"``, input path,
output target, report destination, ``output_format: "bam"``, ``compression:
"bgzf"``, region source, normalized region count, index execution mode,
selected-record counts, records examined, chunks traversed, fallback reason
when applicable, and notes that selected-record output is not biological
interpretation or whole-file validation.

M11.3 does not add a JSON schema, golden example, or fixture because header
behavior, record ordering, duplicate policy, and write-safety remain unfrozen.
M11.8 must add governed schemas, examples, and fixtures after those contracts
are complete.

Header Preservation And Sort Order
----------------------------------

M11.4 freezes the future selected-output header contract without making
``select_region`` runnable.

Selected BAM output must preserve the input BAM reference dictionary exactly.
Binary reference dictionary order, names, lengths, and reference indexes remain
authoritative and are copied without filtering to selected references.
References with no selected records remain present so record ``refID`` and
``next_refID`` values, mate references, read-group references, and auxiliary
payloads remain valid against the unchanged dictionary. Textual ``@SQ`` records
are preserved in encounter order and must continue to reconcile with the binary
reference dictionary.

The textual header is preserved conservatively. ``@RG``, existing ``@PG``,
``@CO``, and unknown SAM-style header records are retained. The only permitted
selected-output header mutation in M11.4 is command provenance: append a new
``@PG`` record for ``bamana select_region``, generate a collision-free ``ID``,
set ``PN:bamana``, include the Bamana version when available, and set ``PP`` to
the previous terminal program only when the program chain is unambiguous.
Unrelated header records must not be removed, rewritten, or reordered for
provenance.

Sort metadata is intentionally conservative. Region selection can emit records
from multiple intervals, repeated intervals, overlapping intervals, region
files, or scan fallback, so selected output must not claim coordinate or
queryname order. If an input ``@HD`` record exists, selected output rewrites
``SO`` to ``unknown`` and removes ``SS``. If no input ``@HD`` exists, M11.4 does
not require synthesizing one solely for sort metadata.

The future JSON report records input ``@HD`` ``SO``/``SS``, output ``@HD``
``SO``/``SS``, and a note that sort-order metadata was downgraded because
selected-region output is not guaranteed to preserve whole-file order. M11.4
does not add schemas, examples, or fixtures because duplicate emission, final
record ordering, and write-safety remain unfrozen; M11.8 must add them after
those contracts are complete.

Duplicate And Overlapping Regions
---------------------------------

M11.5 freezes record ordering and duplicate/overlap behavior without making
``select_region`` runnable.

Selected output emits each physical BAM alignment record at most once. Physical
record identity is the source BAM virtual-offset range for the raw record.
Repeated region strings, repeated region-file lines, overlapping intervals,
adjacent BAI chunks, and broad bins that discover the same record more than
once must not duplicate that record in the selected BAM output.

Record order is deterministic source order. Selected records are emitted in
ascending source BAM virtual-offset order. Scan fallback emits records in
native BAM encounter order, which is the same source-order policy. Region
request order and region-file line order are preserved in normalized request
metadata but do not control output ordering and do not cause selected records to
be repeated. Multi-reference requests produce one source-order output stream,
not per-region output blocks.

When a record matches more than one requested interval, the future report must
retain all matched normalized region identifiers for that record or evidence
path. The selected BAM output still contains one copy of the record. Reports
must distinguish requested region count, normalized region count, duplicate
region request count, overlapping region request count when detectable,
selected unique record count, duplicate physical records suppressed, output
ordering policy ``source_virtual_offset_order``, and duplicate emission policy
``emit_once_per_source_record``.

M11.5 intentionally does not support request-order repeated output, per-region
BAM blocks, or one output file per region. Those modes would need a separate
future contract because they change duplicate multiplicity and sorting claims.
M11.5 does not add schemas, examples, or fixtures because write-safety and final
command implementation remain pending; M11.8 must add them once the remaining
contracts are complete.

Native Selected-Record Writing
------------------------------

M11.6 implements the first runnable ``select_region`` slice for BAM file
output::

   bamana select_region --bam <input.bam> --region <REGION> --out <output.bam> [--dry-run] [--force] [--prefer-index]

The command accepts BGZF-compressed BAM input and repeated CLI ``--region``
values. It writes BGZF-compressed BAM file output through Bamana's native BGZF
writer. Selected alignment records preserve their raw BAM record bytes; the
writer does not reserialize alignment fields or reinterpret auxiliary tags.

With ``--prefer-index`` enabled, a usable non-stale adjacent BAI sidecar drives
native indexed traversal through the M10 chunk planner and random-access
traversal helpers. Missing, stale, unsupported, malformed, or incomplete index
state falls back to native scanner selection. Both execution modes use
``source_virtual_offset_order`` and ``emit_once_per_source_record`` so repeated
regions, overlapping intervals, and broad BAI bins do not duplicate physical
source records in the selected BAM output.

The output header follows the M11.4 policy. The full binary reference
dictionary is preserved, including references with no selected records. Textual
header records are retained except that existing ``@HD`` ``SO`` is rewritten to
``unknown`` and ``SS`` is removed. A collision-free ``@PG`` record for
``bamana select_region`` is appended with ``PN:bamana``, the Bamana version,
and ``PP`` only when the previous program chain has one unambiguous terminal
program.

Dry runs report planned selection without writing BAM output. Applied file
outputs report the selected path, ``output_format: "bam"``, ``compression:
"bgzf"``, selected unique record count, duplicate suppression, execution mode,
index path when used, fallback reason when applicable, and header policy.

M11.6 deliberately leaves ``--out -`` binary stdout routing rejected until
response/report separation is implemented. ``--region-file`` remains
syntax-specified but is not yet a public CLI flag. Final output index
invalidation or regeneration semantics, governed JSON schemas, golden examples,
and fixtures remain M11.7 and M11.8 work.

Write-Safety And Index Invalidation
-----------------------------------

M11.7 completes the file-output safety contract for the current
``select_region`` slice. Applied runs write through a temporary BGZF BAM and
publish only after the writer finishes. Existing BAM output paths are rejected
unless ``--force`` is supplied. Same-path input/output rewrites are rejected
even with ``--force``.

Adjacent output index sidecars are treated as output-safety participants. Before
an applied run writes selected BAM output, Bamana checks conventional BAM index
sidecar names for the requested output: ``<out>.bai``, ``<out>.csi``, the
``.bai`` extension form, and the ``.csi`` extension form. If any already exist
and ``--force`` is absent, the command fails with ``output_exists`` before
writing BAM output. With ``--force``, those pre-existing sidecars are removed
before selected BAM output is written.

``select_region`` does not create a replacement output index in M11.7. The JSON
payload reports ``output.index_invalidation`` with adjacent index candidates,
pre-existing sidecars, removed sidecars, the invalidation action,
``output_index_created: false``, and regeneration guidance of the form
``bamana index --input <output.bam>``. Dry runs report what would be removed by
an applied forced run but do not remove files.

M11.7 does not add public schemas, golden examples, or fixtures; M11.8 remains
responsible for governing the final ``select_region`` JSON schema, examples,
and fixture plan.

Public Contract Artifacts
-------------------------

M11.8 adds governed public-contract artifacts for the implemented
``select_region`` file-output surface:

* ``spec/jsonschema/select_region.schema.json``;
* ``spec/examples/select_region.success.json``;
* ``spec/examples/select_region.failure.json``;
* JSON-output documentation for ``output``, ``index_invalidation``,
  ``region_scope``, ``execution``, and ``header``;
* CLI contract text for supported options and current deferrals;
* fixture-plan reservations for indexed success, scan fallback,
  duplicate/overlap suppression, output-index sidecar collision, forced sidecar
  removal, and same-path rejection.

The schema governs file-output behavior only: ``--bam``, repeated CLI
``--region``, ``--out <output.bam>``, ``--dry-run``, ``--force``, and
``--prefer-index``. Binary stdout output via ``--out -``, public
``--region-file``, report sidecar routing, replacement output index creation,
and broad comparator parity remain outside the current schema until later tasks
promote them.

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
