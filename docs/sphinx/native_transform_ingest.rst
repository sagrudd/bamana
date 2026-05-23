Native Transform, Checksum, Explode, And Ingest
===============================================

Milestone 8 is active as of 2026-05-22, after Milestone 7 closed on
2026-05-22. It hardens Bamana's largest transform and ingest command wave:

* ``sort``
* ``merge``
* ``explode``
* ``checksum``
* ``consume``

The milestone covers ordering, merging, sharding, checksum domains, and
multi-format ingestion. The scope is explicit operational behavior,
native-hot-path ownership, command contracts, benchmark smoke coverage, and
dependency-boundary protection. It does not claim full external-tool parity or
native CRAM ownership.

Baseline Native Paths
---------------------

``sort`` already rewrites BAM through Bamana's native sorting module, supports
coordinate and queryname ordering, updates ``@HD`` sort metadata, writes
through native BAM/BGZF output paths, and can request canonical checksum
verification.

``merge`` already combines BAM inputs with conservative reference-dictionary
compatibility checks, supports input-order, coordinate, and queryname output
modes, and writes through native output paths.

``checksum`` already uses deterministic native header and record serialization
domains for raw encounter-order, canonical order-insensitive, header, and
payload checksums.

``explode`` already has BAM, SAM, and FASTQ.GZ shard planning paths. The
FASTQ.GZ path can create or reuse adjacent ``FASTQ.GZI`` metadata for
contiguous shard planning.

``consume`` already has discovery, format classification, policy enforcement,
FASTQ/SAM/BAM normalization, threaded FASTQ.GZ import, dry-run reporting, and
explicit CRAM reference-policy handling. CRAM remains a documented
compatibility boundary rather than native ownership.

Baseline Gaps
-------------

M8.1 records the following activation gaps for later M8 tasks:

* ``sort``, ``merge``, ``checksum``, BAM-side ``explode``, and BAM-side
  ``consume`` still use older ``BamReader::open``,
  ``parse_bam_header_from_reader``, and ``read_next_record_layout`` paths in
  several places.
* ``sort``, ``merge``, canonical ``checksum``, and BAM/SAM sharding use
  in-memory first-slice strategies that need documented limits, benchmark
  interpretation, and later external-memory follow-up where appropriate.
* ``consume --verify-checksum`` remains planned in this slice and needs either
  implementation or precise deferral language during M8.
* ``explode`` shard guarantees need a full M8 audit across BAM, SAM, and
  FASTQ.GZ, especially around checksums, index metadata, exact boundary claims,
  and uneven ``FASTQ.GZI`` checkpoint-aligned shard sizes.

Contract Freeze
---------------

M8.2 freezes the governed contract surface for ``sort``, ``merge``,
``explode``, ``checksum``, and ``consume``. The command set has JSON schemas,
canonical success and failure examples, CLI contracts, JSON-output
documentation, README coverage, Sphinx coverage, and fixture reservations for
sorted BAMs, merge compatibility, shard planning, checksum filters,
``FASTQ.GZI`` planning, and mixed-format ingest.

The frozen contracts describe ordering semantics, checksum domains, shard
boundaries, ingest policy, dry-run behavior, CRAM compatibility, deferred
checksum verification, deferred index behavior, and in-memory first-slice
caveats. They do not claim full external-tool parity or native CRAM ownership.

Sort Hardening
--------------

M8.3 hardens ``sort`` as the first M8 implementation target. The command now
loads BAM records through ``BamScanner`` and bridges each ``BamRecordView`` into
the native record-layout writer path before ordering and BGZF output. This
keeps header parsing, record loading, ordering, serialization, BGZF writing,
and optional canonical checksum verification within Bamana-native code.

The hardened test surface covers coordinate ordering, unmapped placement,
reverse/tie ordering, lexicographical queryname ordering, header sort metadata,
overwrite safety, checksum verification reporting, and index deferral. The
current implementation remains an in-memory first slice, and natural queryname
ordering remains explicitly deferred. ``scanner_microbench --bamana-bin`` now
includes a ``sort`` command smoke timing for coordinate rewrite with canonical
checksum verification.

Merge Hardening
---------------

M8.4 hardens ``merge`` at the native compatibility and ordering boundary. BAM
input loading now uses ``BamScanner`` and bridges each ``BamRecordView`` into
the native record-layout writer path before input-order, coordinate, or
queryname merge output. Header parsing, reference dictionary compatibility
checks, ordering, serialization, BGZF writing, and optional canonical multiset
checksum verification remain within Bamana-native code.

The hardened test surface covers compatible and incompatible headers,
input-order merge, coordinate merge, lexicographical queryname merge, record
counts, overwrite safety, checksum verification reporting, and coordinate index
deferral. Merge validity is limited to inputs that Bamana parses and checks:
the command does not claim whole-file validity beyond the parsed header,
materialized records, compatible binary reference dictionaries, requested
ordering, write completion, and any optional checksum verification that was
actually performed. The current implementation remains an in-memory first
slice. ``scanner_microbench --bamana-bin`` now includes a ``merge`` command
smoke timing for coordinate merge with canonical checksum verification.

Activation Boundary
-------------------

M8 evidence is limited to ``sort``, ``merge``, ``explode``, ``checksum``, and
``consume``. Native CRAM, BAM index writing, and random-access ownership remain
later milestones unless explicitly included by an M8 task.
