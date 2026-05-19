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

FASTQ and FASTQ.GZ ``subsample`` paths already use the native FASTQ reader,
record, and writer core established in Milestone 4.

BAM ``subsample`` is the remaining migration target. It currently streams
through ``BamReader::open``, parses the header through
``parse_bam_header_from_reader``, reads records with
``read_next_record_layout``, and writes retained records with
``serialize_record_layout`` plus ``BgzfWriter``. This path is free of direct
``noodles`` imports, but it is not yet proven through ``BamScanner`` or a
scanner-owned raw-record bridge.

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
