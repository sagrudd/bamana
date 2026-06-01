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
* `--strict` expands the sequential inspection scope, but `check_sort` still
  does not perform full BAM structural validation

## `check_map`

The `check_map` payload reports whether mapping evidence came from an index or
from record scanning.

Key concepts:

* `evidence_source` distinguishes `index` from `scan`
* `index.used` reports whether the adjacent index actually supplied the result
* `summary.records_examined` is scan evidence and is absent from pure
  index-derived summaries
* index-derived summaries report BAI mapped/unmapped metadata only after the
  selected sidecar is not timestamp-stale, passes implemented structural
  checks, and supplies complete per-reference mapped/unmapped metadata
* generated BAI sidecars and discovered BAI sidecars are indistinguishable in
  payloads after validation; both are reported as index-derived evidence when
  used
* scan-derived summaries report only the records examined by the bounded or
  full scan path
* stale, unsupported, malformed, or incomplete sidecars leave `index.used:
  false`, switch `evidence_source` to `scan`, and describe the fallback in
  `semantic_note`
* `check_map --region <REGION>` records `region_scope` with the normalized
  M10.2 intervals, `input_1_based_closed_output_0_based_half_open` coordinate
  model, request-order duplicate policy, and whether execution used indexed
  traversal, scan fallback, or precise rejection
* indexed region execution reports `index_path`, `chunks_traversed`,
  `raw_records_seen`, and `duplicate_records_suppressed`
* scan fallback reports `fallback_mode: native_scan_required` and
  `scan_records_limit`
* adjacent CSI sidecars are detect-only in M12.6: `check_map --region
  <REGION>` preserves `index.kind: CSI`, reports
  `index.diagnostic_status: unsupported`, and uses native scan fallback rather
  than indexed traversal
* region summaries use `region_records_examined`,
  `region_mapped_records_observed`, and `region_unmapped_records_observed`
  rather than whole-file totals
* region-scoped mapping evidence must not be interpreted as whole-file mapping
  evidence

## `check_index`

The `check_index` payload reports adjacent BAM index discovery and the current
validation depth without claiming full random-access proof.

Key concepts:

* `index.present` and `candidates` describe adjacent sidecar discovery
* `index.kind` distinguishes BAI, CSI, GZI, unknown, or absent sidecars
* `support_level` freezes the current command contract: BAI is `read_write`,
  CSI is `detect_only`, FASTQ.GZI is `planning_sidecar`, unknown sidecars are
  `unsupported`, and absent sidecars are `absent`
* `syntactically_valid` reflects BAI structural checks: magic,
  reference-count agreement, bin uniqueness/range, chunk virtual-offset order,
  linear-index ordering, metadata pseudo-bin shape, and parseability; CSI is
  header detection plus reference-count agreement only until scoped support
  lands
* `stale` and `bam_newer_than_index` are timestamp heuristics, not semantic
  proof that offsets match the BAM
* `usable` is false for stale, malformed, unsupported, absent, GZI, unknown, or
  detected-but-not-supported CSI sidecars
* `usable: true` means the selected BAI passed implemented structural checks;
  it is not proof that every indexed random-access fetch has been exercised

## `index`

The `index` payload reports sidecar intent and whether an output was actually
created.

Key concepts:

* `format` distinguishes BAM from FASTQ.GZ input behavior
* `requested_index_kind` records BAI, CSI, or GZI intent
* `output_index.created` is true only when a sidecar was actually written
* BAM BAI writing creates native BAI sidecars for coordinate-sorted BAM input;
  references longer than 536,870,912 bases are rejected before output is
  written; CSI writing is still unimplemented and keeps `created = false`
* FASTQ.GZ indexing writes FASTQ.GZI sidecars with sampled, record-boundary
  checkpoint metadata for enumeration, explode planning, and consume planning
* `output_index.overwritten` reports whether `--force` replaced an existing
  sidecar

## Index Diagnostics

