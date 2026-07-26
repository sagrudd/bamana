Native Transform, Checksum, Explode, And Ingest
===============================================

Milestone 8 is complete as of 2026-05-23. It became active on 2026-05-22,
after Milestone 7 closed on 2026-05-22, and closed after M8.1 through M8.10
completed. It hardens Bamana's largest transform and ingest command wave:

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
default implementation remains deliberately in-memory. Supplying
``--memory-limit BYTES`` instead accumulates native record layouts to the
target budget, orders each run with up to ``--threads`` workers, spills runs
beside the destination, and performs a stable multiway merge into an atomically
published BAM. Temporary runs are cleaned on success and best-effort cleaned
on failure. The target is a retained-record budget rather than a whole-process
RSS ceiling. Ordered BGZF output uses a bounded asynchronous pipeline with at
most two full blocks in flight per worker. Compression overlaps record
production while sequence-numbered results preserve deterministic bytes; the
fixed 64,000-byte full-block payload fits worst-case DEFLATE and BGZF framing
without cross-block retries. The stable multiway heap itself remains
single-threaded. Natural queryname ordering remains explicitly deferred.
``scanner_microbench --bamana-bin`` includes a ``sort`` command smoke timing
for coordinate rewrite with canonical checksum verification.

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

Checksum Hardening
------------------

M8.5 hardens ``checksum`` at the native checksum-domain boundary. BAM input
loading now uses ``BamScanner`` and bridges each ``BamRecordView`` into the
native checksum serialization path before raw encounter-order, canonical
order-insensitive, payload, or all-domain hashing. The header domain continues
to use Bamana's deterministic header checksum serializer and does not scan
alignment records when ``--mode header`` is requested.

The checksum modes have intentionally narrow meanings. ``raw-record-order``
hashes Bamana's stable per-record serialization in encounter order and is
order-sensitive. ``canonical-record-order`` hashes the same per-record
serialization, sorts per-record digests, and preserves duplicate multiplicity;
it is suitable for selected content comparisons where record order may differ,
but it does not prove whole-file equivalence. ``header`` covers deterministic
header text and binary reference dictionary serialization. ``payload`` covers
the encounter-order record stream and optionally prefixes the deterministic
header serialization when ``--include-header`` is set. Filters and excluded
tags are part of the reported checksum definition. ``scanner_microbench
--bamana-bin`` now includes a ``checksum`` command smoke timing for all-domain
hashing with header inclusion, ``NM`` exclusion, and mapped-only filtering.

Explode Hardening
-----------------

M8.6 hardens ``explode`` at the native sharding boundary. BAM shard counting
and writing now use ``BamScanner`` and bridge scanner-owned records into
Bamana's native BGZF writer while preserving the parsed BAM header in every
shard. SAM sharding preserves leading header lines and writes contiguous
alignment-line ranges. FASTQ.GZ sharding continues to create or reuse adjacent
``FASTQ.GZI`` metadata, derive checkpoint-aware record ranges, and write
concatenated gzip-member shard streams.

The hardened test surface covers BAM shard contents and encounter-order
preservation, SAM shard headers, FASTQ.GZ shard writing, uneven FASTQ.GZ ranges,
``FASTQ.GZI`` planning evidence, empty BAM rejection, too-many-shards
rejection, and output collision behavior. The command guarantees that every
supported record lands in exactly one reported shard range and that encounter
order is preserved within each shard. It does not claim reconstruction
equivalence beyond the emitted range metadata, uniform shard sizes, or generic
random-access parallel gzip inflate. ``scanner_microbench --bamana-bin`` now
includes an ``explode`` command smoke timing for scanner-backed BAM contiguous
shard writing.

Consume Hardening
-----------------

M8.7 hardens ``consume`` at the native ingest and policy boundary. BAM
alignment inputs now use ``BamScanner`` and bridge scanner-owned records into
Bamana's native BGZF writer. SAM alignment ingest remains on the native SAM
parser, FASTQ and FASTQ.GZ unmapped ingest remain on the native FASTQ readers,
and CRAM remains a documented compatibility path governed by explicit reference
policy.

The command keeps policy decisions explicit: mixed alignment/raw inputs are
rejected with per-file reasons, include/exclude glob filtering remains
deferred, coordinate indexing after consume remains deferred, and checksum
verification remains explicitly unimplemented in this slice with requested but
unperformed checksum state in the payload. The hardened test surface covers
recursive directory dry-run discovery, mixed-format rejection, FASTQ.GZ
parallel and indexed import, BAM/SAM alignment mode, CRAM reference policy,
force/overwrite safety, and checksum deferral. ``scanner_microbench
--bamana-bin`` now includes a ``consume`` command smoke timing for
scanner-backed BAM alignment ingest with explicit policy reporting.

Output Safety
-------------

M8.8 strengthens the shared output-safety contract for ``sort``, ``merge``,
``explode``, and ``consume``. These commands write completed payloads to
operation-scoped temporary files and only publish them through a final rename
after the write loop has succeeded. Existing outputs are rejected unless
``--force`` is supplied, and the force path no longer deletes the existing
output before the completed temporary candidate is ready to publish.

The regression surface covers no-force collision cleanup, forced replacement,
multi-output preflight behavior for shard publishing, and command-level
finalization failure where a non-file output target remains intact and the
temporary candidate is removed. Dry-run behavior remains side-effect bounded,
and checksum or index payload fields continue to report only work that was
actually performed.

Dependency And Benchmark Guardrails
-----------------------------------

M8.9 strengthens the dependency-boundary and benchmark guardrails for
``sort``, ``merge``, ``explode``, ``checksum``, and ``consume``. Contract tests
explicitly name those command hot paths and keep them free of direct production
``noodles`` imports outside documented CRAM compatibility. The testing-oracle
policy records the same command set and keeps malformed transform and ingest
expectations owned by native tests.

The scanner microbenchmark command hooks are runnable for every M8 command and
are documented as smoke timings, not comparator-parity claims. The benchmark
notes distinguish process startup, JSON emission, full-record materialization,
in-memory sorting, merge compatibility and merge-ordering cost, native BGZF
compression, checksum-domain traversal, shard planning, ingest normalization,
and CRAM compatibility behavior.

Closeout
--------

M8.10 closes Milestone 8. Closeout verification reran ``cargo test``, ``cargo
test --test contract``, the scanner microbenchmark small smoke profile with
``--bamana-bin``, and the Sphinx HTML build. The scanner smoke profile reported
successful command timings for every M8 command. Milestone 9 remains planned
for explicit activation by M9.1.

Activation Boundary
-------------------

M8 evidence is limited to ``sort``, ``merge``, ``explode``, ``checksum``, and
``consume``. Native CRAM, BAM index writing, and random-access ownership remain
later milestones unless explicitly included by an M8 task.
