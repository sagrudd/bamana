Public CLI Commands
===================

Bamana is a JSON-first CLI. Public commands must have governed command
contracts, JSON schemas, canonical examples, and user-facing documentation.

records
-------

``bamana records`` emits provenance-bound normalized alignment records for
governed downstream consumers. Repeat ``--region`` for one-based closed
intervals:

.. code-block:: bash

   bamana records --bam target.bam --region chr20:1001-2000 \
     --input-object-id sha256:<bam> --reference-assembly CHM13v2.0 \
     --reference-object-id sha256:<fasta> --reference-sha256 <fasta-sha256> \
     --reference-fai-sha256 <fai-sha256> --bamana-git-commit <commit>

Region mode requires a current BAI sidecar and uses native indexed traversal.
Missing, stale, malformed, or unsupported indices fail closed: there is no
whole-file scan fallback. Overlapping requests emit each physical record once
in source virtual-offset order, and ``region_scope`` records normalized
coordinates, index path, chunk counts, raw records examined, and duplicate
suppression. ``--max-records`` bounds selected records after region matching.

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

The command preserves read names and restores sequences and qualities to
original molecule orientation in input encounter order, including
reverse-strand BAM records. Ordinary export omits BAM auxiliary tags. For modified-base remapping,
request the governed tag-safe mode explicitly:

.. code-block:: bash

   bamana fastq --bam modified.bam --out modified.fastq.gz -j 20 \
     --preserve-modification-tags

This mode serializes ``MM``, ``ML``, and ``MN`` as SAM-compatible FASTQ
comment fields. The trio is atomic: a record containing only a subset fails
the export, while a record containing none remains an untagged FASTQ record.
All legal BAM integer widths for ``MN`` are accepted and serialized as the
canonical SAM ``MN:i:<value>`` field.
The JSON response reports ``records_with_modification_tags`` and
``payload_sha256``. The latter hashes the complete decompressed FASTQ payload
in emitted order during export, avoiding a second output read. Downstream
aligners must explicitly retain SAM-style FASTQ comments (for example,
minimap2 ``-y``); the export alone does not prove restoration into the
remapped BAM. BAM headers, alignment state, and all other auxiliary tags are
not represented.

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

filter
------

``bamana filter`` streams one BGZF BAM into one filtered BGZF BAM:

.. code-block:: bash

   bamana filter --bam input.bam --out filtered.bam --min-length 1000 --max-length 50000 --min-mean-quality 10 --min-complexity 0.55 --mapped-only --primary-only --threads 8

``--threads`` controls bounded ordered BGZF output compression. Its
producer/compressor pipeline keeps at most two full blocks in flight per
worker and writes completed blocks in input sequence. Filtering and
retained-record encounter order remain deterministic, and the JSON response
records the effective worker count as ``execution.compression_threads``.

Predicates are inclusive and combine with logical AND. The current public
surface supports read-length bounds, mean non-missing BAM base-quality bounds,
canonical linguistic-complexity bounds, mapped/unmapped selection, and primary
alignment selection. Retained records preserve raw BAM record bytes and input
encounter order.

Linguistic complexity uses the EMBOSS-RS ``complex`` formula over canonical
A/C/G/T k-mers and does not emit plots. Non-canonical bases are dropped by
default when complexity filtering is active, or can fail the command with
``--complexity-noncanonical fail``. ``filter`` does not create an output index;
use ``bamana index --input <filtered.bam>`` when an index is required.

select_region
-------------

``bamana select_region`` is governed for BGZF BAM file output from CLI
``--region`` requests. The M11.6 implementation supports:

.. code-block:: bash

   bamana select_region --bam input.bam --region chr1:1-1000 --out selected.bam

Usable non-stale BAI sidecars drive native indexed traversal; otherwise the
command falls back to native scanner selection. Selected alignment records
preserve raw BAM record bytes, emit once per physical source record, and remain
in source virtual-offset order. Binary stdout output, public ``--region-file``,
and governed schemas/examples/fixtures remain later M11 work.

M11.7 adds the current file-output safety rule: same-path input/output rewrites
are rejected, existing output BAM paths and adjacent output BAI/CSI sidecars
are collisions unless ``--force`` is supplied, forced applied runs remove stale
adjacent output index sidecars, and no replacement output index is created.
Use ``bamana index --input <selected.bam>`` when an index is required.

M11.8 adds governed schema and example artifacts for the current file-output
surface: ``spec/jsonschema/select_region.schema.json``,
``spec/examples/select_region.success.json``, and
``spec/examples/select_region.failure.json``. Fixture plans reserve indexed
success, scan fallback, duplicate/overlap suppression, output-index sidecar
collision, forced sidecar removal, and same-path rejection coverage. Binary
stdout output and public ``--region-file`` remain deferred.

M11.10 closes Milestone 11. Milestone 11 is complete as of 2026-05-28 with
full tests, contract tests, Sphinx HTML documentation, formatting, whitespace
checks, binary builds, and
``scanner_microbench --profile small --iterations 1 --bamana-bin`` smoke
evidence. ``select_region_scan_fallback`` and
``select_region_indexed_output`` both reported ``1/1`` successful in the
closeout smoke profile. Public ``--region-file``, binary stdout output via
``--out -``, replacement output-index creation, CSI large-reference behavior,
native CRAM indexed queries, and broad comparator parity remain deferred.
