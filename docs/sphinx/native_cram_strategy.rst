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

CRAM Direction Decision
-----------------------

M13.2 chooses compatibility-only continuation for Milestone 13. Bamana does not
promote a native CRAM substrate in this milestone, does not remove CRAM
ingestion from the native-core package, and does not broaden CRAM support into
general-purpose indexed CRAM querying.

The chosen direction is:

* keep CRAM support limited to the existing ``consume`` alignment-mode
  normalization path;
* keep direct production ``noodles_*`` imports confined to
  ``src/ingest/cram.rs``;
* keep BAM, BGZF, FASTQ, sampling, indexing, and forensic hot paths
  Bamana-native;
* keep ``cram-compat`` as the explicit transitional compatibility feature
  while later M13 tasks decide whether it should become non-default, narrower,
  or replaced;
* require any future native CRAM work to arrive as a separate staged
  implementation plan with fixtures, contracts, dependency guardrails, and
  benchmark evidence;
* treat CRAI/indexed CRAM queries, native CRAM parsing, native CRAM writing,
  cache-backed decoding, and broad comparator parity as deferred unless a
  later M13 task explicitly changes that contract.

This decision lets M13.3-M13.9 focus on freezing reference/cache semantics,
indexed-query position, fixtures, public contracts, and dependency guardrails
around the compatibility boundary that actually exists.

Reference And Cache Policy Freeze
---------------------------------

M13.3 freezes the CRAM reference and cache semantics for the compatibility-only
Milestone 13 direction:

* ``strict`` is the default and safest policy. Without ``--reference <fasta>``,
  it fails before decode with ``reference_required``.
* ``--reference <fasta>`` must name a readable FASTA with an adjacent ``.fai``.
  Missing FASTA or missing ``.fai`` fails as ``reference_not_found``.
* An explicit FASTA always takes precedence over ``--reference-cache``,
  regardless of the selected policy, and reports ``source_used:
  explicit_fasta`` plus ``decode_without_external_reference: false`` when CRAM
  decoding succeeds.
* ``allow-embedded`` permits only a no-external-reference decode attempt. Dry
  runs validate policy shape, leave ``source_used`` and
  ``decode_without_external_reference`` unknown, and do not prove decode
  success. Real decode failures that require reference material return
  ``reference_required``.
* ``auto-conservative`` uses an explicit FASTA when one is supplied. Without
  explicit FASTA or cache, it behaves like the conservative
  no-external-reference attempt above. With ``--reference-cache``, it returns
  ``unimplemented`` because cache-backed decoding is not implemented.
* ``allow-cache`` is reserved for cache-backed CRAM decoding and returns
  ``unimplemented`` in this slice, whether or not the cache path exists.
* ``--reference-cache`` is a recorded request field only until cache-backed
  decoding is implemented; it is not searched, populated, or used for fallback.

These rules are governed by the ``consume.reference`` JSON object:
``policy``, ``explicit_reference_provided``, ``reference_cache_provided``,
``cram_inputs_present``, optional ``source_used``, and optional
``decode_without_external_reference``.

CRAM Indexed Query Position
---------------------------

M13.4 freezes CRAI and indexed CRAM queries as unsupported/deferred for
Milestone 13. They are not transitional behavior and not native work in this
milestone.

The contract is deliberately narrow:

* CRAI files and adjacent ``.crai`` sidecars are not discovered, parsed,
  planned, or used by public commands in M13.
* ``consume`` remains a sequential CRAM normalization path governed by
  ``consume.reference``; it does not use CRAI and performs no random-access
  traversal.
* ``check_map --region``, ``summary --region``, and ``select_region`` do not
  accept CRAM region input and remain scoped to the existing BAM/BAI native
  random-access and scan-fallback contracts.
* ``index`` does not create CRAI. ``index --format csi`` and BAM index
  behavior remain separate from CRAM indexing.
* JSON outputs expose no JSON CRAI evidence and no CRAM indexed-query fields
  in this slice.
* Any future milestone that promotes indexed CRAM queries must introduce an
  explicit staged plan for fixtures, native or compatibility dependency
  boundaries, CLI contracts, schemas, examples, benchmarks, and regression
  tests before behavior changes.

CRAM Fixture And Oracle Boundary
--------------------------------

M13.5 freezes the CRAM fixture set as a provenance-first plan rather than a
broad binary corpus. The fixture boundary follows the M13.2 compatibility-only
direction, the M13.3 reference/cache policy, and the M13.4 indexed-query
deferral.

Fixture status is:

* present provenance roots: ``tiny.valid.cram.explicit_ref.source_sam`` and
  ``tiny.ref.primary``;
