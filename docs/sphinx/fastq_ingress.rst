fastq_ingress
=============

Purpose
-------

``fastq_ingress`` measures FASTQ.GZ-to-unmapped-BAM ingestion. It is the
profile to use when you want Bamana to own the benchmark setup and compare its
native ingest path against a pragmatic external baseline.

Invocation
----------

Run the profile from the repository root:

.. code-block:: bash

   bamana benchmark \
     --profile fastq_ingress \
     --fastq /abs/path/to/reads.fastq.gz \
     --bam /abs/path/to/reads.bamana.bam \
     --report /abs/path/to/fastq-ingress-report.pdf \
     --force

Required inputs and outputs:

* ``--fastq`` points to the gzipped FASTQ input.
* ``--bam`` declares the Bamana BAM output path.
* ``--report`` declares the PDF report path.

What the profile does
---------------------

The command owns the complete preparation path:

* builds the local release binaries with ``cargo build --release``
* builds the benchmark container from ``benchmarks/Dockerfile``
* runs the benchmark inside the container as the calling user
* aggregates raw results with the repository R scripts
* renders a PDF report from R Markdown

Benchmarked command paths
-------------------------

Bamana path:

.. code-block:: bash

   bamana consume --input <fastq.gz> --out <bamana.bam> --mode unmapped --threads <n> --force

Comparator path:

.. code-block:: bash

   fastcat fastq <fastq.gz> | samtools import -o <comparator.bam> -

The comparator is intentionally described as a partial equivalence in the
benchmark output because it uses ``fastcat`` for ingress and ``samtools
import`` for BAM materialization rather than matching Bamana's internal code
path one-to-one.

Gzip handling
-------------

This profile does not unpack the input ``.fastq.gz`` to disk.

Two points matter here:

* the benchmarked Bamana ingest path reads ``FASTQ.GZ`` directly
* the benchmark prep path now uses Bamana-native FASTQ.GZ enumeration for input
  record counting instead of shelling out to ``gzip -cd``

That means the large ``gzip`` process you may see during other tools or other
profiles is not part of the Bamana ingress implementation for this profile.

Produced artifacts
------------------

Running the profile produces:

* the requested Bamana BAM at ``--bam``
* a comparator BAM beside it named ``<stem>.fastcat_samtools.bam``
* the requested PDF report at ``--report``
* a benchmark work directory beside the report named
  ``<report-stem>.benchmark``

The work directory contains:

* ``raw/`` with per-run result JSON
* ``aggregated/`` with tidy CSV summaries and support matrices
* ``metadata/`` with tool versions and input metrics
* ``logs/`` with the cargo, docker, and benchmark execution logs

Report contents
---------------

The PDF report is rendered from the R Markdown template
``benchmarks/R/fastq_ingress_report.Rmd``. It summarizes:

* wall time and throughput
* output sizes
* support status and semantic equivalence
* tool versions captured inside the container
* profile notes explaining the comparison and gzip-handling policy

Operational notes
-----------------

Use ``--force`` when re-running a profile to replace the BAM, report, and
benchmark work directory. The profile runs inside the benchmark container but
bind-mounts the input, output, and report directories so the resulting files
remain on the host filesystem with the caller's UID and GID.
