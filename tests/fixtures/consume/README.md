# Consume Fixture Family

This directory is reserved for fixture planning related to `bamana consume`.

The initial consume fixture family is intentionally small and should support:

* alignment-mode ingest from BAM, SAM, and staged CRAM
* unmapped-mode ingest from FASTQ and FASTQ.GZ
* mixed-format rejection across alignment-bearing and raw-read inputs
* explicit CRAM reference-policy success and required-reference failure cases
* deterministic directory traversal with supported, unsupported, and nested
  entries