* planned derived alignment fixtures:
  ``tiny.valid.cram.explicit_ref.source_bam``,
  ``tiny.valid.cram.explicit_ref``,
  ``tiny.valid.cram.reference_required``,
  ``tiny.valid.cram.compatible_refdict``,
  ``tiny.valid.bam.compatible_refdict``, and
  ``tiny.valid.bam.incompatible_refdict``;
* deferred no-external-reference fixture:
  ``tiny.valid.cram.no_external_ref``;
* no CRAI fixture in M13.5. ``.crai`` artifacts, indexed CRAM fixtures, and
  CRAM random-access oracle outputs are explicitly deferred.

The source SAM and FASTA remain the auditable source of truth. Derived BAM and
CRAM files must be regenerated from those sources with documented commands and
reviewed as derived artifacts, not treated as opaque authorities.

Oracle use is limited to fixture generation, fixture validation, and test-only
compatibility checks. ``noodles`` or external tools may help produce or compare
derived CRAM artifacts only under that explicit test/fixture boundary; they
must not define production behavior, reference-cache semantics, CRAI behavior,
indexed CRAM traversal, or native BAM/BGZF/FASTQ hot paths.

Implemented CRAM Boundary
-------------------------

M13.6 implements only the chosen compatibility behavior and adds a concrete
guard against accidental indexed-CRAM expansion:

* CRAM remains accepted only as consume-only alignment-mode normalization.
* Strict, explicit-FASTA, allow-embedded, allow-cache, and auto-conservative
  reference behavior remains the M13.3 behavior.
* Cache-backed CRAM decoding remains ``unimplemented``.
* Directory discovery skips adjacent ``.crai`` sidecars before probing with
  reason ``cram_index_sidecar_deferred``.
* A directly requested ``.crai`` path is rejected before probing as
  ``unsupported_format``.
* CRAI bytes are not parsed, planned, or used for traversal, and no native CRAM
  parser or writer is introduced.
* Direct production ``noodles_*`` imports remain confined to
  ``src/ingest/cram.rs``.

CRAM-Facing Contract Refresh
----------------------------

M13.7 updates the governed command contracts for the M13.6 behavior without
expanding CRAM support:

* ``consume`` is the only command with changed CRAM-facing behavior in this
  slice.
* ``spec/jsonschema/consume.schema.json`` documents
  ``cram_index_sidecar_deferred`` as the directory-discovery skipped-file
  reason for ``.crai`` sidecars and keeps direct ``.crai`` requests represented
  as ``unsupported_format`` failures before discovery payload population.
* ``spec/examples/consume.success.crai_sidecar_skipped.json`` is the canonical
  directory-discovery example for a skipped ``.crai`` sidecar beside a CRAM
  input.
* ``spec/examples/consume.failure.crai_sidecar_direct.json`` is the canonical
  direct-request failure example for a ``.crai`` input.
* Inspection commands and region commands do not gain CRAM behavior in M13.7:
  ``check_map --region``, ``summary --region``, ``select_region``,
  ``check_index``, and ``index`` keep their existing BAM/BAI/CSI contracts and
  expose no CRAM indexed-query fields.

Public Docs And Schema Inventory
--------------------------------

M13.8 consolidates the public documentation, schema, and example inventory for
the M13.6-M13.7 boundary. It is a documentation/schema refresh only and does
not add runtime CRAM behavior.

The governed artifacts are:

* ``spec/jsonschema/consume.schema.json``, including M13 CRAM contract metadata
  for consume-only compatibility normalization, ``cram_index_sidecar_deferred``,
  direct ``unsupported_format`` rejection, and no CRAM indexed-query fields.
* ``spec/examples/consume.success.crai_sidecar_skipped.json`` for the
  directory-discovery skipped-sidecar shape.
* ``spec/examples/consume.failure.crai_sidecar_direct.json`` for the direct
  ``.crai`` rejection shape.
* ``spec/cli/commands.md``, ``README.md``, ``docs/cli.md``,
  ``docs/json-output.md``, roadmap notes, the current-milestone note, these
  Sphinx notes, and the task map.

Inspection, region, indexing, and benchmark contracts remain unchanged for
CRAM in M13.8. There is no CRAI parser, no CRAM region input, no CRAI creation,
no CRAM random-access evidence, no benchmark CRAM indexed-query evidence, and
no CRAM indexed-query JSON field.

Non-Goals
---------

M13.1 does not implement native CRAM parsing, native CRAM writing, CRAI
handling, indexed CRAM queries, cache-backed decoding, or comparator parity. It
also does not broaden the documented production ``noodles`` exception beyond
CRAM ingestion and reference-policy compatibility.
