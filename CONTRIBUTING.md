# Contributing To Bamana

## Core Rule

Performance-critical BAM, BGZF, FASTQ, sampling, ingest, and forensic paths
must be implemented in Bamana-native modules.

Do not introduce or expand `noodles` or similar general-purpose format crates
inside hot paths without an explicit architectural exception.

## Hot-Path Review Rule

Any pull request that introduces or expands external format-crate use in a hot
path must explain:

* why a Bamana-native implementation is not being used
* why the added dependency is compatible with Bamana's performance and control
  goals
* whether the change is transitional and, if so, how it will be removed

If that explanation is missing, the default review position should be to reject
the hot-path dependency expansion.

## Where External Crates Are Acceptable

External parser and format crates are acceptable in:

* tests
* fixture tooling
* differential or compatibility oracles
* non-hot development scaffolding
* explicitly marked transitional compatibility slices

They are not acceptable as the primary execution engine for:

* BAM/BGZF readers and writers
* FASTQ/FASTQ.GZ readers and writers
* record scanning and selective field decoding
* hot-loop transforms such as `subsample`, `sort`, `merge`, `deduplicate`, and
  provenance or duplication scans

## Transitional Code Rule

If a production-path compatibility layer still exists:

* label it explicitly as transitional
* keep it centralized rather than scattered
* add comments that say the usage must not expand
* document the migration path in
  [docs/migration/noodles-demotion.md](/Users/stephen/Projects/bamana/docs/migration/noodles-demotion.md)

## Code Organization Direction

Prefer contributing to these Bamana-native core modules:

* `src/bgzf/`
* `src/bam/`
* `src/fastq/`
* `src/sampling/`
* `src/forensics/`
* `src/ingest/`

Compatibility shims under older paths may remain temporarily, but new hot-path
logic should target the native core modules directly.

## Current Milestone

The active migration milestone is tracked in:

* [docs/roadmap/current_milestone.md](/Users/stephen/Projects/bamana/docs/roadmap/current_milestone.md)

Before starting a large change, check that file and the canonical roadmap:

* [ROADMAP.md](/Users/stephen/Projects/bamana/ROADMAP.md)
* [docs/roadmap.md](/Users/stephen/Projects/bamana/docs/roadmap.md)

If the work touches benchmark infrastructure, also check:

* [benchmarks/benchmark_readiness_checklist.md](/Users/stephen/Projects/bamana/benchmarks/benchmark_readiness_checklist.md)
* [benchmarks/status_for_tomorrow.md](/Users/stephen/Projects/bamana/benchmarks/status_for_tomorrow.md)

## What “Done” Means For A Milestone

A milestone is not done just because code landed. At minimum, it should have:

* the owned modules implemented or materially strengthened
* command integration identified or completed
* tests added or updated
* benchmark hooks defined and, where practical, run
* docs updated
* `noodles` usage reduced, isolated, or checkpointed explicitly

## Benchmark-Aware Development Rule

Native-core work should come with measurement hooks.

Examples:

* BGZF throughput for BGZF work
* header parse latency for header-codec work
* records/sec scan throughput for BAM scanner work
* FASTQ parse throughput for FASTQ work
* before/after command benchmarks for migrated commands

If a change moves code but does not improve ownership or measurability, it is
not sufficient progress for this roadmap.
