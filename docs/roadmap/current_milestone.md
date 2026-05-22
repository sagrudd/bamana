# Current Milestone

## Milestone Status

**Milestone 6: Native Inspection And Validation Commands** is complete as of
2026-05-21. **Milestone 7: Native Mutation, Remediation, And Forensics
Commands** is active as of 2026-05-22.

Milestone 6 became active after **Milestone 5: Command Migration Off
`noodles`** closed on 2026-05-20, and closed after M6.1 through M6.10
completed on 2026-05-21. Milestone 7 became active only after that M6
closeout was recorded.

See:

* [milestone-07-mutation-forensics.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-07-mutation-forensics.md)
* [milestone-06-inspection-validation.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-06-inspection-validation.md)
* [milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Completed Backbone

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. Milestone 3
completed selective native BAM record scanning and migrated the first
scanner-compatible BAM command consumers. Milestone 4 completed the native
FASTQ and FASTQ.GZ parser/writer core. Milestone 5 completed the proof-command
migration for `verify`, `header`, and `subsample`. Milestone 6 completed the
native inspection and validation hardening wave for `check_eof`, `check_sort`,
`check_map`, `summary`, `check_tag`, and `validate`.

## Milestone 7 Scope

Milestone 7 hardens native mutation, conservative remediation, and provenance
inspection command paths:

* `reheader`
* `annotate_rg`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

The milestone is about native mutation safety, conservative remediation
boundaries, provenance evidence, command contracts, benchmark smoke coverage,
and dependency-boundary protection for these higher-blast-radius command
paths.

## Milestone 6 Closeout

Milestone 6 hardened the first operational BAM inspection and validation command
wave:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

The milestone covered bounded evidence, full-scan claims, index-versus-scan
distinctions, structural validation caveats, command contracts, benchmark smoke
coverage, and dependency-boundary protection for these command paths.

Closeout evidence:

* command contracts, schemas, examples, CLI docs, JSON-output docs, README, and
  Sphinx notes cover the six M6 commands;
* bounded versus full-scan evidence is documented for absence and validity
  claims;
* index-derived and scan-derived mapping/summary evidence is distinct;
* `validate` explicitly remains structural/internal-consistency validation and
  does not claim biological correctness, external reference concordance, or
  complete optional-field semantic validation;
* `bgzf_microbench --bamana-bin` emits `check_eof` command smoke timing;
* `scanner_microbench --bamana-bin` emits `check_sort`, `check_map`,
  `summary`, `check_tag`, and `validate` command smoke timings;
* dependency-boundary tests name the six M6 command paths and keep them free of
  direct production `noodles` imports.

## Command-Surface Boundary

Milestone 7 evidence is limited to `reheader`, `annotate_rg`,
`inspect_duplication`, `deduplicate`, and `forensic_inspect`.

Large transform, ordering, merge, checksum, explode, ingest, native CRAM, BAM
index writing, and random-access work remain later milestones unless an
explicit M7 task includes them. The public contract commands `benchmark`,
`fastq`, and `unmap` remain protected while M7 work proceeds.
