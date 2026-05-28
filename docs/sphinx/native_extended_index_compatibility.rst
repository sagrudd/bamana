Native Extended Index Compatibility
===================================

Milestone 12 is complete as of 2026-05-28. It follows the completed Milestone 11
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

CSI Support-Level Freeze
------------------------

M12.2 freezes CSI as detect-only. The public ``check_index`` JSON contract now
reports ``support_level`` for the selected index and every discovered
candidate:

* ``read_write`` for BAI sidecars. Bamana can validate, read for current
  indexed traversal, and write native BAI for coordinate-sorted BAM.
* ``detect_only`` for CSI sidecars. Bamana can parse enough header metadata to
  report min-shift, depth, auxiliary length handling, and reference-count
  agreement, but CSI is not usable for traversal.
* ``planning_sidecar`` for FASTQ.GZI sidecars. These support FASTQ.GZ
  enumeration, consume planning, and explode planning, not BAM random access.
* ``unsupported`` for unknown sidecars.
* ``absent`` when no adjacent sidecar is selected.

Command-specific behavior is fixed for this milestone slice: ``index --format
csi`` remains an explicit BAM ``unimplemented`` path; ``check_map``,
``summary``, and ``select_region`` keep BAI-first traversal and use native scan
fallback when CSI is the selected adjacent sidecar. M12.2 does not implement
CSI bin parsing, chunk planning, random-access fetch, CSI writing, or
large-reference promotion.

Large-Reference Boundary
------------------------

M12.3 freezes BAI large-reference behavior at the BAI coordinate ceiling:
536,870,912 bases, or 2^29. ``bamana index --format bai`` rejects BAM input
when any reference length is greater than that threshold and fails before
writing an output sidecar. CSI remains detect-only, so ``index --format csi``
is not a large-reference fallback in this slice.

Diagnostic Hardening
--------------------

M12.4 makes ``check_map`` fallback diagnostics machine-readable. The
``index`` object reports ``diagnostic_status`` and optional
``diagnostic_detail``. The status values are ``usable``, ``absent``,
``stale``, ``unsupported``, ``malformed``, ``mismatched_reference``,
``incomplete``, ``disabled``, and ``not_checked``.

``summary`` and ``select_region`` keep their existing fallback notes and
region-scope fields in this slice. Those notes must continue to name stale,
unsupported, malformed, mismatched-reference, missing-index, and disabled-index
fallback causes explicitly when those states are observed.

Fixture Extension
-----------------

M12.5 extends the fixture plan for BAI/CSI/GZI compatibility without adding
binary assets in this slice. The reserved fixture ids are:

* ``tiny.invalid.large_reference.bam`` for BAI large-reference rejection during
  ``index --format bai``.
* ``tiny.invalid.mismatched_reference_count.csi`` for CSI reference-count
  mismatch diagnostics and scan fallback.
* ``tiny.invalid.fastq_gz.bad_gzi`` for malformed FASTQ.GZI planner-sidecar
  behavior in ``enumerate``, ``explode``, and ``consume``.

Materialized M12 fixtures must preserve the M12.2 support-level vocabulary and
the M12.4 diagnostic vocabulary.

Read-Only CSI Region Behavior
-----------------------------

M12.6 wires the detect-only CSI decision into read-only region evidence without
promoting CSI traversal. ``check_map --region`` and ``summary --region`` keep
BAI as the only index kind used for region traversal. When a CSI header sidecar
is the available adjacent index, both commands use native scan fallback and
preserve CSI context in the JSON payload:

* ``check_map --region`` reports ``index.kind: CSI`` with
  ``index.diagnostic_status: unsupported``.
* ``summary --region`` reports ``index_derived.kind: CSI`` and names CSI
  unsupported fallback in ``semantic_note``.

Selected-Region Compatibility
-----------------------------

M12.7 applies the M12 support decisions to ``select_region``. The selected BAM
file-output command still uses only usable non-stale BAI for indexed traversal.
It now reports the selected adjacent input sidecar in ``input_index``:

* usable BAI reports ``input_index.compatibility: used_bai``;
* CSI reports ``input_index.kind: CSI`` and
  ``input_index.compatibility: detect_only_csi``, then uses native scan
  fallback with ``execution.fallback_reason: detect_only_csi_index``;
* unsupported, missing, stale, malformed, incomplete, and disabled input-index
  states remain explicit fallback causes.

Output BAI/CSI invalidation remains governed by
``output.index_invalidation``. ``select_region`` still does not create a
replacement output index.

Public Contract Refresh
-----------------------

M12.8 consolidates the public schema, example, CLI, roadmap, Sphinx, and
fixture documentation for the M12 support decisions:

* ``check_index`` governs ``support_level`` for selected and candidate
  sidecars: BAI ``read_write``, CSI ``detect_only``, FASTQ.GZI
  ``planning_sidecar``, unknown ``unsupported``, and absent ``absent``.
* ``check_map`` governs ``index.diagnostic_status`` in whole-file and region
  examples.
* ``summary`` governs ``index_derived`` in the schema and region examples,
  including the rule that region-scoped indexed traversal omits whole-file BAI
  totals.
* ``select_region`` governs ``input_index`` and its compatibility values,
  including ``used_bai`` and ``detect_only_csi``.
* Fixture docs map the reserved M12 assets to the public contracts for
  large-reference BAI rejection, CSI reference-count mismatch, and malformed
  FASTQ.GZI planner-sidecar behavior.

Benchmark And Dependency Guardrails
-----------------------------------

M12.9 adds command smoke timing and dependency-boundary evidence for the
extended index compatibility surface. ``scanner_microbench --bamana-bin`` emits
``check_index_csi_detect_only``, ``check_map_region_csi_fallback``,
``summary_region_csi_fallback``, and ``select_region_csi_fallback`` rows.
Those rows cover CSI detect-only support-level reporting, CSI-preserving native
scan fallback, and selected-region ``input_index`` compatibility.

The benchmark notes are deliberately narrow: the M12 CSI rows do not claim CSI
bin parsing, CSI chunk planning, CSI random-access traversal, CSI writing, or
large-reference CSI support. Dependency-boundary tests keep ``check_index``,
``check_map``, ``summary``, and ``select_region`` index compatibility hot paths
Bamana-native outside the documented CRAM compatibility boundary.

Closeout
--------

M12.10 closes Milestone 12 after M12.1 through M12.10 completed on
2026-05-28. Closeout verification passed with ``cargo test``,
``cargo test --test contract``, Sphinx HTML documentation,
``cargo build --bin bamana --bin scanner_microbench``, scanner smoke evidence,
``cargo fmt --check``, and ``git diff --check``.

The scanner smoke run used ``target/debug/scanner_microbench --profile small
--iterations 1 --bamana-bin target/debug/bamana --out
/tmp/bamana-m12-10-scanner-small.json`` and confirmed
``check_index_csi_detect_only``, ``check_map_region_csi_fallback``,
``summary_region_csi_fallback``, and ``select_region_csi_fallback`` as
``1/1``.

Residual risk remains explicit and deferred: CSI bin parsing, CSI chunk
planning, CSI random-access traversal, CSI writing, large-reference CSI
support, native CRAM indexed queries, replacement output-index creation, and
broad comparator parity are not complete in M12.

Non-Goals
---------

M12 does not make CRAM indexing native, does not claim biological
interpretation, and does not claim external comparator parity beyond measured
index compatibility checks.
