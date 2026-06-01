# Bamana Dependency Policy

## Core Policy

`noodles-bam` and other general-purpose bioinformatics crates must not be used
as the **primary execution engine** for performance-critical Bamana commands.

Hot-path code must prefer Bamana-native implementations.

## What Counts As Hot Path

Hot path includes code involved in commands such as:

* `verify`
* `check_eof`
* `header`
* `reheader`
* `annotate_rg`
* `subsample`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`
* `sort`
* `merge`
* `explode`
* `summary`
* `check_sort`
* `check_map`
* `check_tag`
* `checksum`
* `consume`

## Allowed Uses Of `noodles`

`noodles` may still be used in:

* tests
* fixture tooling
* compatibility checks
* differential or oracle-style validation
* temporary migration layers that are explicitly marked for replacement

## Disallowed Uses Of `noodles`

`noodles` must not be used as:

* the production BAM reader for hot-path commands
* the production BAM writer for hot-path commands
* the primary record iteration engine for performance-critical scans
* the primary BGZF I/O layer for BAM hot loops
* the transform engine for `sort`, `merge`, `explode`, `subsample`,
  duplication inspection, deduplication, or provenance forensics

## Current Transitional Exception

The current explicit exception is conservative CRAM ingestion support in
`src/ingest/cram.rs`.

That slice remains transitional and compatibility-oriented. It does not define
the architecture for BAM, BGZF, or FASTQ core execution.

The exception is narrow:

* direct production `noodles_*` imports are allowed only in
  `src/ingest/cram.rs`;
* the usage must stay strictly within CRAM ingestion and reference-policy
  compatibility;
* it must not expand into BAM, BGZF, FASTQ, or command hot paths;
* the boundary is enforced by
  `tests/contract/dependency_boundary.rs`.

### Removal criteria

The CRAM exception should be removed or redesigned when one of these is true:

* Bamana has a native CRAM compatibility plan and implementation;
* CRAM support is moved behind a narrower optional compatibility feature;
* the project decides to drop CRAM ingestion from the native-core package.

Until then, CRAM compatibility remains explicitly transitional and must not be
used as precedent for new production `noodles` usage.

M13.2 chooses compatibility-only continuation for Milestone 13. The decision
keeps CRAM ingestion in the native-core package through the explicit
transitional `cram-compat` feature but does not promote a native CRAM substrate
or broaden the `noodles` exception beyond `src/ingest/cram.rs`.

## Dependency Review Rule

Any new dependency added to hot-path code must justify:

* why Bamana-native implementation is insufficient
* why the dependency improves or at least does not compromise performance
* why the dependency preserves architectural control
* whether the dependency is temporary or intended to remain

If that case is weak, the dependency should not enter the hot path.
