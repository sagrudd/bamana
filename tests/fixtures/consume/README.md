# Consume Fixture Family

This directory is reserved for fixture planning related to `bamana consume`.

The initial consume fixture family is intentionally small and should support:

* alignment-mode ingest from BAM, SAM, and staged CRAM
* unmapped-mode ingest from FASTQ and FASTQ.GZ
* mixed-format rejection across alignment-bearing and raw-read inputs
* explicit CRAM reference-policy success and required-reference failure cases
* deterministic directory traversal with supported, unsupported, and nested
  entries

The CRAM companion set should remain even smaller:

* one explicit-reference success fixture
* one strict missing-reference failure scenario
* one compatible/incompatible reference-dictionary group for mixed
  alignment-bearing consume

If a no-external-reference CRAM is later added, it should be treated as a
separate deferred contract, not assumed by default.
