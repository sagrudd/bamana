# AGENTS.md

## Scope

These instructions apply to the entire repository.

## Required Working Practice

Code changes must be committed and pushed after each prompt that modifies code.
Do not leave implementation changes only in the local working tree unless the
user explicitly asks for local-only work or a blocker prevents commit or push.

When a prompt modifies code, the agent must:

1. inspect the existing working tree before editing;
2. preserve unrelated user changes;
3. update tests, contracts, and documentation that are affected by the change;
4. run the most relevant verification commands;
5. commit the completed change with a focused message;
6. push the branch; and
7. report the commit, push target, verification performed, and any residual
   risk.

## Documentation Discipline

Technical Sphinx documentation and user-facing documentation must be maintained
religiously. Any change to public behavior, command semantics, JSON output,
schemas, examples, benchmark behavior, or operational guarantees must update
the corresponding documentation in the same prompt.

Documentation updates must include the relevant subset of:

* Sphinx sources under `docs/sphinx/`;
* user-facing documentation such as `README.md` and `docs/cli.md`;
* CLI contracts under `spec/cli/`;
* JSON schemas under `spec/jsonschema/`;
* example outputs under `spec/examples/`;
* benchmark documentation under `benchmarks/` when benchmark behavior changes;
* roadmap and task-map files when milestone scope changes.

## Public Contract Commands

`benchmark`, `fastq`, and `unmap` are public contract commands. They must be
treated with the same contract, schema, example, documentation, and regression
test expectations as the rest of the governed CLI surface.

## Architecture Constraints

Bamana's performance-critical BAM, BGZF, FASTQ, sampling, ingest, and forensic
hot paths must remain Bamana-native unless a documented transitional exception
exists. `noodles` usage must remain isolated to CRAM compatibility, tests,
fixtures, compatibility checks, or oracle-style validation.
