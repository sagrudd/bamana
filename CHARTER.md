# Bamana Charter

The repository charter is maintained in
[docs/project-charter.md](/Users/stephen/Projects/bamana/docs/project-charter.md).

Current scope explicitly includes `bamana consume` as the governed ingestion
gateway for normalizing supported upstream formats into BAM. The current staged
ingestion surface covers:

* BAM
* SAM
* CRAM in alignment mode under explicit reference-policy handling
* FASTQ and FASTQ.GZ in unmapped mode

The project charter remains explicit that:

* CRAM support must not silently guess reference behavior
* ingestion semantics are conservative and automation-facing
* adjacent format support is intended to normalize into BAM rather than widen
  the public data model without discipline
