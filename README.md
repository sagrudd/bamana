# Bamana

Bamana is a high-performance Rust toolkit for verification, quality control,
inspection, and transformation of BAM files and related bioinformatics formats.

The current repository contains the first concrete CLI slice for:

* `bamana identify <path>`
* `bamana subsample --input <file> --out <output>`
* `bamana inspect_duplication --input <file>`
* `bamana deduplicate --input <file> --out <cleaned_output>`
* `bamana forensic_inspect --input <file>`
* `bamana consume --input <path...> --out <result.bam>`
* `bamana annotate_rg --bam <input.bam> --rg-id <id>`
* `bamana reheader --bam <input.bam>`
* `bamana verify --bam <bamfile>`
* `bamana check_eof --bam <bamfile>`
* `bamana header --bam <bamfile>`
* `bamana check_map --bam <bamfile>`
* `bamana check_sort --bam <bamfile>`
* `bamana check_index --bam <bamfile>`
* `bamana index --bam <bamfile>`
* `bamana summary --bam <bamfile>`
* `bamana check_tag --tag <TAG> --bam <bamfile>`
* `bamana validate --bam <bamfile>`
* `bamana checksum --bam <bamfile>`
* `bamana sort --bam <bamfile> --out <result.bam>`
* `bamana merge --bam <bamfile1> <bamfile2> ... --out <result.bam>`

All command output is JSON.

## Native Performance Core

Bamana is being re-anchored around a **Bamana-native performance core** for
BGZF, BAM, FASTQ, sampling, ingest, and forensic hot paths.

This repository rule is now explicit:

* performance-critical BAM and FASTQ operations must be implemented using
  Bamana-native parsing, I/O, scanning, serialization, and transformation
  primitives
* general-purpose crates such as `noodles` are demoted to compatibility,
  testing, oracle, or transitional roles
* the current explicit transitional exception is conservative CRAM ingestion
  support

See:

* [ARCHITECTURE.md](/Users/stephen/Projects/bamana/ARCHITECTURE.md)
* [docs/architecture.md](/Users/stephen/Projects/bamana/docs/architecture.md)
* [docs/dependency-policy.md](/Users/stephen/Projects/bamana/docs/dependency-policy.md)
* [docs/performance-core.md](/Users/stephen/Projects/bamana/docs/performance-core.md)

The current semantics are intentionally narrow:

* `identify` determines the most likely file type quickly using extension hints, magic bytes, and shallow text heuristics
* `subsample` selects a subset of BAM, FASTQ, or FASTQ.GZ records under an explicit random or deterministic policy, preserves encounter order of retained records, and reports seed, identity basis, filter policy, and retained counts explicitly for production and benchmarking workflows
* `inspect_duplication` inspects BAM, FASTQ, and FASTQ.GZ inputs for suspicious collection-duplication signatures such as exact repeated records and adjacent repeated blocks that are more consistent with operator error or provenance mishandling than with ordinary duplicate biology
* `deduplicate` removes suspicious duplicated contiguous collection blocks conservatively according to an explicit remediation policy, with first-slice focus on adjacent repeated blocks and whole-file append signatures rather than molecular duplicate biology
* `forensic_inspect` inspects BAM provenance anomalies and coercion hallmarks such as suspicious header/program/read-group mismatches, read-name regime shifts, abrupt metadata transitions, and duplicate-block signatures that remain parseable but operationally suspicious
* `consume` is the ingestion gateway that discovers files/directories, classifies inputs, enforces mixed-format policy, and normalizes supported upstream formats into BAM according to an explicit mode and explicit CRAM reference policy
* `annotate_rg` performs record-level `RG:Z:` aux-tag insertion, replacement, or normalization across BAM alignment records, with optional coordinated `@RG` header updates
* `reheader` performs BAM header-only mutation planning and execution without modifying per-record `RG:Z` tags in alignment records
* `verify` performs shallow BAM verification only by confirming a BAM-like BGZF container and `BAM\1` magic in the first inflated block
* `check_eof` checks only for the canonical 28-byte BGZF EOF marker
* `header` parses the BAM header only, including the binary reference dictionary and textual SAM-style header records
* `check_map` prefers index-derived mapping summaries when a usable BAI is present and otherwise falls back to scan-based evidence
* `check_sort` combines BAM header declarations with a bounded scan of alignment records to assess coordinate or queryname ordering
* `check_index` inspects adjacent BAM indices for presence, type, shallow syntactic validity, timestamp-based staleness, and apparent usability
* `index` currently validates the BAM and resolves the output index path and format, but reports index creation as not yet implemented in this slice
* `summary` provides a fast operational BAM overview from header metadata, optional index-derived totals, and bounded or full record scans
* `check_tag` tests for BAM auxiliary tag presence using a bounded scan by default and full-file absence only when a complete scan succeeds
* `validate` performs a deeper streaming BAM structural and internal-consistency pass than `verify`, with finding severities and bounded modes
* `checksum` computes explicit machine-verifiable checksum domains over deterministic BAM header and record serializations, with order-sensitive and order-insensitive modes
* `sort` rewrites a BAM into an explicitly requested order using a deterministic in-memory first-slice engine with optional canonical checksum verification
* `merge` combines multiple BAM inputs into one BAM using conservative header compatibility checks, explicit input-order or sorted output modes, and optional canonical checksum verification

