BGZF Microbenchmarks
====================

Milestone 1 includes lightweight native BGZF microbenchmarks for local
regression checks. They do not require private input data: the benchmark binary
generates deterministic BAM-like BGZF fixtures and emits machine-readable JSON.

Build the benchmark binary:

.. code-block:: bash

   cargo build --release --bin bamana --bin bgzf_microbench

Run a small profile with command timings:

.. code-block:: bash

   target/release/bgzf_microbench \
     --profile small \
     --iterations 10 \
     --bamana-bin target/release/bamana \
     --out bgzf-small.json

Profiles are:

* ``small``: 128 KiB payload, useful for CI and quick local checks
* ``medium``: 8 MiB payload, useful for normal development comparisons
* ``large``: 128 MiB payload, useful for local throughput comparisons

The JSON result contains:

* native BGZF write throughput
* native BGZF read throughput
* BGZF EOF-check latency
* optional ``verify`` and ``check_eof`` command timings when ``--bamana-bin`` is
  supplied

Results conform to
``benchmarks/results/bgzf_microbench.schema.json`` and can be archived beside
other benchmark result artifacts.

Milestone 1 Closeout
--------------------

The Milestone 1 closeout used the ``small`` profile with one iteration and
``--bamana-bin`` so the JSON included both substrate timings and command
timings for ``verify`` and ``check_eof``. Longer local runs should increase
``--iterations`` and use ``medium`` or ``large`` when comparing throughput
changes.
