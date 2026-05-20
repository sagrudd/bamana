Native Command Migration
========================

Milestone 5 is active. It uses the completed native BGZF, BAM header, BAM
scanner, and FASTQ substrates to migrate the first proof commands away from
external production hot-path dependencies.

Scope
-----

The proof-command set is intentionally narrow:

* ``verify``
* ``header``
* ``subsample``

This milestone does not claim native CRAM support, broad command parity, or
complete migration of every transform command.

M5.1 Baseline
-------------

``verify`` already uses Bamana-native primitives. It probes the input, requires
a BGZF-backed BAM container, and parses the BAM header through
``parse_bam_header_from_native_bgzf``.

``header`` follows the same native BGZF and BAM header parse path and returns
the native ``HeaderPayload``.

FASTQ and FASTQ.GZ ``subsample`` paths use the native FASTQ reader, record,
and writer core established in Milestone 4.

BAM ``subsample`` now streams through ``BamScanner`` and writes retained
scanner-owned raw record bytes. The earlier transitional
``BamReader::open`` plus ``read_next_record_layout`` path is no longer used by
the command.

Guardrails
----------

Contract tests protect direct production ``noodles`` usage so it remains
isolated to the CRAM compatibility boundary. Separate checks protect native
header, verify, scanner, migrated BAM-record, and FASTQ hot paths.

``benchmark``, ``fastq``, and ``unmap`` are public contract commands. Their
schemas, examples, and CLI documentation remain protected while M5 work
proceeds on the proof-command set.

Benchmark Evidence
------------------

``header_microbench`` can time ``verify`` and ``header`` command paths when it
is supplied a Bamana binary. The benchmark framework also exposes
``subsample_only`` workflow variants for larger command-level subsample timing.

M5 closeout must record command-level evidence for ``verify``, ``header``, and
``subsample`` after the remaining migration work is complete.

M5.2 Contract Freeze
--------------------

M5.2 freezes the pre-migration proof-command contract baseline without changing
command behavior. ``verify``, ``header``, and ``subsample`` each have governed
JSON schemas, canonical success and failure examples, CLI documentation, and
JSON-output documentation.

Focused contract tests now fail if those proof-command schemas, examples, or
documentation surfaces disappear. The fixture manifest also reserves
``subsample`` coverage for BAM, FASTQ, FASTQ.GZ, malformed FASTQ, and truncated
BAM inputs:

* ``tiny.clean.bam``
* ``tiny.clean.fastq``
* ``tiny.valid.fastq_gz``
* ``tiny.invalid.fastq.truncated``
* ``tiny.invalid.bam.truncated_record``

M5.3 Verify Migration
---------------------

M5.3 confirms ``verify`` as a native proof-command path. Production ``verify``
performs shallow path probing, requires a BGZF-backed BAM container, and parses
BAM magic plus the header/reference dictionary through
``parse_bam_header_from_native_bgzf``.

The command remains intentionally limited. It does not scan alignment records,
validate the full BAM body, or report BGZF EOF-marker status. A valid header can
therefore verify even when the canonical BGZF EOF marker is absent; use
``check_eof`` for EOF-marker checks and ``validate`` for deeper structure.

Command-level timing evidence for ``verify`` is captured through
``header_microbench --bamana-bin``.

M5.4 Header Migration
---------------------

M5.4 confirms ``header`` as a native proof-command path. Production ``header``
performs shallow path probing, requires a BGZF-backed BAM container, parses BAM
magic plus the header/reference dictionary through
``parse_bam_header_from_native_bgzf``, and returns the native ``HeaderPayload``.

The command remains header-only. It does not validate alignment records,
validate the complete BAM body, or report BGZF EOF-marker status. Malformed
alignment-body bytes after an otherwise valid header are therefore outside the
``header`` failure contract.

Command-level timing evidence for ``header`` is captured through
``header_microbench --bamana-bin``.

M5.5 BAM Subsample Migration
----------------------------

