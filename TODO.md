# TODO.md

Tasks here focus on Bamana's role as a maintained Mnemosyne I/O and benchmark
support dependency. Keep detailed command-specific work in the existing roadmap
and benchmark documents where that is more precise.

## 1. Governance And Roadmap Alignment

- [x] Add root `MILESTONES.md` and `TODO.md`.
- [x] Update `AGENTS.md` to name root milestone/TODO alignment and Platage
      cross-repository obligations.
- [x] Record current `cargo clippy --all-targets -- -D warnings` debt as a
      release blocker rather than silently treating Bamana as lint-clean.
- [ ] Cross-check `ROADMAP.md`, `docs/roadmap.md`, and `MILESTONES.md` for
      duplicated or stale milestone claims.
- [ ] Add a release checklist that names Rust checks, Sphinx checks, schema
      checks, benchmark checks, version validation, commit, and push.
- [ ] Add a blocker register for release-candidate readiness.

## 2. Platage I/O Dependency

- [x] Add strict indexed-region selection to the governed `records` response
      for AlleleAnchor held-out metric sharding; require a current BAI and
      prohibit whole-file scan fallback.
- [x] Add an atomic NDJSON output mode to `records` that traverses indexed BAM
      chunks incrementally, retains only virtual-offset deduplication state,
      records complete reference/filter/provenance metadata, and allows
      AlleleAnchor to derive bounded-memory evidence without collecting a
      chromosome or JSON envelope.

- [ ] Implement a Bamana-owned streaming FASTA record API suitable for Platage's
      `io::bamana_fasta` adapter.
- [ ] Add FASTA API tests for multiline records, descriptions, empty records,
      invalid symbols, gzipped inputs where supported, and clear parse errors.
- [ ] Document the FASTA/FASTQ/BGZF/gzip API boundary that Platage may depend
      on.
- [ ] Coordinate the Platage dependency update after the Bamana FASTA API is
      available.
- [ ] Preserve existing public contracts and semantic versioning for all new I/O
      APIs.

## 3. Benchmark And GB10 Readiness

- [ ] Verify the benchmark framework from a clean containerised checkout.
- [ ] Add or confirm GB10 Linux ARM64 benchmark instructions.
- [ ] Ensure benchmark rows capture command, comparator, architecture, thread
      count, input checksum, output checksum, wall time, memory, and status.
- [ ] Mark which benchmark profiles support formal downstream comparison and
      which remain smoke or development evidence.
- [ ] Add artifact-retention guidance so raw sequencing data and bulky generated
      output are not committed accidentally.

## 4. Public Contract Commands

- [ ] Keep `benchmark`, `fastq`, and `unmap` covered by schema, example,
      documentation, and regression tests.
- [ ] Add contract snapshots for any CLI help or JSON shape changes.
- [ ] Confirm command documentation states non-goals and non-equivalence claims
      as explicitly as implemented output does.
- [ ] Add versioned deprecation notes before changing any public JSON field.

## 5. Native Performance Core

- [x] Add deterministic bounded external BAM sorting for downstream
      whole-genome transforms, with a caller-selected record-memory budget,
      parallel stable run ordering, atomic publication, temporary-run cleanup,
      contracts, documentation, and spill/merge regression coverage.
- [ ] Benchmark bounded coordinate and queryname sorting on the GB10 with real
      whole-genome input; record peak RSS, worker utilization, run count,
      storage throughput, and checksum identity before claiming downstream
      performance acceptance.
- [ ] Use the GB10 evidence to decide whether ordered parallel BGZF compression,
      pipelined run spill/merge, or both are required; preserve deterministic
      bytes and avoid increasing the configured record-memory budget.
  - [x] Add bounded ordered parallel BGZF compression to in-memory, spill-run,
        and final external-sort writers; preserve byte-identical output across
        worker counts and expose worker-count evidence in `bgzf_microbench`.
  - [ ] Compare the bounded asynchronous BGZF pipeline and remaining
        single-threaded heap merge on retained real GB10 data before promoting
        the pipeline downstream. Require wall-time improvement, deterministic
        bytes, and bounded in-flight block evidence; do not infer success from
        CPU utilization alone.
- [ ] Resolve existing clippy `-D warnings` failures, including needless borrows,
      single-match control flow, type-complexity warnings, MSRV-incompatible
      `is_multiple_of`, large error/enum variants, pointer-argument warnings,
      and repeated forensic/FASTQ style findings.
- [ ] Keep production direct `noodles` imports constrained to approved
      compatibility paths.
- [ ] Add checks that prevent new production `noodles` use in hot paths without
      a documented exception.
- [ ] Continue splitting large modules before they approach 1000 lines.
- [ ] Extend native BGZF/BAM/FASTQ regression fixtures as new command paths are
      promoted.

## 6. Trinity Support

- [ ] Confirm the Bamana commit or version used by Platage is reproducible and
      documented.
- [ ] Align Bamana benchmark manifests or outputs with the Platage,
      rustedBloom, and newONform head-to-head reporting conventions where
      useful.
- [ ] Track every downstream deficit as either a Bamana TODO item, a downstream
      TODO item, or an accepted waiver.
- [ ] Re-run the downstream Platage checks after Bamana API changes are made.

## 7. Release Candidate

- [ ] Run the relevant Rust formatting, lint, and test checks.
- [ ] Run documentation/schema checks required by the release checklist.
- [ ] Run GB10 benchmark profiles required for downstream dependency acceptance.
- [ ] Confirm no raw data, bulky generated outputs, or unrelated dirty files are
      staged.
- [ ] Commit and push the release-readiness updates.
