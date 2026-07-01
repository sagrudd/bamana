# MILESTONES.md

This roadmap tracks Bamana's release-candidate obligations as the maintained
Mnemosyne I/O and benchmark support resource for Phanerognostikon, Platage, and
related head-to-head evaluations.

## Milestone 0: Governance And Public Contract Baseline

Goal: keep repository instructions, roadmap material, public contracts, and
release discipline explicit.

Acceptance criteria:

- `AGENTS.md`, `MILESTONES.md`, `TODO.md`, and `ROADMAP.md` describe current
  scope without conflicting with each other.
- Public command contracts remain documented in README, Sphinx sources, CLI
  specs, JSON schemas, examples, and benchmark documentation as appropriate.
- Semantic versioning is preserved, and public contract changes include tests
  and documentation in the same prompt.

Status: active. Bamana already has strong AGENTS and roadmap material; this
milestone adds the missing root milestone and TODO layer.

## Milestone 1: Native Performance Core

Goal: keep performance-critical BAM, BGZF, FASTQ, sampling, ingest, and forensic
hot paths Bamana-native.

Acceptance criteria:

- Production `noodles` use remains limited to documented CRAM compatibility,
  tests, fixtures, compatibility checks, or oracle validation.
- Native BGZF/BAM/FASTQ paths have focused regression and benchmark coverage.
- Public command JSON contracts remain stable unless a versioned breaking
  change is explicitly approved.
- File-size and module boundaries remain maintainable as new hot paths are
  added.

Status: substantially complete with continuing command-specific expansion.

## Milestone 2: Platage Biological I/O Support

Goal: provide the reusable I/O capabilities Platage requires without copying
Bamana logic into Platage.

Acceptance criteria:

- Streaming FASTQ and FASTQ.GZ APIs remain stable and documented for downstream
  use.
- Bamana-owned streaming FASTA record API is implemented, tested, and
  documented.
- Gzip/BGZF detection and error reporting are suitable for Platage adapters and
  benchmark harnesses.
- Any new reusable I/O API preserves existing Bamana contracts and semantic
  versioning.

Status: active. Platage has an open dependency on Bamana-owned streaming FASTA
records before adding its `io::bamana_fasta` adapter.

## Milestone 3: Containerised Benchmark Framework

Goal: keep Bamana's benchmarking layer suitable for GB10 and formal comparison
work.

Acceptance criteria:

- Benchmark workflows run from clean checkouts using documented containers.
- Reports capture command, comparator, platform, architecture, input checksum,
  output checksum, wall time, memory, and status.
- GB10 Linux ARM64 execution is supported and documented.
- Comparator scope is explicit and does not overclaim parity where no comparator
  run occurred.

Status: active. The benchmark framework exists and should be extended as GB10
evidence is gathered.

## Milestone 4: Documentation And Schema Completeness

Goal: keep public behaviour discoverable and machine-checkable.

Acceptance criteria:

- README, Sphinx docs, CLI specs, JSON schemas, and example outputs match the
  implemented command surface.
- Public contract commands `benchmark`, `fastq`, and `unmap` keep schema,
  example, documentation, and regression coverage current.
- Cross-repository integration expectations for Platage are documented in both
  Bamana and Platage.

Status: active.

## Milestone 5: Trinity Support Readiness

Goal: make Bamana a stable support dependency for the Platage, rustedBloom, and
newONform formal head-to-head.

Acceptance criteria:

- Required Bamana I/O APIs are version-pinned or otherwise reproducibly
  referenced by downstream projects.
- Downstream projects can consume Bamana without private copy-paste forks.
- Benchmark and fixture data conventions are compatible with the trinity
  comparison set.
- Deficits needed by Platage or Phanerognostikon are represented in `TODO.md`.

Status: planned.

## Milestone 6: Release Candidate

Goal: declare Bamana ready as a governed dependency for downstream release
candidates.

Acceptance criteria:

- Relevant Rust checks pass.
- Sphinx and contract-documentation checks pass where configured.
- GB10 benchmark evidence is available for the required profiles.
- `TODO.md` has no untriaged release blockers.
- The current branch is committed and pushed.

Status: not ready. Current release blockers include GB10 benchmark evidence and
existing `cargo clippy --all-targets -- -D warnings` debt across several
production and test modules.
