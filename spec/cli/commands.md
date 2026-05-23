# Bamana Command Contracts

This document describes the governed CLI contract for Bamana commands. It is
deliberately explicit about semantics and limitations so automation can rely on
the command surface without inferring guarantees that Bamana does not make.

## Global Rules

* Output is JSON only.
* Command names are stable public identifiers.
* Field names in JSON are snake_case unless a format-specific convention is
  already established.
* A command may return `ok: false` while still including structured `data`
  payloads when that helps automation reason about partial outcomes.

## `benchmark`

Synopsis:
`bamana benchmark --profile <fastq_ingress|fastq_gz_enumerate> --fastq <reads.fastq.gz> --report <report.pdf> [--bam <output.bam>] [-j, --threads <N>] [--container-image <IMAGE>] [--force]`

Semantics:
Runs an owned benchmark profile from the Bamana CLI. The command builds the
local release binary, builds the benchmark container, executes the selected
profile inside that container, records logs and machine-readable outputs, and
renders a PDF report.

Current profiles:

* `fastq_ingress`: compares Bamana FASTQ.GZ-to-unmapped-BAM ingestion against a
  fastcat-plus-samtools baseline and requires `--bam`
* `fastq_gz_enumerate`: compares Bamana FASTQ.GZ enumeration against gzip
  decompression plus line counting and does not require `--bam`

Does prove:
Only the benchmark setup and command paths reported in the JSON payload. The
profile output describes what was run, where logs were written, and where the
report and raw/aggregated artifacts were placed.

Does not prove:
Broad comparator parity across every Bamana command. A benchmark profile is a
specific measured workflow, not a blanket performance or semantic equivalence
claim.

Key output concepts:
`profile`, `fastq`, `bamana_output`, `comparator_output`, `report_pdf`,
`workdir`, `raw_results_dir`, `aggregated_dir`, `metadata_dir`, `logs_dir`,
`steps`, `notes`.

## `identify`

Synopsis:
`bamana identify <path>`

Semantics:
Determines the most likely file type quickly using extension hints, magic bytes,
and shallow text heuristics.

Does prove:
Likely format/container classification only.

Does not prove:
Deep content validity or full semantic correctness.

Key output concepts:
`detected_format`, `container`, `confidence`.

## `enumerate`

Synopsis:
`bamana enumerate --input <file> [-j, --threads <N>]`

Semantics:
Counts top-level records in a supported single-file input using format-aware
streaming parsing. In the current slice, a FASTA entry counts as one record,
each FASTQ record counts as one record, each SAM alignment line counts as one
record, and each BAM alignment record counts as one record. For `FASTQ.GZ`,
enumeration now materializes a sibling `FASTQ.GZI` sidecar when one is absent
and then reuses the stored exact total-record count on subsequent runs instead
of rewalking the gzip stream.

Current input support:

* `BAM`
* `SAM`
* `FASTQ`
* `FASTQ.GZ`
* `FASTA`

Does prove:
That the input parsed cleanly enough for the selected format-specific counter
to reach EOF and report a stable record count. For `FASTQ.GZ`, it proves either
that the sidecar was built successfully or that an existing sidecar carried a
usable exact record total.

Does not prove:
Deep semantic validation beyond the parsing actually required for counting, nor
support for directories, CRAM, BED, or GFF in this slice.

Key output concepts:
`detected_format`, `records`.

## `consume`

Synopsis:
`bamana consume --input <path1> <path2> ... --out <result.bam> --mode <alignment|unmapped> [--recursive] [--dry-run] [-j, --threads <N>] [--sort <none|coordinate|queryname>] [--create-index] [--verify-checksum] [--force] [--reference <FASTA>] [--reference-cache <PATH>] [--reference-policy <strict|allow-embedded|allow-cache|auto-conservative>] [--sample <NAME>] [--read-group <ID>] [--platform <ont|illumina|pacbio|unknown>] [--include-glob <PATTERN>] [--exclude-glob <PATTERN>]`

Semantics:
Acts as Bamana’s input normalization gateway. It discovers files and
directories deterministically, classifies supported inputs, enforces a
conservative mixed-format policy, and normalizes them into BAM according to an
explicit ingest mode.

