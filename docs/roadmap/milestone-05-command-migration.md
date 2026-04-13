# Milestone 5: Command Migration Off `noodles`

## Technical Goal

Use the substrate milestones to migrate the first proof commands off external
hot-path dependencies in this order:

1. `verify`
2. `header`
3. `subsample`

## Owned Modules

Primary ownership:

* `src/bgzf/`
* `src/bam/header.rs`
* `src/bam/reader.rs`
* `src/bam/write.rs`
* `src/fastq/`
* `src/sampling/`
* command orchestration in `src/commands/verify.rs`,
  `src/commands/header.rs`, and `src/commands/subsample.rs`

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF
* Milestone 2 native BAM header codec
* Milestone 3 native BAM record scanner
* Milestone 4 native FASTQ parser for full `subsample` coverage

## Why This Order

* `verify` is shallow and proves BGZF plus header ownership quickly
* `header` is the natural follow-on proof of header codec ownership
* `subsample` is the first strong end-to-end scan and transform proof across
  BAM and FASTQ

## Acceptance Criteria

* `verify` uses native BGZF and native header path only
* `header` uses native header codec only
* `subsample` uses native BAM scanning and native FASTQ parsing
* command JSON contracts remain stable
* differential tests and fixtures continue to pass
* `noodles` no longer appears in production code paths for these commands

## Benchmark Hooks

* benchmark `verify` before and after migration
* benchmark `header` before and after migration
* benchmark `subsample` before and after migration
* capture command-level deltas in the benchmark framework where possible

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility only
* tests, oracles, fixtures

## Risks / Follow-Up

* command migrations must not regress JSON contract stability
* benchmark regressions should be treated as real signals, not postponed