`check_map.index.diagnostic_status` distinguishes index states that previously
only appeared in prose fallback notes: `usable`, `absent`, `stale`,
`unsupported`, `malformed`, `mismatched_reference`, `incomplete`, `disabled`,
and `not_checked`. `diagnostic_detail` carries the parser or policy detail
when a selected sidecar is unusable.

## `summary`

The `summary` payload is an operational overview, not a validation certificate.

Key concepts:

* `mode` distinguishes `bounded_scan`, `full_scan`, and `indeterminate`
* `evidence.full_file_scanned` controls whether full-file claims are supported
* `counts.records_total_known` is present only when the scan reaches EOF
* `index_derived` keeps BAI-derived totals separate from scan-derived counts
  and is present only when the selected BAI is not timestamp-stale, passes
  implemented structural checks, and supplies complete mapped/unmapped metadata
* region-scoped `index_derived` is governed in M12.8: indexed traversal reports
  `present`, `kind`, `used`, and a note, but omits whole-file BAI mapped and
  unmapped totals because the index is used only to find records
* stale, unsupported, malformed, or incomplete sidecars are explained in
  `semantic_note`; the summary then uses native scan evidence without
  `index_derived`
* `fractions_observed` is scoped to examined records when the scan is bounded
* malformed-record failures can return an `indeterminate` payload because no
  stable operational summary was completed
* `summary --region <REGION>` records `region_scope` with the normalized
  M10.2 intervals, `input_1_based_closed_output_0_based_half_open` coordinate
  model, request-order duplicate policy, and whether execution used indexed
  traversal, scan fallback, or precise rejection
* indexed region execution reports `index_path`, `chunks_traversed`,
  `raw_records_seen`, and `duplicate_records_suppressed`
* scan fallback reports `fallback_mode: native_scan_required` and
  `scan_records_limit`
* adjacent CSI sidecars are detect-only in M12.6: `summary --region <REGION>`
  preserves `index_derived.kind: CSI`, reports a CSI unsupported fallback note,
  and uses native scan fallback rather than indexed traversal
* region-scoped `counts`, `fractions_observed`, `mapq`, `mapping`,
  `anomalies`, and optional `flag_categories` describe only requested
  intervals
* whole-file BAI totals are intentionally omitted from region-scoped
  `index_derived`; the index is used only to find records
* region-scoped operational metrics must not be interpreted as full-file totals
* M10.8 keeps indexed region selection deferred: region-aware JSON from
  `check_map --region <REGION>` and `summary --region <REGION>` is read-only
  evidence and does not claim output semantics, header preservation, record
  ordering, duplicate-region behavior, index invalidation, or write-safety for
  selecting or writing records

## `select_region`

The `select_region` payload reports governed selected-record BAM file output
for CLI `--region` requests.

Key concepts:

* `output.output_format` is `bam` and `output.compression` is `bgzf`
* `output.records_written` counts selected records actually written in applied
  runs; dry runs report zero written records and `output_created: false`
* `output.duplicate_emission_policy` is `emit_once_per_source_record`
* `output.output_ordering_policy` is `source_virtual_offset_order`
* `output.index_invalidation` lists adjacent output index candidates,
  pre-existing sidecars, removed sidecars, the invalidation action,
  `output_index_created: false`, and the regeneration command
* `region_scope` records CLI-region source, requested and normalized region
  counts, duplicate request count, and normalized region objects using the M10
  coordinate model
* `input_index` records the selected adjacent input sidecar context:
  `present`, `path`, `kind`, `used`, `compatibility`, and `fallback_reason`
* `input_index.compatibility` distinguishes `used_bai`, `detect_only_csi`,
  `unsupported_index`, `missing_index`, `stale_bai`, `malformed_bai`,
  `incomplete_bai`, and `disabled`
* adjacent CSI sidecars remain detect-only in M12.7: `select_region` preserves
  `input_index.kind: CSI`, reports
  `input_index.compatibility: detect_only_csi`, uses native scan fallback, and
  keeps output index invalidation unchanged
