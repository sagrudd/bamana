Native BAM Record Scanner
=========================

Milestone 3 is active. Bamana's next native-core layer is a selective BAM
record scanner built above the native BGZF reader and native BAM header codec.

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

The first command beneficiaries are ``check_sort``, ``check_map``,
``summary``, ``check_tag``, ``validate``, ``inspect_duplication``,
``forensic_inspect``, and BAM-side ``subsample``.

Baseline
--------

The current record bridge is ``src/bam/records.rs``. It validates the BAM core
layout and variable-section lengths, then materializes read name, CIGAR bytes,
sequence bytes, quality bytes, and aux bytes into ``RecordLayout``.
``LightAlignmentRecord`` already exposes the fields needed by some scan
commands, but it is derived from that fully materialized layout. It is therefore
a convenience view, not the final selective scanner.

``src/bam/tags.rs`` already provides bounded auxiliary-field traversal and tag
lookup over materialized aux bytes. Milestone 3 should keep the bounded
traversal behavior while moving it onto scanner-owned aux slices or ranges.

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

This loop is the scanner substrate, not command migration. Later milestone
tasks should centralize selective field helpers, move auxiliary traversal onto
scanner-owned ranges, and migrate command consumers onto ``BamScanner``.

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

The active task list is maintained in ``taskmap.md`` as M3.1 through M3.10.
Closeout evidence must include passing full tests, passing contract tests,
passing Sphinx documentation, runnable scanner microbenchmarks with
machine-readable output, and recorded command-migration evidence for the first
scanner consumers.