Neither `verify` nor `check_eof` implies deep validation of the BAM payload.
`inspect_duplication` does not perform Picard/GATK-style duplicate marking, does not treat BAM duplicate flags as primary evidence, and does not make biological claims about PCR or molecular duplication.
`deduplicate` is the conservative remediation companion to `inspect_duplication`; it removes duplicated collection blocks according to an explicit keep policy and does not act as Picard/GATK-style duplicate marking, duplicate-flag cleanup, or broad biological duplicate collapse.
`forensic_inspect` is an evidence-driven provenance inspection command; it is not a structural validator, not duplicate marking, and not a fraud detector.
`consume` does not imply that heterogeneous upstream inputs were normalized unless the response explicitly reports a written BAM output, and it does not silently combine alignment-bearing and raw-read inputs across the alignment/unmapped boundary.
`annotate_rg` is a record-touching transformation and therefore more expensive than `reheader`; it does not silently act as a header-only command.
`reheader` does not imply any record-level `RG:Z` tagging change, full BAM validation, or true in-place editing unless the response explicitly reports a proven-safe in-place mode in a future slice.
`header` does not imply that alignment records are readable, that EOF is present, or that the full BAM body is valid.
`check_map` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_sort` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_index` does not imply that every random-access offset is correct or that the BAM and index are semantically matched beyond shallow inspection.
`index` does not imply that BAM index writing has completed unless the response explicitly reports a created output.
`summary` does not imply full BAM validity, valid EOF state, or validation of every optional field, tag, or record invariant.
`check_tag` does not imply full BAM validity, valid EOF state, or semantic correctness of tag values beyond the auxiliary-field traversal actually performed.
`validate` does not imply biological correctness, external reference concordance, or correctness of every optional-field semantic beyond the checks actually implemented.
`checksum` does not imply full BAM validity, biological correctness, or semantic equivalence under any mode other than the one explicitly reported in the response.
`sort` does not imply full BAM validity beyond what was parsed, semantic preservation unless checksum verification was actually performed, or index correctness unless index creation and inspection explicitly succeeded.
`merge` does not imply full validity of all inputs beyond what was parsed, semantic preservation unless checksum verification was actually performed, or index correctness unless index creation and inspection explicitly succeeded.
`subsample` does not imply exact-count sampling, quality filtering, duplicate marking, provenance cleanup, or BAM index regeneration unless those behaviors are reported explicitly.

## Benchmark Framework

The repository now contains a containerized benchmarking framework under
[benchmarks/](/Users/stephen/Projects/bamana/benchmarks). It provides:

* a modular Nextflow DSL2 workflow
* explicit comparator support for `samtools`, `sambamba`, `seqtk`, `rasusa`,
  and `EPI2ME fastcat`
* seeded replication and warmup-run support
* per-run machine-readable benchmark rows
* R-based aggregation and publication-ready plotting

The canonical BAM baseline is `samtools`. `fastcat` is included explicitly for
ONT-style ingestion and concatenation comparisons. The benchmark framework is
designed for real large user-supplied BAM and FASTQ.GZ files and records
unsupported or partial comparisons explicitly instead of silently dropping them.

## Example Invocations

