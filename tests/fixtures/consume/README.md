# Consume Fixture Family

This directory is reserved for fixture planning related to `bamana consume`.

The initial consume fixture family is intentionally small and should support:

* alignment-mode ingest from BAM and SAM
* unmapped-mode ingest from FASTQ and FASTQ.GZ
* mixed-format rejection across alignment-bearing and raw-read inputs
* deterministic directory traversal with supported, unsupported, and nested
  entries
