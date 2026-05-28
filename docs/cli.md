# Bamana CLI Contract

Bamana is a JSON-first CLI. The CLI itself is part of the governed public
surface and is intended to remain stable for workflow engines, validators, and
controlled operational environments.

The detailed command contract is maintained in:

* [spec/cli/commands.md](/Users/stephen/Projects/bamana/spec/cli/commands.md)
* [spec/cli/global-options.md](/Users/stephen/Projects/bamana/spec/cli/global-options.md)
* [spec/cli/exit-codes.md](/Users/stephen/Projects/bamana/spec/cli/exit-codes.md)

## Core Rules

* Bamana emits JSON only.
* `--json-pretty` affects formatting only.
* Command names and option names are public contract elements.
* Contract changes require schema/example/doc updates.

## Implemented And Planned Commands

The spec layer covers both:

* implemented commands already present in the CLI
* implemented commands already present in the CLI, including `explode`

This separation is deliberate: repository-facing contract design should not wait
for every implementation detail to be finished.

`benchmark`, `fastq`, and `unmap` are public contract commands. They are not
experimental aliases: each command must have a JSON schema, canonical success
and failure examples, user-facing documentation, and contract tests that keep
the public surface visible.

`subsample` is now an implemented command for BAM, FASTQ, and FASTQ.GZ inputs.
It provides seeded random Bernoulli-style subsampling and deterministic
hash-based subsampling with explicit identity semantics. The command preserves
encounter order of retained records, emits JSON only, and is designed to
support both production workflows and reproducible benchmarking.

`enumerate` is the simple record-counting command. It accepts a single
`BAM`, `SAM`, `FASTQ`, `FASTQ.GZ`, or `FASTA` input via `--input`, emits JSON
only, and reports a top-level record count derived from format-aware streaming
parsing rather than filename heuristics. For `FASTQ.GZ`, the first enumerate
run now materializes a sibling `FASTQ.GZI` sidecar and later enumerate runs
reuse the exact record total stored in that sidecar.

`index` is now format-aware. It still validates BAM inputs and reports honest
BAI/CSI writer limitations, but it also creates sampled `FASTQ.GZI` sidecars
for `FASTQ.GZ` inputs via `--input`. The default `FASTQ.GZI` rule now places
checkpoints at approximately 0.1% compressed-offset intervals, pins each
checkpoint to the next completed FASTQ record boundary, and stores cumulative
record totals plus planner metadata for exact indexed enumeration, explode
planning, and consume planning.

`explode` is now implemented for `BAM`, `SAM`, and `FASTQ.GZ`. It accepts one
input via `--input`, writes shards under `--out-dir`, and splits by contiguous
encounter-order record ranges so the original read or alignment order is
preserved within every shard and every sequence lands in exactly one shard.
The `FASTQ.GZ` path uses adjacent `FASTQ.GZI` metadata for checkpoint-aligned
shard planning and parallel gzip-member compression, so shard sizes are only
as uniform as the available cutpoints allow. `BAM` and `SAM` use conservative
contiguous-range splitting in the current slice.

`fastq` is the BAM-to-FASTQ.GZ export command. It accepts one BAM via `--bam`,
writes a single ordered FASTQ.GZ stream, and reports records read, records
written, and worker-thread usage. It preserves read names, sequences, and
qualities in input encounter order; it does not preserve BAM header metadata,
alignment fields, or auxiliary tags in the FASTQ output.

`unmap` rewrites one BAM as unmapped BAM. It removes reference dictionary state
from the output header and strips reference-bound alignment state from each
record while preserving non-mapping auxiliary metadata. `--dry-run` is a
first-class planning path and reports record counts without writing output.

`header` parses the BAM header through Bamana's native BGZF stream reader and
native BAM header codec without scanning alignment records. The binary
reference dictionary is authoritative for reference names, lengths, and order;
textual `@SQ` mismatches are reported as non-fatal
`header.reference_diagnostics` entries. A successful response proves only that
the BGZF container and BAM header prefix/reference dictionary were readable
enough to parse; it does not validate the BAM body.

`verify` performs the same native BGZF plus BAM header-level structural check
as a command contract. It confirms BGZF container recognition, BAM magic, and
native header/reference-dictionary parsing. It does not scan alignment records
and does not report BGZF EOF-marker presence; use `check_eof` for EOF-marker
checks and `validate` for deeper BAM structure.

