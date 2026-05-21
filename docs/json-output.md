# Bamana JSON Output Contracts

The `spec/jsonschema/` directory contains the machine-readable contract layer
for Bamana command outputs.

## Envelope Model

Every command response follows the same top-level pattern:

* `ok`
* `command`
* `path`
* `analysis_wall_seconds`
* `data`
* `error`

This repository treats those fields as governed public interface.

`analysis_wall_seconds` reports Bamana's in-process wall time for the command's
analysis and execution path up to JSON emission. It excludes any outer wrapper
or benchmark harness timing.

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

## `header`

The `header` payload exposes the BAM header parsed through the native BGZF
stream reader and native BAM header codec without implying alignment-record
validation.

Key concepts:

* `raw_header_text` preserves the SAM-style text declared in the BAM header
* `references` is the binary reference dictionary and is authoritative for BAM
  decoding
* `reference_diagnostics` reports non-fatal textual `@SQ` mismatches such as
  missing, extra, duplicate, reordered, name-mismatched, or length-mismatched
  records
* `read_groups`, `programs`, `comments`, and `other_header_records` preserve
  parsed textual metadata for downstream inspection

## `verify`

The `verify` payload reports header-level BAM verification, not full BAM body
validation.

Key concepts:

* `shallow_verified` remains true only when BGZF container recognition, BAM
  magic, and native BAM header/reference-dictionary parsing succeeded
* `deep_validated` remains false because alignment records are not scanned
* `checks_performed` lists the successful header-level checks
* EOF-marker status is intentionally outside `verify`; use `check_eof` for
  that contract

## `check_eof`

The `check_eof` payload reports only canonical BGZF EOF-marker evidence.

Key concepts:

* `bgzf_eof_present` reports whether the canonical EOF marker was found
* `complete` mirrors tail-completeness evidence, not BAM semantic validity
* `semantic_note` states that EOF presence does not imply full BAM validity
* a successful payload does not prove that BAM magic, the BAM header,
  alignment records, or optional fields are readable

## `check_sort`

The `check_sort` payload compares declared sort metadata with observed
record-order evidence.

Key concepts:

* `declared_sort` preserves `SO`, `SS`, and `GO` values from the header
* `observed_sort.records_examined` bounds the evidence scope
* `observed_sort.first_violation` records the first detected ordering problem
* `confidence` and `semantic_note` keep bounded evidence separate from full
  validation

## `check_map`

The `check_map` payload reports whether mapping evidence came from an index or
from record scanning.

Key concepts:

* `evidence_source` distinguishes `index` from `scan`
* `index.used` reports whether the adjacent index actually supplied the result
* `summary.records_examined` is scan evidence and is absent from pure
  index-derived summaries
* `semantic_note` describes the limits of the reported evidence source

## `summary`

The `summary` payload is an operational overview, not a validation certificate.

Key concepts:

* `mode` distinguishes `bounded_scan`, `full_scan`, and `indeterminate`
* `evidence.full_file_scanned` controls whether full-file claims are supported
* `index_derived` keeps BAI-derived totals separate from scan-derived counts
* `fractions_observed` is scoped to examined records when the scan is bounded

## `check_tag`

The `check_tag` payload reports selected auxiliary-tag evidence.

Key concepts:

* `mode` distinguishes bounded and full-scan tag checks
* `result` distinguishes observed presence, bounded non-observation,
  full-scan absence, and indeterminate traversal
* `records_examined` and `full_file_scanned` define the evidence scope
* bounded non-observation must not be interpreted as full-file absence

## `validate`

The `validate` payload reports structural and internal-consistency findings.

Key concepts:

* `mode` distinguishes header-only, bounded-record, and full validation scopes
* `summary.full_file_examined` states whether the requested validation reached
  EOF cleanly
* `findings` report structured `error`, `warning`, and `info` evidence
* `semantic_note` states that validation does not imply biological correctness
  or external reference concordance

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

## `subsample`

The `subsample` payload is benchmark-friendly, explicit about reproducibility,
and suitable for large BAM, FASTQ, and FASTQ.GZ inputs.

Key concepts:

* explicit selection mode (`random` or `deterministic`)
* explicit requested fraction and the actual seed used for random mode
* explicit deterministic identity basis (`qname`, `qname_seq`, or
  `full_record`) when hash-based selection is used
* exact execution counts for records examined, eligible records, retained
  records, and the observed retained fraction
* explicit order-preservation reporting
* explicit BAM filter policy when `mapped_only` or `primary_only` is active
* explicit BAM index invalidation and deferred reindex reporting

This contract is intentionally explicit that seeded random mode is suitable for
repeatable benchmarks and that deterministic mode is suitable for stable
exact-repeatability checks across repeated runs of the same build and
configuration.

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

## `forensic_inspect`

The `forensic_inspect` payload is provenance-oriented, anomaly-focused, and
explicit about evidence scope.

Key concepts:

* explicit inspected scopes for header, read groups, program chain, read names,
  tags, and duplication hallmarks
* explicit scan mode (`bounded` or `full`) for body-oriented evidence
* stable finding taxonomy for provenance anomalies such as
  `program_chain_anomaly`, `read_group_inconsistency`, and
  `concatenation_hallmark`
* severity, confidence, evidence strength, and evidence-scope fields kept
  distinct from one another
* conservative overall assessment that can report `null` for likely
  concatenation/coercion when bounded evidence is insufficient
* follow-up recommendations that point automation toward other Bamana commands
  rather than taking action automatically

This contract is intentionally not a structural-validation contract and not a
fraud-detection contract. It reports evidence-driven provenance anomalies and
collection-hygiene hallmarks only.
