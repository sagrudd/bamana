Native BAM Record Scanner
=========================

Milestone 3 is active. Bamana's next native-core layer is a selective BAM
record scanner built above the native BGZF reader and native BAM header codec.

Scope
-----

The scanner milestone is responsible for:

* iterating BAM alignment records without full generic decode;
* exposing lightweight views for ``refID``, ``pos``, flags, MAPQ, read name,
  sequence length, and aux-region boundaries;
* skipping unneeded CIGAR, sequence, quality, and auxiliary payload regions
  safely;
* traversing selected auxiliary tags without materializing every optional
  field;
* migrating selected record-scanning command consumers onto shared scanner
  primitives.

The first command beneficiaries are ``check_sort``, ``check_map``,
``summary``, ``check_tag``, ``validate``, ``inspect_duplication``,
``forensic_inspect``, and BAM-side ``subsample``.

Boundaries
----------

Milestone 3 does not claim full BAM semantic validation, BAI/CSI random access,
native CRAM scanning, biological interpretation, or broad command parity. Those
remain downstream unless an explicit task in ``taskmap.md`` includes them.

Production BAM record hot-path scanning must not depend on ``noodles``. Test
oracles may use external parsers only when the test boundary is explicit and
production code remains native.

Task Tracking
-------------

The active task list is maintained in ``taskmap.md`` as M3.1 through M3.10.
Closeout evidence must include passing full tests, passing contract tests,
passing Sphinx documentation, runnable scanner microbenchmarks with
machine-readable output, and recorded command-migration evidence for the first
scanner consumers.
