Native BAM Scanner Microbenchmarks
==================================

Milestone 3 includes lightweight native BAM scanner microbenchmarks for local
regression checks. They do not require private input data: the benchmark binary
generates deterministic BAM fixtures with synthetic references, alignment
records, sequence and quality payloads, and selected auxiliary tags, then emits
machine-readable JSON.

Build the benchmark binary:

.. code-block:: bash

   cargo build --release --bin bamana --bin scanner_microbench

Run a small profile with command timings:

.. code-block:: bash

   target/release/scanner_microbench \
     --profile small \
     --iterations 10 \
     --bamana-bin target/release/bamana \
     --out scanner-small.json

Profiles are:

* ``small``: 1,024 records across 2 references, useful for CI and quick local
  checks
* ``medium``: 50,000 records across 8 references, useful for normal development
  comparisons
* ``large``: 250,000 records across 24 references, useful for stressing scanner
  throughput on larger synthetic BAM bodies

The JSON result contains:

* ``record_scan_throughput``: records-per-second and bytes-per-second for
  native BGZF/header opening plus complete alignment-record iteration through
  ``BamScanner``
* ``selective_field_extraction_throughput``: the same traversal plus selective
  access to core fields, read name, sequence length, and ``NM`` aux-tag
  presence
* optional ``summary``, ``check_sort``, ``check_map``, ``validate``,
  ``check_tag``, ``subsample_bam``, ``sort``, ``merge``,
  ``inspect_duplication``, ``deduplicate``, and ``forensic_inspect`` command timings when
  ``--bamana-bin`` is supplied

The scanner timings and command timings answer different questions. Scanner
timings measure the in-process substrate and selected field access. Command
timings include process startup, CLI parsing, JSON envelope emission, file
probing, and command-specific payload construction; use them for before/after
command migration checks, not as pure scanner measurements.

Milestone 6 command timings are smoke timings over deterministic synthetic BAM
input. ``check_sort`` is run in strict mode. ``check_map`` and ``summary`` run
without adjacent index sidecars, so they exercise scanner-derived evidence
rather than index-derived evidence. ``check_tag`` performs a full scan for the
synthetic ``NM`` auxiliary tag, ``validate`` runs the default full structural
pass, ``sort`` runs a coordinate rewrite with canonical checksum verification,
``merge`` runs a coordinate merge with canonical checksum verification over two
copies of the deterministic fixture, ``inspect_duplication`` runs a full
``qname-seq-qual-rg`` CLI scan over the deterministic BAM fixture, and
``deduplicate`` runs a full dry-run ``qname-seq-qual-rg`` CLI plan.
``forensic_inspect`` runs explicit full-scan header, read-group, program-chain,
read-name, tag, and duplication-hallmark scopes. These timings do not exercise
malformed-input paths and do not imply comparator parity with external tools.

``subsample_bam`` is a command-level dry-run timing over the generated BAM
fixture. It proves the BAM ``subsample`` CLI path is runnable through the
benchmark hook, but it should be interpreted as command smoke timing rather
than output-write throughput.

``sort`` is a command-level rewrite timing over the generated BAM fixture. It
proves the current scanner-backed load, in-memory coordinate ordering, native
BGZF write, and canonical checksum verification path is runnable through the
benchmark hook, but it should be interpreted as command smoke timing rather
than external-memory sort throughput or comparator parity.

``merge`` is a command-level merge timing over two copies of the generated BAM
fixture. It proves the current scanner-backed input loading, reference
dictionary compatibility check, in-memory coordinate merge, native BGZF write,
and canonical checksum verification path is runnable through the benchmark
hook, but it should be interpreted as command smoke timing rather than
external-memory merge throughput or comparator parity.

``inspect_duplication`` is a command-level inspection timing over the generated
BAM fixture. It proves the native scanner-backed duplication inspection CLI path
is runnable through the benchmark hook, but it should be interpreted as command
smoke timing rather than duplicate-detection sensitivity.

``deduplicate`` is a command-level dry-run timing over the generated BAM
fixture. It proves the conservative remediation CLI can build a native
scanner-backed plan through the benchmark hook, but it should be interpreted as
command smoke timing rather than applied output-write throughput.

``forensic_inspect`` is a command-level inspection timing over the generated
BAM fixture. It proves the native header and scanner-backed provenance
inspection CLI path is runnable through the benchmark hook, but it should be
interpreted as command smoke timing rather than forensic sensitivity.

Milestone 7 scanner-backed command timings distinguish scan cost from rewrite
cost. ``inspect_duplication`` and ``forensic_inspect`` perform inspection-only
scan paths and never write output artifacts. ``deduplicate`` is run in
dry-run mode, so it includes process startup, file probing, scanner traversal,
planning, and JSON emission, but it does not include applied rewrite cost,
output BGZF compression cost, removed-report writing, or checksum verification
cost. These command timings are for regression smoke evidence and do not imply
comparator parity with external duplicate-marking, remediation, or provenance
tools.

Results conform to
``benchmarks/results/scanner_microbench.schema.json`` and can be archived
beside other benchmark result artifacts.

Milestone 3 Closeout
--------------------

The Milestone 3 closeout ran the ``small`` profile with one iteration and a
JSON smoke check. The smoke check verified the selected profile, iteration
count, 1,024 generated records, and the expected result schema. Longer local
runs should increase ``--iterations`` and use ``medium`` or ``large`` when
comparing scanner throughput across changes.
