Native CRAM Strategy And Compatibility Boundary
===============================================

Milestone 13 is active as of 2026-06-01. It follows the completed Milestone 12
extended index compatibility milestone and does not reopen BAM, BGZF, FASTQ,
or index contracts.

Activation Baseline
-------------------

M13.1 activates the native CRAM strategy milestone without changing CLI
behavior. The current baseline is deliberately conservative:

* CRAM support is compatibility-oriented and concentrated in
  ``src/ingest/cram.rs``.
* Direct production ``noodles_*`` imports are allowed only in that documented
  CRAM compatibility boundary. BAM, BGZF, FASTQ, sampling, ingest planning,
  indexing, and forensic hot paths remain Bamana-native.
* The default feature set includes ``cram-compat``, which keeps
  ``noodles-cram``, ``noodles-bam``, ``noodles-fasta``, and ``noodles-sam``
  available for the transitional compatibility layer.
* ``consume`` is the only current public CRAM-facing command path. CRAM is
  accepted only in alignment mode and is normalized to BAM before downstream
  Bamana-native handling.
* The default CRAM reference policy is ``strict``. Under ``strict``, CRAM
  ingestion requires ``--reference <fasta>`` and an adjacent ``.fai``.
* An explicit reference FASTA takes precedence over ``--reference-cache`` in
  the current slice.
* ``allow-cache`` and cache-backed ``--reference-cache`` decoding are planned
  but unimplemented.
* ``allow-embedded`` and ``auto-conservative`` may attempt decode without
  external reference material. Dry runs validate only policy shape and cannot
  prove decode success; real decode failures that require reference material
  are reported as ``reference_required``.
* CRAM normalization currently decodes through the compatibility reader, writes
  a temporary BAM, and then re-enters the Bamana-native BAM reader and record
  layout path.
* CRAI handling, indexed CRAM queries, native CRAM parsing, native CRAM
  writing, cache-backed decoding, and broad comparator parity remain deferred.
* The CRAM fixture plan is partially reserved: source SAM and explicit FASTA
  provenance are present, while derived CRAM/BAM binaries and no-external-ref
  fixtures remain planned or deferred until reproducible generation is
  documented.

M13.1 is an audit and activation task. Later M13 tasks must decide whether CRAM
remains compatibility-only, moves behind a narrower optional compatibility
feature, or receives a staged Bamana-native implementation.

Non-Goals
---------

M13.1 does not implement native CRAM parsing, native CRAM writing, CRAI
handling, indexed CRAM queries, cache-backed decoding, or comparator parity. It
also does not broaden the documented production ``noodles`` exception beyond
CRAM ingestion and reference-policy compatibility.