```bash
cargo run -- identify example.bam
cargo run -- subsample --input input.bam --out input.subsampled.bam --fraction 0.1 --mode random --seed 12345
cargo run -- subsample --input reads.fastq.gz --out reads.subsampled.fastq.gz --fraction 0.25 --mode deterministic --identity full_record
cargo run -- inspect_duplication --input input.fastq.gz --full-scan
cargo run -- inspect_duplication --input input.bam --identity qname_seq_qual_rg --min-block-size 100 --sample-records 250000
cargo run -- deduplicate --input input.fastq.gz --out input.cleaned.fastq.gz --mode contiguous-block --dry-run
cargo run -- deduplicate --input input.bam --out input.cleaned.bam --mode whole-file-append --keep first --verify-checksum
cargo run -- forensic_inspect --input input.bam --full-scan --inspect-tags
cargo run -- consume --input run.fastq.gz --out reads.bam --mode unmapped --dry-run
cargo run -- consume --input run.fastq.gz reads_dir --out reads.bam --mode unmapped --recursive
cargo run -- consume --input a.sam b.bam --out combined.bam --mode alignment
cargo run -- consume --input sample.cram --out sample.bam --mode alignment --reference ref.fa --reference-policy strict
cargo run -- consume --input sample.cram extra.bam --out combined.bam --mode alignment --reference ref.fa
cargo run -- annotate_rg --bam example.bam --rg-id rg001 --replace-existing --create-header-rg --out example.annotated.bam
cargo run -- annotate_rg --bam example.bam --rg-id rg001 --only-missing --require-header-rg --verify-checksum --out example.annotated.bam
cargo run -- reheader --bam example.bam --add-rg ID=rg1,SM=sample1,PL=ONT --out example.reheadered.bam
cargo run -- reheader --bam example.bam --set-sample sample1 --target-rg rg1 --rewrite-minimized --out example.reheadered.bam
cargo run -- reheader --bam example.bam --header new_header.sam --dry-run --in-place
cargo run -- verify --bam example.bam
cargo run -- check_eof --bam example.bam
cargo run -- header --bam example.bam
cargo run -- check_map --bam example.bam
cargo run -- check_map --bam example.bam --sample-records 50000 --full-scan
cargo run -- check_sort --bam example.bam
cargo run -- check_sort --bam example.bam --sample-records 50000 --strict
cargo run -- check_index --bam example.bam
cargo run -- check_index --bam example.bam --require
cargo run -- index --bam example.bam
cargo run -- index --bam example.bam --format csi --out example.bam.csi
cargo run -- summary --bam example.bam
cargo run -- summary --bam example.bam --sample-records 250000 --include-mapq-hist --include-flags
cargo run -- summary --bam example.bam --full-scan --prefer-index
cargo run -- check_tag --tag NM --bam example.bam
cargo run -- check_tag --tag RG --require-type Z --bam example.bam --count-hits
cargo run -- check_tag --tag SA --bam example.bam --full-scan
cargo run -- validate --bam example.bam
cargo run -- validate --bam example.bam --header-only
cargo run -- validate --bam example.bam --records 100000 --include-warnings
cargo run -- checksum --bam example.bam --mode raw-record-order
cargo run -- checksum --bam example.bam --mode canonical-record-order --only-primary --mapped-only
cargo run -- checksum --bam example.bam --mode payload --include-header --exclude-tags NM,MD,AS
cargo run -- checksum --bam example.bam --mode all
cargo run -- sort --bam example.bam --out sorted.bam
cargo run -- sort --bam example.bam --out qname.bam --order queryname --queryname-suborder lexicographical
cargo run -- sort --bam example.bam --out sorted.bam --verify-checksum --create-index
cargo run -- merge --bam shard1.bam shard2.bam --out merged.bam
cargo run -- merge --bam a.bam b.bam --out merged.sorted.bam --sort --verify-checksum
cargo run -- merge --bam lane1.bam lane2.bam --out merged.qname.bam --order queryname --queryname-suborder lexicographical
```

`header` uses the binary BAM reference section as authoritative for reference
names and lengths, and joins optional fields from textual `@SQ` records into the
structured JSON view when present.

`subsample` is Bamana's explicit selection command for BAM, FASTQ, and
FASTQ.GZ inputs. The current slice supports seeded random Bernoulli-style
per-record selection and deterministic hash-based selection using one of three
identity bases: `qname`, `qname_seq`, or `full_record`. Encounter order of
retained records is preserved. BAM headers are preserved, BAM-only filters
(`mapped_only` and `primary_only`) exclude non-eligible records before
sampling, and any pre-existing BAM index must be treated as invalid for the
subsampled output unless a future slice reports successful regeneration
explicitly. This command is intended both for production downsampling workflows
and for reproducible benchmarking on large user-supplied inputs.

`inspect_duplication` is the collection-duplication and operator-error
inspection command. It is intentionally distinct from ordinary PCR duplicate
marking semantics. The current slice supports BAM, FASTQ, and FASTQ.GZ inputs,
uses explicit identity modes (`qname_seq`, `qname_seq_qual`, and BAM-only
`qname_seq_qual_rg`), reports exact duplicate-identity statistics, and detects
adjacent repeated blocks of record identities. Direct adjacent repeated blocks,
especially whole-file append signatures, are treated as strong evidence of
unsafe concatenation, repeated appends, or coerced monolithic collections.
Non-contiguous repeated-block detection is reserved for a later slice.

