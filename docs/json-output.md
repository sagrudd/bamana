# Bamana JSON Output Contracts

The `spec/jsonschema/` directory contains the machine-readable contract layer
for Bamana command outputs.

## Envelope Model

Every command response follows the same top-level pattern:

* `ok`
* `command`
* `path`
* `data`
* `error`

This repository treats those fields as governed public interface.

## Schema Use

Each command has:

* a command-specific schema file
* canonical success and failure examples under `spec/examples/`
* shared common definitions under `spec/jsonschema/common/`

Consumers should treat the schema and canonical examples together as the output
contract.

## Stability Rules

Breaking output changes require:

* schema update
* example update
* contract-test update
* release-note disclosure

See:

* [spec/contracts/versioning.md](/Users/stephen/Projects/bamana/spec/contracts/versioning.md)
* [spec/contracts/compatibility.md](/Users/stephen/Projects/bamana/spec/contracts/compatibility.md)

## `consume`

The `consume` payload introduces an ingestion-oriented contract layer in
addition to Bamana’s inspection and transformation outputs.

Key concepts:

* requested paths versus discovered files
* deterministic directory traversal reporting
* consumed, skipped, and rejected file lists
* explicit ingest mode (`alignment`, `unmapped`)
* explicit CRAM reference policy and reference-resolution reporting
* output sort/index/checksum intent
* notes that separate implemented behavior from deferred options

The contract is designed so automation can reason about dry-run discovery
results, mixed-format rejection, CRAM reference decisions, and staged
normalization behavior without needing to infer semantics from ad hoc log text.

## `annotate_rg`

The `annotate_rg` payload is the record-level companion to `reheader`.

Key concepts:

* the requested `rg_id`
* explicit record mutation mode (`only_missing`, `replace_existing`,
  `fail_on_conflict`)
* explicit header policy (`require_existing`, `create_if_missing`,
  `add_header_rg`, `set_header_rg`)
* record-summary counts for missing, already matching, and conflicting RG tags
  observed before mutation
* explicit checksum reporting with `RG` excluded when the command demonstrates
  that only read-group tagging changed within the checksum domain

This command is intentionally more expensive than `reheader` because it
rewrites alignment records, not just the BAM header.

## `reheader`

The `reheader` payload captures both planning and execution because safe
header-only mutation depends on whether Bamana can prove that a true in-place
patch is safe.

Key concepts:

* requested header mutation operations
* planning output that reports `mode_requested`, `in_place_feasible`,
  `recommended_mode`, and the planning reason
* execution output that distinguishes dry-run planning from a written BAM
* output/index/checksum reporting for rewrite-based execution
* notes that explicitly state `reheader` does not modify per-record `RG:Z`
  tags

The current slice reports true in-place feasibility conservatively and uses a
rewrite path for actual execution. Checksum verification, when requested, is
reported over alignment-record content with header bytes excluded so the JSON
can demonstrate header-only semantics honestly.

## `inspect_duplication`

The `inspect_duplication` payload is evidence-driven and operator-error
oriented.

Key concepts:

* explicit identity mode (`qname_seq`, `qname_seq_qual`, or BAM-only
  `qname_seq_qual_rg`)
* explicit scan mode (`bounded` or `full`)
* exact duplicate-identity summary metrics
* stable duplication taxonomy for findings such as
  `exact_record_duplicate`, `contiguous_block_duplicate`, and
  `whole_file_append_duplicate`
* confidence and evidence-strength fields that stay separate from finding type
* an assessment that distinguishes duplication detection from the stronger claim
  that operator error is likely

This contract is intentionally not a PCR duplicate-marking contract. It is
designed for collection inspection, provenance review, and future remediation
work such as controlled deduplication workflows.

## `deduplicate`

The `deduplicate` payload is remediation-oriented, dry-run-first, and explicitly
conservative.

Key concepts:

* explicit remediation mode (`contiguous-block` or `whole-file-append` in the
  current practical slice)
* explicit identity mode aligned with `inspect_duplication`
* explicit keep policy (`first` or `last`)
* execution reporting that separates dry-run planning from applied output
  writing
* deterministic 1-based keep/remove record ranges in encounter order
* output, index-invalidation, and optional checksum-provenance fields
* notes that keep collection-duplication remediation distinct from PCR
  duplicate marking

This contract is intentionally not a molecular duplicate-marking contract. It
describes removal of suspicious collection-duplication blocks under an explicit
policy, not broad biological duplicate collapse.
