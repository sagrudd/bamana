# Bamana Performance Core

## Principle

Bamana's performance ceiling depends on owning its hot loops.

For BAM, BGZF, and FASTQ operations, that means Bamana-native code must own:

* scanning
* selective decoding
* serialization
* transformation
* output writing

## Why Generic Record Abstractions Are Often Too Expensive

General-purpose format crates optimize for breadth, safety, and reuse across
many consumers. Bamana optimizes for:

* narrow operational scope
* high-throughput scans
* deterministic structured outputs
* deliberate transform semantics

Those goals often require lower-level control over:

* allocation
* copying
* branch behavior
* field decoding breadth
* output serialization shape

## Hot-Loop Design Tendencies

Hot-path Bamana code should prefer:

* bounded parsing with explicit safety limits
* selective field extraction instead of full semantic decoding
* stable record-layout structures over rich generic object graphs
* deterministic write paths
* reusable buffers where that materially helps
* explicit ownership of BGZF block and EOF behavior

## Partial-Decode Examples

The following command families benefit from selective decoding rather than full
record materialization:

* `verify`
* `check_eof`
* `check_sort`
* `check_map`
* `check_tag`
* `summary`
* `inspect_duplication`
* `forensic_inspect`
* `checksum`

These commands often need only:

* header state
* block structure
* selected fixed BAM fields
* read names
* specific aux tags
* lightweight record identity fingerprints

They do not always need a fully interpreted alignment object.

## Transform-Oriented Examples

Commands such as:

* `subsample`
* `sort`
* `merge`
* `deduplicate`
* `reheader`
* `annotate_rg`
* `consume`

need Bamana-owned control over read/write loops, transform staging, and output
serialization. That control is part of performance, not a later optimization
detail.

## CRAM Nuance

This native-core rule is primarily about:

* BAM
* BGZF
* FASTQ

CRAM may continue to require a staged compatibility approach while the project
keeps its BAM/FASTQ performance core narrow and controlled.

## Milestone Measurement Hooks

The performance-core migration is tied to measurement, not just code movement.

Expected benchmark hooks by milestone:

* native BGZF
  * BGZF read throughput
  * BGZF write throughput
  * EOF-check latency
* native BAM header codec
  * header parse latency
  * header serialization cost
* native BAM record scanner
  * records/sec scan throughput
  * selective-field extraction throughput
* native FASTQ
  * FASTQ parse throughput
  * FASTQ.GZ parse throughput
* command migration
  * before/after `verify`
  * before/after `header`
  * before/after `subsample`

These hooks should be treated as milestone evidence and reviewed alongside the
code that introduces a new native substrate.