BAM alignment inputs are loaded through `BamScanner` and written through
Bamana's native BGZF writer. SAM alignment inputs use the native SAM parser.
FASTQ and FASTQ.GZ unmapped inputs use the native FASTQ readers. CRAM remains a
compatibility path governed by explicit reference policy.

Mixed-format policy:

* `alignment` accepts alignment-bearing inputs only (`BAM`, `SAM`, `CRAM`)
* `unmapped` accepts raw-read inputs only (`FASTQ`, `FASTQ.GZ`)
* by default, alignment-bearing and raw-read inputs are not allowed in the same
  request
* `CRAM` is valid only in `alignment` mode

CRAM reference policy:

* `strict` is the safest policy, the current default, and currently requires an explicit indexed
  FASTA supplied with `--reference`
* `allow-embedded` permits a conservative decode attempt without external FASTA
  and reports whether decode completed without one
* `allow-cache` is reserved for cache-backed CRAM decoding and remains
  unimplemented in the current slice
* `auto-conservative` uses an explicit FASTA when provided and otherwise falls
  back only to conservative no-external-reference decode attempts
* Bamana does not silently guess CRAM reference behavior

Directory traversal rules:

* file paths are considered directly
* directory paths are scanned top-level only unless `--recursive` is supplied
* discovered paths are ordered lexically by normalized path string
* symlinks are not followed in the current slice
* unsupported or skipped entries are reported explicitly in JSON

FASTQ.GZ execution rules:

* `-j 0` means use all available cores
* `-j 1` is the deterministic fallback for otherwise parallelisable `FASTQ.GZ`
  imports
* when a single `FASTQ.GZ` input has an adjacent `FASTQ.GZI`, consume uses the
  checkpoint totals to size a worker-batch conversion pipeline
* when multiple `FASTQ.GZ` inputs are present, consume parallelizes import
  across files and does not preserve discovery-order determinism unless the
  caller constrains execution to one thread

Does prove:
Deterministic discovery, input classification, mixed-format policy enforcement,
and staged BAM normalization for supported inputs. In dry-run mode it proves
what would be consumed without writing a BAM and validates CRAM reference-policy
configuration conservatively.

Does not prove:
Successful BAM normalization unless the response explicitly reports a written
output. It does not imply alignment for raw-read inputs, and it does not imply
reference independence for CRAM unless that is explicitly reported. Cache-backed
CRAM decoding beyond the selected reference policy, include/exclude glob
filtering, checksum verification, and post-ingest BAM index creation remain
deferred in the current slice. `--verify-checksum` reports requested but
unperformed checksum state and fails before writing in this slice.

Key output concepts:
`mode`, `inputs`, `discovery`, `reference`, `output`, `header`, `index`,
`checksum_verification`, `notes`.

Output safety:
Dry-run writes no BAM output. Non-dry-run execution publishes a completed
temporary BAM through a final rename; `output.written` is true only after the
write and requested reporting stages complete.

## `subsample`

Synopsis:
`bamana subsample --input <file> --out <output> --fraction <f> [--seed <int>] [--mode <random|deterministic>] [--force] [--json-pretty]`

Semantics:
Downsamples a single BAM, FASTQ, or FASTQ.GZ input under an explicit
deterministic or random policy. The command is intended for production
workflows and for reproducible comparison against `samtools`, `seqtk`,
`rasusa`, and related tools in the benchmark framework.

Current planned modes:

* `deterministic`: stable inclusion policy given identical input, fraction, and
  version, implemented in the current slice with stable hash-based selection
* `random`: seeded random Bernoulli-style policy using `--seed`; a seed is
  generated and reported when one is not supplied explicitly

Current input support:

* `BAM`
* `FASTQ`
* `FASTQ.GZ`

Deterministic identity semantics:

* `qname`: read name only
* `qname_seq`: read name plus sequence
* `full_record`: full record representation; this is the current default for
  deterministic mode

Current filter semantics:

* `--mapped-only`: BAM only; non-mapped records are excluded from output
* `--primary-only`: BAM only; secondary and supplementary records are excluded
  from output

Output-order semantics:

* retained records preserve encounter order in the current slice
* no implicit sorting is performed by `subsample`

Does prove:
Only the explicit subsampling policy, fraction, seed, deterministic identity,
filters, and output path reported in JSON.

