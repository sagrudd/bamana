# Bamana

Bamana is a high-performance Rust toolkit for verification, quality control,
inspection, and transformation of BAM files and related bioinformatics formats.

The current repository contains the first concrete CLI slice for:

* `bamana identify <path>`
* `bamana enumerate --input <file> [-j <threads>]`
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
* `bamana index --input <file>`
* `bamana summary --bam <bamfile>`
* `bamana check_tag --tag <TAG> --bam <bamfile>`
* `bamana validate --bam <bamfile>`
* `bamana checksum --bam <bamfile>`
* `bamana sort --bam <bamfile> --out <result.bam>`
* `bamana merge --bam <bamfile1> <bamfile2> ... --out <result.bam>`
* `bamana explode --input <file> --out-dir <dir> --explode <N>`
* `bamana fastq --bam <bamfile>`
* `bamana unmap --bam <bamfile>`
* `bamana benchmark --profile <profile> --fastq <reads.fastq.gz> --report <report.pdf>`

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
* [ROADMAP.md](/Users/stephen/Projects/bamana/ROADMAP.md)
* [docs/roadmap.md](/Users/stephen/Projects/bamana/docs/roadmap.md)
* [benchmarks/status_for_tomorrow.md](/Users/stephen/Projects/bamana/benchmarks/status_for_tomorrow.md)

The current semantics are intentionally narrow:

* `identify` determines the most likely file type quickly using extension hints, magic bytes, and shallow text heuristics
* `enumerate` counts top-level records in a single BAM, SAM, FASTQ, FASTQ.GZ, or FASTA input using format-aware parsing, auto-materializes `FASTQ.GZI` sidecars for `FASTQ.GZ`, and then reuses the exact indexed total on later runs
* `subsample` selects a subset of BAM, FASTQ, or FASTQ.GZ records under an explicit random or deterministic policy, preserves encounter order of retained records, and reports seed, identity basis, filter policy, and retained counts explicitly for production and benchmarking workflows
* `inspect_duplication` inspects BAM, FASTQ, and FASTQ.GZ inputs for suspicious collection-duplication signatures such as exact repeated records and adjacent repeated blocks that are more consistent with operator error or provenance mishandling than with ordinary duplicate biology
* `deduplicate` removes suspicious duplicated contiguous collection blocks conservatively according to an explicit remediation policy, with first-slice focus on adjacent repeated blocks and whole-file append signatures rather than molecular duplicate biology
* `forensic_inspect` inspects BAM provenance anomalies and coercion hallmarks such as suspicious header/program/read-group mismatches, read-name regime shifts, abrupt metadata transitions, and duplicate-block signatures that remain parseable but operationally suspicious
* `consume` is the ingestion gateway that discovers files/directories, classifies inputs, enforces mixed-format policy, and normalizes supported upstream formats into BAM according to an explicit mode and explicit CRAM reference policy; `FASTQ.GZ` imports now parallelize across files and use adjacent `FASTQ.GZI` checkpoint totals to drive single-file worker-batch conversion
* `annotate_rg` performs record-level `RG:Z:` aux-tag insertion, replacement, or normalization across BAM alignment records, with optional coordinated `@RG` header updates
* `reheader` performs BAM header-only mutation planning and execution without modifying per-record `RG:Z` tags in alignment records
* `verify` performs header-level BAM verification by confirming a BGZF container, BAM magic, and native BAM header parse without scanning alignment records or checking EOF
* `check_eof` checks only for the canonical 28-byte BGZF EOF marker
* `header` parses the BAM header only through Bamana's native BGZF and BAM header codec, including the binary reference dictionary and textual SAM-style header records
* `check_map` prefers index-derived mapping summaries when a usable, non-stale, structurally valid BAI with complete mapped/unmapped metadata is present and otherwise falls back to scan-based evidence
* `check_sort` combines BAM header declarations with a bounded scan of alignment records to assess coordinate or queryname ordering
* `check_index` inspects adjacent BAM indices for presence, type, BAI structural validity, CSI header status, timestamp-based staleness, and apparent usability
* `index` creates native BAI sidecars for coordinate-sorted BAM inputs, still defers CSI writing, and creates sampled `FASTQ.GZI` sidecars for `FASTQ.GZ` inputs with dense planner checkpoints, cumulative record totals, and approximate parallel explode metadata stored at each checkpoint
* `explode` splits one `BAM`, `SAM`, or `FASTQ.GZ` input into contiguous shards while preserving the original encounter order of reads or alignments within each shard; BAM shards preserve the parsed header and use scanner-backed records through the native BGZF writer, while the `FASTQ.GZ` path auto-creates or reuses adjacent `FASTQ.GZI` metadata, aligns shard boundaries to available index cutpoints, and allows shard sizes to vary so every sequence lands in exactly one shard without extra reordering work
* `summary` provides a fast operational BAM overview from header metadata, optional index-derived totals, and bounded or full record scans
* `check_tag` tests for BAM auxiliary tag presence using a bounded scan by default and full-file absence only when a complete scan succeeds
* `validate` performs a deeper streaming BAM structural and internal-consistency pass than `verify`, with finding severities and bounded modes
* `checksum` computes explicit machine-verifiable checksum domains over deterministic BAM header and record serializations, with order-sensitive and order-insensitive modes
* `sort` rewrites a BAM into an explicitly requested order using a deterministic in-memory first-slice engine with optional canonical checksum verification
* `merge` combines multiple BAM inputs into one BAM using conservative header compatibility checks, explicit input-order or sorted output modes, and optional canonical checksum verification
* `fastq` exports BAM records to an ordered `FASTQ.GZ` stream, preserving input encounter order for read names, sequences, and qualities while intentionally dropping BAM header metadata
* `unmap` rewrites a BAM as unmapped BAM by removing reference-bound mapping state, CIGAR/mate/coordinate fields, and mapping-related auxiliary tags while preserving non-mapping metadata
* `benchmark` owns selected benchmark profiles by building the local release binary, building the benchmark container, running the profile, and rendering the requested report

