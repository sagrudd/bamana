# Current Milestone

## Milestone Status

**Milestone 3: Native BAM Record Scanner** is active as of 2026-05-18.

See:

* [milestone-03-bam-record-scan.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-03-bam-record-scan.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Why This Is Current

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. The next dependency
layer is selective native BAM record scanning: iterating alignment records,
exposing lightweight field views, safely skipping unneeded variable sections,
and supporting selected aux-tag traversal without full generic decode.

This milestone is intentionally smaller than full BAM semantic validation,
BAI/CSI random access, native CRAM scanning, or broad command parity. It should
make the first record-scanning command paths depend on Bamana-native BGZF,
native BAM headers, and a shared native scanner without implying that every BAM
operation has migrated.

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

Milestone 1 and 2 completion evidence remains recorded in:

* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Milestone 3 Current State

Known present pieces:

* native BGZF reading can feed BAM payload bytes;
* native BAM header parsing can position readers at the first alignment record;
* `src/bam/records.rs` contains current record layout helpers and richer
  record decoding pieces used by existing commands;
* record-facing commands already exist, including `check_sort`, `check_map`,
  `summary`, `check_tag`, `validate`, `inspect_duplication`,
  `forensic_inspect`, and BAM-side `subsample`;
* dependency-boundary tests already prohibit production `noodles` usage outside
  the CRAM compatibility exception.

Known gaps:

* no dedicated shared scanner API owns selective BAM record iteration yet;
* no stable lightweight record-view contract owns core fields and aux
  boundaries;
* skip-oriented field extraction is not centralized;
* aux-region traversal for selected tag lookup is not yet a scanner-owned API;
* first command consumers have not yet been migrated onto a shared scanner;
* scanner oracle coverage and microbenchmarks are not yet present.

## Completion Boundary

Milestone 3 completion will mean:

* BAM alignment records can be iterated natively without a full generic decode;
* lightweight record views expose at least `refID`, `pos`, flags, MAPQ, read
  name, sequence length, and aux-region boundaries;
* scanner helpers can skip CIGAR, sequence, quality, and aux payloads when a
  consumer does not need them;
* selected aux tags can be traversed safely without rich materialization;
* selected first command consumers use the shared scanner for record traversal;
* scanner microbenchmark hooks are runnable and documented;
* production BAM record hot-path scanning does not depend on `noodles`.

## Command-Surface Boundary

Milestone 3 evidence is limited to native BAM record scanning and the selected
command paths that consume scanner-owned record views.

Commands such as `check_sort`, `check_map`, `summary`, `check_tag`,
`validate`, `inspect_duplication`, `forensic_inspect`, and BAM-side
`subsample` are beneficiaries, but each command's broader semantics remain
bounded by its existing public contract unless an explicit M3 task updates that
contract.

## What Should Not Happen

Do not treat Milestone 3 as full BAM semantic validation, BAI/CSI random
access, native CRAM scanning, broad command parity, biological interpretation,
or wholesale replacement of every rich record conversion path. Those remain
later milestones or downstream command work unless an explicit M3 task includes
them.
