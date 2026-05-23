# Milestone 14: Interoperability And Benchmark Evidence

Status: planned. Milestone 14 turns benchmark and interoperability claims into
governed evidence.

## Goal

Make Bamana's comparator, benchmark, and fixture evidence strong enough to
support release-facing claims without implying broad parity where it has not
been measured. This milestone should expand the `benchmark` command and
repository-local smoke hooks in lockstep with public command contracts.

## Ten-Task Outline

1. M14.1 activate scope and audit all benchmark profiles and smoke hooks.
2. M14.2 define which public commands have comparator evidence, smoke evidence,
   or no external comparator claim.
3. M14.3 extend benchmark result schemas for post-M10 command families.
4. M14.4 add reproducible fixture generation and fixture provenance metadata.
5. M14.5 add measured comparator profiles only where semantics are aligned.
6. M14.6 document unsupported comparator cases and semantic mismatch reasons.
7. M14.7 add CI or local harness checks for benchmark schema stability.
8. M14.8 update benchmark docs, README, CLI docs, roadmap, and Sphinx docs.
9. M14.9 add dependency-boundary tests for benchmark-only tools and oracles.
10. M14.10 close the milestone with archived smoke evidence and residual risk
    notes.

## Non-Goals

M14 does not convert smoke timings into performance promises, does not claim
biological equivalence, and does not treat external tool output as authoritative
unless a command-specific oracle contract says so.
