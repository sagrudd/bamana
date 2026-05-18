# Current Milestone

## Active Milestone

**Milestone 2: Native BAM Header Codec**

See:

* [milestone-02-bam-header.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-02-bam-header.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Why This Is Current

Milestone 1 completed the native BGZF substrate. The next dependency layer is
native BAM header ownership: BAM magic, `l_text`, textual SAM-style header
content, the binary reference dictionary, and deterministic header
serialization.

This milestone is intentionally smaller than full BAM record scanning. It
should make `verify` and `header` depend on Bamana-native BGZF plus native BAM
header parsing without implying full alignment-record validation.

## Completed Milestone

**Milestone 1: Native BGZF Core**

See:

* [milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)

Milestone 1 completion evidence remains recorded in:

* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

The M1 closeout established:

* native BGZF block reading and EOF-marker handling;
* BAM-compatible native BGZF writing;
* virtual-offset groundwork for later random-access work;
* BGZF microbenchmark hooks;
* dependency guardrails that keep production `noodles` usage isolated to CRAM
  compatibility.

## Milestone 2 Baseline

Known present pieces:

* `src/bam/header.rs` contains native `parse_bam_header_from_reader` and
  `parse_bam_header` entry points;
* the native parser already checks BAM magic, signed `l_text`, signed `n_ref`,
  signed reference-name length, NUL-terminated reference names, and signed
  reference length;
* raw SAM-style header text, parsed `@HD`, `@SQ`, `@RG`, `@PG`, `@CO`, and
  unknown header records are represented in the JSON-facing header view;
* binary reference names, lengths, and encounter-order indexes are exposed;
* `serialize_bam_header_payload` can emit BAM header bytes for current writer
  consumers;
* `src/commands/header.rs` routes the command through native BGZF streaming and
  native BAM header parsing;
* `src/commands/verify.rs` routes the command through native BGZF recognition
  and native BAM header parsing while leaving alignment records and EOF-marker
  checks out of scope;
* test-only header oracle coverage is isolated in `tests/header_oracle.rs`, and
  dependency-boundary tests protect native header and verify paths from direct
  `noodles` imports;
* production dependency-boundary tests already prohibit direct `noodles` usage
  outside the CRAM compatibility exception.

Known gaps:

* header parse and serialization microbenchmarks are not yet present.

## What “Done” Means

For contributors, Milestone 2 is done only when:

* BAM header text and binary reference dictionaries parse natively;
* malformed and negative header lengths fail safely with structured errors;
* deterministic header serialization is tested and shared by writer consumers;
* textual and binary reference metadata reconciliation is documented and tested;
* production `header` and `verify` behavior is not backed by `noodles`;
* header microbenchmark hooks are runnable and documented;
* completion evidence is recorded in `taskmap.md`.

## Command-Surface Boundary

Milestone 2 completion evidence is limited to native BAM header ownership and
the `verify` and `header` command paths that consume it.

Commands such as `reheader`, `check_sort`, `check_map`, `summary`, `merge`,
`checksum`, and later scanner-driven commands may benefit from the header codec,
but their broader semantics remain downstream work unless an explicit M2 task
updates them.

## What Should Not Happen

Do not treat Milestone 2 as full BAM validation, full record scanning, BAI/CSI
random access, or command-wide migration. Those remain later milestones or
downstream command work.
