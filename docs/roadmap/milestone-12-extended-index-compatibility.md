# Milestone 12: Extended Index Compatibility

Status: planned. Milestone 12 follows M11 or any explicit decision to defer
selected-record output again.

## Goal

Harden index compatibility beyond the M9/M10 BAI-first contract. The milestone
focuses on CSI, large-reference behavior, stale-index policy, cross-index
selection, and compatibility evidence for every command that relies on index
state.

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
