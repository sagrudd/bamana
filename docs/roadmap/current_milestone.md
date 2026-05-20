# Current Milestone

## Milestone Status

**Milestone 5: Command Migration Off `noodles`** is complete as of
2026-05-20.

The next planned milestone is **Milestone 6: Native Inspection And Validation
Commands**. It should become active through M6.1, after this M5 closeout state
has been preserved.

See:

* [milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* [milestone-06-inspection-validation.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-06-inspection-validation.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Completed Backbone

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. Milestone 3
completed selective native BAM record scanning and migrated the first
scanner-compatible BAM command consumers. Milestone 4 completed the native
FASTQ and FASTQ.GZ parser/writer core.

Milestone 5 used those substrates to prove command migration boundaries on the
first command set: `verify`, `header`, and `subsample`. It was intentionally
about production command paths and dependency boundaries, not broad native CRAM
support or wholesale replacement of every transform command.

## Milestone 5 Closeout State

M5 closeout established:

* `verify` as a native BGZF plus native BAM header proof command;
* `header` as a native BAM header extraction proof command;
* BAM-side `subsample` through `BamScanner`, `BamRecordView`, and
  scanner-owned retained raw record bytes;
* FASTQ-side `subsample` through `open_fastq_reader`,
  `read_next_fastq_record`, and `FastqWriter`;
* stable governed JSON contracts for the proof commands;
* protected public contracts for `benchmark`, `fastq`, and `unmap`;
* dependency-boundary tests that keep direct production `noodles` usage
  isolated to CRAM compatibility and separately protect the M5 proof-command
  hot paths;
* fixture and differential evidence for BAM, FASTQ, and FASTQ.GZ `subsample`
  selection and encounter-order behavior;
* command-level smoke benchmark rows for `verify`, `header`, `subsample_bam`,
  `subsample_fastq`, and `subsample_fastq_gz`.

M5 closeout verification passed with `cargo test`, `cargo test --test
contract`, Sphinx, and the proof-command benchmark smoke profiles.

## Previously Completed Milestones

**Milestone 1: Native BGZF Core**

See:

* [milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)

The M1 closeout established native BGZF block reading and EOF-marker handling,
BAM-compatible native BGZF writing, virtual-offset groundwork, BGZF
microbenchmark hooks, and dependency guardrails that keep production `noodles`
usage isolated to CRAM compatibility.

**Milestone 2: Native BAM Header Codec**

See:

* [milestone-02-bam-header.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-02-bam-header.md)

The M2 closeout established native BAM magic, header text, SAM-style textual
header, binary reference dictionary parsing, deterministic BAM header
serialization helpers, production `verify` and `header` paths backed by native
BGZF plus native BAM header parsing, malformed-header tests, test-only header
oracle boundaries, header microbenchmark hooks, and dependency-boundary checks.

**Milestone 3: Native BAM Record Scanner**

See:

* [milestone-03-bam-record-scan.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-03-bam-record-scan.md)

The M3 closeout established `BamRecordView`, `BamScanner`, scanner-owned
helpers for common BAM fields and selected aux traversal, scanner-backed
inspection paths, malformed-record tests, dependency-boundary protection, and
`scanner_microbench` hooks.

**Milestone 4: Native FASTQ / FASTQ.GZ Parser**

See:

* [milestone-04-fastq.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-04-fastq.md)

The M4 closeout established native FASTQ modules for record contracts, reader
validation, writer finalization, gzip handling, unmapped conversion, and
`FASTQ.GZI` sidecar work, plus native parser/writer command-consumer evidence,
dependency-boundary checks, and `fastq_microbench` hooks.

## Next Milestone Boundary

Milestone 6 is planned for the first operational BAM inspection and validation
command wave:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

Milestone 6 should not broaden Milestone 5's proof-command claims. Native CRAM
support, broad transform parity, indexing, and random-access work remain later
milestones unless an explicit future task promotes them.
