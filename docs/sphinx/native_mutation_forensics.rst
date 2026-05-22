Native Mutation, Remediation, And Forensics
===========================================

Milestone 7 is active as of 2026-05-22, after Milestone 6 closed on
2026-05-21. It hardens Bamana's native mutation, conservative remediation, and
provenance-inspection command wave:

* ``reheader``
* ``annotate_rg``
* ``inspect_duplication``
* ``deduplicate``
* ``forensic_inspect``

The milestone does not expand the scope of ``benchmark``, ``fastq``, or
``unmap``. Those commands remain public contract commands and keep their
existing contract, schema, example, documentation, and regression-test
expectations while M7 proceeds.

Baseline Native Paths
---------------------

``reheader`` uses Bamana's native BAM header codec to plan header-only
mutation, serializes replacement headers with native header serialization, and
writes rewrite-mode output through the native BGZF writer. It reports checksum
and index-invalidation evidence, but it does not add, remove, or replace
per-record ``RG:Z`` tags.

``annotate_rg`` is the record-level companion to ``reheader``. It uses the
native header codec, native aux-tag traversal, native record-layout
serialization, and native BGZF writer to annotate alignment records according
to explicit record-mode and header-policy options.

``inspect_duplication`` scans BAM inputs through ``BamScanner`` and scans
FASTQ or FASTQ.GZ inputs through the native FASTQ reader. Its evidence remains
collection-duplication and operator-error oriented; it is not biological
duplicate marking.

``deduplicate`` supports conservative BAM, FASTQ, and FASTQ.GZ remediation.
FASTQ input and output use the native FASTQ reader and writer. BAM planning
uses ``BamScanner`` plus ``BamRecordView`` conversion into the native
record-layout writer bridge, and BAM output uses native header serialization,
record-layout serialization, and the native BGZF writer.

``forensic_inspect`` is BAM-first and uses ``BamScanner`` plus native header,
record, aux-tag, read-name, and duplication-hallmark evidence. It remains
provenance inspection, not structural validation, duplicate marking, or fraud
detection.

Remaining Hardening
-------------------

M7 still needs contract and fixture hardening for dry-run versus applied
mutation, explicit remediation limits, forensic caveats, command-level smoke
benchmark evidence, and dependency-boundary tests that name all five M7 command
paths as one protected milestone set.