`deduplicate` is the conservative remediation command for the signatures that
`inspect_duplication` reports. The current slice supports BAM, FASTQ, and
FASTQ.GZ inputs, requires an explicit remediation mode, and is intentionally
narrow: it removes adjacent repeated contiguous blocks and whole-file append
signatures under explicit identity and keep-policy semantics. Dry-run planning
is a first-class workflow, applied execution writes a new output only, and
existing BAM indices must be treated as invalid after record removal unless a
future slice reports successful regeneration explicitly. Global exact duplicate
collapse, non-contiguous block removal, and any molecular duplicate semantics
remain deferred.

`forensic_inspect` is the provenance-inspection companion to `validate`,
`inspect_duplication`, and `deduplicate`. The current slice is BAM-first and
focuses on evidence-driven hallmarks such as duplicate or append-like blocks,
header and body read-group mismatches, disconnected `@PG` histories, sparse or
weak provenance metadata, read-name regime shifts, and selected aux-tag regime
changes. Findings carry explicit category, severity, confidence, evidence
strength, and evidence-scope fields so bounded body scans do not overclaim
whole-file conclusions. This command does not assert fraud or intent; it
surfaces suspicious provenance and collection-hygiene anomalies with suggested
follow-up commands.

`consume` is the front-door normalization command for Bamana. In alignment mode
it preserves alignments from BAM, SAM, and Stage 2 CRAM inputs while
normalizing them into BAM. In unmapped mode it converts FASTQ and FASTQ.GZ
inputs into unmapped BAM without implying alignment. Mixed alignment-bearing and
raw-read ingestion remains rejected by default. CRAM support is conservative:
it is available only in alignment mode, it is governed by an explicit
`--reference-policy`, and Bamana does not silently guess CRAM reference
behavior. The current Rust slice supports explicit indexed FASTA
(`--reference <fasta>` with adjacent `.fai`) and conservative no-external-
reference decode attempts under `allow-embedded` or `auto-conservative`.
Cache-backed CRAM decoding, include/exclude glob filtering, consume-driven
index creation, and checksum verification remain explicitly deferred.

`annotate_rg` is the explicit per-record read-group tagging command. It scans
every BAM alignment record, inspects existing `RG:Z:` aux tags, and either
inserts, replaces, or conflict-checks them according to the selected mode. It
can also require or create a matching `@RG` header line explicitly. The current
slice uses a safe rewrite path and can optionally compare canonical
record-order checksums with `RG` excluded so automation can confirm that only
read-group annotation changed within the checksum domain.

`reheader` is a header-only metadata mutation command. It can replace the full
header from a SAM-style header file or apply targeted mutations such as adding,
updating, or removing `@RG` records, updating `SM`/`PL` on a targeted read
group, appending `@CO` lines, and adding or updating `@PG` lines. The current
slice always plans true in-place editing conservatively and falls back to a
rewrite path for actual execution. That rewrite path still preserves serialized
alignment-record layout bytes directly instead of performing semantic
record-level mutation, but it is not a true in-place patch. `reheader` does not
add, remove, or rewrite per-record `RG:Z` tags.

`check_map` prefers index-derived mapping summaries when a usable BAI is present.
Without a usable index it falls back to scan-based evidence. Bounded scan mode is
a fast assessment, not an exhaustive proof unless full-scan mode is used.

`check_sort` preserves declared sort metadata from the BAM header and compares it
with observed ordering in a bounded record scan. Coordinate and queryname sorts
are the primary observed modes in this slice; specialized modes such as
template-coordinate or minimiser-related sub-sorts are preserved from the header
with limited observed confirmation.

`check_index` looks for adjacent companion indices using the repository's current
priority order and reports whether a selected index is BAI, CSI, or unknown.
Stale-index detection is heuristic and based on file modification times rather
than proof that every indexed offset still matches the BAM.

`index` currently establishes the index lifecycle command path by validating the
BAM, selecting a default output path (`<bam>.bai` or `<bam>.csi`), and enforcing
overwrite rules. Actual BAI/CSI writing is still deferred, and the JSON error
response makes that limitation explicit instead of pretending an index was built.

`summary` combines BAM header metadata with a bounded scan by default and
switches to full-file totals only when EOF is actually reached or `--full-scan`
is used. When a usable BAI is available and `--prefer-index` is enabled,
index-derived mapped/unmapped totals are reported separately from scan-derived
record-category counts so the evidence source stays explicit.

