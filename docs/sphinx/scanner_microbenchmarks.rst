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
* optional ``summary``, ``validate``, and ``check_tag`` command timings when
  ``--bamana-bin`` is supplied

The scanner timings and command timings answer different questions. Scanner
timings measure the in-process substrate and selected field access. Command
timings include process startup, CLI parsing, JSON envelope emission, file
probing, and command-specific payload construction; use them for before/after
command migration checks, not as pure scanner measurements.

Results conform to
``benchmarks/results/scanner_microbench.schema.json`` and can be archived
beside other benchmark result artifacts.

Milestone 3 Closeout
--------------------

The Milestone 3 closeout should run the ``small`` profile with one iteration
and ``--bamana-bin`` so the JSON includes both scanner timings and migrated
command timings. Longer local runs should increase ``--iterations`` and use
``medium`` or ``large`` when comparing scanner throughput across changes.
