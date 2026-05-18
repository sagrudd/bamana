# Milestone 4: Native FASTQ / FASTQ.GZ Parser

## Technical Goal

Implement and stabilize a Bamana-native FASTQ and FASTQ.GZ parser / writer
core.

## Owned Modules

Primary ownership:

* `src/fastq/mod.rs`
* `src/fastq/record.rs`
* `src/fastq/reader.rs`
* `src/fastq/writer.rs`
* `src/fastq/gzip.rs`
* `src/fastq/unmapped.rs`
* `src/fastq/gzi.rs`

Compatibility shim:

* `src/ingest/fastq.rs`

## Dependencies / Prerequisites

Independent of BAM record scanning in principle, but sequenced after BAM-first
substrate work because:

* BAM-first command migration is higher priority
* FASTQ work still benefits from the same native-core patterns

## Commands Enabled Or Migrated

Primary beneficiaries:

* FASTQ-side `subsample`
* `consume`
* `inspect_duplication`
* `deduplicate`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility
* tests and oracles

Disallowed:

* external generic bioinformatics format crates as the FASTQ hot path

## Acceptance Criteria

* plain FASTQ parsing works robustly
* FASTQ.GZ parsing works robustly
* 4-line structure is validated
* sequence and quality length equality is validated
* valid FASTQ and FASTQ.GZ writing is supported
* no FASTQ hot path depends on external generic bioinformatics format crates

## Benchmark Hooks

* FASTQ parse throughput microbenchmark
* FASTQ.GZ parse throughput microbenchmark
* rerun FASTQ-side `subsample` timing
* compare ingest throughput where meaningful against earlier implementation and
  external comparators such as `fastcat` or `seqtk`

## Risks / Follow-Up

* gzip strategy and buffering need to stay honest in benchmark interpretation
* future paired-read semantics should not bloat the base parser abstraction

## Closeout Evidence

Milestone 4 closed on 2026-05-18. The closeout established:

* explicit native FASTQ modules for record contracts, reader validation, writer
  finalization, gzip handling, unmapped-BAM conversion, and `FASTQ.GZI`
  sidecar work;
* native plain FASTQ validation for record structure, header and plus markers,
  sequence/quality length equality, and usable read names;
* native FASTQ.GZ parsing through extension-selected `MultiGzDecoder`,
  including single-member and concatenated multi-member streams;
* structured failures for malformed FASTQ, malformed FASTQ inside valid gzip,
  corrupt gzip, and truncated gzip inputs;
* plain and gzip FASTQ writer round trips with LF line endings, header/plus
  preservation, and gzip finalization before success;
* selected command-consumer evidence for `enumerate`, FASTQ-side `subsample`,
  `consume`, `inspect_duplication`, `deduplicate`, and FASTQ.GZ `explode`;
* dependency-boundary tests keeping production FASTQ hot paths free of direct
  external generic bioinformatics parser crates;
* `fastq_microbench` parser/writer benchmark hooks with machine-readable JSON
  output and `benchmarks/results/fastq_microbench.schema.json`.

Closeout verification passed with:

* `cargo test`;
* `cargo test --test contract`;
* `cargo run --bin fastq_microbench -- --profile small --iterations 1` with a
  JSON smoke check;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`.

Remaining work is intentionally downstream: field-only streaming integration,
paired-read semantics, adapter trimming, richer platform-specific FASTQ
metadata interpretation, broad comparator parity, and command-specific behavior
outside the selected Milestone 4 consumers.
