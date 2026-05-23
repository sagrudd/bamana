Native BAM Index And Random Access
==================================

Milestone 9 is active as of 2026-05-23 after Milestone 8 closed the transform,
checksum, explode, and ingest command wave. The active M9 scope is native BAM
index writing, deeper BAM index validation, virtual-offset-backed
random-access groundwork, and first index-aware consumer evidence.

Current Baseline
----------------

The repository already has the following native index groundwork:

* ``src/bgzf/virtual_offset.rs`` owns packed BGZF virtual offsets with bounds
  checks and ordering.
* ``src/bgzf/reader.rs`` exposes typed virtual offsets for the current native
  BGZF read cursor by combining compressed member starts with uncompressed
  in-block offsets.
* ``src/bam/scan.rs`` exposes ``next_record_with_virtual_offsets`` so scanner
  and future index code can obtain typed start/end offsets for each alignment
  record.
* ``src/bam/index.rs`` exposes ``build_bai_index_from_bam`` for native
  in-memory BAI bin, chunk, linear-index, and unmapped-count construction from
  scanner-owned record traversal.
* ``src/bam/index.rs`` detects BAI, CSI, GZI, and unknown sidecar magic,
  discovers adjacent index candidates, parses shallow BAI reference-count and
  pseudo-bin metadata summaries, and parses CSI headers enough to report
  detected-but-not-supported status.
* ``check_index`` reports adjacent index presence, selected path, kind,
  shallow syntactic validity, timestamp-based staleness, and compatibility.
  BAI inspection is currently limited to magic, reference-count agreement,
  top-level parseability, and metadata summaries where present. CSI inspection
  is header-only detection until scoped support lands.
* ``index`` creates real FASTQ.GZI sidecars for FASTQ.GZ inputs and reports
  BAM BAI/CSI writing as unimplemented instead of claiming sidecar creation.
  Existing sidecars are not overwritten unless ``--force`` is supplied, and
  BAM responses keep ``output_index.created`` false until a real BAI/CSI
  sidecar is written.
* ``check_map`` and ``summary`` distinguish index-derived BAI metadata evidence
  from scan-derived evidence and fall back to scanner evidence when index
  metadata is absent or insufficient.

Outstanding M9 Work
-------------------

The current M9 gaps are intentional and must remain visible until implemented:

* BAM ``index`` cannot yet write real BAI or CSI sidecars.
* BAI serialization and metadata pseudo-bin emission remain to be implemented.
* ``check_index`` does not yet validate chunks, virtual-offset ordering,
  linear-index monotonicity, reference span plausibility, or random-access
  usability.
* CSI remains header-only detection until M9 implements a scoped contract or
  records a precise deferral.
* ``check_map`` and ``summary`` do not yet use validated chunks for indexed
  acceleration.

Frozen Fixture Plan
-------------------

M9.2 freezes the planned index fixture taxonomy before implementation work:

* valid BAI: ``tiny.valid.coordinate.bai``
* malformed BAI: ``tiny.invalid.bad_bai``
* mismatched BAI reference count:
  ``tiny.invalid.mismatched_reference_count.bai``
* stale BAI timestamp heuristic: ``tiny.valid.coordinate.stale_bai``
* CSI header detection with unsupported fallback:
  ``tiny.valid.coordinate.csi_header``
* malformed CSI: ``tiny.invalid.bad_csi``
* coordinate-sorted BAM source: ``tiny.valid.coordinate``
* unsorted BAM index rejection: ``tiny.invalid.unsorted_coordinate``
* FASTQ.GZ source and FASTQ.GZI sidecar:
  ``tiny.valid.fastq_gz`` and ``tiny.valid.fastq_gz.gzi``

Virtual-Offset Plumbing
-----------------------

M9.3 adds the native offset substrate used by later BAI/CSI implementation.
The BGZF reader tracks each member's compressed file offset and reports a
``VirtualOffset`` for the current uncompressed cursor. The BAM scanner reports
record start and end offsets through ``next_record_with_virtual_offsets``.

The offsets feed index construction as follows:

* BAI chunks use the virtual offset at the first and last record boundary in a
  candidate span.
* BAI linear-index windows use the earliest start virtual offset observed for a
  genomic window.
* CSI support, if promoted, will reuse the same typed offsets.
* A record ending exactly at a BGZF member boundary is represented as the next
  compressed member start with in-block offset zero, matching BGZF
  virtual-offset semantics.

Native BAI Builder
------------------

M9.4 adds native BAI data construction but does not yet change public command
behavior. ``build_bai_index_from_bam`` traverses BAM records through
``BamScanner::next_record_with_virtual_offsets`` and produces in-memory
reference indexes with:

* BAI bin assignment from mapped record start/end coordinates.
* Per-reference, per-bin chunks using typed virtual-offset record spans.
* Adjacent or overlapping chunk coalescing.
* 16kb linear-index windows populated from record start offsets.
* Counts for mapped reads, reference-associated unmapped reads, and unplaced
  unmapped reads.

The builder rejects mapped records outside the header reference dictionary,
negative mapped coordinates, coordinates outside the BAI addressable range, and
mapped records that violate coordinate order. M9.5 remains responsible for BAI
serialization, metadata pseudo-bin emission, atomic sidecar writing, and
``bamana index`` payload changes.

Contract Boundary
-----------------

M9 does not change the public contract commands ``benchmark``, ``fastq``, or
``unmap``. Native CRAM parsing, indexed region selection commands, broad
random-access APIs, external comparator parity, and FASTQ.GZI work beyond the
existing sidecar contract remain outside M9 unless a later M9 task explicitly
promotes them.
