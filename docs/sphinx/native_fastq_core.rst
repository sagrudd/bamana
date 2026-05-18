Native FASTQ Core
=================

Milestone 4 makes the FASTQ and FASTQ.GZ path an explicit Bamana-native core
instead of an ingest-local helper. The public Rust facade remains
``crate::fastq`` so existing command consumers can keep stable imports while
the implementation is owned by focused modules.

Owned module layout
-------------------

``src/fastq/mod.rs``
   Narrow facade that exports the stable FASTQ API used by commands and keeps
   ``FASTQ.GZI`` available as ``crate::fastq::gzi``.

``src/fastq/record.rs``
   Owned ``FastqRecord`` data model, borrowed ``FastqRecordView``,
   read-name parsing, validation, plus-line preservation, field accessors, and
   FASTQ identity-byte construction.

``src/fastq/reader.rs``
   Plain FASTQ and extension-selected FASTQ.GZ reader entry points, record
   parsing, four-line validation, sequence/quality length validation, and
   record counting.

``src/fastq/writer.rs``
   Plain FASTQ and extension-selected gzip writer entry points, record writing,
   and finish/flush behavior.

``src/fastq/gzip.rs``
   The current FASTQ.GZ boundary. It owns extension-based gzip detection,
   ``MultiGzDecoder`` reader construction, and shared thread-count resolution
   for FASTQ.GZ consumers.

``src/fastq/unmapped.rs``
   FASTQ-to-unmapped-BAM conversion, including threaded FASTQ.GZ conversion and
   selected HTS-style methylation header tags.

``src/fastq/gzi.rs``
   ``FASTQ.GZI`` sidecar construction, reading, checkpoint metadata, and shard
   planning for FASTQ.GZ consumers.

``src/ingest/fastq.rs``
   Transitional compatibility shim that re-exports ``crate::fastq``. New FASTQ
   hot-path code should not be added there.

Current scope
-------------

The record contract validates the four-line FASTQ structure, ``@`` header
marker, ``+`` marker, sequence/quality length equality, and usable read name.
``FastqRecord`` is the stable owned record for consumers that retain or write
records. ``FastqRecordView`` is the borrowed view for field access and
identity construction when a consumer already has record line storage.

The gzip boundary follows the filename extension: paths ending in ``.gz``
case-insensitively are decoded with ``flate2::read::MultiGzDecoder`` and other
paths are treated as plain FASTQ. Concatenated gzip members are supported and
records are yielded in member order. Corrupt or truncated gzip input is
reported as a structured I/O error with the input path; it must not silently
truncate record counts.

``FASTQ.GZI`` sidecar construction uses the same multi-member gzip decoder
policy and remains compatible with indexed ``enumerate``, ``consume``, and
``explode`` paths.

The writer emits exactly four LF-terminated lines for each owned
``FastqRecord``: raw header line, sequence, preserved plus line, and quality.
It does not rewrite header comments or plus-line comments. Output compression is
selected by the final filename extension, matching reader behavior: paths
ending in ``.gz`` case-insensitively are written as gzip streams and all other
paths are written as plain FASTQ. ``FastqWriter::finish`` flushes plain output
and finalizes and flushes gzip output before returning success. Writer creation,
record writes, flushing, and gzip finalization failures are reported as
structured write errors with the output path.

Later M4 tasks harden command-consumer audits; the current contract establishes
module ownership, record ownership, borrowed field access, centralized FASTQ
identity bytes, auditable gzip member semantics, and writer round-trip
guarantees without changing command behavior.

Command consumers
-----------------

``enumerate`` uses ``count_fastq_records`` for plain FASTQ inputs, so plain
record counts flow through the same validated reader facade as other native
FASTQ consumers. FASTQ.GZ enumeration remains ``FASTQ.GZI`` sidecar-aware:
the command builds or reuses the sidecar and reports the indexed record total.

FASTQ-side ``subsample`` streams records through ``open_fastq_reader`` and
``read_next_fastq_record``, applies deterministic identity selection with
``FastqRecord`` identity bytes, and writes retained records with
``FastqWriter``. Because output is staged through a temporary file, the
temporary path preserves the final ``.gz`` writer policy case-insensitively so
``.fastq.gz`` and ``.FASTQ.GZ`` outputs both use gzip compression before being
renamed into place.

Unmapped ``consume`` imports use ``open_fastq_reader_with_label`` and
``read_next_fastq_record`` through the FASTQ-to-unmapped-BAM facade. Threaded
FASTQ.GZ import keeps logical input labels for structured errors and can use
``FASTQ.GZI`` totals to size worker batches without changing record semantics.

``inspect_duplication`` scans FASTQ and FASTQ.GZ with the same reader facade.
``deduplicate`` loads FASTQ records with the stable reader and writes retained
FASTQ records with ``write_fastq_records``. FASTQ.GZ ``explode`` uses
``FASTQ.GZI`` for shard planning, reads records through the stable reader, and
serializes each compressed shard batch through the shared writer record-line
helper before emitting concatenated gzip members.

Oracle And Benchmark Boundary
-----------------------------

Malformed FASTQ and FASTQ.GZ expectations are native-owned unit tests under
``src/fastq``. External parser crates may be used only in clearly labelled
test-only oracle surfaces; production FASTQ parser, writer, ``FASTQ.GZI``, and
command-consumer hot paths are protected by dependency-boundary contract tests.

``fastq_microbench`` provides local parser and writer smoke benchmarks for
plain FASTQ and FASTQ.GZ. It generates deterministic synthetic fixtures,
requires no private data, and emits JSON conforming to
``benchmarks/results/fastq_microbench.schema.json``.