The repository also now contains a minimal but real benchmark execution and
analysis path under [benchmarks/](/Users/stephen/Projects/bamana/benchmarks).
That layer is intended to make tomorrow's first benchmark runs interpretable,
not to claim full comparator or command parity already exists.

Neither `verify` nor `check_eof` implies deep validation of the BAM payload.
`verify` is limited to BGZF plus native BAM header structure; `check_eof` is
limited to the canonical BGZF EOF marker.
`inspect_duplication` does not perform Picard/GATK-style duplicate marking, does not treat BAM duplicate flags as primary evidence, and does not make biological claims about PCR or molecular duplication.
`deduplicate` is the conservative remediation companion to `inspect_duplication`; it removes duplicated collection blocks according to an explicit keep policy and does not act as Picard/GATK-style duplicate marking, duplicate-flag cleanup, or broad biological duplicate collapse.
`forensic_inspect` is an evidence-driven provenance inspection command; it is not a structural validator, not duplicate marking, and not a fraud detector.
`consume` does not imply that heterogeneous upstream inputs were normalized unless the response explicitly reports a written BAM output, and it does not silently combine alignment-bearing and raw-read inputs across the alignment/unmapped boundary.
`annotate_rg` is a record-touching transformation and therefore more expensive than `reheader`; it does not silently act as a header-only command.
`reheader` does not imply any record-level `RG:Z` tagging change, full BAM validation, or true in-place editing unless the response explicitly reports a proven-safe in-place mode in a future slice.
`header` does not imply that alignment records are readable, that EOF is present, or that the full BAM body is valid.
`check_map` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_sort` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_index` does not imply that every random-access offset is correct or that the BAM and index are semantically matched beyond implemented structural checks.
`index` does not imply that CSI writing, indexed random access, or deeper index validation has completed unless the response explicitly reports those behaviors.
`summary` does not imply full BAM validity, valid EOF state, or validation of every optional field, tag, or record invariant.
`check_tag` does not imply full BAM validity, valid EOF state, or semantic correctness of tag values beyond the auxiliary-field traversal actually performed.
`validate` does not imply biological correctness, external reference concordance, or correctness of every optional-field semantic beyond the checks actually implemented.
`checksum` does not imply full BAM validity, biological correctness, or semantic equivalence under any mode other than the one explicitly reported in the response.
`sort` does not imply full BAM validity beyond what was parsed, semantic preservation unless checksum verification was actually performed, or index correctness unless index creation and inspection explicitly succeeded.
`merge` does not imply full validity of all inputs beyond what was parsed, semantic preservation unless checksum verification was actually performed, or index correctness unless index creation and inspection explicitly succeeded.
`fastq` does not imply BAM validation beyond the records parsed during export, pairing repair, read filtering, or preservation of BAM-only metadata in FASTQ output.
`unmap` does not imply biological remapping, realignment, or reference-independent validation of the source BAM.
`benchmark` does not imply broad comparator parity; each profile reports the exact command paths and comparison scope it ran.
`subsample` does not imply exact-count sampling, quality filtering, duplicate marking, provenance cleanup, or BAM index regeneration unless those behaviors are reported explicitly.

