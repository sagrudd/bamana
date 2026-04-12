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