Does not prove:
It is not quality filtering, duplicate marking, or provenance cleanup. It does
not imply semantic equivalence with comparator tools whose subsampling model is
coverage-based or otherwise not directly fractional.

Key output concepts:
`format`, `selection`, `execution`, `output`, `index`, `filters`, `notes`.

## `inspect_duplication`

Synopsis:
`bamana inspect_duplication --input <file> [--identity <qname_seq|qname_seq_qual|qname_seq_qual_rg>] [--min-block-size <N>] [--sample-records <N>] [--full-scan] [--max-findings <N>]`

Semantics:
Inspects a single BAM, FASTQ, or FASTQ.GZ input for suspicious
collection-duplication signatures that are more consistent with operator error,
unsafe concatenation, repeated appends, or coerced monolithic collections than
with ordinary duplicate biology.

Identity semantics:

* `qname_seq`: read name plus sequence
* `qname_seq_qual`: read name plus sequence plus quality; this is the current
  default
* `qname_seq_qual_rg`: BAM-only read name plus sequence plus quality plus read
  group

Current detection layers:

* exact duplicate-record identity statistics
* adjacent repeated-block detection with deterministic record ranges
* whole-file append classification when the examined file halves repeat exactly

Does prove:
Only the duplication evidence reported in the JSON payload, under the explicit
identity mode and scan scope that were actually used.

Does not prove:
It is not a Picard/GATK-style duplicate-marking contract. It does not interpret
BAM duplicate flags as primary evidence. It does not make biological claims
about PCR or molecular duplication. In bounded mode it does not prove whole-file
absence of suspicious duplication.

Key output concepts:
`format`, `identity_mode`, `scan_mode`, `records_examined`, `summary`,
`findings`, `assessment`, `notes`.

## `deduplicate`

Synopsis:
`bamana deduplicate --input <file> --out <cleaned_output> --mode <contiguous-block|whole-file-append|global-exact> [--identity <qname_seq|qname_seq_qual|qname_seq_qual_rg>] [--dry-run] [--min-block-size <N>] [--keep <first|last>] [--verify-checksum] [--emit-removed-report <json>] [--sample-records <N>] [--full-scan] [--reindex] [--force]`

Semantics:
Removes suspicious collection-duplication signatures according to an explicit,
conservative remediation policy. The current slice supports a single BAM, FASTQ,
or FASTQ.GZ input and focuses on adjacent repeated contiguous blocks, including
whole-file append signatures when the second half duplicates the first half
under the selected identity mode.

Mode semantics:

* `contiguous-block`: detect adjacent repeated blocks and remove one copy
  according to `--keep`
* `whole-file-append`: restrict removal to strong whole-file append signatures
* `global-exact`: reserved for a later, more aggressive slice and currently
  returns `unimplemented`

Identity semantics:

* `qname_seq`: read name plus sequence
* `qname_seq_qual`: read name plus sequence plus quality; this is the current
  default
* `qname_seq_qual_rg`: BAM-only read name plus sequence plus quality plus read
  group

Keep-policy semantics:

* `first`: retain the first copy and remove the later adjacent copy
* `last`: retain the later copy and remove the earlier adjacent copy

Execution semantics:

* `--dry-run` is the recommended first operational step and writes nothing
* applied execution always writes a distinct output path
* existing output or removed-report paths are rejected unless `--force` is
  supplied
* BAM headers are preserved, but pre-existing BAM indices must be treated as
  invalid after record removal unless successful regeneration is reported

Does prove:
Only that the reported contiguous duplicate ranges were planned or removed under
the explicit mode, identity policy, keep policy, and scan scope that were
actually used.

Does not prove:
It is not a Picard/GATK-style duplicate-marking contract. It does not treat BAM
duplicate flags as primary evidence. It does not imply broad biological
duplicate removal, non-contiguous duplicate collapse, or pair-aware molecular
duplicate semantics. In bounded dry-run mode it does not prove that no
additional removable ranges exist beyond the examined records.

Key output concepts:
`format`, `mode`, `identity_mode`, `keep_policy`, `execution`, `summary`,
`ranges`, `output`, `index`, `checksum_verification`, `notes`.

## `forensic_inspect`

