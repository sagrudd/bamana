# Milestone 12: Extended Index Compatibility

Status: active as of 2026-05-28. Milestone 12 follows the completed M11
selected-region file-output milestone and does not reopen Milestone 11
contracts.

## Goal

Harden index compatibility beyond the M9/M10 BAI-first contract. The milestone
focuses on CSI, large-reference behavior, stale-index policy, cross-index
selection, and compatibility evidence for every command that relies on index
state.

## M12.1 Activation And Baseline Audit

M12.1 activates the extended index compatibility milestone without changing CLI
behavior. The current baseline is intentionally conservative:

* BAI detection, parsing, structural validation, mapped/unmapped metadata
  extraction, timestamp-staleness checks, and native BAI writing are
  implemented for coordinate-sorted BAM inputs.
* `check_index` discovers adjacent BAI, CSI, GZI, and unknown sidecars.
  `--prefer-csi` changes discovery priority, but CSI remains detected rather
  than usable for random-access traversal.
* CSI parsing is header-only enough to report min-shift, depth, auxiliary
  length, and reference count. Matching CSI headers are reported as
  detected-but-not-supported; mismatched or malformed CSI sidecars are invalid.
* `index --format csi` remains explicitly unimplemented for BAM. CSI
  large-reference behavior, CSI writing, and CSI-backed random access are not
  promoted by M12.1.
* `check_map`, `summary`, and `select_region` use usable non-stale BAI sidecars
  for indexed evidence or selected-output traversal. Missing, stale,
  unsupported, malformed, or incomplete index state falls back to native scans
  where those commands already support fallback.
* `select_region` invalidates adjacent output BAI/CSI sidecars but does not
  create a replacement output index.
* FASTQ.GZ indexing writes Bamana-native `FASTQ.GZI` sidecars for enumeration,
  consume planning, and explode planning. FASTQ.GZI is not a BAM random-access
  index and does not imply generic gzip random-access parity.

M12.1 reserves later tasks for CSI support-level decisions, large-reference
thresholds, stale/mismatched/unsupported diagnostic hardening, fixtures,
selected-region compatibility updates, schemas/examples/docs, benchmarks, and
closeout evidence.

## M12.2 CSI Support-Level Freeze

M12.2 freezes CSI as **detect-only** for Milestone 12 until a later task
explicitly promotes read compatibility. This is a contract decision, not an
implementation accident:

* `check_index` reports machine-readable `support_level` values. BAI is
  `read_write`; CSI is `detect_only`; FASTQ.GZI is `planning_sidecar`;
  unknown sidecars are `unsupported`; and absent sidecars are `absent`.
* CSI detection remains limited to magic, min-shift, depth, auxiliary length
  skipping, and reference-count agreement with the BAM header. Matching CSI
  headers are syntactically valid but not usable for random-access traversal.
* `index --format csi` remains an explicit `unimplemented` BAM path and must
  not create placeholder CSI output.
* `check_map`, `summary`, and `select_region` remain BAI-first. CSI sidecars
  trigger native scan fallback rather than indexed traversal.
* M12.2 does not add CSI bin parsing, CSI chunk planning, CSI random-access
  fetch, CSI writing, or large-reference promotion.

## M12.3 Large-Reference Behavior

M12.3 freezes the BAI coordinate ceiling at 2^29 bases. BAM `index` rejects BAI
creation when any reference length is greater than 536,870,912 bases and
returns `invalid_index` before writing an output sidecar. References at or
below the threshold remain eligible for BAI creation, subject to the existing
coordinate-sort and record-coordinate checks. CSI remains detect-only, so
`index --format csi` is still not a large-reference replacement in this slice.

## M12.4 Index Diagnostic Hardening

M12.4 adds machine-readable `check_map.index.diagnostic_status` and optional
`diagnostic_detail` fields so fallback causes are explicit rather than only
embedded in prose. The statuses distinguish `usable`, `absent`, `stale`,
`unsupported`, `malformed`, `mismatched_reference`, `incomplete`, `disabled`,
and `not_checked`. `summary` and `select_region` retain their existing
fallback-note fields for this slice, and the M12.4 docs require those notes to
continue naming stale, unsupported, malformed, mismatched-reference, and
missing-index fallback causes.

