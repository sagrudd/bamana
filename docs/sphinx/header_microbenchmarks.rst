Native BAM Header Microbenchmarks
=================================

Milestone 2 includes lightweight native BAM header microbenchmarks for local
regression checks. They do not require private input data: the benchmark binary
generates deterministic BAM fixtures with synthetic SAM-style header text and a
binary reference dictionary, then emits machine-readable JSON.

Build the benchmark binary:

.. code-block:: bash

   cargo build --release --bin bamana --bin header_microbench

Run a small profile with command timings:

.. code-block:: bash

   target/release/header_microbench \
     --profile small \
     --iterations 10 \
     --bamana-bin target/release/bamana \
     --out header-small.json

Profiles are:

* ``small``: 2 references, useful for CI and quick local checks
* ``medium``: 128 references, useful for normal development comparisons
* ``large``: 4096 references, useful for stressing reference-dictionary scale

The JSON result contains:

* native header parse latency
* deterministic native header serialization latency
* optional ``verify``, ``header``, and ``reheader`` command timings when
  ``--bamana-bin`` is supplied

The codec timings and command timings answer different questions. Header parse
latency measures native BGZF streaming plus native BAM header parsing for a
locally generated fixture. Header serialization latency measures deterministic
BAM header payload serialization without writing BGZF blocks. Command timings
include process startup, CLI parsing, JSON envelope emission, file probing, and
the command-specific path; use them for before/after command migration checks,
not as pure codec measurements.

Results conform to
``benchmarks/results/header_microbench.schema.json`` and can be archived beside
other benchmark result artifacts.

Milestone 2 Closeout
--------------------

The Milestone 2 closeout ran the ``small`` profile with one iteration and
``--bamana-bin`` so the JSON included both codec timings and command timings
for ``verify`` and ``header``. Milestone 7 extends the same hook with a
``reheader`` dry-run smoke path. Longer local runs should increase
``--iterations`` and use ``medium`` or ``large`` when comparing header
reference-dictionary scaling.
