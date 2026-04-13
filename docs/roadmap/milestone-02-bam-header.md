# Milestone 2: Native BAM Header Codec

## Technical Goal

Implement Bamana-native BAM header parsing and serialization for:

* BAM magic
* `l_text`
* textual SAM-style header content
* binary reference dictionary
* deterministic reserialization

## Owned Modules

Primary ownership:

* `src/bam/header.rs`
* `src/bam/reader.rs`
* header serialization portion of `src/bam/write.rs`

Future extension targets:

* header mutation helpers for `reheader`
* compatibility-aware header merge helpers for `merge`

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF substrate

## Commands Enabled Or Migrated

Primary migration targets:

* `verify`
* `header`

Secondary beneficiaries:

* `reheader`
* `check_sort`
* `check_map`
* `summary`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility
* tests and oracles for header differential checks

Disallowed:

* production `header` or `verify` behavior backed by `noodles`

## Acceptance Criteria

* BAM header text and references parse natively
* malformed and negative lengths are detected safely
* BAM header serialization is deterministic
* textual and binary reference information are merged consistently
* production `header` command does not rely on `noodles`
* production `verify` uses native BGZF plus native header path only

## Benchmark Hooks

* header parse latency microbenchmark
* header serialization microbenchmark
* startup / read-prefix cost comparison before and after migration
* command-level `verify` and `header` reruns in the benchmark framework where
  appropriate

## Risks / Follow-Up

* reserialization rules must remain stable for checksum and reheader work
* header mutation ergonomics should not introduce expensive generic structures
