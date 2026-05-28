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