Synopsis:
`bamana forensic_inspect --input <file> [--sample-records <N>] [--full-scan] [--inspect-header] [--inspect-rg] [--inspect-pg] [--inspect-readnames] [--inspect-tags] [--inspect-duplication] [--max-findings <N>]`

Semantics:
Performs forensic-style provenance inspection of a single BAM input and reports
hallmarks that are consistent with concatenation, repeated appended blocks,
coerced monolithic collections, weak provenance discipline, or suspicious
metadata/body transitions.

Current inspection areas:

* header anomalies such as duplicate `@RG`/`@PG` identifiers, sparse
  provenance, and suspicious sample/platform mixtures
* read-group mismatches between header declarations and record-level `RG:Z`
  usage
* broken or disconnected `@PG` chains
* read-name regime shifts between early and late body windows
* duplicate-block and whole-file-append hallmarks using `qname_seq_qual`
  identity
* selected aux-tag regime shifts when `--inspect-tags` is enabled

Scope semantics:

* when no explicit `--inspect-*` flags are supplied, the default suite is
  `header`, `read_groups`, `program_chain`, `read_names`, and
  `duplication_hallmarks`
* `--inspect-tags` is opt-in in the current slice
* the first slice is BAM-only; other formats are reported as unsupported for
  this command

Scan semantics:

* header inspection is always complete for the parsed BAM header
* bounded mode inspects the first `N` records only for body-oriented evidence
* full-scan mode inspects the BAM body to EOF and can support stronger
  collection-level conclusions
* every finding reports whether its evidence was header-only, bounded body
  evidence, full body evidence, or a combined header/body basis

Does prove:
Only the provenance anomalies and collection-hallmark findings explicitly
reported in the JSON payload under the inspected scopes and scan mode that were
actually used.

Does not prove:
It is not a structural-validation contract, not a duplicate-marking contract,
and not a fraud-detection contract. It does not prove intentional misconduct.
In bounded mode it does not prove whole-file absence of suspicious body-level
anomalies.

Key output concepts:
`format`, `scan_mode`, `scopes`, `records_examined`, `summary`, `findings`,
`assessment`, `notes`.

## `annotate_rg`

Synopsis:
`bamana annotate_rg --bam <input.bam> --rg-id <ID> [--out <output.bam>] [--only-missing | --replace-existing | --fail-on-conflict] [--require-header-rg | --create-header-rg | --add-header-rg <KEY=VALUE,...> | --set-header-rg <KEY=VALUE,...>] [--reindex] [--verify-checksum] [-j, --threads <N>] [--force] [--dry-run]`

Semantics:
Performs record-level read-group annotation by scanning every BAM alignment
record, inspecting existing `RG:Z:` aux tags, and inserting, replacing, or
conflict-checking them according to the selected mode.

Record modes:

* `only_missing`: insert `RG:Z:<ID>` only when a record currently lacks an RG
  tag
* `replace_existing`: normalize every record to the requested RG ID
* `fail_on_conflict`: fail if any record already contains a different RG value;
  this is the current conservative default when no explicit mode flag is given

Header policy:

* `require_existing` is the current conservative default
* `create_if_missing` adds a minimal matching `@RG` line when absent
* `add_header_rg` adds a fully specified new `@RG` line
* `set_header_rg` updates the existing target `@RG` line

Does prove:
That the BAM stream was rewritten with the reported record mode and header
policy, and that the reported output file was written when `written: true`.

Does not prove:
It does not behave like `reheader`, because it touches alignment records. It
does not imply broader BAM validation than what was parsed during the rewrite.
It does not imply record-content preservation unless checksum verification was
performed with the explicitly reported tag-exclusion policy.

Key output concepts:
`request`, `execution`, `records`, `header`, `output`, `index`,
`checksum_verification`, `notes`.

## `reheader`

Synopsis:
`bamana reheader --bam <input.bam> [--header <new_header.sam>] [--add-rg <KEY=VALUE,...>] [--set-rg <KEY=VALUE,...>] [--remove-rg <ID>] [--set-sample <NAME>] [--set-platform <ont|illumina|pacbio|unknown>] [--target-rg <ID>] [--set-pg <KEY=VALUE,...>] [--add-comment <TEXT>] [--in-place] [--rewrite-minimized] [--safe-rewrite] [--dry-run] [--out <output.bam>] [--force] [--reindex] [--verify-checksum]`