Milestone 10 is complete for native indexed-region workflow development. M10.2
defines the internal region grammar as `reference` and `reference:start-end`,
where interval input is 1-based closed and normalized internally to 0-based
half-open coordinates. M10.3 freezes the first region-aware workflow contracts
for `check_map --region <REGION>` and `summary --region <REGION>` support,
including `region_scope` JSON payload examples and fixture plans. M10.6
promotes `check_map --region <REGION>` to public command behavior, and M10.7
promotes `summary --region <REGION>` to public command behavior: usable BAI
sidecars drive indexed traversal, while missing, stale, unsupported, or
invalid index state falls back to native scan evidence with explicit scan
limits. M10.8 deliberately defers a public indexed region selection command:
`check_map --region <REGION>` and `summary --region <REGION>` are read-only
evidence surfaces, and no command currently promises to select, copy, or write
records for a region. Future selection work must first define output
semantics, header preservation, record ordering, duplicate-region behavior,
index invalidation or regeneration rules, and output write-safety behavior.
Region files also remain deferred.
The public contract commands `benchmark`, `fastq`, and `unmap` remain protected
while M10 work proceeds.
M10.4 adds internal validated-BAI chunk planning for those future workflows:
candidate bins are expanded, chunks are coalesced by typed virtual offsets, and
unsupported, stale, incompatible, or impossible index inputs are rejected before
indexed evidence can be claimed. This is not yet public CLI behavior.
M10.5 adds internal region-bounded traversal in `src/bam/region_traversal.rs`.
`traverse_planned_region_chunks` uses planned `VirtualOffset` chunk ranges and
`raw_records_in_virtual_range`, filters retrieved records against normalized
intervals by overlap, and deduplicate by virtual-offset range when broad BAI
bins or overlapping region requests return the same record more than once. The
fallback payload is explicit as `NativeScanRequired` when no usable index is
available. M10.6 wires that traversal into `check_map --region <REGION>` and
keeps region-scoped `region_records_examined` evidence separate from whole-file
mapping totals.
M10.7 wires the same traversal into `summary --region <REGION>` and keeps
region-scoped operational counts, observed fractions, MAPQ, flag categories,
and mapping status separate from whole-file totals.
M10.9 strengthens the dependency and benchmark guardrails for indexed-region
workflows: production direct `noodles` imports remain limited to documented
CRAM compatibility paths, and `scanner_microbench --bamana-bin` now emits
`check_map_region_scan_fallback`, `summary_region_scan_fallback`,
`check_map_region_indexed`, and `summary_region_indexed` smoke timings. Those
rows distinguish index lookup, BAI chunk planning, random-access traversal,
region filtering, scan fallback, command startup, and JSON emission without
claiming comparator parity, native CRAM indexed queries, biological
interpretation, or selected-record output.

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
cargo run -- enumerate --input reads.fastq.gz
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
cargo run -- check_map --bam example.bam --region chr1:100-200
cargo run -- check_sort --bam example.bam
cargo run -- check_sort --bam example.bam --sample-records 50000 --strict
cargo run -- check_index --bam example.bam
cargo run -- check_index --bam example.bam --require
cargo run -- index --input example.bam
cargo run -- index --input example.bam --format csi --out example.bam.csi
cargo run -- index --input reads.fastq.gz
cargo run -- index --input reads.fastq.gz --format gzi --out reads.fastq.gzi
cargo run -- summary --bam example.bam
cargo run -- summary --bam example.bam --sample-records 250000 --include-mapq-hist --include-flags
cargo run -- summary --bam example.bam --full-scan --prefer-index
cargo run -- summary --bam example.bam --region chr1:100-200 --include-flags
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
cargo run -- explode --input reads.fastq.gz --out-dir shards --explode 8
cargo run -- explode --input input.bam --out-dir bam_shards --explode 4
cargo run -- explode --input input.sam --out-dir sam_shards --explode 4
cargo run -- fastq --bam input.bam --out input.fastq.gz -j 8
cargo run -- unmap --bam aligned.bam --out aligned.unmapped.bam --dry-run
cargo run -- benchmark --profile fastq_gz_enumerate --fastq reads.fastq.gz --report fastq-gz-enumerate.pdf --force
```

`header` uses Bamana's native BGZF stream reader and native BAM header codec.
The binary BAM reference section is authoritative for reference names and
lengths, and optional fields from textual `@SQ` records are joined into the
structured JSON view when present. Non-fatal textual-vs-binary reference
mismatches are reported in `header.reference_diagnostics`; they do not rewrite
the binary reference dictionary used by downstream BAM decoding. A successful
`header` run does not imply that alignment records or the full BAM body are
valid.

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
conclusions. This command does not assert fraud or intent; it
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

`check_map` prefers index-derived mapping summaries when a usable BAI is
present. Usable means the selected sidecar is not timestamp-stale, passes the
implemented BAI structural checks, and supplies complete mapped/unmapped
metadata. Generated BAI sidecars and discovered BAI sidecars use the same
validation path. Without a usable index it falls back to native scan-based
evidence. Bounded scan mode is a fast assessment, not an exhaustive proof
unless full-scan mode is used.

`check_sort` preserves declared sort metadata from the BAM header and compares it
with observed ordering in a bounded record scan. Coordinate and queryname sorts
are the primary observed modes in this slice; specialized modes such as
template-coordinate or minimiser-related sub-sorts are preserved from the header
with limited observed confirmation.

`check_index` looks for adjacent companion indices using the repository's current
priority order and reports whether a selected index is BAI, CSI, or unknown.
Stale-index detection is heuristic and based on file modification times rather
than proof that every indexed offset still matches the BAM.

`index` now handles two distinct paths. For BAM it validates the input, creates
native BAI sidecars for coordinate-sorted BAM files, selects the default
`<bam>.bai` output path, writes through a temporary file, and enforces
overwrite rules. CSI creation is still explicitly unimplemented. For
`FASTQ.GZ` it writes a binary `FASTQ.GZI` sidecar, defaulting to `<input>.gzi`,
with checkpoints sampled at approximately 0.1% compressed-offset intervals and
pinned to completed FASTQ record boundaries rather than every read. The
`FASTQ.GZI` sidecar now stores header metadata, planner flags, and sampled
`(compressed_offset, uncompressed_offset, cumulative_records)`
checkpoint pairs, so enumerate can reuse an exact indexed record total,
explode can derive dense contiguous shard plans, and consume can size parallel
worker batches from the same sidecar.

`summary` combines BAM header metadata with a bounded scan by default and
switches to full-file totals only when EOF is actually reached or `--full-scan`
is used. When a usable BAI is available and `--prefer-index` is enabled,
index-derived mapped/unmapped totals are reported separately from scan-derived
record-category counts so the evidence source stays explicit. Stale,
unsupported, malformed, or incomplete BAI sidecars fall back to native scanner
evidence and are explained in the semantic note.

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
sorting while preserving duplicate multiplicity. It does not prove whole-file
semantic equivalence outside the reported filters, excluded tags, and checksum
domain. `header` hashes raw header text plus the binary reference dictionary in
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

`fastq` exports BAM alignment records as an ordered `FASTQ.GZ` stream. It emits
read names, sequences, and qualities in input encounter order and uses
concatenated gzip members so decode and compression work can run across worker
threads. FASTQ output cannot preserve BAM header records, reference
dictionaries, alignment flags, or auxiliary tags.

`unmap` rewrites a BAM into an unmapped BAM by clearing reference-bound header
state and stripping alignment coordinates, CIGAR data, mate coordinates,
template length, mapping quality, and mapping-related auxiliary tags from every
record. Non-mapping auxiliary tags are preserved. `--dry-run` reports the
planned rewrite and record counts without writing an output file.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## Current Status

Milestone 3 is complete. The repository now contains a production-minded native
BAM stack with shared JSON contracts, structured error handling, fast file
probing, native BGZF reading and writing, native BAM header parsing and
serialization, header microbenchmarks, real BGZF EOF inspection, and a native
BAM record scanner. `BamRecordView` defines the borrowed record-view contract,
and `BamScanner` provides the native BGZF/header record iteration substrate.
Scanner-facing helpers centralize common flag, coordinate, MAPQ, read-name,
sequence-length, section-range, skip-offset, and selected aux-tag access without
materializing richer record layouts. `check_sort`, `check_map`, `summary`,
`check_tag`, `validate`, `inspect_duplication`, and `forensic_inspect` now use
the scanner for scanner-compatible record traversal while preserving existing
JSON contracts. `scanner_microbench` provides records-per-second and selective
field-extraction JSON benchmarks for the native scanner. Full BAM semantic
validation, BAI/CSI random access, native CRAM scanning, writer-heavy BAM
transforms, and broader BAM operations remain incremental downstream work under
the project charter in
[`docs/project-charter.md`](docs/project-charter.md).

Milestone 4 is complete. Bamana now owns an explicit native FASTQ and FASTQ.GZ
parser/writer core with stable module boundaries, stronger validation,
sidecar-aware command-consumer evidence, dependency guardrails, and
`fastq_microbench` smoke coverage.

Milestone 5 is complete. `verify` and `header` are documented and tested as
native BGZF plus native BAM header proof commands, while `subsample` uses the
native BAM scanner path for BAM inputs and the native FASTQ/FASTQ.GZ
reader/writer APIs for raw-read inputs. Command contracts remained stable,
proof-command benchmark smoke evidence is recorded, and direct production
`noodles` usage remains isolated to CRAM compatibility.

Milestone 6 is complete. Bamana's first operational BAM inspection and
validation command wave is now hardened on native substrates:
`check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
`validate`. The closeout includes governed contracts and documentation,
bounded/full-scan evidence language, index-versus-scan distinctions, structural
validation caveats, command smoke benchmark evidence, and explicit
dependency-boundary protection for the M6 command paths.

