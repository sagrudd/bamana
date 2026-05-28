Native Extended Index Compatibility
===================================

Milestone 12 is active as of 2026-05-28. It follows the completed Milestone 11
selected-region file-output milestone and does not reopen Milestone 11
contracts.

Activation Baseline
-------------------

M12.1 activates the extended index compatibility scope without changing CLI
behavior. The current baseline is:

* BAI detection, parsing, structural validation, mapped/unmapped metadata
  extraction, timestamp-staleness checks, and native BAI writing are
  implemented for coordinate-sorted BAM inputs.
* ``check_index`` discovers adjacent BAI, CSI, GZI, and unknown sidecars.
  ``--prefer-csi`` changes discovery priority, but CSI remains detected rather
  than usable for random-access traversal.
* CSI parsing is header-only enough to report min-shift, depth, auxiliary
  length, and reference count. Matching CSI headers are reported as
  detected-but-not-supported; mismatched or malformed CSI sidecars are invalid.
* ``index --format csi`` remains explicitly unimplemented for BAM. CSI
  large-reference behavior, CSI writing, and CSI-backed random access are not
  promoted by M12.1.
* ``check_map``, ``summary``, and ``select_region`` use usable non-stale BAI
  sidecars for indexed evidence or selected-output traversal. Missing, stale,
  unsupported, malformed, or incomplete index state falls back to native scans
  where those commands already support fallback.
* ``select_region`` invalidates adjacent output BAI/CSI sidecars but does not
  create a replacement output index.
* FASTQ.GZ indexing writes Bamana-native ``FASTQ.GZI`` sidecars for
  enumeration, consume planning, and explode planning. ``FASTQ.GZI`` is not a
  BAM random-access index and does not imply generic gzip random-access parity.

Remaining M12 Work
------------------

Later M12 tasks must freeze CSI support levels, large-reference thresholds,
stale/mismatched/unsupported-index diagnostics, fixtures, selected-region
compatibility behavior, schemas, examples, docs, benchmark guardrails, and
closeout evidence.

Non-Goals
---------

M12 does not make CRAM indexing native, does not claim biological
interpretation, and does not claim external comparator parity beyond measured
index compatibility checks.
