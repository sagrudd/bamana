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
* ``src/bam/index.rs`` detects BAI, CSI, GZI, and unknown sidecar magic,
  discovers adjacent index candidates, parses shallow BAI reference-count and
  pseudo-bin metadata summaries, and parses CSI headers enough to report
  detected-but-not-supported status.
* ``check_index`` reports adjacent index presence, selected path, kind,
  shallow syntactic validity, timestamp-based staleness, and compatibility.
* ``index`` creates real FASTQ.GZI sidecars for FASTQ.GZ inputs and reports
  BAM BAI/CSI writing as unimplemented instead of claiming sidecar creation.
* ``check_map`` and ``summary`` distinguish index-derived BAI metadata evidence
  from scan-derived evidence and fall back to scanner evidence when index
  metadata is absent or insufficient.

Outstanding M9 Work
-------------------

The current M9 gaps are intentional and must remain visible until implemented:

* BAM ``index`` cannot yet write real BAI or CSI sidecars.
* Native BGZF reading and BAM scanning do not yet expose per-record virtual
  offsets for BAI chunk and linear-index construction.
* BAI binning, chunk coalescing, metadata pseudo-bin emission, linear-index
  construction, and unplaced-unmapped accounting remain to be implemented.
* ``check_index`` does not yet validate chunks, virtual-offset ordering,
  linear-index monotonicity, reference span plausibility, or random-access
  usability.
* CSI remains header-only detection until M9 implements a scoped contract or
  records a precise deferral.
* ``check_map`` and ``summary`` do not yet use validated chunks for indexed
  acceleration.

Contract Boundary
-----------------

M9 does not change the public contract commands ``benchmark``, ``fastq``, or
``unmap``. Native CRAM parsing, indexed region selection commands, broad
random-access APIs, external comparator parity, and FASTQ.GZI work beyond the
existing sidecar contract remain outside M9 unless a later M9 task explicitly
promotes them.