Milestone 7 is complete. Bamana's native mutation, conservative remediation,
and provenance-inspection command wave is now hardened for `reheader`,
`annotate_rg`, `inspect_duplication`, `deduplicate`, and `forensic_inspect`,
with governed contracts, smoke benchmark evidence, output-safety guarantees,
and dependency-boundary protection.

Milestone 8 is complete. It hardened the large transform, checksum, explode,
and ingest command wave: `sort`, `merge`,
`explode`, `checksum`, and `consume`. BAM alignment consume uses
scanner-backed record loading through the native BGZF writer; SAM, FASTQ,
FASTQ.GZ, and CRAM retain separate native or explicitly documented
compatibility paths. The M8 writer commands publish completed temporary outputs
through final rename steps, reject collisions unless `--force` is supplied, and
keep checksum/index payloads limited to work actually performed. Dependency
guardrails explicitly protect the M8 command set from direct production
`noodles` imports outside CRAM compatibility, and scanner microbenchmark smoke
timings cover each M8 command without claiming external comparator parity.
Milestone 9 is complete for native BAM index and random-access groundwork.
It was activated by M9.1 after Milestone 8 closed. M9.1 recorded the
baseline: Bamana can detect and inspect BAI/CSI sidecars, create FASTQ.GZI
sidecars, and distinguish index-derived evidence from scan-derived evidence.
M9.3 adds scanner-exposed virtual offsets, M9.4 adds native in-memory BAI
bin/chunk/linear-index construction, M9.5 writes native BAI sidecars for
coordinate-sorted BAM input, and M9.6 hardens BAI structural validation in
`check_index`. M9.7 adds typed BGZF virtual-offset seeking and scanner helpers
that can read records from validated chunk ranges for internal consumers. M9.8
routes first consumer evidence through the hardened index usability rules:
`check_map` and `summary` use BAI metadata only when validation says it is
usable, and otherwise document native scan fallback. M9.9 adds dependency
guardrails and `scanner_microbench --bamana-bin` smoke timing rows for
`index_bam`, `check_index`, `check_map_indexed`, and `summary_indexed`, with
notes separating index construction, validation, metadata evidence, scan
fallback, random-access deferral, startup, JSON emission, and comparator
non-parity. M9.10 closes the milestone with full tests, contract tests,
Sphinx, and M9 benchmark smoke checks passing. CSI writing and public
indexed-region command acceleration remain deferred beyond M9.