* `execution.mode` distinguishes `indexed` from `scan_fallback`; indexed
  execution reports `index_path` and chunk/record counts, while fallback
  execution reports `fallback_reason`
* `header` records reference dictionary preservation, retained reference count,
  provenance `@PG` insertion, and conservative sort metadata downgrade
* successful output preserves selected records' raw BAM record bytes and does
  not imply biological interpretation or whole-file validation

Binary stdout output via `--out -`, public `--region-file`, and replacement
output index creation remain deferred outside the current governed
`select_region` schema.

## `check_tag`

The `check_tag` payload reports selected auxiliary-tag evidence.

Key concepts:

* `mode` distinguishes bounded and full-scan tag checks
* `result` distinguishes observed presence, bounded non-observation,
  full-scan absence, and indeterminate traversal
* `records_examined` and `full_file_scanned` define the evidence scope
* `records_with_tag` counts records with at least one matching tag, not
  duplicate tag occurrences inside a single record
* `required_type` filters on the BAM auxiliary type code before reporting a
  match
* bounded non-observation must not be interpreted as full-file absence
* malformed or unsupported auxiliary shapes produce an indeterminate failure
  payload rather than a successful absence claim

## `validate`

The `validate` payload reports structural and internal-consistency findings.

Key concepts:

* `mode` distinguishes header-only, bounded-record, and full validation scopes
* `summary.full_file_examined` states whether the requested validation reached
  EOF cleanly
* `findings` report structured `error`, `warning`, and `info` evidence
* `summary.errors`, `summary.warnings`, and `summary.infos` count observed
  findings even when `--max-errors` or `--max-warnings` limits stored findings
* `scope=aux` means auxiliary-field traversal failed structurally; it does not
  validate arbitrary tag value semantics
* `semantic_note` states that validation does not imply biological correctness
  or external reference concordance

## `checksum`

The `checksum` payload reports explicit checksum domains instead of a single
ambiguous digest.

Key concepts:

* `algorithm` records the digest algorithm used for every result
* `results[].mode` identifies the checksum domain, such as raw record order,
  canonical record order, payload, header, or all requested domains
* `filters` records mapped/primary filters as part of the checksum definition
* `excluded_tags` records auxiliary tags omitted from record-content domains
* `order_sensitive` states whether record encounter order affects the digest
* `semantic_note` describes what the selected domain can and cannot compare

Canonical record-order mode is order-insensitive but still multiplicity-aware:
duplicate canonical records contribute duplicate per-record digests. It is not
a full BAM-validity proof, a biological equivalence proof, or a guarantee for
any checksum mode other than the one explicitly reported.

Raw record-order mode is an encounter-order stream domain. Header mode uses the
deterministic header text and binary reference dictionary serialization and does
not require an alignment-record scan. Payload mode uses the encounter-order
record payload stream and only includes the deterministic header serialization
when `header_included` is true.

## `sort`

The `sort` payload records a transformational BAM rewrite.

Key concepts:

* `sort.requested_order` and `sort.produced_order` identify the requested and
  emitted ordering contracts
* `sort.queryname_suborder` records the queryname comparator family when
  queryname ordering is requested
* `records.read` and `records.written` expose the transformation count surface
* `index` reports index intent and any deferred index behavior
* `checksum_verification` reports whether canonical checksum verification was
  requested, completed, and matched
* `notes` carry caveats such as in-memory first-slice execution

The payload does not imply full BAM validity, external comparator parity,
content preservation without a matching checksum verification result, or index
correctness when index writing is reported as deferred.
The written output path is published from a completed temporary file, and
existing targets require `--force`.

## `merge`

The `merge` payload records multi-BAM combination under explicit compatibility
and ordering semantics.

Key concepts:

* `inputs` records all requested BAM inputs in deterministic order
* `merge.requested_mode` and `merge.produced_mode` distinguish input-order,
  coordinate, and queryname merge behavior
* header compatibility failures are surfaced as structured failures rather than
  partial merge success
