# Current Milestone

## Milestone Status

**Milestone 6: Native Inspection And Validation Commands** is active as of
2026-05-21.

Milestone 6 became active after **Milestone 5: Command Migration Off
`noodles`** closed on 2026-05-20.

See:

* [milestone-06-inspection-validation.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-06-inspection-validation.md)
* [milestone-05-command-migration.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-05-command-migration.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

## Completed Backbone

Milestone 1 completed the native BGZF substrate. Milestone 2 completed native
BAM header parsing and deterministic header serialization. Milestone 3
completed selective native BAM record scanning and migrated the first
scanner-compatible BAM command consumers. Milestone 4 completed the native
FASTQ and FASTQ.GZ parser/writer core. Milestone 5 completed the proof-command
migration for `verify`, `header`, and `subsample`.

## Milestone 6 Scope

Milestone 6 hardens the first operational BAM inspection and validation command
wave:

* `check_eof`
* `check_sort`
* `check_map`
* `summary`
* `check_tag`
* `validate`

The milestone is about bounded evidence, full-scan claims, index-versus-scan
distinctions, structural validation caveats, command contracts, benchmark smoke
coverage, and dependency-boundary protection for these command paths.

## Baseline State

Known present pieces:

* `check_eof` uses the native BGZF EOF-marker detection path;
* `check_sort` uses `BamScanner` and scanner-owned record-view helpers for
  ordering evidence;
* `check_map` prefers usable BAI-derived mapping summaries and otherwise falls
  back to `BamScanner` traversal;
* `summary` combines header metadata, optional index-derived totals, and
  bounded or full `BamScanner` record scans;
* `check_tag` uses `BamScanner` plus native aux traversal helpers for selected
  tag lookup;
* `validate` uses the native validation substrate built on header parsing,
  `BamScanner`, and `BamRecordView`;
* existing schemas, examples, CLI docs, and README guidance cover the six M6
  commands;
* dependency-boundary tests already protect the scanner substrate and selected
  migrated hot paths from direct production `noodles` imports.

Known M6 hardening work:

* freeze the six command contracts and examples as one governed M6 wave;
* sharpen bounded versus full-scan language for absence and validity claims;
* keep index-derived and scan-derived mapping evidence distinct;
* make `validate` caveats explicit enough to avoid biological,
  reference-level, or optional-field semantic overclaiming;
* add command-level benchmark or smoke benchmark evidence for the complete M6
  command set;
* strengthen dependency-boundary tests so the six M6 command files are named as
  one protected milestone set.

## Command-Surface Boundary

Milestone 6 evidence is limited to `check_eof`, `check_sort`, `check_map`,
`summary`, `check_tag`, and `validate`.

Mutation, rewrite, normalization, deduplication, forensic, checksum, sort,
merge, explode, ingest, native CRAM, BAM index writing, and random-access work
remain later milestones unless an explicit M6 task includes them.
