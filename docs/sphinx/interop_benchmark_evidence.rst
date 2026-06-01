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

M14.3 Result Schema Extension
-----------------------------

M14.3 extends benchmark result schemas for post-M10 command families without
changing runtime benchmark behavior. Existing emitted result files remain
valid.

The raw result schema ``benchmarks/results/result.schema.json`` and tidy row
schema ``benchmarks/results/benchmark_row.schema.json`` now allow optional
``command_family``, ``evidence_level``, ``evidence_source``, and
``comparator_scope`` fields. These fields let future aggregation attach the
M14.2 evidence matrix directly to raw and tidy rows.

The scanner microbenchmark schema
``benchmarks/results/scanner_microbench.schema.json`` records the post-M10
command family taxonomy for ``indexed_region``, ``selected_region``,
``csi_fallback``, ``remediation``, ``forensics``, ``transform_ingest``,
``inspection``, and ``index`` command timing rows. The header microbenchmark
schema records the ``header``/``mutation`` split for ``verify``, ``header``,
``reheader``, and ``annotate_rg``. The FASTQ microbenchmark schema records the
``fastq`` family for FASTQ command timings. These annotations are schema
metadata and optional command-timing fields; they do not add benchmark
profiles, fixture generation, comparator claims, or command behavior.

M14.4 Fixture Provenance Metadata
---------------------------------

M14.4 defines benchmark input fixture provenance metadata without materializing
new fixtures or changing runtime benchmark behavior. The governed shape is the
``fixture_provenance`` object in
``benchmarks/inputs/manifest.schema.json`` and the human-readable contract is
``benchmarks/inputs/fixture_provenance.md``.

Each generated, derived, selected, or comparator fixture should record source
kind, source description, source URI, ``derived_from``, generation command,
generation environment, expected semantic scope, review boundary, checksum
policy, and reproducibility notes. This keeps future benchmark evidence tied to
auditable fixture origins before release-facing comparator claims are promoted.

M14.5 Aligned Comparator Profiles
---------------------------------

M14.5 records the aligned public comparator profile catalog in
``benchmarks/comparator_profiles.json``, governed by
``benchmarks/comparator_profiles.schema.json``. The measured public profiles
remain ``fastq_ingress`` and ``fastq_gz_enumerate``.

Each catalog entry records the benchmark runner, Bamana path, comparator path,
result artifacts, semantic equivalence assumptions, unsupported mismatch cases,
and release claim boundary. The public ``benchmark`` JSON payload also exposes
``semantic_equivalence_assumptions`` and ``unsupported_mismatch_cases`` so
automation can distinguish an aligned measured profile from broad comparator
parity.

M14.6 Comparator Mismatch Register
----------------------------------

M14.6 documents unsupported comparator cases in
``benchmarks/comparator_mismatches.md``. The register marks no-claim and
scaffolded mismatch surfaces as intentional benchmark boundaries rather than
missing implementation.

The register covers public-contract no-claim surfaces such as ``fastq`` and
``unmap``, CRAM consume behavior, mixed-directory ingest, scaffolded BAM and
FASTQ subsampling, ``rasusa`` downsampling, mapped sort/index pipelines,
``select_region``, mutation commands, and forensic commands. Each row records
the evidence status, semantic mismatch reason, current boundary, and promotion
requirement needed before a release-facing comparator claim can be made.

M14.7 Benchmark Schema Stability
--------------------------------

M14.7 adds a local benchmark schema stability harness:
``benchmarks/bin/check_schema_stability.py``. The checker reads
``benchmarks/schema_stability_manifest.json`` and requires every
``benchmarks/**/*.schema.json`` file to be listed with its path, ``$id``,
contract surface, version pointer, version value, and required stability
pointers.

The harness catches benchmark schema additions, removals, renames, unexpected
``$id`` changes, missing version metadata, and missing M14 result/profile
metadata before benchmark evidence can drift silently.
