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

## `consume`

Synopsis:
`bamana consume --input <path1> <path2> ... --out <result.bam> --mode <alignment|unmapped> [--recursive] [--dry-run] [-j, --threads <N>] [--sort <none|coordinate|queryname>] [--create-index] [--verify-checksum] [--force] [--reference <FASTA>] [--reference-cache <PATH>] [--reference-policy <strict|allow-embedded|allow-cache|auto-conservative>] [--sample <NAME>] [--read-group <ID>] [--platform <ont|illumina|pacbio|unknown>] [--include-glob <PATTERN>] [--exclude-glob <PATTERN>]`

Semantics:
Acts as Bamana’s input normalization gateway. It discovers files and
directories deterministically, classifies supported inputs, enforces a
conservative mixed-format policy, and normalizes them into BAM according to an
explicit ingest mode.

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

Does prove:
Deterministic discovery, input classification, mixed-format policy enforcement,
and staged BAM normalization for supported inputs. In dry-run mode it proves
what would be consumed without writing a BAM and validates CRAM reference-policy
configuration conservatively.

Does not prove:
Successful BAM normalization unless the response explicitly reports a written
output. It does not imply alignment for raw-read inputs, and it does not imply
reference independence for CRAM unless that is explicitly reported. Cache-backed
CRAM decoding, include/exclude glob filtering, checksum verification, and
post-ingest index creation remain deferred in the current slice.

Key output concepts:
`mode`, `inputs`, `discovery`, `reference`, `output`, `header`, `index`,
`checksum_verification`, `notes`.

## `subsample`

Synopsis:
`bamana subsample --input <file> --out <output> --fraction <f> [--seed <int>] [--mode <random|deterministic>] [--force] [--json-pretty]`

Semantics:
Planned benchmark-governed command for downsampling a single BAM, FASTQ.GZ, or
future FASTQ input under explicit deterministic or random policy. This command
is reserved because the benchmark framework needs a stable Bamana subsampling
contract for reproducible comparison against `samtools`, `seqtk`, `rasusa`, and
related tools.

Current planned modes:

* `deterministic`: stable inclusion policy given identical input, fraction, and
  version
* `random`: seeded random policy using `--seed`

Current planned input support:

* `BAM`
* `FASTQ.GZ`
* optional `FASTQ` later if implemented in the same slice

Does prove:
Only the explicit subsampling policy, fraction, seed, and output path reported
in JSON.

Does not prove:
It is not quality filtering, duplicate marking, or provenance cleanup. It does
not imply semantic equivalence with comparator tools whose subsampling model is
coverage-based or otherwise not directly fractional.

Key output concepts:
`format`, `mode`, `fraction`, `seed`, `records_examined`, `records_retained`,
`output`, `notes`.

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
Performs shallow BAM verification only by confirming a BAM-like BGZF container
and `BAM\1` magic in the first inflated block.

Does prove:
The file is BAM-like enough to satisfy Bamana’s shallow verification contract.

Does not prove:
Full record-stream validity, EOF presence, or deep validation.

Key output concepts:
`is_bam`, `shallow_verified`, `deep_validated`.

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
Parses the BAM header only.

Does prove:
The decompressed BAM header and binary reference dictionary were readable enough
 to parse.

Does not prove:
That alignment records are valid or that the full file body is readable.

Key output concepts:
`header.raw_header_text`, `header.hd`, `header.references`, `read_groups`,
`programs`, `comments`, `other_header_records`.

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

Synopsis:
`bamana check_map --bam <bamfile> [--sample-records <N>] [--full-scan] [--prefer-index]`

Semantics:
Assesses mapping state using the header, an index if usable, and otherwise a
bounded or full alignment scan.

Does prove:
Mapping evidence from the sources explicitly reported.

Does not prove:
Full BAM validity or complete mapping semantics beyond the examined data.

Key output concepts:
`mapping_status`, `evidence_source`, `index`, `references`, `summary`,
`confidence`.

## `check_index`

Synopsis:
`bamana check_index --bam <bamfile> [--require] [--prefer-csi]`

Semantics:
Inspects adjacent BAM index files for presence, type, shallow validity, and
plausible usability.

Does prove:
Index discovery and shallow structure checks only.

Does not prove:
That every index offset is correct or that random-access semantics are fully
validated.

Key output concepts:
`index.present`, `selected_path`, `kind`, `usable`, `stale`, `compatibility`,
`candidates`.

## `index`

Synopsis:
`bamana index --bam <bamfile> [--out <path>] [--force] [--format <bai|csi>]`

Semantics:
Establishes the index-creation command path, validates BAM plausibility, and
resolves output rules. In the current slice it reports writer limitations
honestly rather than pretending an index was created.

Does prove:
Command input validation and output-path resolution.

Does not prove:
That index creation occurred unless `created: true` is returned.

Key output concepts:
`requested_index_kind`, `output_index`, `notes`.

## `summary`

Synopsis:
`bamana summary --bam <bamfile> [--sample-records <N>] [--full-scan] [--prefer-index] [--include-mapq-hist] [--include-flags]`

Semantics:
Produces a fast operational BAM overview from header metadata, optional index
signals, and bounded or full record scans.

Does prove:
Only the metrics that correspond to the reported evidence mode.

Does not prove:
Full-file totals when the command explicitly reports bounded scan evidence.

Key output concepts:
`mode`, `evidence`, `counts`, `fractions`, `mapq`, `mapping`, `anomalies`,
`confidence`.

## `check_tag`

Synopsis:
`bamana check_tag --tag <TAG> --bam <bamfile> [--sample-records <N>] [--full-scan] [--require-type <TYPE>] [--count-hits]`

Semantics:
Traverses BAM auxiliary fields just deeply enough to establish observed tag
presence, bounded non-observation, or full-scan absence.

Does prove:
Presence in examined records, or absence across a successful full scan.

Does not prove:
Full-file absence in bounded mode.

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
The specific structural and consistency checks that were actually run.

Does not prove:
Biological correctness, reference concordance, or all optional-field semantics.

Key output concepts:
`mode`, `valid`, `summary`, `findings`, `semantic_note`.

## `checksum`

Synopsis:
`bamana checksum --bam <bamfile> [--mode <MODE>] [--algorithm <ALG>] [--include-header] [--exclude-tags <TAG,TAG,...>] [--only-primary] [--mapped-only]`

Semantics:
Computes explicit checksum domains over deterministic BAM header and record
serializations.

Does prove:
Only the meaning of the reported checksum mode, algorithm, filters, and tag
exclusion set.

Does not prove:
Full BAM validity or equivalence under any other comparison mode.

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

## `explode`

Synopsis:
`bamana explode --bam <bamfile> --out-dir <dir> [future options]`

Semantics:
Planned contract for splitting one BAM into multiple BAM outputs for workflow
distribution and reconstruction.

Current status:
Specified in the repository contract layer but not implemented in the current
CLI slice.

Planned key output concepts:
`input`, `explode`, `outputs`, `index`, `checksum_verification`, `notes`.
