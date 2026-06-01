Interoperability And Benchmark Evidence
=======================================

Milestone 14 is active as of 2026-06-01. M14.1 activates the interoperability
and benchmark evidence milestone after Milestone 13 closeout. This activation
is an audit-only baseline: it does not add benchmark profiles, comparator
claims, schemas, or command behavior.

Public Benchmark Profiles
-------------------------

The public ``benchmark`` command currently exposes two governed profiles:

* ``fastq_ingress``: a containerized FASTQ.GZ-to-unmapped-BAM benchmark
  comparing Bamana against a fastcat-plus-samtools path, rendered through
  ``benchmarks/bin/run_fastq_ingress_benchmark.sh``.
* ``fastq_gz_enumerate``: a containerized FASTQ.GZ record-count benchmark
  comparing Bamana-native enumeration against gzip decompression plus line
  counting, rendered through
  ``benchmarks/bin/run_fastq_gz_enumerate_benchmark.sh``.

Repository-Local Smoke Hooks
----------------------------

M14.1 records the existing smoke hooks as regression guardrails:

* ``bgzf_microbench`` covers native BGZF throughput and optional ``check_eof``
  command timing.
* ``header_microbench`` covers native BAM header parsing and serialization,
  plus optional ``verify``, ``header``, ``reheader``, and ``annotate_rg``
  command timings.
* ``scanner_microbench`` covers native BAM scanner throughput and governed
  command-timing rows for inspection, transform/ingest, index, indexed-region,
  selected-region, CSI fallback, mutation, remediation, and forensics smoke
  paths.
* ``fastq_microbench`` covers native FASTQ and FASTQ.GZ parser and writer
  throughput, plus optional ``subsample_fastq`` and ``subsample_fastq_gz``
  command timings.

Framework Inventory
-------------------

The broader benchmark framework inventory includes:

* ``benchmarks/main.nf``;
* ``benchmarks/params.schema.json``;
* ``benchmarks/inputs/manifest.schema.json``;
* ``benchmarks/results/result.schema.json``;
* ``benchmarks/results/benchmark_row.schema.json``;
* the microbenchmark result schemas under ``benchmarks/results/``;
* ``benchmarks/tools/tool_registry.example.json``;
* ``benchmarks/tools/workflow_variant_matrix.md``.

The primary wrapper tools are Bamana, samtools, and fastcat. Sambamba, seqtk,
and rasusa are represented for explicit support or unsupported handling.

Evidence Boundary
-----------------

Existing smoke timings are regression guardrails. Existing comparator rows are
profile- and scenario-specific. They are not broad comparator parity,
biological equivalence, release performance promises, CRAM comparator claims,
or external-tool authority.

M14.2 Command Evidence Matrix
-----------------------------

M14.2 defines which public commands have comparator evidence, smoke evidence,
or no external comparator claim. The governed source is
``benchmarks/command_evidence_matrix.md``.

The current evidence levels are:

* ``public_profile_comparator`` for ``enumerate`` on FASTQ.GZ through
  ``fastq_gz_enumerate`` and for ``consume`` on FASTQ.GZ unmapped ingest
  through ``fastq_ingress``.
* ``local_smoke`` for commands covered by ``bgzf_microbench``,
  ``header_microbench``, ``scanner_microbench``, or ``fastq_microbench``.
* ``scenario_matrix_comparator_scaffold`` for workflow-matrix rows such as
  BAM ``subsample``/``sort`` and FASTQ.GZ ``subsample`` where comparator tools
  are represented but not promoted to release-facing comparator evidence.
* ``no_external_comparator_claim`` for ``identify``, ``fastq``, ``unmap``,
  unmeasured command modes, CRAM consume behavior, and any workflow-matrix row
  that lacks command-specific fixtures, semantic assumptions, and schema
  coverage.

The public contract commands ``benchmark``, ``fastq``, and ``unmap`` remain
explicitly governed: ``benchmark`` owns profile execution and reporting,
``fastq`` currently has no benchmark hook or comparator claim, and ``unmap``
currently has no benchmark hook or comparator claim.