Semantics:
Performs BAM header-only metadata mutation. It can replace the full BAM header
from a SAM-style header file or apply targeted header mutations such as adding,
updating, or removing `@RG` records; updating `SM` or `PL` on a targeted read
group; adding or updating `@PG`; and appending `@CO` lines.

Execution-mode semantics:

* `--in-place` requests true in-place header patching only if Bamana can prove
  it is safe
* if `--in-place` is not feasible, the command fails unless
  `--rewrite-minimized` is also supplied to permit fallback
* `--rewrite-minimized` is the practical execution path in the current slice
* `--safe-rewrite` requests an explicit conservative rewrite path
* `--dry-run` performs mutation validation and execution planning without
  writing output

Does prove:
Only the BAM header mutation described in the response, the planning outcome
for in-place feasibility, and the execution mode actually used.

Does not prove:
It does not add, remove, or rewrite per-record `RG:Z` tags in alignment
records. If downstream tooling requires per-record `RG:Z` tags, the correct
command is `annotate_rg`, not `reheader`. `reheader` does not imply full BAM
validation, and it does not imply content preservation unless checksum
verification was explicitly performed and matched.

Index and checksum behavior:

* existing indices should be treated as invalidated after reheader in the
  current slice unless a future narrowly proven-safe case says otherwise
* `--reindex` is accepted and reported, but index writing remains deferred in
  the current slice
* `--verify-checksum` uses canonical record-order checksum semantics with the
  BAM header excluded so the command can demonstrate header-only behavior

Key output concepts:
`mutation`, `planning`, `execution`, `output`, `index`,
`checksum_verification`, `notes`.

## `verify`

Synopsis:
`bamana verify --bam <bamfile>`

Semantics:
Performs header-level BAM verification by confirming a BGZF container, BAM
magic, and native BAM header parse.

Does prove:
The file is BAM-like enough for Bamana to read the native BAM header and binary
reference dictionary.

Does not prove:
Full record-stream validity, EOF presence, or deep validation.

Key output concepts:
`is_bam`, `shallow_verified`, `deep_validated`, `checks_performed`.

## `check_eof`

Synopsis:
`bamana check_eof --bam <bamfile>`

Semantics:
Checks only for the canonical BGZF EOF marker.

Does prove:
Tail EOF-marker presence or absence.

Does not prove:
Overall BAM validity or full stream readability.

Key output concepts:
`bgzf_eof_present`, `complete`, `semantic_note`.

## `header`

Synopsis:
`bamana header --bam <bamfile>`

Semantics:
Parses the BAM header only through Bamana's native BGZF stream reader and
native BAM header codec.

Does prove:
The decompressed BAM header and binary reference dictionary were readable enough
to parse.

Does not prove:
That alignment records are valid or that the full file body is readable.

Key output concepts:
`header.raw_header_text`, `header.hd`, `header.references`,
`header.reference_diagnostics`, `read_groups`, `programs`, `comments`,
`other_header_records`. `header.references` is the binary reference dictionary
and remains authoritative for BAM decoding; `header.reference_diagnostics`
reports non-fatal textual `@SQ` mismatches such as missing, extra, duplicate,
reordered, name-mismatched, or length-mismatched records.

## `check_sort`

Synopsis:
`bamana check_sort --bam <bamfile> [--sample-records <N>] [--strict]`

Semantics:
Combines header-declared sort metadata with a bounded or stricter scan of
records to assess apparent ordering.

Does prove:
Observed ordering evidence over the examined records.

Does not prove:
That every record in the file obeys the declared order unless full-file
validation is performed elsewhere.

Key output concepts:
`declared_sort`, `observed_sort`, `agreement`, `confidence`,
`first_violation`.

## `check_map`

Current synopsis:
`bamana check_map --bam <bamfile> [--sample-records <N>] [--full-scan] [--prefer-index] [--region <REGION> ...]`

Semantics:
Assesses mapping state using the header, a selected BAI sidecar only when it is
not timestamp-stale, structurally valid under Bamana's implemented BAI checks,
and complete for mapped/unmapped reference metadata, and otherwise a bounded
or full native alignment scan.