`check_tag` traverses BAM auxiliary fields just deeply enough to establish tag
presence, optional type-constrained presence, or full-scan absence. In bounded
mode, a missing tag only means it was not found in the examined records. In
full-scan mode, absence is reported only when the scan reaches EOF cleanly.

`validate` is the first deeper integrity pass in the repository. It checks BAM
file/header structure, streams through records, validates record layout and aux
traversal, and reports findings as `error`, `warning`, or `info`. Header-only
and bounded-record modes are supported, and finding-bearing invalid BAMs still
return structured validation payloads instead of collapsing to an opaque error.

`checksum` exposes a small number of explicit checksum domains instead of one
overloaded digest. `raw-record-order` hashes the deterministic per-record
serialization in encounter order, so it is suitable for order-sensitive stream
preservation checks. `canonical-record-order` hashes per-record canonical
serializations, sorts the per-record digests, and hashes the sorted digest list,
so it is intended for comparing BAM content across reordering operations such as
sorting. `header` hashes raw header text plus the binary reference dictionary in
order. `payload` hashes the deterministic record payload stream and can prefix
the header serialization when `--include-header` is requested.

Filters and exclusions are part of the checksum definition. `--only-primary`,
`--mapped-only`, `--include-header`, and `--exclude-tags` must match when
comparing digests. Auxiliary-tag exclusions apply to the deterministic
record-content serialization used by the record-based checksum modes in this
slice. The current order-insensitive canonical mode collects per-record digests
in memory before sorting them, which is correct and explicit but may need a
chunked or external-sort strategy for very large BAMs later.

`sort` is the first transformational command in the repository. The current
implementation reads records into memory, derives deterministic coordinate or
queryname lexicographical sort keys, rewrites the `@HD` sort metadata, and
writes a new BGZF/BAM output. Coordinate output is intended to be suitable for
standard BAM indexing. Queryname output is not suitable for standard coordinate
BAI indexing. Optional `--verify-checksum` support compares canonical
order-insensitive checksums of the input and output so content preservation can
be confirmed explicitly rather than implied.

`merge` builds on the same writer and comparator family. By default it preserves
input-file concatenation order. With `--sort` or `--order coordinate`, it reads
all records, applies the same coordinate comparator family used by `sort`, and
writes coordinate-ordered output. Queryname merge uses the same lexicographical
queryname ordering family. The current implementation is in-memory and requires
identical binary reference dictionaries across all inputs. Queryname and
input-order merge outputs are not suitable for standard coordinate BAI
indexing. Optional checksum verification compares the canonical
order-insensitive multiset checksum of the combined inputs against the merged
output.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## Current Status

The repository now contains a production-minded shallow BAM slice with shared
JSON contracts, structured error handling, fast file probing, and real BGZF EOF
inspection. Full BAM validation and broader BAM operations will be implemented
incrementally under the project charter in
[`docs/project-charter.md`](docs/project-charter.md).

## Specification Layer

The repository now carries a dedicated `spec/` tree for governed external
contracts. It exists so the CLI surface, JSON outputs, examples, and
interoperability expectations can be reviewed and versioned as public
interfaces.

Key paths:

* `spec/jsonschema/` contains machine-readable schemas for command outputs
* `spec/examples/` contains canonical success and failure JSON examples
* `spec/cli/` contains command, option, and exit-code contracts
* `spec/contracts/` documents versioning, compatibility, and naming rules
* `tests/contract/` contains contract-test scaffolding

When a pull request changes Bamana’s external contract, it should update:

* the relevant schema file
* the canonical examples
* the CLI/docs contract pages
* the contract tests or fixtures when applicable

Run the contract scaffolding with:

```bash
cargo test --test contract
```

Golden/example updates should be intentional and reviewable. Treat schema field
renames, enum-literal changes, nullability changes, and meaning changes as
breaking until explicitly reviewed under the contract versioning rules in
`spec/contracts/versioning.md`.

## Fixture Suite

The repository also carries a planned tiny synthetic fixture suite under
`tests/fixtures/`. This is the path from schema-only contract checks to
executable interop tests against real BAM and BAI inputs.

Key fixture assets:

* `tests/fixtures/manifest.json` defines the planned fixture inventory
* `tests/fixtures/plans/` documents taxonomy, coverage, and regeneration
* `tests/fixtures/expected/` is reserved for fixture-backed golden JSON outputs
* `docs/fixtures.md` explains how fixture changes should be reviewed

The first fixture suite is intentionally small and deterministic. It favors a
few purpose-built BAMs over large downloaded datasets or opaque binary blobs.
