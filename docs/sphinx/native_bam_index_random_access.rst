Native BAM Index And Random Access
==================================

Milestone 9 is complete as of 2026-05-23 after Milestone 8 closed the
transform, checksum, explode, and ingest command wave. The completed M9 scope
is native BAM index writing, deeper BAM index validation, virtual-offset-backed
random-access groundwork, and first index-aware consumer evidence.

Current Baseline
----------------

The repository already has the following native index groundwork:

* ``src/bgzf/virtual_offset.rs`` owns packed BGZF virtual offsets with bounds
  checks and ordering.
* ``src/bgzf/reader.rs`` exposes typed virtual offsets for the current native
  BGZF read cursor by combining compressed member starts with uncompressed
  in-block offsets. It can also seek back to a typed ``VirtualOffset`` by
  reopening a BGZF member and setting the in-block cursor.
* ``src/bam/scan.rs`` exposes ``next_record_with_virtual_offsets`` so scanner
  and future index code can obtain typed start/end offsets for each alignment
  record. It also exposes internal raw-record range helpers for reading records
  from validated virtual-offset chunk ranges.
* ``src/bam/index.rs`` exposes ``build_bai_index_from_bam`` for native
  in-memory BAI bin, chunk, linear-index, and unmapped-count construction from
  scanner-owned record traversal.
* ``src/bam/index.rs`` serializes native BAI sidecars, including regular bins,
  metadata pseudo-bin counts, linear-index windows, and trailing unplaced
  unmapped counts.
* ``src/bam/index.rs`` detects BAI, CSI, GZI, and unknown sidecar magic,
  discovers adjacent index candidates, validates BAI reference counts, bin
  shape, chunk virtual-offset ordering, linear-index ordering, metadata
  pseudo-bin shape, and parses CSI headers enough to report
  detected-but-not-supported or reference-mismatch status.
* ``check_index`` reports adjacent index presence, selected path, kind,
  syntactic validity, timestamp-based staleness, compatibility, and apparent
  usability. BAI inspection validates implemented structural invariants without
  claiming every random-access offset has been exercised. CSI inspection is
  header-only detection until scoped support lands.
* ``index`` creates native BAI sidecars for coordinate-sorted BAM inputs and
  real FASTQ.GZI sidecars for FASTQ.GZ inputs. Existing sidecars are not
  overwritten unless ``--force`` is supplied, and CSI writing still returns an
  explicit ``unimplemented`` response.
* ``check_map`` and ``summary`` distinguish index-derived BAI metadata evidence
  from scan-derived evidence. They use BAI metadata only when the selected
  sidecar is not timestamp-stale, passes the implemented structural checks, and
  supplies complete mapped/unmapped reference metadata. Generated BAI sidecars
  and discovered BAI sidecars use the same validation path because BAI has no
  provenance marker. Absent, stale, unsupported, malformed, or incomplete
  sidecars fall back to scanner evidence with a payload note.

Deferred Beyond M9
------------------

The following gaps are intentional post-M9 deferrals:

* BAM ``index`` cannot yet write CSI sidecars.
* Public commands do not yet use random-access chunk traversal for acceleration
  or region filtering.
* CSI remains header-only detection until a later scoped contract or precise
  long-term deferral is recorded.
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

Native BAI Builder And Writer
-----------------------------

M9.4 adds native BAI data construction and M9.5 routes BAM BAI creation through
that builder and a native serializer. ``build_bai_index_from_bam`` traverses
BAM records through
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
mapped records that violate coordinate order. ``write_bai_index`` serializes
regular bins, the metadata pseudo-bin with mapped/unmapped counts, the linear
index, and trailing unplaced unmapped counts. ``bamana index`` writes BAI
through a temporary file and final rename, rejects existing outputs unless
``--force`` is supplied, and rejects header-declared non-coordinate BAM order.

Random-Access Groundwork
------------------------

M9.7 adds the first native random-access substrate without promoting a public
region-query command. ``NativeBgzfReader::seek_virtual_offset`` consumes a
``VirtualOffset``, seeks to the compressed BGZF member, inflates that member,
and positions the uncompressed cursor at the in-block component. Invalid
members, EOF-marker targets, and impossible in-block offsets fail as structured
BAM errors.

``BamScanner`` exposes ``seek_virtual_offset`` plus
``raw_records_in_virtual_range`` for internal consumers. The range helper
accepts typed start/end virtual offsets, rejects empty or reversed ranges, and
returns positioned raw BAM records that tests can parse back into
``BamRecordView`` values. M9.7 tests prove both a manually constructed
virtual-offset range and a native BAI chunk can retrieve expected records.

First Consumer Evidence
-----------------------

M9.8 integrates the hardened BAI usability rules into ``check_map`` and
``summary`` without promoting indexed random-access acceleration. The commands
may use complete BAI mapped/unmapped metadata as index-derived evidence after
timestamp and structural checks succeed. They do not use stale sidecars,
malformed BAI payloads, unsupported sidecar kinds, or BAI files missing complete
metadata for the requested references. Those cases remain native scanner
fallbacks and the JSON payload notes identify the fallback reason.

Dependency And Benchmark Guardrails
-----------------------------------

M9.9 protects the BAM index and random-access set in contract tests. The
dependency-boundary tests explicitly name ``index``, ``check_index``, indexed
``check_map``, indexed ``summary``, and the BGZF/BAM random-access substrate as
Bamana-native hot paths that must stay free of direct ``noodles`` imports
outside the documented CRAM compatibility boundary.

``scanner_microbench --bamana-bin`` emits M9 command smoke timings for
``index_bam``, ``check_index``, ``check_map_indexed``, and
``summary_indexed``. The existing ``check_map`` and ``summary`` timings remain
scan-fallback timings because they run before the synthetic BAI sidecar is
created. The M9 timing notes distinguish BAM index construction, BAI structural
validation, index metadata-backed consumer evidence, scan fallback timings,
random-access lookup deferral, process startup, JSON emission, and comparator
non-parity.

Closeout Evidence
-----------------

M9.10 closes the milestone with native BAI sidecar creation, hardened
``check_index`` validation, typed virtual-offset capture, internal range
retrieval helpers, first index-aware ``check_map`` and ``summary`` evidence,
dependency-boundary protection, and M9 command benchmark smoke rows in place.

Full tests, contract tests, Sphinx, and the M9 scanner microbenchmark smoke
profile passed at closeout. CSI writing, public indexed-region command
acceleration, broad random-access APIs, and comparator parity remain deferred
beyond M9.

Contract Boundary
-----------------

M9 does not change the public contract commands ``benchmark``, ``fastq``, or
``unmap``. Native CRAM parsing, indexed region selection commands, broad
random-access APIs, external comparator parity, and FASTQ.GZI work beyond the
existing sidecar contract remain outside M9 unless a later M9 task explicitly
promotes them.
