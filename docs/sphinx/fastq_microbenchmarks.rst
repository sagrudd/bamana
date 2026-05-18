Native FASTQ Microbenchmarks
============================

Milestone 4 includes lightweight native FASTQ and FASTQ.GZ microbenchmarks for
local regression checks. They do not require private input data: the benchmark
binary generates deterministic synthetic FASTQ records, writes plain and gzip
fixtures through the Bamana-native writer, and emits machine-readable JSON.

Build the benchmark binary:

.. code-block:: bash

   cargo build --release --bin bamana --bin fastq_microbench

Run a small profile with command timings:

.. code-block:: bash

   target/release/fastq_microbench \
     --profile small \
     --iterations 10 \
     --bamana-bin target/release/bamana \
     --out fastq-small.json

Profiles are:

* ``small``: 1,024 records with 75-base reads, useful for CI and quick local
  checks
* ``medium``: 50,000 records with 100-base reads, useful for normal development
  comparisons
* ``large``: 250,000 records with 150-base reads, useful for stressing parser
  and writer throughput on larger synthetic FASTQ collections

The JSON result contains:

* ``plain_parse_throughput``: records-per-second and bytes-per-second for
  native plain FASTQ parsing and validation through ``count_fastq_records``
* ``gzip_parse_throughput``: the same measurement for extension-selected
  FASTQ.GZ parsing through the native reader facade
* ``plain_writer_throughput``: records-per-second and bytes-per-second for
  writing owned ``FastqRecord`` values as plain FASTQ
* ``gzip_writer_throughput``: the same writer measurement with gzip output,
  including gzip finalization before the write iteration completes
* optional ``enumerate_fastq`` and ``enumerate_fastq_gz`` command timings when
  ``--bamana-bin`` is supplied

The in-process measurements and command timings answer different questions.
In-process timings measure the native FASTQ substrate. Command timings include
process startup, CLI parsing, file probing, JSON envelope emission, and
command-specific behavior such as FASTQ.GZI creation or reuse for FASTQ.GZ
enumeration.

Results conform to
``benchmarks/results/fastq_microbench.schema.json`` and can be archived beside
other benchmark result artifacts.

Milestone 4 Closeout
--------------------

The Milestone 4 closeout ran the ``small`` profile with one iteration and a
JSON smoke check. The smoke check verified the benchmark name, selected
profile, iteration count, 1,024 generated records, and the expected result
keys. Longer local runs should increase ``--iterations`` and use ``medium`` or
``large`` when comparing FASTQ parser or writer throughput across changes.
