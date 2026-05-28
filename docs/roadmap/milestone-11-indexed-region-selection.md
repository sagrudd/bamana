# Milestone 11: Public Indexed Region Selection And Region Files

Status: active as of 2026-05-28. Milestone 11 follows the completed Milestone
10 read-only indexed-region evidence surface. M11.1 activates this milestone
without changing CLI behavior.

## Goal

Promote selected-record indexed region output only after the public contract is
precise. M10 proved region parsing, BAI chunk planning, random-access
traversal, scan fallback, and read-only `check_map --region <REGION>` and
`summary --region <REGION>` evidence. M11 adds the contract runway for selected
BAM record output and region-file input as governed public behavior.

## M11.1 Activation Decision

M11.1 freezes the selection surface decision: M11 will specify a new planned
public command named `select_region`. It must not overload `check_map`,
`summary`, or `subsample`.

The command is not implemented by M11.1. No public `select_region` CLI synopsis
exists until M11.3 through M11.8 define output semantics, header preservation,
record ordering, duplicate-region behavior, region-file behavior, index
invalidation or regeneration notes, and write-safety. Until those tasks complete,
`check_map --region <REGION>` and `summary --region <REGION>` remain the only
public region-aware behavior, and they remain read-only evidence surfaces.

Baseline substrate inherited from M10:

* `src/bam/region.rs` owns bounded region string parsing and normalization.
* `src/bam/region_plan.rs` owns validated BAI chunk planning.
* `src/bam/region_traversal.rs` owns random-access traversal, interval
  filtering, and duplicate virtual-offset suppression.
* `src/commands/check_map.rs` and `src/commands/summary.rs` prove the read-only
  region evidence payload contract.
* `src/bam/write.rs`, `src/output_safety.rs`, and existing writer commands
  provide patterns for later selected-record output and collision safety.

The M11 public contract must explicitly keep native CRAM indexed queries, CSI
large-reference support, biological interpretation, pileup/genotyping behavior,
and broad external comparator parity out of scope unless a later task promotes
one of them.

## Ten-Task Outline

1. M11.1 activate scope and freeze the selection command decision.
2. M11.2 specify region-file syntax, validation, ordering, and rejection
   behavior.
3. M11.3 specify selected-record output semantics for stdout, files, and dry
   runs.
4. M11.4 specify header preservation, `@HD` sort-order semantics, and reference
   dictionary behavior.
5. M11.5 specify overlapping-region and duplicate-region record policy.
6. M11.6 implement native selected-record writing from indexed traversal.
7. M11.7 implement write-safety, collision handling, index invalidation, and
   optional regeneration notes.
8. M11.8 update CLI contracts, JSON schemas, examples, fixtures, README, CLI
   docs, and Sphinx docs.
9. M11.9 add dependency-boundary and benchmark guardrails for selection output.
10. M11.10 close the milestone with full tests, contract tests, Sphinx, and
    benchmark smoke evidence.

## Non-Goals

M11 does not imply native CRAM indexed queries, CSI large-reference support,
biological interpretation, pileup/genotyping behavior, or broad external
comparator parity.
