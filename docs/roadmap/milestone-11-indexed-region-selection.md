# Milestone 11: Public Indexed Region Selection And Region Files

Status: planned. Milestone 11 follows the completed Milestone 10 read-only
indexed-region evidence surface.

## Goal

Promote selected-record indexed region output only after the public contract is
precise. M10 proved region parsing, BAI chunk planning, random-access
traversal, scan fallback, and read-only `check_map --region <REGION>` and
`summary --region <REGION>` evidence. M11 decides whether to add selected BAM
record output and region-file input as governed public behavior.

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
