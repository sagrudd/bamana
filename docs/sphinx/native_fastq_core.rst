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

The gzip boundary currently follows the filename extension and uses
``flate2::read::MultiGzDecoder`` for ``.gz`` inputs. The writer emits
line-oriented FASTQ records and chooses gzip output from the output filename
extension. Later M4 tasks harden the stream semantics, round-trip guarantees,
and command-consumer audits; the current contract establishes module ownership,
record ownership, borrowed field access, and centralized FASTQ identity bytes
without changing command behavior.
