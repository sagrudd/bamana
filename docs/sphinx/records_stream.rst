Bounded alignment-record streams
================================

``bamana records`` normally returns its retained records and rejections in one
JSON response. For chromosome and genome-scale adapters, add
``--stream-out <PATH>`` to keep the response bounded and write one record at a
time as newline-delimited JSON.

.. code-block:: bash

   bamana records \
     --bam normalized.bam \
     --region chr20 \
     --input-object-id aaid:v1:object:input-bam \
     --reference-assembly CHM13v2.0 \
     --reference-object-id aaid:v1:object:reference-fasta \
     --reference-sha256 <64-lowercase-hex> \
     --reference-fai-sha256 <64-lowercase-hex> \
     --bamana-git-commit <commit> \
     --stream-out records.ndjson

Each line is independently parseable and has either
``{"kind":"record","data":...}`` or
``{"kind":"rejection","data":...}``. The normal command response contains
the exact requested, retained, and rejected totals; stable reference and tool
identities; rejection counts; and stream format and ordering metadata. The
``stream.path`` value is the portable output basename, not a transient host or
container path.

Safety and ordering
-------------------

The stream is written to a sibling temporary file and renamed only after a
successful traversal. Existing outputs are rejected unless ``--force`` is
supplied. A failed run removes its temporary output.

Region streams require a usable, current BAI. They do not silently scan the
whole BAM. Coalesced index chunks are visited incrementally, records are
deduplicated by physical virtual offset, and output follows source virtual
offset order. Memory therefore scales with index planning and duplicate-offset
state rather than sequence, quality, and auxiliary-tag payload volume.

The stream proves native decoding and the recorded filter operation. It does
not establish biological correctness, reference concordance beyond the
supplied identities, or downstream marker assignment. Consumers should hash
the completed NDJSON file and bind that digest into their derived provenance.
