fastq_gz_enumerate
==================

Purpose
-------

``fastq_gz_enumerate`` benchmarks one narrow question directly: how quickly can
records be counted from a ``FASTQ.GZ`` stream without materializing an unpacked
file on disk?

This profile exists to compare Bamana-native FASTQ.GZ enumeration against a
simple external baseline built from ``gzip`` plus line counting.

Invocation
----------

Run the profile from the repository root:

.. code-block:: bash

   bamana benchmark \
     --profile fastq_gz_enumerate \
     --fastq /abs/path/to/reads.fastq.gz \
     --report /abs/path/to/fastq-gz-enumerate-report.pdf \
     --force

Required inputs and outputs:

* ``--fastq`` points to the gzipped FASTQ input.
* ``--report`` declares the PDF report path.

This profile does not require ``--bam`` because its primary outputs are count
artifacts rather than BAM files.

What the profile does
---------------------

The command owns the complete setup:

* builds the local release binaries with ``cargo build --release``
* builds the benchmark container from ``benchmarks/Dockerfile``
* runs the benchmark inside that container
* aggregates raw results with the repository R scripts
* renders a PDF report from R Markdown

Benchmarked command paths
-------------------------

Bamana path:

.. code-block:: bash

   bamana enumerate --input <fastq.gz> --json-pretty > <bamana.count.json>

Comparator path:

.. code-block:: bash

   gzip -cd <fastq.gz> | awk 'END { printf "%.0f\n", NR / 4 }' > <gzip.count.txt>

The Bamana path now uses the main CLI ``enumerate`` subcommand, which counts
records directly from the gzipped stream and auto-materializes a sibling
``FASTQ.GZI`` sidecar when one is absent. The comparator intentionally exposes
the traditional shell pipeline so the benchmark can answer whether Bamana beats
that baseline on the same input.

Gzip handling
-------------

This profile also avoids unpacking ``FASTQ.GZ`` to disk. The comparator does
stream decompression through ``gzip -cd``, but it writes only the final count
text file, not a temporary FASTQ. The Bamana path reads the gzip stream through
the in-process reader and writes JSON output with the counted record total.

Produced artifacts
------------------

Running the profile produces:

* ``<report-stem>.benchmark/fastq_gz_enumerate.bamana.count.json``
* ``<report-stem>.benchmark/fastq_gz_enumerate.gzip.count.txt``
* the requested PDF report at ``--report``
* a benchmark work directory beside the report named
  ``<report-stem>.benchmark``

The work directory also contains:

* ``raw/`` with per-run result JSON
* ``aggregated/`` with tidy CSV summaries and support matrices
* ``metadata/`` with tool versions and input metrics
* ``logs/`` with cargo, docker, and benchmark execution logs

Report contents
---------------

The PDF report is rendered from the R Markdown template
``benchmarks/R/fastq_gz_enumerate_report.Rmd``. It focuses on:

* wall time for Bamana versus the ``gzip`` pipeline
* count-output artifacts for both paths
* support status and semantic equivalence
* tool versions for ``bamana``, ``gzip``, and ``awk``
* notes explaining that the benchmark compares native enumeration with stream
  decompression plus line counting

Interpretation
--------------

This profile is deliberately narrow. It does not claim to benchmark full FASTQ
ingestion or BAM materialization. It only answers whether Bamana can enumerate
records from ``FASTQ.GZ`` faster than a plain ``gzip``-driven counting
pipeline, while keeping the benchmark honest about the exact workflow each tool
ran.