Does prove:
Mapping evidence from the sources explicitly reported. Index-derived results
prove only a usable BAI metadata summary was available; scan-derived results
prove only what was observed in the examined alignment records.

Does not prove:
Full BAM validity or complete mapping semantics beyond the examined data.

M10 region contract:

* `--region` uses the M10.2 grammar: `reference` or `reference:start-end`
* interval input is 1-based closed and reported in normalized form as 0-based
  half-open coordinates
* repeated `--region` values preserve request order and are not merged or
  deduplicated
* region-aware output adds `region_scope` and must keep region-scoped
  mapping evidence distinct from whole-file mapping evidence
* usable BAI sidecars drive indexed traversal; missing, stale, unsupported, or
  invalid index state falls back to native scan evidence with explicit scan
  limits
* region summaries use `region_records_examined` and
  `region_mapped_records_observed` rather than whole-file totals
* region files remain deferred and must fail precisely until a later task
  promotes them

Key output concepts:
`mapping_status`, `evidence_source`, `index`, `references`, `region_scope`,
`summary`, `confidence`.

## `check_index`

Synopsis:
`bamana check_index --bam <bamfile> [--require] [--prefer-csi]`

Semantics:
Inspects adjacent BAM index files for presence, type, implemented structural
validity, and plausible usability. BAI inspection validates magic,
reference-count agreement, bin uniqueness and range, regular chunk
virtual-offset order, linear-index ordering, metadata pseudo-bin shape, and
parseability. CSI inspection parses and checks header reference counts enough
to report detected-but-not-supported behavior or a deterministic mismatch.
Stale-index detection is timestamp based: if the BAM modification time is newer
than the selected sidecar, the selected index is reported as stale and not
usable.

Does prove:
Index discovery and implemented BAI/CSI structural checks only.

Does not prove:
That every index offset is correct or that random-access semantics are fully
validated.

Key output concepts:
`index.present`, `selected_path`, `kind`, `usable`, `stale`, `compatibility`,
`candidates`.

## `index`

Synopsis:
`bamana index --input <file> [--out <path>] [--force] [--format <bai|csi|gzi>]`

Semantics:
Creates a format-appropriate sidecar index for supported inputs. BAM BAI input
validates plausibility, rejects header-declared unsupported sort orders, builds
native BAI bins/chunks/linear windows from scanner virtual offsets, and
publishes the sidecar through a temporary path and final rename. Existing output
paths are refused unless `--force` is supplied. BAM CSI writing remains
explicitly unimplemented in the current slice. `FASTQ.GZ` input writes a binary
`FASTQ.GZI` sidecar by scanning the gzip stream once and sampling checkpoint
boundaries at approximately 0.1% compressed-offset intervals by default, pinned
to completed FASTQ record boundaries rather than arbitrary byte positions. The
binary `FASTQ.GZI` payload stores header metadata, planner flags, and sampled
`(compressed_offset, uncompressed_offset, cumulative_records)` checkpoint pairs
for indexed enumeration, explode planning, and consume planning.

Does prove:
For BAM BAI: command input validation, native BAI sidecar creation, and
completion of the requested output path.
For FASTQ.GZ: that a `FASTQ.GZI` sidecar was written and that its checkpoints
track approximate compressed-stream progress at record-safe boundaries.

Does not prove:
That CSI creation or indexed random-access acceleration occurred.
For BAM CSI and BAM failure responses, `output_index.created` remains `false`.
That a `FASTQ.GZI` checkpoint exists for every read or every gzip member
boundary; the sidecar is intentionally sampled rather than exhaustive.

Key output concepts:
`requested_index_kind`, `output_index`, `notes`.

## `summary`

Current synopsis:
`bamana summary --bam <bamfile> [--sample-records <N>] [--full-scan] [--prefer-index] [--include-mapq-hist] [--include-flags] [--region <REGION> ...]`

Semantics:
Produces a fast operational BAM overview from header metadata, optional index
signals, and bounded or full record scans. Optional index-derived totals are
used only from a selected BAI sidecar that is not timestamp-stale, passes the
implemented structural checks, and contains complete mapped/unmapped metadata.
Generated and discovered BAI sidecars follow the same validation path.