## M12.5 Fixture Extension

M12.5 extends the planned fixture suite for the M12 compatibility surface
without checking in new binary assets yet:

* `tiny.invalid.large_reference.bam` reserves the BAI large-reference rejection
  case for `index --format bai`;
* `tiny.invalid.mismatched_reference_count.csi` reserves CSI reference-count
  mismatch diagnostics for `check_index` plus scan-fallback outputs for
  `check_map` and `summary`;
* `tiny.invalid.fastq_gz.bad_gzi` reserves malformed FASTQ.GZI
  planner-sidecar behavior for `enumerate`, `explode`, and `consume`.

These fixture reservations must preserve the M12.2 support-level vocabulary
and the M12.4 index diagnostic vocabulary when materialized.

## M12.6 Read-Only CSI Region Behavior

M12.6 applies the M12.2 detect-only CSI decision to read-only region evidence.
No CSI traversal is promoted in this slice. Instead, when a CSI header sidecar
is the available adjacent index:

* `check_map --region` reports scan fallback, preserves `index.kind: CSI`,
  sets `index.diagnostic_status: unsupported`, and keeps the fallback note
  explicit;
* `summary --region` reports scan fallback, keeps `index_derived.kind: CSI`,
  and names CSI unsupported fallback in `semantic_note`;
* BAI remains the only index kind used for read-only region traversal.

## M12.7 Selected-Region Compatibility

M12.7 applies the same M12 support decisions to selected BAM file output.
`select_region` now reports an `input_index` object for the selected adjacent
input sidecar:

* usable non-stale BAI reports `input_index.compatibility: used_bai` and may
  drive indexed traversal;
* adjacent CSI headers report `input_index.kind: CSI` and
  `input_index.compatibility: detect_only_csi`, then use native scan fallback
  with `execution.fallback_reason: detect_only_csi_index`;
* unsupported, missing, stale, malformed, incomplete, and disabled input-index
  states remain explicit fallback causes;
* output BAI/CSI invalidation remains under `output.index_invalidation`, and
  `select_region` still does not create replacement output indexes.

## M12.8 Public Contract Refresh

M12.8 consolidates the public contract artifacts for the M12 decisions already
implemented in M12.1-M12.7. The refreshed governed surface is:

* `check_index` schema and examples include `support_level` for selected and
  candidate sidecars, with BAI `read_write`, CSI `detect_only`, FASTQ.GZI
  `planning_sidecar`, unknown `unsupported`, and absent `absent`;
* `check_map` schema and examples include `index.diagnostic_status`, including
  region examples, so stale, unsupported, malformed, mismatched-reference,
  incomplete, disabled, absent, and usable states are machine-readable;
* `summary` schema and region examples govern `index_derived`, including
  region-scoped traversal context where whole-file BAI totals are omitted;
* `select_region` schema and examples govern `input_index`, including
  `used_bai`, `detect_only_csi`, unsupported, missing, stale, malformed,
  incomplete, and disabled compatibility states;
* fixture documentation maps the M12 reserved assets to those public contracts:
  large-reference BAI rejection, CSI reference-count mismatch, and malformed
  FASTQ.GZI planner-sidecar behavior.

## Ten-Task Outline

1. M12.1 activate scope and audit current BAI, CSI, and FASTQ.GZI behavior.
2. M12.2 freeze CSI support levels: detect-only, read-compatible, or write
   support.
3. M12.3 define large-reference behavior and rejection thresholds.
4. M12.4 strengthen stale-index, mismatched-reference, and unsupported-index
   diagnostics.
5. M12.5 extend index fixtures for BAI/CSI compatibility and failure modes.
6. M12.6 wire supported CSI behavior into read-only region evidence where
   explicitly contracted.
7. M12.7 update any selected-region output from M11 to respect index
   compatibility decisions.
8. M12.8 update schemas, examples, CLI docs, README, roadmap, and Sphinx docs.
9. M12.9 add index compatibility benchmark and dependency guardrails.
10. M12.10 close the milestone with full verification and explicit residual
    risk notes.

## Non-Goals

M12 does not make CRAM indexing native, does not promise biological
interpretation, and does not claim comparator parity beyond measured index
compatibility checks.