Milestone 10 is complete for native indexed-region workflows. Bamana now owns
the bounded region syntax and normalization layer, validated BAI chunk
planning, random-access traversal over typed virtual offsets, and read-only
public region evidence for `check_map --region <REGION>` and
`summary --region <REGION>`. Those commands distinguish indexed traversal,
scan fallback, precise `invalid_region` rejection, and region-scoped evidence
from whole-file totals. `scanner_microbench --bamana-bin` records M10 smoke
timings for scan fallback and indexed-region paths. Region files, CSI
large-reference behavior, CRAM indexed queries, public selected-record output,
and broad comparator parity remain explicitly deferred.

Milestone 11 is complete for public indexed region selection and region files.
M11.1 freezes the selection surface decision as a planned new `select_region`
command, rather than overloading `check_map`, `summary`, or `subsample`. At
M11.1 activation, no new CLI behavior was public yet: output semantics, header
preservation, record ordering, duplicate-region behavior, region-file syntax,
index invalidation, and write-safety had to be specified before
implementation.
M11.2 freezes the future region-file syntax as UTF-8, one M10-style region per
non-comment line, with blank lines and leading `#` comment lines ignored, LF or
CRLF accepted, request order preserved, and duplicate or overlapping lines not
merged. Region-file parsing remains non-public until `select_region` is
implemented; malformed lines, unknown references, empty/reversed intervals,
BED-like rows, 0-based half-open files, open-ended ranges, and tabular metadata
must be rejected precisely.
M11.3 freezes selected-record output semantics without making `select_region`
runnable. Future applied runs are BAM-only: BGZF-compressed BAM input,
BGZF-compressed BAM output, explicit `--out`, JSON report to stdout for file
outputs, and mandatory `--report <path>` when `--out -` sends binary BAM to
stdout. SAM, CRAM, FASTQ, FASTQ.GZ, FASTA, text output, uncompressed BAM, and
alternate compression modes remain rejected or unpromoted. Dry runs write no
BAM output and must report `dry_run: true` and `output_created: false`.
M11.4 freezes selected-output header semantics without making `select_region`
runnable. Future output preserves the full binary reference dictionary and
textual header records, keeps unselected references in the header, appends only
a collision-free `@PG` provenance record for `bamana select_region`, rewrites
existing `@HD SO` to `unknown`, removes `@HD SS`, and reports input/output sort
metadata so selected output does not overclaim coordinate or queryname order.
M11.5 freezes duplicate and overlapping-region output policy without making
`select_region` runnable. Future selected BAM output emits each physical source
record at most once, ordered by source BAM virtual offset, while preserving
region request order only in metadata. Repeated regions, region-file duplicate
lines, overlapping intervals, adjacent chunks, and broad BAI bins suppress
duplicate physical records rather than repeating them in the output.
M11.6 implements the first runnable `select_region` slice for BGZF BAM file
output with CLI `--region` values: usable non-stale BAI sidecars drive native
indexed traversal, unusable index state falls back to native scanning, selected
records preserve raw BAM record bytes, output uses `source_virtual_offset_order`
and `emit_once_per_source_record`, and the header preserves the reference
dictionary while downgrading `@HD SO` to `unknown`, removing `SS`, and appending
`@PG` provenance. Binary stdout output via `--out -`, public `--region-file`,
and governed schemas/examples/fixtures remain deferred to later M11 tasks.
M11.7 completes the current `select_region` file-output safety contract:
same-path input/output rewrites are rejected, existing BAM outputs and adjacent
output index sidecars are collisions unless `--force` is supplied, forced
applied runs remove stale adjacent `.bai`/`.csi` sidecars before writing
selected BAM output, dry runs report planned index invalidation without
removing files, and the response reports `output.index_invalidation` with
`output_index_created: false` plus `bamana index --input <output.bam>`
regeneration guidance.
M11.8 adds governed public-contract artifacts for the implemented
`select_region` file-output surface: `spec/jsonschema/select_region.schema.json`,
canonical success and failure examples, JSON-output documentation, CLI contract
coverage, and fixture-plan reservations for indexed success, scan fallback,
duplicate/overlap suppression, output-index sidecar collisions, forced sidecar
removal, and same-path rejection. Binary stdout output and public
`--region-file` remain deferred.
M11.9 adds selected-region output guardrails: production direct `noodles`
imports remain limited to documented CRAM compatibility paths, and
`scanner_microbench --bamana-bin` now emits `select_region_scan_fallback` and
`select_region_indexed_output` smoke timings for
`select_region --bam <input.bam>`. Those rows distinguish scan fallback, BAI
chunk planning, random-access traversal, selected-record filtering, duplicate
suppression, raw-record preservation, BGZF BAM file writing, temporary-output
finalization, header provenance, index-invalidation reporting, command startup,
and JSON emission without claiming stdout-output evidence, public region-file
evidence, replacement output-index evidence, comparator-parity claims, native
CRAM indexed-query support, or biological interpretation.
M11.10 closes Milestone 11 after M11.1 through M11.10 completed on
2026-05-28. Closeout verification covered full tests, contract tests, Sphinx
HTML documentation, formatting, whitespace checks, binary builds, and the
`scanner_microbench --profile small --iterations 1 --bamana-bin` smoke profile.
The milestone leaves `select_region` governed for BGZF BAM file output from CLI
`--region` requests, with public `--region-file`, `--out -`, replacement output
index creation, CSI large-reference behavior, native CRAM indexed queries, and
broad comparator parity still deferred.

Milestone 12 is active for extended index compatibility. M12.1 records the
baseline without changing CLI behavior: BAI detection, parsing, structural
validation, mapped/unmapped metadata extraction, timestamp-staleness checks,
and native BAI writing are implemented for coordinate-sorted BAM inputs;
`check_index` discovers adjacent BAI, CSI, GZI, and unknown sidecars; CSI
remains header-only detected-but-not-supported for random-access use; `index
--format csi` remains explicitly unimplemented; `check_map`, `summary`, and
`select_region` remain BAI-first with native scan fallback for missing, stale,
unsupported, malformed, or incomplete index state; FASTQ.GZI remains a
FASTQ.GZ planning sidecar and is not a BAM random-access index.

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