Milestone 6 inspection commands are governed as one public command wave:
`check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
`validate`. `check_eof` reports EOF marker presence or absence only; it does
not prove BAM header validity or alignment-record validity. `check_sort`,
`check_map`, `summary`, and `check_tag` report bounded evidence unless their
output states that a full scan reached EOF cleanly. `check_sort --strict`
expands ordering inspection beyond the bounded sample window but still does not
perform full BAM structural validation. `check_map` and `summary` keep
index-derived evidence separate from scan-derived evidence. For `check_map`,
index-derived results describe usable BAI mapped/unmapped metadata; scan-derived
results describe only the examined alignment records. For `summary`, bounded
output reports observed operational metrics only; full-file totals require
`evidence.full_file_scanned: true` and are not BAM structural validation.
`check_tag` reports scanner-owned auxiliary traversal evidence only: bounded
non-observation is not full-file absence, duplicate tags are counted at the
record level, and unsupported aux shapes are structured traversal failures.
`validate` reports structural and internal-consistency checks over the requested
scope, including aux traversal structure, not biological correctness, external
reference concordance, or full optional-field semantic validation.

Milestone 8 transform and ingest commands are governed as one public command
wave: `sort`, `merge`, `explode`, `checksum`, and `consume`. `sort` and
`merge` report explicit ordering semantics, in-memory first-slice caveats,
deferred index behavior, and checksum-verification state. `checksum` reports
explicit checksum domains, algorithms, filters, excluded tags, and whether the
reported domain is order-sensitive. `explode` reports shard boundaries and
`FASTQ.GZI` planning evidence without claiming uniform shard sizes or generic
random-access gzip inflate. `consume` reports ingest mode, dry-run discovery,
mixed-format policy decisions, CRAM reference policy, deferred checksum
verification, and deferred post-ingest index behavior. BAM alignment consume
uses scanner-backed record loading, while SAM, FASTQ, FASTQ.GZ, and CRAM keep
separate native or documented compatibility paths.
The M8 writer commands publish completed temporary files through final rename
steps, so `written: true` should be interpreted as completed command reporting
rather than early write intent.

Milestone 9 is complete for native BAM index and random-access groundwork. Current
`index` and `check_index` behavior remains intentionally conservative:
FASTQ.GZ indexing creates FASTQ.GZI sidecars, BAM index creation writes native
BAI sidecars for coordinate-sorted BAM inputs, and BAM index inspection covers
adjacent sidecar discovery, BAI structural checks, CSI header detection, and
timestamp-based staleness. `index` refuses to overwrite existing sidecars
unless `--force` is supplied, and BAM BAI responses report
`output_index.created: true` only after the sidecar is finalized.
CSI writing remains deferred for BAM inputs and still returns `unimplemented`.
`check_map` and `summary` may use parsed BAI metadata as index-derived
evidence only when the selected sidecar is not timestamp-stale, passes the
implemented BAI structural checks, and supplies complete mapped/unmapped
metadata for the needed references. Generated BAI sidecars and discovered BAI
sidecars follow the same validation path because BAI carries no provenance
marker. When the sidecar is absent, stale, unsupported, malformed, or
incomplete, both commands fall back to native scanner evidence and explain that
fallback in the payload note. Public indexed random-access acceleration is not
claimed until a later task promotes the M9.7 internal virtual-offset seek and
chunk traversal helpers into command behavior.

Milestone 10 is complete for indexed-region workflow development. M10.2 defines
the internal region string grammar as `reference` for a whole-reference request
and `reference:start-end` for a 1-based closed interval normalized to 0-based
half-open coordinates. Multiple regions preserve request order and are not
merged or deduplicated yet. M10.3 freezes `check_map --region <REGION>` and
`summary --region <REGION>` as the first planned region-aware command
surfaces, with `region_scope` JSON schema and examples. M10.6 promotes
`check_map --region <REGION>` to public command behavior, and M10.7 promotes
`summary --region <REGION>` to public command behavior: usable BAI sidecars
drive indexed traversal, while missing, stale, unsupported, or invalid index
state falls back to native scan evidence with explicit scan limits. Region
files remain explicitly deferred. M10.8 also deliberately defers a public
indexed region selection command: `check_map --region <REGION>` and
`summary --region <REGION>` are read-only evidence surfaces, and no synopsis is
public for selecting, copying, or writing records by region. Future selection
work must first define output semantics, header preservation, record ordering,
duplicate-region behavior, index invalidation or regeneration rules, and
output write-safety behavior. The
public contract commands `benchmark`, `fastq`, and `unmap` remain protected
while this region surface is developed.

M10.4 adds internal BAI chunk planning for those future region workflows. The
planner expands normalized intervals to candidate BAI bins, collects chunks
from validated `BaiIndex` structures, coalesces overlapping virtual-offset
ranges, preserves provenance, and rejects unsupported, stale, incompatible, or
impossible index inputs before indexed evidence may be reported. This is still
not a public CLI behavior change.

M10.5 adds the internal traversal baseline in `src/bam/region_traversal.rs`.
`traverse_planned_region_chunks` consumes planned `VirtualOffset` ranges through
`raw_records_in_virtual_range`, filters records against normalized intervals by
overlap, and deduplicate by virtual-offset range so broad bins and overlapping
region requests do not double-count records. Missing or unusable index state is
represented explicitly as the scan fallback `NativeScanRequired`. M10.6 wires
that traversal into `check_map --region <REGION>` and keeps
`region_records_examined` separate from whole-file scan counters and index
metadata totals.
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

Milestone 11 is active for public indexed region selection and region files.
M11.1 freezes the selection surface decision as a planned new `select_region`
command, not an extension of `check_map`, `summary`, or `subsample`. No
`select_region` CLI synopsis is public yet. Later M11 tasks must define output
semantics, header preservation, record ordering, duplicate-region behavior,
region-file syntax, index invalidation, and write-safety before any selected
record output is implemented.
M11.2 freezes the future region-file input contract without publishing a
`select_region` synopsis. Future region files are UTF-8, one M10-style
`reference` or `reference:start-end` region per non-comment line, with LF or
CRLF line endings, surrounding whitespace trimmed, blank lines ignored, and
leading `#` comment lines ignored. Request order is preserved, duplicate and
overlapping lines are not merged by parsing, and unsupported coordinate models
such as BED-like rows, 0-based half-open rows, open-ended ranges,
comma-separated ranges, strand/name columns, or other tabular metadata must be
rejected before output is written.
M11.3 freezes selected-record output semantics without making `select_region`
runnable. The planned data stream is BAM-only: BGZF-compressed BAM input,
BGZF-compressed BAM output, explicit `--out`, JSON report to stdout for file
outputs, and mandatory `--report <path>` when `--out -` writes binary BAM to
stdout. SAM, CRAM, FASTQ, FASTQ.GZ, FASTA, text output, uncompressed BAM, and
alternate compression modes are not promoted. Dry runs write no BAM output and
must report `dry_run: true`, `output_created: false`, the requested output
target, region source, intended execution mode, and planned rejection or
fallback state.

