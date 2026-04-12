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

Current scope also includes `bamana reheader` as the governed header-only BAM
metadata mutation path for workflows that need explicit `@RG`, `@PG`, `@CO`,
and related header updates without implying record-level tag mutation.
Current scope further includes `bamana annotate_rg` as the governed
record-level read-group annotation path for workflows that require explicit
`RG:Z:` aux tags on BAM alignment records, optionally coordinated with `@RG`
header lines.
Current scope also includes `bamana inspect_duplication` as the governed
collection-duplication inspection path for BAM, FASTQ, and FASTQ.GZ inputs when
operators need explicit evidence of unsafe concatenation, repeated appends, or
provenance mishandling without conflating those signatures with ordinary PCR
duplicate biology.
Current scope also includes `bamana deduplicate` as the governed conservative
remediation path for suspicious contiguous collection-duplication signatures
when operators need a dry-run-first, auditable way to remove repeated blocks
without conflating that action with molecular duplicate marking.
Current scope also includes `bamana forensic_inspect` as the governed
provenance-inspection path for BAM collections when operators need explicit
hallmark reporting for concatenation, coercion, weak provenance discipline, or
metadata/body mismatches that are operationally suspicious even when the file
still parses.
Current roadmap scope now also reserves `bamana subsample` as a planned
benchmark-driven command contract for deterministic or seeded-random
subsampling of BAM and FASTQ.GZ inputs. This requirement exists so the
benchmark framework can compare Bamana fairly against `samtools`, `seqtk`,
`rasusa`, and related comparators without inventing an implicit command shape.
The repository also now includes a reproducible benchmark framework under
`benchmarks/`, with containerized Nextflow execution, R-based aggregation, and
explicit comparator treatment for `samtools` as the canonical BAM baseline and
`fastcat` as the ingestion-oriented ONT comparator.

The project charter remains explicit that:

* CRAM support must not silently guess reference behavior
* ingestion semantics are conservative and automation-facing
* header-only mutation must remain distinct from record-level alignment-tag
  mutation
* record-level read-group annotation must remain distinct from header-only
  metadata mutation
* collection-duplication inspection must remain distinct from PCR duplicate
  marking and duplicate-flag interpretation
* collection-duplication remediation must remain distinct from PCR duplicate
  marking, duplicate-flag interpretation, and aggressive global duplicate
  collapse unless a future contract says otherwise explicitly
* provenance inspection must remain distinct from both structural validation
  and fraud accusation; it reports evidence-driven anomalies and recommended
  follow-up only
* benchmarking must remain explicit about unsupported, partial, or
  roadmap-blocked comparator paths rather than presenting them as simple speed
  outcomes
* adjacent format support is intended to normalize into BAM rather than widen
  the public data model without discipline
