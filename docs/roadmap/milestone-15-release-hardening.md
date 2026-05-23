# Milestone 15: Release Hardening And Public Contract Freeze

Status: planned. Milestone 15 is the release-boundary hardening milestone for
the public command surface accepted up to that point.

## Goal

Prepare Bamana for a coherent release boundary: stable CLI contracts, complete
documentation, reproducible verification, packaging checks, and explicit
operational support guarantees.

## Ten-Task Outline

1. M15.1 activate release scope and list public commands accepted into the
   release boundary.
2. M15.2 freeze CLI option semantics and JSON output compatibility rules.
3. M15.3 audit every schema, example, fixture, benchmark note, and Sphinx page.
4. M15.4 define deprecation, compatibility, and versioning policy.
5. M15.5 harden installation, packaging, binary naming, and release artifact
   checks.
6. M15.6 add CI verification for formatting, tests, contracts, Sphinx, schemas,
   examples, and smoke benchmarks.
7. M15.7 audit operational failure modes and error-message quality.
8. M15.8 update README, user docs, technical docs, roadmap, and release notes.
9. M15.9 add final dependency-boundary and license/supply-chain checks.
10. M15.10 close the release hardening milestone with archived verification
    evidence and explicit known limitations.

## Non-Goals

M15 does not require completing every aspirational Bamana feature. It freezes a
coherent release boundary around the behavior actually implemented, documented,
tested, and benchmarked.