`consume` now uses the thread count for raw-read import. `FASTQ.GZ` inputs are
parallelized across files when multiple gzip inputs are present, and a single
indexed `FASTQ.GZ` input uses worker-batch conversion guided by the adjacent
`FASTQ.GZI` checkpoint totals. `-j 0` means use all available cores, while
`-j 1` remains the deterministic fallback for otherwise parallelisable
`FASTQ.GZ` ingestion.

`consume` is the ingestion gateway into Bamana. It is the command that accepts
files and directories containing supported upstream formats and normalizes them
into a single BAM according to an explicit ingest mode. The current staged
implementation supports alignment-mode BAM/SAM/CRAM normalization and unmapped
FASTQ/FASTQ.GZ normalization, with deterministic directory discovery and dry-run
support. CRAM is available only in alignment mode and is governed by an
explicit `--reference-policy`; Bamana does not silently guess CRAM reference
behavior. Its detailed contract is documented in
[spec/cli/commands.md](/Users/stephen/Projects/bamana/spec/cli/commands.md).

`annotate_rg` is the record-level read-group annotation command. It rewrites
alignment records to insert, replace, or normalize `RG:Z:` aux tags and can
explicitly require or create a matching `@RG` header line. It is intentionally
distinct from `reheader`: `annotate_rg` touches records, while `reheader`
modifies only header metadata.

`reheader` is the header-only BAM metadata mutation command. It updates the BAM
header and only the BAM header. The current slice supports full header
replacement from a SAM-style header file plus targeted `@RG`, `@PG`, and `@CO`
mutations. It plans true in-place feasibility conservatively and executes via a
rewrite path in this slice. It does not add, remove, or replace per-record
`RG:Z` tags in alignment records.

`inspect_duplication` is the collection-duplication inspection command. It
accepts a single BAM, FASTQ, or FASTQ.GZ input via `--input`, emits JSON only,
and is explicitly scoped to suspicious collection mishandling signatures such as
exact repeated records, adjacent repeated blocks, and whole-file append
patterns. It is not ordinary PCR duplicate marking, does not use BAM duplicate
flags as primary evidence, and reports findings with explicit confidence and
evidence-strength fields rather than a flat duplicate count.

`deduplicate` is the conservative remediation companion to
`inspect_duplication`. It accepts a single BAM, FASTQ, or FASTQ.GZ input plus
an explicit output path, emits JSON only, and is explicitly scoped to removing
contiguous duplicated collection blocks under a selected policy. The first slice
focuses on adjacent repeated blocks and whole-file append signatures, requires
an explicit remediation mode, and keeps molecular duplicate semantics out of
scope.

`forensic_inspect` is the provenance-inspection companion to `validate`,
`inspect_duplication`, and `deduplicate`. The first slice is BAM-first and
inspects header structure, read-group usage, program-chain anomalies, read-name
regime changes, duplicate-block hallmarks, and optional aux-tag regime shifts.
It is explicitly not a structural validator, not duplicate marking, and not a
fraud detector; it emits evidence-driven findings with conservative follow-up
recommendations.

The repository also contains a benchmark framework under
[benchmarks/](/Users/stephen/Projects/bamana/benchmarks). It uses Nextflow,
containerized toolchains, replicated benchmark runs, and R-based aggregation to
compare Bamana against `samtools`, `fastcat`, and other relevant comparators
without forcing unsupported workflows into misleading timing results.

`benchmark` is the owned operator entry point for selected benchmark profiles.
It builds the local release binary, builds the benchmark container, runs the
selected profile, captures logs and machine-readable outputs, and renders the
requested report. Benchmark-profile operator documentation for
`bamana benchmark --profile ...` lives under
[sphinx/index.rst](/Users/stephen/Projects/bamana/docs/sphinx/index.rst).
