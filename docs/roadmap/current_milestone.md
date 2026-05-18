# Current Milestone

## Milestone Status

**Milestone 5: Command Migration Off `noodles`** is active as of 2026-05-18.

See:

* [milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Why This Is Current

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. Milestone 3
completed selective native BAM record scanning and migrated the first
scanner-compatible BAM command consumers. Milestone 4 completed the native
FASTQ and FASTQ.GZ parser/writer core.

Milestone 5 uses those substrates to prove command migration boundaries on the
first command set: `verify`, `header`, and `subsample`. It is intentionally
about production command paths and dependency boundaries, not broad native CRAM
support or wholesale replacement of every transform command.

## Previously Completed Milestones

**Milestone 1: Native BGZF Core**

See:

* [milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)

The M1 closeout established:

* native BGZF block reading and EOF-marker handling;
* BAM-compatible native BGZF writing;
* virtual-offset groundwork for later random-access work;
* BGZF microbenchmark hooks;
* dependency guardrails that keep production `noodles` usage isolated to CRAM
  compatibility.

**Milestone 2: Native BAM Header Codec**

See:

* [milestone-02-bam-header.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-02-bam-header.md)

The M2 closeout established:

* native BAM magic, `l_text`, SAM-style textual header, and binary reference
  dictionary parsing;
* deterministic BAM header serialization helpers;
* production `verify` and `header` paths backed by native BGZF plus native BAM
  header parsing;
* malformed-header tests and test-only header oracle boundaries;
* header microbenchmark hooks with machine-readable output;
* dependency-boundary checks protecting production native header code from
  direct `noodles` imports.

**Milestone 3: Native BAM Record Scanner**

See:

* [milestone-03-bam-record-scan.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-03-bam-record-scan.md)

The M3 closeout established:

* `BamRecordView` as the borrowed native BAM record-view contract;
* `BamScanner` as the native BGZF/header-backed BAM record iteration substrate;
* scanner-owned helpers for flags, coordinates, MAPQ, read names, sequence
  length, section ranges, skip offsets, borrowed section slices, and selected
  aux traversal;
* scanner-backed `check_sort`, `check_map`, `summary`, `check_tag`,
  `validate`, `inspect_duplication`, and `forensic_inspect` paths;
* scanner malformed-record tests, dependency-boundary protection, and
  `scanner_microbench` hooks.

Milestone 1 through 3 completion evidence remains recorded in:

* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

**Milestone 4: Native FASTQ / FASTQ.GZ Parser**

See:

* [milestone-04-fastq.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-04-fastq.md)

The M4 closeout established:

* explicit native FASTQ modules for record contracts, reader validation,
  writer finalization, gzip handling, unmapped conversion, and `FASTQ.GZI`
  sidecar work;
* native plain FASTQ and FASTQ.GZ parsing, validation, structured malformed
  input errors, and writer round trips;
* selected command-consumer evidence for `enumerate`, FASTQ-side `subsample`,
  `consume`, `inspect_duplication`, `deduplicate`, and FASTQ.GZ `explode`;
* dependency-boundary checks keeping production FASTQ hot paths free of direct
  external generic bioinformatics parser crates;
* `fastq_microbench` parser/writer benchmark hooks with machine-readable JSON
  output.

M4 closeout verification passed with `cargo test`, `cargo test --test
contract`, Sphinx, and `fastq_microbench --profile small --iterations 1` plus a
JSON smoke check.

Milestone 1 through 4 completion evidence remains recorded in:

* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Milestone 5 Current State

Known present pieces:

* native BGZF, BAM header, BAM scanner, and FASTQ substrates are available;
* `verify` and `header` already use native BGZF plus native BAM header parsing;
* FASTQ-side `subsample` already uses the native FASTQ parser/writer core;
* public command contracts exist for `benchmark`, `fastq`, and `unmap`;
* dependency-boundary tests already restrict direct production `noodles` usage
  to the CRAM compatibility boundary.

Known gaps:

* BAM-side `subsample` still needs explicit native scanner or raw-record bridge
  evidence in the M5 task map;
* proof-command migration evidence needs to be collected and recorded for
  `verify`, `header`, and `subsample`;
* benchmark or differential evidence for the proof commands remains to be
  recorded.

First consumer order:

1. refresh M5 baseline and dependency audit;
2. confirm `verify` and `header` native proof-command evidence;
3. migrate or prove `subsample` across BAM and FASTQ paths;
4. add fixture, differential, and benchmark closeout evidence.

## Completion Boundary

Milestone 5 completion will mean:

* `verify`, `header`, and `subsample` have explicit native-substrate evidence;
* direct production `noodles` usage remains isolated to CRAM compatibility;
* public JSON contracts remain stable unless deliberately versioned;
* command-level tests, dependency-boundary tests, and benchmark or smoke
  evidence are recorded.

## Command-Surface Boundary

Milestone 5 evidence is limited to the proof commands named above. Broader
transform, indexing, ingest, region-query, and native CRAM migration remain
later milestones unless an explicit M5 task includes them.

## What Should Not Happen

Do not treat Milestone 5 as native CRAM implementation, broad command parity,
or a mandate to rewrite unrelated transform paths. Keep the focus on the proof
commands and on preserving public contracts.
