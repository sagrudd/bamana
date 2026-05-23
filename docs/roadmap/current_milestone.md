# Current Milestone

## Milestone Status

**Milestone 6: Native Inspection And Validation Commands** is complete as of
2026-05-21. **Milestone 7: Native Mutation, Remediation, And Forensics
Commands** is complete as of 2026-05-22. **Milestone 8: Native Transform,
Checksum, Explode, And Ingest Commands** is complete as of 2026-05-23.
**Milestone 9: Native BAM Index And Random Access** remains planned and should
be activated by M9.1 only after the M8 closeout commit is in place.

Milestone 6 became active after **Milestone 5: Command Migration Off
`noodles`** closed on 2026-05-20, and closed after M6.1 through M6.10
completed on 2026-05-21. Milestone 7 became active only after that M6
closeout was recorded, and closed after M7.1 through M7.10 completed on
2026-05-22. Milestone 8 became active only after that M7 closeout was
recorded, and closed after M8.1 through M8.10 completed on 2026-05-23.

See:

* [milestone-08-transform-ingest.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-08-transform-ingest.md)
* [milestone-09-bam-index-random-access.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-09-bam-index-random-access.md)
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
`check_map`, `summary`, `check_tag`, and `validate`. Milestone 7 completed
native mutation, conservative remediation, and provenance inspection for
`reheader`, `annotate_rg`, `inspect_duplication`, `deduplicate`, and
`forensic_inspect`. Milestone 8 completed the native transform, checksum,
sharding, and ingest hardening wave for `sort`, `merge`, `explode`,
`checksum`, and `consume`.

## Milestone 8 Closeout

Milestone 8 hardened Bamana's largest transform, checksum, explode, and ingest
command paths:

* `sort`
* `merge`
* `explode`
* `checksum`
* `consume`

Closeout evidence:

* `sort`, `merge`, `explode`, `checksum`, and `consume` have governed CLI,
  JSON-output, Sphinx, README, schema, example, fixture, and task-map coverage;
* BAM-side `sort`, `merge`, `explode`, `checksum`, and `consume` use
  scanner-backed native record loading and native writer/checksum bridges
  where applicable;
* SAM, FASTQ, and FASTQ.GZ transform/ingest behavior remains native, while
  CRAM remains a documented compatibility boundary under explicit reference
  policy;
* output-safety rules publish completed temporary outputs through final rename
  steps and keep dry-run paths side-effect bounded;
* `scanner_microbench --bamana-bin` emits command smoke timings for every M8
  command without claiming external comparator parity;
* dependency-boundary tests name the five M8 command paths and keep them free
  of direct production `noodles` imports outside documented CRAM
  compatibility.

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

## Milestone 7 Closeout

Milestone 7 hardened native mutation, conservative remediation, and
provenance-inspection command paths:

* `reheader`
* `annotate_rg`
* `inspect_duplication`
* `deduplicate`
* `forensic_inspect`

Closeout evidence:

* command contracts, schemas, examples, CLI docs, JSON-output docs, README, and
  Sphinx notes cover the five M7 commands;
* `reheader` is documented and tested as header-only mutation, while
  `annotate_rg` is documented and tested as record-level read-group
  annotation;
* `inspect_duplication` remains collection-duplication/operator-error
  inspection, not biological duplicate marking;
* `deduplicate` remains conservative remediation over native BAM/FASTQ paths,
  not broad duplicate collapse;
* `forensic_inspect` remains evidence-driven provenance inspection, not fraud
  detection;
* `header_microbench --bamana-bin` emits `reheader` and `annotate_rg` command
  smoke timings;
* `scanner_microbench --bamana-bin` emits `inspect_duplication`,
  `deduplicate`, and `forensic_inspect` command smoke timings;
* dependency-boundary tests name the five M7 command paths and keep them free
  of direct production `noodles` imports.

## Command-Surface Boundary

Milestone 8 evidence is limited to `sort`, `merge`, `explode`, `checksum`, and
`consume`.

Native CRAM, BAM index writing, and random-access work remain later milestones
unless an explicit M8 task includes them. The public contract commands
`benchmark`, `fastq`, and `unmap` remain protected after M8 closeout.

## Next Planned Milestone

Milestone 9 is planned for native BAM index writing, deeper index validation,
and virtual-offset-backed random-access groundwork. M9 should become active
only through M9.1 after this M8 closeout is recorded.