* `records.read` and `records.written` expose the merged count surface
* `checksum_verification` compares a canonical multiset checksum of the inputs
  against the output when requested
* `notes` carry caveats such as in-memory first-slice execution and index
  deferral

The payload does not imply that every input was fully valid beyond what was
parsed, that input-order output is suitable for coordinate indexing, or that
content was preserved unless checksum verification completed and matched.
The merged BAM path is published from a completed temporary file, and existing
targets require `--force`.

## `explode`

The `explode` payload records contiguous sharding of one input.

Key concepts:

* `input` records the detected input format and source path
* `explode.requested_parts` and range fields describe the requested shard
  count and emitted shard boundaries
* `outputs` records every shard path and the record range assigned to it
* `index` reports shard-planning metadata, including `FASTQ.GZI` use for
  FASTQ.GZ inputs
* `checksum_verification` records checksum intent and deferred behavior
* `notes` carry shard-size and planning caveats

The payload proves only the emitted contiguous shard plan and output list. It
does prove encounter-order preservation within each shard for BAM, SAM, and
FASTQ.GZ outputs. It does not claim global reconstruction equivalence beyond
the reported ranges, generic random-access parallel inflate for gzip, or
uniform shard sizes when `FASTQ.GZI` checkpoint-aligned boundaries are uneven.
Shard paths are finalized from completed temporary files after preflight; a
failure before finalization must not be treated as a partial success payload.

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
BAM alignment consume uses scanner-backed record loading, SAM/FASTQ/FASTQ.GZ
use their native ingest paths, and CRAM remains a reference-policy-governed
compatibility path. Checksum verification and post-ingest index creation remain
reported intent rather than performed work in this slice.
Non-dry-run consume output is published from a completed temporary BAM and
final rename. Dry-run mode is side-effect bounded and writes no BAM output.

M13.3 freezes the CRAM reference/cache policy fields under
`consume.reference`. `strict` requires an explicit indexed FASTA and otherwise
fails with `reference_required`; missing FASTA or missing `.fai` fails as
`reference_not_found`; explicit FASTA takes precedence over `--reference-cache`
and reports `source_used: explicit_fasta` with
`decode_without_external_reference: false` when CRAM decoding succeeds.
`allow-embedded` and cache-free `auto-conservative` permit only
no-external-reference decode attempts, while dry runs leave `source_used` and
`decode_without_external_reference` unknown. `allow-cache` and
`auto-conservative --reference-cache` return `unimplemented`, and
`--reference-cache` is recorded but not searched, populated, or used for
fallback in this slice.

M13.4 freezes CRAI and indexed CRAM queries as unsupported/deferred. `consume`
remains sequential CRAM normalization governed by `consume.reference`; it does
not use CRAI, does not inspect adjacent `.crai` sidecars, and performs no
random-access traversal. `check_map --region`, `summary --region`, and
`select_region` do not accept CRAM region input, `index` does not create CRAI,
and JSON outputs expose no JSON CRAI evidence or CRAM indexed-query fields
until a future milestone defines a staged promotion plan.

M13.6 adds the implemented discovery guard for this boundary. Directory
traversal skips `.crai` sidecars before probing and reports skipped entries
with reason `cram_index_sidecar_deferred`. Direct `.crai` requests fail before
probing with `unsupported_format`. Neither path parses CRAI bytes, plans indexed
CRAM traversal, or adds CRAM indexed-query fields to JSON.

M13.7 governs the JSON contract for those outcomes. The consume schema
documents `cram_index_sidecar_deferred`, and the canonical examples
`consume.success.crai_sidecar_skipped.json` and
`consume.failure.crai_sidecar_direct.json` show the two public shapes. A
directory-discovered `.crai` sidecar appears in `discovery.skipped_files` with
`detected_format: "UNSUPPORTED"` and reason `cram_index_sidecar_deferred`.
A direct `.crai` request fails with `unsupported_format` before discovery
payload population, so it does not create consumed, skipped, or rejected file
entries.

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