Does prove:
Only the metrics that correspond to the reported evidence mode. Bounded scan
metrics describe examined records; index-derived totals remain separate from
scan-derived counts. Stale, unsupported, malformed, or incomplete sidecars
fall back to native scanner evidence.

Does not prove:
Full-file totals when the command explicitly reports bounded scan evidence, or
full BAM structural validity.

M10 region contract:

* `--region` uses the M10.2 grammar: `reference` or `reference:start-end`
* interval input is 1-based closed and reported in normalized form as 0-based
  half-open coordinates
* repeated `--region` values preserve request order and are not merged or
  deduplicated
* region-aware output adds `region_scope` and must keep region-scoped
  operational metrics distinct from whole-file totals
* usable BAI sidecars drive indexed traversal; missing, stale, unsupported, or
  invalid index state falls back to native scan evidence with explicit scan
  limits
* region-scoped `counts`, `fractions_observed`, `mapq`, `mapping`,
  `anomalies`, and optional `flag_categories` describe only requested
  intervals
* whole-file BAI totals are intentionally omitted from region-scoped summary
  `index_derived`; the index is used only to find records
* region files remain deferred and must fail precisely until a later task
  promotes them

Key output concepts:
`mode`, `evidence`, `counts`, `fractions`, `mapq`, `mapping`, `region_scope`,
`anomalies`, `confidence`.

## `check_tag`

Synopsis:
`bamana check_tag --tag <TAG> --bam <bamfile> [--sample-records <N>] [--full-scan] [--require-type <TYPE>] [--count-hits]`

Semantics:
Traverses BAM auxiliary fields just deeply enough to establish observed tag
presence, bounded non-observation, or full-scan absence.

Does prove:
Presence in examined records, type-filtered presence when `--require-type` is
used, or absence across a successful full scan.

Does not prove:
Full-file absence in bounded mode, duplicate tag prevalence inside a record, or
general auxiliary-tag value semantics beyond supported traversal.

Key output concepts:
`tag`, `required_type`, `mode`, `result`, `records_examined`,
`records_with_tag`, `full_file_scanned`, `confidence`.

## `validate`

Synopsis:
`bamana validate --bam <bamfile> [--max-errors <N>] [--max-warnings <N>] [--header-only] [--records <N>] [--fail-fast] [--include-warnings]`

Semantics:
Performs a deeper streaming structural and internal-consistency validation pass
than `verify`.

Does prove:
The specific structural and consistency checks that were actually run for the
reported scope.

Does not prove:
Biological correctness, reference concordance, or all optional-field semantics.
It also does not prove full-file validity in bounded or header-only modes.

Key output concepts:
`mode`, `valid`, `summary`, `findings`, `semantic_note`.

## `checksum`

Synopsis:
`bamana checksum --bam <bamfile> [--mode <MODE>] [--algorithm <ALG>] [--include-header] [--exclude-tags <TAG,TAG,...>] [--only-primary] [--mapped-only]`

Semantics:
Computes explicit checksum domains over deterministic BAM header and record
serializations. `raw-record-order` is encounter-order sensitive,
`canonical-record-order` is order-insensitive but duplicate-multiplicity aware,
`header` covers deterministic header text plus binary reference dictionary
serialization, and `payload` covers the encounter-order record payload stream
with optional header inclusion.

Does prove:
Only the meaning of the reported checksum mode, algorithm, filters, and tag
exclusion set.

Does not prove:
Full BAM validity, biological equivalence, whole-file semantic equivalence, or
equivalence under filters, excluded tags, header inclusion, or comparison modes
other than those explicitly reported.

Key output concepts:
`algorithm`, `results[].mode`, `digest`, `order_sensitive`, `filters`,
`excluded_tags`.

## `sort`

Synopsis:
`bamana sort --bam <bamfile> --out <result.bam> [--order <coordinate|queryname>] [--queryname-suborder <natural|lexicographical>] [-j, --threads <N>] [--memory-limit <BYTES>] [--create-index] [--verify-checksum] [--force]`

Semantics:
Rewrites a BAM into an explicitly requested order using a deterministic
in-memory engine in the current slice.

Does prove:
The output file was written according to the produced order and reported options.

Does not prove:
Content preservation unless checksum verification was performed and matched.

Key output concepts:
`output`, `sort`, `records`, `index`, `checksum_verification`, `notes`.

