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

``sort`` already rewrites BAM through Bamana's native sorting module, updates
``@HD`` sort metadata, writes through native BAM/BGZF output paths, and can
request canonical checksum verification.

``merge`` already combines BAM inputs with conservative reference-dictionary
compatibility checks and native output writing.

``checksum`` already uses deterministic native header and record serialization
domains for raw and canonical checksums.

``explode`` already has BAM, SAM, FASTQ, and FASTQ.GZ shard planning paths, but
Milestone 8 must audit shard guarantees and output behavior across formats.

``consume`` already has native discovery and FASTQ/SAM/BAM ingest paths, while
CRAM remains a documented compatibility boundary rather than native ownership.

Activation Boundary
-------------------

M8 evidence is limited to ``sort``, ``merge``, ``explode``, ``checksum``, and
``consume``. Native CRAM, BAM index writing, and random-access ownership remain
later milestones unless explicitly included by an M8 task.
