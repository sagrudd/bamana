Native Inspection And Validation
================================

Milestone 6 is complete. It used the completed native BGZF, BAM header, BAM
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

M6.4 Check Sort Evidence
------------------------

M6.4 hardens ``check_sort`` as a native scanner-backed ordering-evidence
command. Production ``check_sort`` opens BAM input through ``BamScanner`` and
uses ``BamRecordView`` fields for coordinates, flags, read names, and queryname
ordering comparisons.

The command distinguishes bounded and strict evidence. Bounded mode reports
ordering evidence only for the sampled window. ``--strict`` continues
sequential inspection until EOF or until the command has enough evidence for a
stronger conclusion, but it is still not full BAM structural validation.
Specialized header sort modes such as ``template-coordinate`` are preserved and
reported as limited observed confirmation rather than overclaimed as fully
validated.

Command-level timing evidence for ``check_sort`` is captured through
``scanner_microbench --bamana-bin``.

M6.5 Check Map Evidence
-----------------------

M6.5 hardens ``check_map`` as an index-preferred mapping-evidence command with
a native scanner fallback. When a usable BAI sidecar supplies complete
per-reference mapped/unmapped metadata, the command reports
``evidence_source=index``, ``index.used=true``, and omits
``summary.records_examined`` because alignment records were not scanned for the
result.

When an index is absent, disabled, unsupported, malformed, or missing required
metadata, ``check_map`` falls back to ``BamScanner`` traversal and reports
``evidence_source=scan``. Scan-derived output reports
``summary.records_examined`` plus observed mapped, unmapped, per-reference, and
inconsistent-record counts. Bounded scans remain bounded evidence: absence of a
mapped read in the sampled window is not a full-file absence claim. Full-scan
mode expands the scan to EOF, but ``check_map`` still does not perform full BAM
structural validation.

Command-level timing evidence for ``check_map`` is captured through
``scanner_microbench --bamana-bin``.

M6.6 Summary Evidence
---------------------

M6.6 hardens ``summary`` as an operational overview command built from native
header metadata, optional BAI-derived totals, and native ``BamScanner`` record
evidence. BAI metadata is reported in ``index_derived`` and reference
mapped/unmapped fields, but it remains separate from scan-derived counts and
flag categories.

Bounded summaries report ``mode=bounded_scan`` and place fractions in
``fractions_observed``. ``counts.records_total_known`` and full-file fractions
are emitted only when scanner traversal reaches EOF cleanly. Header-only BAM
bodies are valid operational evidence with zero scanned records and low
confidence. Malformed record traversal returns an indeterminate failure payload
rather than a partial summary.

For BAMs still being written, whole-file ``summary`` scans can use
``--live-progress`` to emit a single carriage-return-updated stderr status line
about every 0.5 seconds. The line reports parsed reads, mean BAM base quality
over non-missing quality bytes, and mean read length. ``--allow-incomplete``
lets the scan stop at a missing EOF marker, incomplete trailing BGZF member, or
incomplete trailing BAM record and report complete-prefix evidence only;
``evidence.full_file_scanned`` remains false in that case. These flags apply to
whole-file native scan evidence and are not accepted with ``--region``.

``summary`` is not full BAM structural validation. It is a fast operational
overview; use ``validate`` for structural record validation and interpret
bounded summaries as sampled evidence only.

Command-level timing evidence for ``summary`` is captured through
``scanner_microbench --bamana-bin``.

M6.7 Check Tag Aux Evidence
---------------------------

M6.7 hardens ``check_tag`` as a native scanner-owned auxiliary traversal
command. Production ``check_tag`` opens BAM input through ``BamScanner`` and
uses ``record_aux_contains_tag`` over borrowed ``BamRecordView`` auxiliary bytes
for selected tag lookup and optional BAM auxiliary type filtering.

The command distinguishes presence, bounded non-observation, complete-scan
absence, and indeterminate traversal. ``records_with_tag`` counts records with
at least one matching tag, not duplicate occurrences inside one record.
Malformed auxiliary payloads and unsupported B-array shapes return structured
``tag_parse_uncertainty`` failures with an indeterminate payload instead of
successful absence claims.

``check_tag`` does not validate general tag value semantics. It reports
traversal and supported extraction evidence for the requested tag/type only.
Bounded non-observation is sampled evidence and must not be interpreted as
full-file absence.

Command-level timing evidence for ``check_tag`` is captured through
``scanner_microbench --bamana-bin``.

M6.8 Validate Boundary
----------------------

M6.8 hardens ``validate`` as a native structural validation command. Production
``validate`` probes BAM/BGZF input, opens the native scanner with
``BamScanner::open``, and validates scanner-compatible records through borrowed
``BamRecordView`` fields. Auxiliary-field structure is traversed with native
aux helpers; malformed aux regions are reported as structural findings with
``scope=aux``.

The command distinguishes header-only, bounded-record, and full validation
scopes. Header-only validation does not imply record validity. Bounded
validation reports only the examined prefix and leaves
``summary.full_file_examined=false``. Full validation reports EOF-backed
coverage only when scanner traversal reaches the end of the BAM body cleanly.
Finding counters record all observed severities even when ``--max-errors`` or
``--max-warnings`` limits the stored finding list.

``validate`` remains structural and internal-consistency validation. It does
not prove biological correctness, external reference concordance, or complete
optional-field semantic validity.

Command-level timing evidence for ``validate`` is captured through
``scanner_microbench --bamana-bin``.

Guardrails
----------

Contract tests protect the scanner substrate, selected migrated hot paths, and
the six M6 command paths from direct production ``noodles`` imports.

``benchmark``, ``fastq``, and ``unmap`` remain public contract commands. Their
schemas, examples, and CLI documentation stay governed while M6 focuses on the
inspection and validation command wave.

M6.10 Closeout
--------------

M6.10 closed the milestone after all command-specific hardening tasks and
milestone guardrails completed. Closeout evidence includes:

* governed schemas, examples, CLI contracts, JSON-output docs, README text, and
  Sphinx notes for ``check_eof``, ``check_sort``, ``check_map``, ``summary``,
  ``check_tag``, and ``validate``
* bounded versus full-scan evidence coverage for ordering, mapping, summaries,
  tag absence, and structural validation
* index-derived versus scan-derived evidence boundaries for ``check_map`` and
  ``summary``
* explicit structural-only caveats for ``validate``
* command smoke timing rows in ``bgzf_microbench --bamana-bin`` and
  ``scanner_microbench --bamana-bin``
* dependency-boundary tests that name the full M6 command set