Output safety:
The sorted BAM is written to a temporary file and published through a final
rename. Existing targets are rejected unless `--force` is supplied.

## `merge`

Synopsis:
`bamana merge --bam <bamfile1> <bamfile2> ... --out <result.bam> [--sort] [--order <coordinate|queryname|input>] [--queryname-suborder <natural|lexicographical>] [--create-index] [--verify-checksum] [-j, --threads <N>] [--force]`

Semantics:
Combines multiple BAM inputs into one BAM using conservative header
compatibility rules and explicit merge modes.

Does prove:
The output mode and compatibility policy reported in the JSON response.

Does not prove:
Content preservation unless checksum verification was performed and matched.

Key output concepts:
`inputs`, `output`, `merge`, `records`, `index`, `checksum_verification`,
`notes`.

Output safety:
The merged BAM is written to a temporary file and published through a final
rename. Existing targets are rejected unless `--force` is supplied.

## `explode`

Synopsis:
`bamana explode --input <file> --out-dir <dir> --explode <N> [-j, --threads <N>] [--force]`

Semantics:
Splits one `BAM`, `SAM`, or `FASTQ.GZ` input into `N` contiguous shards using
encounter-order record ranges. BAM shards preserve the parsed BAM header and
write scanner-backed records through Bamana's native BGZF writer. SAM shards
preserve leading header lines. FASTQ.GZ shards use adjacent `FASTQ.GZI`
planning metadata and write concatenated gzip-member streams.

Does prove:
Each shard preserves the original order of reads or alignments within that
shard, every sequence or alignment lands in exactly one shard, and shard
boundaries are reported explicitly.

Does not prove:
Global equivalence reconstruction beyond the recorded shard ranges, or true
random-access parallel inflate for generic single-member gzip streams. Shard
sizes may be uneven, especially when FASTQ.GZ ranges follow available
`FASTQ.GZI` cutpoints.

Key output concepts:
`input`, `explode`, `outputs`, `index`, `checksum_verification`, `notes`.

Output safety:
Shard outputs are written to temporary files and preflighted before final
publication. Existing shard targets are rejected unless `--force` is supplied.

Operational notes:
For `FASTQ.GZ`, `explode` auto-creates or reuses an adjacent `FASTQ.GZI`
sidecar, plans shard boundaries from available index cutpoints, and then
writes contiguous `FASTQ.GZ` shards without reordering reads inside any shard.
Shard sizes are therefore only as uniform as the available cutpoints allow and
may differ slightly to keep the method fast.

## `fastq`

Synopsis:
`bamana fastq --bam <input.bam> [--out <output.fastq.gz>] [-j, --threads <N>] [--force]`

Semantics:
Exports one BAM as a single ordered `FASTQ.GZ` stream. Read names, sequences,
and qualities are emitted in input encounter order. The output is written as an
ordered stream of gzip members so decode and compression work can use multiple
worker threads while preserving record order.

Does prove:
The reported BAM records were parsed and emitted to the reported FASTQ.GZ
output path in encounter order.

Does not prove:
Full BAM validation, pair repair, filtering, alignment preservation, or
retention of BAM-only metadata. BAM header records, alignment fields, and
auxiliary tags are not represented in FASTQ output.

Key output concepts:
`format`, `output`, `execution.records_read`, `execution.records_written`,
`execution.threads_used`, `notes`.

## `unmap`

Synopsis:
`bamana unmap --bam <input.bam> [--out <output.bam>] [--dry-run] [-j, --threads <N>] [--force]`

Semantics:
Rewrites one BAM as unmapped BAM. The output header has reference dictionary
state removed, and each record has reference-bound mapping state stripped:
coordinates, CIGAR data, mate coordinates, template length, mapping quality,
and mapping-related auxiliary tags. Non-mapping auxiliary metadata is
preserved.

Does prove:
Only the unmapped rewrite described by the JSON payload. In dry-run mode it
proves planning and record traversal only; it does not write output.

Does not prove:
Biological remapping, realignment, source-reference correctness, or full BAM
validity beyond the records parsed during the rewrite.

Key output concepts:
`format`, `output`, `execution.records_read`, `execution.records_written`,
`execution.mapping_tags_removed`, `notes`.
