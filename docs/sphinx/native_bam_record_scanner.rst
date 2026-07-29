Native BAM Record Scanner
=========================

Milestone 3 is complete. Bamana's native-core stack now includes a selective
BAM record scanner built above the native BGZF reader and native BAM header
codec.

Scope
-----

The scanner milestone is responsible for:

* iterating BAM alignment records without full generic decode;
* exposing lightweight views for ``refID``, ``pos``, flags, MAPQ, read name,
  sequence length, and aux-region boundaries;
* skipping unneeded CIGAR, sequence, quality, and auxiliary payload regions
  safely;
* traversing selected auxiliary tags without materializing every optional
  field;
* migrating selected record-scanning command consumers onto shared scanner
  primitives.

The completed first command beneficiaries are ``check_sort``, ``check_map``,
``summary``, ``check_tag``, ``validate``, ``inspect_duplication``, and
``forensic_inspect``. BAM-side ``subsample`` and writer-heavy transforms remain
downstream where command behavior needs owned records or serialization.

Baseline
--------

The current record bridge is ``src/bam/records.rs``. It validates the BAM core
layout and variable-section lengths, then materializes read name, CIGAR bytes,
sequence bytes, quality bytes, and aux bytes into ``RecordLayout``.
``LightAlignmentRecord`` already exposes the fields needed by some scan
commands, but it is derived from that fully materialized layout. It is therefore
a convenience view, not the final selective scanner.

``src/bam/tags.rs`` provides bounded auxiliary-field traversal and tag lookup
over scanner-owned aux slices as well as materialized aux bytes. Milestone 3
kept the bounded traversal behavior while adding scanner-facing helpers over
``BamRecordView::aux_bytes``.

Modified-base decoding
----------------------

``src/bam/modifications.rs`` provides a strict library boundary for a complete
``MM:Z:C+m`` / ``ML:B:C`` / ``MN:i`` trio.  It validates atomic tag presence,
``MN`` against ``SEQ`` length, delta addressing against canonical cytosines,
and one-to-one ``ML`` cardinality before projecting calls through BAM CIGAR.
Soft-clipped and inserted calls have no reference coordinate; deletions and
reference skips advance the reference cursor.

The first supported surface intentionally rejects non-cytosine bases,
non-5mC codes, multiple MM groups, and malformed or differently typed tags.
MM positions are interpreted against BAM ``SEQ`` exactly as required by SAM.
Reverse-strand records are therefore not reversed a second time during CIGAR
projection.  Consumers should treat ``Ok(None)`` as all three tags absent;
partial trios are errors.

The first migration target is ``check_sort``, followed by ``check_map``,
``summary``, and ``check_tag``. Validation and forensics paths come after those
because they mix lightweight record fields with aux traversal and sometimes
sequence or quality decoding. Raw-record writers and transformers should move
after the scanner exposes raw record access or a lossless bridge back to richer
record layouts.

Record View Contract
--------------------

``src/bam/record.rs`` defines ``BamRecordView`` as the scanner-facing
lightweight record contract. The view borrows one complete length-prefixed BAM
record and exposes core fields, sequence length, raw record bytes, and stable
ranges for the core, read name, CIGAR, sequence, quality, and auxiliary
regions.

The view does not allocate CIGAR, sequence, quality, or auxiliary payloads for
field-only consumers. Those sections are available as borrowed slices when a
consumer asks for them. Existing richer consumers are preserved through an
explicit ``to_record_layout`` bridge back to the owned ``RecordLayout`` type.

Native Scan Loop
----------------

``src/bam/scan.rs`` defines ``BamScanner``. The scanner opens BAM input through
the native BGZF backend, parses the native BAM header once, and iterates
complete raw alignment records into ``BamRecordView``.

Clean EOF returns no next record. Negative record sizes, block sizes smaller
than the BAM core, truncated block-size prefixes, truncated record payloads, and
record-view parse failures are surfaced as structured errors with input path
context.

This loop is the scanner substrate. M3.4 through M3.9 centralized selective
field helpers, moved auxiliary traversal onto scanner-owned ranges, migrated
the selected first command consumers onto ``BamScanner``, and added scanner
tests plus microbenchmark hooks.

