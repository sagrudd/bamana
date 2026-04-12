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
