Public CLI Commands
===================

Bamana is a JSON-first CLI. Public commands must have governed command
contracts, JSON schemas, canonical examples, and user-facing documentation.

benchmark
---------

``bamana benchmark`` is the owned operator entry point for selected benchmark
profiles. It builds the local release binary, builds the benchmark container,
runs the selected profile inside that container, captures logs and
machine-readable outputs, and renders a PDF report.

Current public profiles are:

* ``fastq_ingress``
* ``fastq_gz_enumerate``

``benchmark`` reports exact command paths and produced artifact locations. It
does not imply broad comparator parity outside the selected profile.

fastq
-----

``bamana fastq`` exports one BAM as an ordered ``FASTQ.GZ`` stream:

.. code-block:: bash

   bamana fastq --bam input.bam --out input.fastq.gz -j 8

The command preserves read names, sequences, and qualities in input encounter
order. BAM header records, alignment state, and auxiliary tags are not
represented in FASTQ output.

unmap
-----

``bamana unmap`` rewrites one BAM as unmapped BAM:

.. code-block:: bash

   bamana unmap --bam aligned.bam --out aligned.unmapped.bam

The command removes reference-bound header state, coordinates, CIGAR data,
mate coordinates, template length, mapping quality, and mapping-related
auxiliary tags. Non-mapping auxiliary metadata is preserved. Use ``--dry-run``
to plan and count without writing output.

annotate_rg
-----------

``bamana annotate_rg`` is the public record-level read-group annotation
command. It inserts, replaces, or conflict-checks per-record ``RG:Z`` tags and
can coordinate that record-level rewrite with explicit ``@RG`` header policy.
It is intentionally distinct from ``reheader``, which is header-only.