Selective Field Helpers
-----------------------

``BamRecordView`` exposes the stable scanner helper surface for common field
access. ``flag_summary`` returns decoded flag booleans, ``coordinates`` groups
reference and mate coordinates, and direct accessors expose MAPQ, read name,
and sequence length.

Variable sections remain allocation-light. Range helpers, borrowed section
slices, section-presence helpers, and ``skip_offsets`` let consumers skip CIGAR,
sequence, quality, or auxiliary payloads without rebuilding byte-offset
arithmetic. Parser internals and the owned ``RecordLayout`` bridge remain
separate from this stable scanner helper surface.

Scanner-Owned Aux Traversal
---------------------------

``src/bam/tags.rs`` provides record-view helpers for bounded auxiliary-field
traversal over ``BamRecordView::aux_bytes``. Scanner consumers can test selected
tag presence, count matching tags, collect tag keys, and extract string tags
without materializing ``RecordLayout``.

The helpers reuse the existing bounded aux parser and its type-skipping logic
for scalar values, NUL-terminated strings, hex strings, and B-arrays. Malformed
aux payloads fail with precise parse errors instead of being silently truncated
or over-read.

Command paths such as ``check_tag``, read-group evidence, validation, and
forensics now use these scanner-owned helpers where their first-slice behavior
fits the lightweight view.

First Command Migration
-----------------------

``check_sort`` now consumes ``BamScanner`` directly. It builds its sort-only
comparison snapshot from ``BamRecordView`` and scanner-owned flag helpers while
preserving bounded scans, strict scans, specialized-sort handling, JSON payloads,
and user-facing caveats.

``check_map``, ``summary``, and ``check_tag`` now consume the scanner as well.
``check_map`` preserves usable index summaries as the preferred evidence source
and uses scanner traversal for fallback scans. ``summary`` observes
``BamRecordView`` records for bounded and full scans. ``check_tag`` performs
selected aux lookup through record-view aux helpers. Their JSON payloads remain
unchanged.

``validate``, ``inspect_duplication``, and ``forensic_inspect`` now use the
scanner for their scanner-compatible BAM body scans. Validation reads core
fields and aux structure from ``BamRecordView``. The forensic paths use borrowed
record sections for read names, sequences, qualities, RG evidence, and aux-tag
regime evidence. Writer-heavy transforms and paths that need owned whole-record
serialization remain on ``RecordLayout`` until later work moves them.

M3.9 added scanner-owned malformed-record tests, native differential coverage
against Bamana's ``RecordLayout`` bridge, dependency-boundary checks for scanner
and migrated hot paths, and ``scanner_microbench``. The benchmark emits JSON
with record scan throughput and selective field extraction throughput; see
``scanner_microbenchmarks`` for commands and schema details.

Boundaries
----------

Milestone 3 does not claim full BAM semantic validation, BAI/CSI random access,
native CRAM scanning, biological interpretation, or broad command parity. Those
remain downstream unless an explicit task in ``taskmap.md`` includes them.

Production BAM record hot-path scanning must not depend on ``noodles``. Test
oracles may use external parsers only when the test boundary is explicit and
production code remains native.

Task Tracking
-------------

The Milestone 3 task list is maintained in ``taskmap.md`` as M3.1 through
M3.10. Closeout evidence is recorded there and includes passing full tests,
passing contract tests, passing Sphinx documentation, a runnable scanner
microbenchmark smoke profile with machine-readable output, and recorded
command-migration evidence for the first scanner consumers.

Closeout Evidence
-----------------

M3.10 closed Milestone 3 on 2026-05-18. The closeout run completed ``cargo
test`` with 180 library tests, 18 contract tests, 2 header-oracle integration
tests, binary tests, and doc tests passing; ``cargo test --test contract`` with
18 contract tests passing; the Sphinx HTML build; and
``scanner_microbench --profile small --iterations 1`` with a JSON smoke check
verifying the small profile, one iteration, 1,024 generated records, and the
expected result schema.
