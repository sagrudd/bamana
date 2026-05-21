Native Inspection And Validation
================================

Milestone 6 is active. It uses the completed native BGZF, BAM header, BAM
scanner, and proof-command migration substrate to harden Bamana's first
operational BAM inspection and validation command wave.

Scope
-----

The M6 command set is:

* ``check_eof``
* ``check_sort``
* ``check_map``
* ``summary``
* ``check_tag``
* ``validate``

This milestone does not claim native CRAM support, BAM index writing,
random-access workflows, mutation/remediation hardening, or broad transform
parity.

M6.1 Baseline
-------------

``check_eof`` already uses native BGZF EOF-marker detection after probing that
the input is a BGZF-backed BAM container. The command remains narrow:
EOF-marker presence is tail-completeness evidence only and does not imply BAM
header validity, full stream readability, or alignment-record validity.

``check_sort`` opens BAM input through ``BamScanner`` and uses
``BamRecordView``-derived coordinate, flag, and read-name fields to compare
header-declared sort metadata with bounded or strict record-order evidence.

``check_map`` preserves usable BAI-derived mapping summaries as the preferred
evidence source. When an index is disabled, unsupported, incomplete, or
unusable, the command falls back to ``BamScanner`` record traversal and reports
scan-derived evidence separately.

``summary`` combines native header metadata, optional index-derived totals, and
bounded or full ``BamScanner`` record scans. Its output must continue to state
which evidence mode was used.

``check_tag`` uses ``BamScanner`` plus native auxiliary-field traversal helpers
for selected tag lookup and optional type filtering. Bounded non-observation is
not full-file absence; full-file absence is only supported after a complete
scan succeeds.

``validate`` uses the native validation substrate built on native header
parsing, ``BamScanner``, and ``BamRecordView``. It remains structural and
internal-consistency validation, not biological correctness, external reference
concordance, or complete optional-field semantic validation.

M6.2 Contract Freeze
--------------------

M6.2 freezes the inspection-command contract baseline before command-specific
hardening. ``check_eof``, ``check_sort``, ``check_map``, ``summary``,
``check_tag``, and ``validate`` each have governed JSON schemas, canonical
success and failure examples, CLI documentation, JSON-output documentation, and
reserved fixture coverage.

The frozen fixture plan covers missing EOF, sorted and unsorted BAMs,
mapped and unmapped evidence, index-derived mapping and summary evidence,
observed and absent tags, malformed aux payloads, and structural validation
failures. Bounded examples must remain bounded: they may describe examined
records only. Full-file absence and full-file summary claims require a complete
scan that reaches EOF cleanly.

M6.3 Check EOF Boundary
-----------------------

M6.3 hardens ``check_eof`` as the native BGZF EOF-marker command. Production
``check_eof`` performs shallow path probing, requires a BGZF-backed BAM
container, and then calls Bamana's native ``has_bgzf_eof`` tail check.

The command remains intentionally narrow. It does not parse BAM magic, does not
parse the BAM header, does not inspect alignment records, and does not validate
auxiliary fields. A BGZF stream with invalid BAM header bytes can therefore
pass ``check_eof`` when the canonical EOF marker is present. Use ``verify`` for
native BAM header checks and ``validate`` for deeper structural validation.

Command-level timing evidence for ``check_eof`` is captured through
``bgzf_microbench --bamana-bin``.

Guardrails
----------

Contract tests already protect the scanner substrate and selected migrated hot
paths from direct production ``noodles`` imports. Later M6 tasks should name
the six M6 command files as one protected milestone set.

``benchmark``, ``fastq``, and ``unmap`` remain public contract commands. Their
schemas, examples, and CLI documentation stay governed while M6 focuses on the
inspection and validation command wave.

Remaining M6 Work
-----------------

M6 still needs:

* a milestone-level contract and example freeze for all six commands;
* fixture evidence for bounded scans, full scans, absent evidence,
  malformed-input handling, and index-versus-scan distinctions;
* explicit ``validate`` caveats that prevent structural validation from
  overclaiming biological or reference-level correctness;
* command-level smoke benchmark evidence for the full M6 command set;
* dependency-boundary tests that name the full M6 command set.