M5.5 migrates BAM-side ``subsample`` traversal to ``BamScanner``. The command
opens BAM input through the scanner, reuses the scanner-owned native header for
output header serialization, applies filters and sampling decisions from
``BamRecordView``, and writes retained records from
``BamRecordView::raw_record``.

This is an explicit scanner-owned raw-record bridge. It preserves retained BAM
record bytes without rebuilding each record through the older transitional
``BamReader::open`` plus ``read_next_record_layout`` loop.

The migration preserves deterministic selection, seeded-random selection,
``--mapped-only`` and ``--primary-only`` filtering, encounter-order output, and
the existing JSON contract.

M5.6 FASTQ Subsample Migration
------------------------------

M5.6 records FASTQ-side ``subsample`` as complete on the stable Milestone 4
FASTQ APIs. Plain FASTQ and FASTQ.GZ inputs are opened through
``open_fastq_reader``, streamed as owned ``FastqRecord`` values via
``read_next_fastq_record``, selected with ``FastqRecord`` identity bytes, and
written with ``FastqWriter``.

Output compression remains an output-path policy: temporary files created for
``.gz`` targets keep a gzip suffix so ``FastqWriter`` produces gzip-compressed
FASTQ before the final rename. BAM-only controls, including mapped-only,
primary-only, and index creation, remain rejected before FASTQ streaming begins.

The migration preserves deterministic selection, seeded-random selection,
identity-basis semantics for raw-read inputs, encounter-order output, and the
existing JSON contract.

M5.7 Dependency Boundary
------------------------

M5.7 makes the proof-command dependency boundary explicit. Contract tests name
``verify``, ``header``, and ``subsample`` as the Milestone 5 proof-command set
and guard their production command files plus native substrate hot paths against
direct ``noodles`` imports.

The broader production dependency check still allows direct ``noodles`` usage
only in the documented CRAM compatibility boundary, ``src/ingest/cram.rs``.
Test-only oracle policy is recorded in ``docs/testing-oracles.md``: header
oracle usage remains isolated to ``tests/header_oracle.rs``, while BAM and
FASTQ ``subsample`` expectations are owned by native scanner and native FASTQ
tests unless a future differential check is explicitly labelled as an oracle or
compatibility comparison.

M5.8 Subsample Fixture Coverage
-------------------------------

M5.8 strengthens ``subsample`` fixture-style coverage across BAM, FASTQ, and
FASTQ.GZ. Command tests cover deterministic and seeded-random selection for
BAM through the scanner-owned raw-record bridge and for FASTQ/FASTQ.GZ through
the Milestone 4 FASTQ reader and writer.

The command suite also includes ordered record-digest checks for retained BAM,
FASTQ, and FASTQ.GZ records. Those checks compare output records against source
fixture records after command execution, proving retained record bytes or FASTQ
record structure and encounter order remain stable for the covered inputs.

Governed failure examples now cover unsupported input format, invalid fraction,
and invalid FASTQ filter/index combinations for BAM-only controls.

M5.9 Benchmark Evidence
-----------------------

M5.9 records runnable proof-command benchmark hooks for ``verify``, ``header``,
and ``subsample``.

``header_microbench --bamana-bin`` emits command-level smoke timings for
``verify`` and ``header``. ``scanner_microbench --bamana-bin`` emits
``subsample_bam`` dry-run timing over a generated BAM fixture.
``fastq_microbench --bamana-bin`` emits ``subsample_fastq`` and
``subsample_fastq_gz`` dry-run timings over generated FASTQ and FASTQ.GZ
fixtures.

The M5.9 smoke run used the ``small`` profile with one iteration for each
benchmark. Command timing rows reported ``verify: 1/1``, ``header: 1/1``,
``subsample_bam: 1/1``, ``subsample_fastq: 1/1``, and
``subsample_fastq_gz: 1/1``.

These command timings include process startup, CLI parsing, path probing,
command execution, and JSON emission. They are not pure substrate
microbenchmarks and should be interpreted separately from in-process BGZF,
scanner, or FASTQ throughput rows.
