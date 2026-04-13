# Bamana Native-Core Architecture

## Architectural Rule

Performance-critical BAM and FASTQ operations in Bamana must use
**Bamana-native parsing, I/O, scanning, serialization, and transformation
primitives**.

This is a project rule, not a stretch goal.

## Why This Reset Exists

Bamana is intended to become faster than standard toolkit competitors for the
operations it chooses to own. That requires direct ownership of:

* BGZF I/O
* BAM header parsing and serialization
* BAM record scanning and selective field decoding
* BAM record writing
* FASTQ and FASTQ.GZ parsing and writing
* record hashing and fingerprinting
* transformation loops for sampling, sorting, merging, and duplication or
  provenance inspection

Generic parser crates can help with bootstrapping and conformance, but they
should not define the ceiling of Bamana's hot-path performance.

## Core Module Ownership

The codebase is being re-anchored around these native modules:

* `src/bgzf/`: BGZF structure, EOF logic, block parsing, and future reader /
  writer ownership
* `src/bam/`: BAM header, record layout, scan loops, validation, sort, merge,
  checksum, and related transforms
* `src/fastq/`: FASTQ and FASTQ.GZ record parsing and writing
* `src/sampling/`: deterministic and seeded-random sampling logic
* `src/forensics/`: duplication and provenance-oriented scanning and
  remediation
* `src/ingest/`: orchestration of ingestion flows using Bamana-native BAM and
  FASTQ machinery
* `src/json/`: command contracts and response emission
* `src/commands/`: CLI-facing orchestration only

Compatibility shims may remain temporarily in legacy paths such as
`src/formats/bgzf.rs` and `src/ingest/fastq.rs`, but they are migration aids,
not the architectural center.

## Implementation Sequence

The implementation sequence is intentionally substrate-first:

1. native BGZF
2. native BAM header codec
3. native BAM record scanner
4. native FASTQ / FASTQ.GZ parser
5. command migration beginning with `verify`, `header`, and `subsample`

This sequence is governed by:

* [../ROADMAP.md](/Users/stephen/Projects/bamana/ROADMAP.md)
* [roadmap.md](/Users/stephen/Projects/bamana/docs/roadmap.md)

The architecture and roadmap must stay aligned. If command migration order
changes, the rationale should be updated here and in the roadmap together.

## Command-To-Core Ownership Map

### Header And Metadata Commands

* `header`
  * hot path: yes
  * ownership: `bgzf`, `bam::reader`, `bam::header`
  * `noodles` allowed in hot path: no
* `reheader`
  * hot path: yes
  * ownership: `bgzf`, `bam::header`, `bam::reader`, `bam::write`,
    `bam::reheader`
  * `noodles` allowed in hot path: no
* `annotate_rg`
  * hot path: yes
  * ownership: `bam::reader`, `bam::records`, `bam::tags`, `bam::write`,
    `bam::annotate_rg`
  * `noodles` allowed in hot path: no

### Scan And Inspection Commands

* `verify`
  * hot path: yes
  * ownership: `bgzf`, `bam::reader`
  * `noodles` allowed in hot path: no
* `check_eof`
  * hot path: yes
  * ownership: `bgzf`
  * `noodles` allowed in hot path: no
* `check_sort`
  * hot path: yes
  * ownership: `bam::reader`, `bam::records`, `bam::sort`
  * `noodles` allowed in hot path: no
* `check_map`
  * hot path: yes
  * ownership: `bam::reader`, `bam::index`, `bam::records`
  * `noodles` allowed in hot path: no
* `check_tag`
  * hot path: yes
  * ownership: `bam::reader`, `bam::tags`, `bam::records`
  * `noodles` allowed in hot path: no
* `summary`
  * hot path: yes
  * ownership: `bam::header`, `bam::index`, `bam::summary`, `bam::records`
  * `noodles` allowed in hot path: no
* `validate`
  * hot path: yes
  * ownership: `bgzf`, `bam::reader`, `bam::header`, `bam::validate`,
    `bam::records`, `bam::tags`
  * `noodles` allowed in hot path: only in tests or oracles
* `inspect_duplication`
  * hot path: yes
  * ownership: `bam::reader`, `bam::records`, `fastq`, `sampling`,
    `forensics::duplication`
  * `noodles` allowed in hot path: no
* `forensic_inspect`
  * hot path: yes
  * ownership: `bam::header`, `bam::reader`, `bam::records`, `bam::tags`,
    `forensics::forensic_inspect`
  * `noodles` allowed in hot path: no
* `checksum`
  * hot path: yes
  * ownership: `bam::header`, `bam::reader`, `bam::records`, `bam::checksum`
  * `noodles` allowed in hot path: no

### Transform Commands

* `subsample`
  * hot path: yes
  * ownership: `bam::reader`, `bam::write`, `fastq`, `sampling`
  * `noodles` allowed in hot path: no
* `sort`
  * hot path: yes
  * ownership: `bam::reader`, `bam::records`, `bam::sort`, `bam::write`
  * `noodles` allowed in hot path: no
* `merge`
  * hot path: yes
  * ownership: `bam::reader`, `bam::header`, `bam::merge`, `bam::write`
  * `noodles` allowed in hot path: no
* `deduplicate`
  * hot path: yes
  * ownership: `bam::reader`, `bam::write`, `fastq`, `forensics::deduplicate`
  * `noodles` allowed in hot path: no
* `explode`
  * hot path: yes
  * ownership: `bam::reader`, `bam::write`, future `bam::explode`
  * `noodles` allowed in hot path: no

### Ingest Commands

* `consume`
  * hot path: yes
  * ownership: `fastq`, `bam::reader`, `bam::write`, `ingest::consume`,
    `ingest::discovery`, `ingest::sam`
  * `noodles` allowed in hot path: no for BAM / FASTQ paths
  * transitional exception: CRAM compatibility only

### Non-Hot Or Oracle Roles

* test fixture generation
  * hot path: no
  * `noodles` allowed: yes
* parser differential checks
  * hot path: no
  * `noodles` allowed: yes

## Transitional Compatibility Boundary

Current explicit transitional boundary:

* `src/ingest/cram.rs`

This module exists to keep a conservative CRAM ingestion slice available while
the BAM, BGZF, and FASTQ core is made native and performance-oriented.

That boundary must stay narrow. It must not become the default design pattern
for BAM or FASTQ hot paths.
