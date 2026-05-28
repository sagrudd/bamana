# Milestone 11: Public Indexed Region Selection And Region Files

Status: complete as of 2026-05-28. Milestone 11 follows the completed
Milestone 10 read-only indexed-region evidence surface. M11.1 activated this
milestone without changing CLI behavior, and M11.10 closed it after M11.1
through M11.10 completed.

## Goal

Promote selected-record indexed region output only after the public contract is
precise. M10 proved region parsing, BAI chunk planning, random-access
traversal, scan fallback, and read-only `check_map --region <REGION>` and
`summary --region <REGION>` evidence. M11 adds the contract runway for selected
BAM record output and region-file input as governed public behavior.

## M11.1 Activation Decision

M11.1 freezes the selection surface decision: M11 will specify a new planned
public command named `select_region`. It must not overload `check_map`,
`summary`, or `subsample`.

The command is not implemented by M11.1. No public `select_region` CLI synopsis
exists until M11.3 through M11.8 define output semantics, header preservation,
record ordering, duplicate-region behavior, region-file behavior, index
invalidation or regeneration notes, and write-safety. Until those tasks complete,
`check_map --region <REGION>` and `summary --region <REGION>` remain the only
public region-aware behavior, and they remain read-only evidence surfaces.

Baseline substrate inherited from M10:

* `src/bam/region.rs` owns bounded region string parsing and normalization.
* `src/bam/region_plan.rs` owns validated BAI chunk planning.
* `src/bam/region_traversal.rs` owns random-access traversal, interval
  filtering, and duplicate virtual-offset suppression.
* `src/commands/check_map.rs` and `src/commands/summary.rs` prove the read-only
  region evidence payload contract.
* `src/bam/write.rs`, `src/output_safety.rs`, and existing writer commands
  provide patterns for later selected-record output and collision safety.

The M11 public contract must explicitly keep native CRAM indexed queries, CSI
large-reference support, biological interpretation, pileup/genotyping behavior,
and broad external comparator parity out of scope unless a later task promotes
one of them.

## M11.2 Region-File Syntax And Rejection Contract

M11.2 specifies the region-file contract, but it does not implement public
`select_region` CLI behavior. Until the command is implemented, any direct
region-file request must fail as unimplemented with a deterministic message
that region-file parsing is specified for M11.2 but is not yet public CLI
behavior.

Accepted future region files are UTF-8 text files with one region per
non-comment line. LF and CRLF line endings are accepted. Each line is trimmed
for surrounding ASCII whitespace before classification. Blank lines are
ignored. A comment line is any line whose first non-whitespace character is
`#`; inline comments are not recognized, so `#` after a region is part of the
region token and must be rejected unless it is a literal reference-name match.

Each non-comment line uses the same grammar as M10 CLI region strings:

* `reference` for a whole-reference request;
* `reference:start-end` for an explicit interval;
* interval coordinates are 1-based closed in the file;
* normalized output remains 0-based half-open.

Region-file order is request order. Duplicate lines and overlapping intervals
are preserved by parsing and normalization; later M11 duplicate-output policy
work decides whether selected records are emitted once or repeated. Region-file
parsing must not sort, merge, or deduplicate request lines.

Rejection behavior is precise:

* unreadable, missing, or non-UTF-8 files fail before any output is written;
* malformed lines fail with the region file path and 1-based line number;
* empty or comment-only files, including files with only blank lines, fail as
  `invalid_region`;
* unknown references, ambiguous references, zero-length whole-reference
  requests, empty intervals, zero coordinates, reversed intervals, out-of-range
  intervals, and non-numeric coordinates reuse the native `invalid_region`
  taxonomy;
* unsupported coordinate models are rejected, including BED-like
  `chrom start end` rows, 0-based half-open interval files, comma-separated
  ranges, open-ended ranges, strand/name columns, and other tabular metadata.

No JSON schema or example output is introduced by M11.2 because there is still
no public `select_region` command or report payload. M11.8 must add schemas,
examples, and fixtures once the command contract is fully specified.

## M11.3 Selected-Record Output Semantics

M11.3 specifies selected-record output semantics, but it does not implement
`select_region` and does not make the command runnable. The planned invocation
shape is:

`bamana select_region --bam <input.bam> (--region <REGION> ... | --region-file <regions.txt>) --out <output.bam|-> [--report <report.json>] [--dry-run] [--force]`

The selected-record data stream is BAM-only:

* input must be BGZF-compressed BAM;
* SAM, CRAM, FASTQ, FASTQ.GZ, FASTA, and unknown inputs are rejected before any
  output is written;
* output is BGZF-compressed BAM for both file output and stdout output;
* no SAM, CRAM, FASTQ, FASTQ.GZ, text, or uncompressed BAM output mode is
  introduced by M11.3;
* output compression uses the native BGZF writer default unless a later task
  explicitly promotes compression controls.

Applied runs require an explicit `--out`. File output writes the selected BAM to
the requested path and emits a JSON report to stdout. `--report <report.json>`
additionally writes the same JSON report to a file. Existing output collision
rules and `--force` behavior are specified by M11.7, so M11.3 does not weaken
output-safety requirements.

`--out -` writes the selected BGZF BAM stream to stdout. Because stdout is then
binary BAM, `--out -` requires `--report <report.json>` and must reject
`--report -` or an omitted report path. Human diagnostics must use stderr.

Dry runs never write BAM output, never create or replace a report sidecar unless
`--report <report.json>` is explicitly supplied, and must report
`dry_run: true`, `output_created: false`, the requested output target, region
source, intended execution mode, and planned rejection or fallback state.
Dry-run stdout is JSON unless a report file is supplied and later CLI contract
work chooses to suppress stdout.

The future JSON report must distinguish at least:

* `command: "select_region"` and `dry_run`;
* input path, output target, and report destination;
* `output_format: "bam"` and `compression: "bgzf"`;
* region source (`cli_regions` or `region_file`) and normalized region count;
* index execution mode (`indexed`, `scan_fallback`, or `rejected`);
* selected-record counts, records examined, chunks traversed, and fallback
  reason when applicable;
* notes that selected-record output is not biological interpretation and does
  not imply whole-file validation.

No JSON schema, golden example, or fixture is introduced by M11.3 because
header behavior, record ordering, duplicate policy, and write-safety remain
unfrozen. M11.8 must add governed schemas, examples, and fixtures once those
contracts are complete.

## M11.4 Header Preservation And Sort-Order Contract

M11.4 specifies the future selected-output header contract, but it does not
implement `select_region` and does not make the command runnable.

Selected BAM output must preserve the input BAM reference dictionary exactly:

* binary reference dictionary order, names, lengths, and reference indexes are
  authoritative and must be copied without filtering to only selected
  references;
* record `refID`, `next_refID`, CIGAR, mate, auxiliary, and read-group
  references remain valid against the unchanged dictionary;
* textual `@SQ` records are preserved in encounter order and must continue to
  reconcile with the binary reference dictionary;
* references with no selected records remain in the output header.

The textual header must be preserved conservatively. `@RG`, existing `@PG`,
`@CO`, and unknown SAM-style header records are retained. The only permitted
selected-output header mutation in M11.4 is command provenance:

* append a new `@PG` record for `bamana select_region`;
* generate a collision-free `ID`;
* set `PN:bamana`;
* include the Bamana version when available;
* set `PP` to the previous terminal program only when the program chain is
  unambiguous;
* never remove, rewrite, or reorder unrelated header records for provenance.

Sort metadata is intentionally conservative. Region selection can emit records
from multiple intervals, repeated intervals, overlapping intervals, region files,
or scan fallback, so the output must not claim coordinate or queryname order
until a later task proves a narrower case:

* if an input `@HD` record exists, selected output rewrites `SO` to `unknown`;
* selected output removes `SS` from `@HD`;
* if no input `@HD` exists, M11.4 does not require synthesizing one solely for
  sort metadata;
* the JSON report records input `@HD` `SO`/`SS`, output `@HD` `SO`/`SS`, and a
  note that sort-order metadata was downgraded because selected-region output
  is not guaranteed to preserve whole-file order.

M11.4 does not add a JSON schema, golden example, or fixture because duplicate
and overlapping-region emission policy, final record ordering, and write-safety
remain unfrozen. M11.8 must add governed schemas, examples, and fixtures once
those contracts are complete.

## M11.5 Duplicate And Overlapping Region Policy

M11.5 specifies record ordering and duplicate/overlap behavior, but it does not
implement `select_region` and does not make the command runnable.

Selected output emits each physical BAM alignment record at most once. Physical
record identity is the source BAM virtual-offset range for the raw record.
Repeated region strings, repeated region-file lines, overlapping intervals,
adjacent BAI chunks, or broad bins that discover the same record more than once
must not duplicate that record in the selected BAM output.

Record order is deterministic source order:

* selected records are emitted in ascending source BAM virtual-offset order;
* scan fallback emits records in native BAM encounter order, which is the same
  source-order policy;
* region request order is preserved in normalized request metadata but does not
  control output ordering;
* region-file line order is preserved in normalized request metadata but does
  not cause selected records to be repeated;
* multi-reference requests do not create per-region output blocks; all selected
  records share one source-order output stream.

When a record matches more than one requested interval, the future report must
retain all matched normalized region identifiers for that record or for the
selection evidence path. The selected BAM output still contains one copy of the
record. The report must distinguish:

* requested region count;
* normalized region count;
* duplicate region request count;
* overlapping region request count when detectable;
* selected unique record count;
* duplicate physical records suppressed;
* output ordering policy: `source_virtual_offset_order`;
* duplicate emission policy: `emit_once_per_source_record`.

M11.5 intentionally does not support a request-order repeated-output mode,
per-region BAM block output, or one-output-file-per-region behavior. Those
would require a separate future contract because they change duplicate
multiplicity and sorting claims.

M11.5 does not add a JSON schema, golden example, or fixture because
write-safety and final command implementation remain pending. M11.8 must add
governed schemas, examples, and fixtures once the remaining contracts are
complete.

## M11.6 Native Selected-Record Writing

M11.6 implements the first runnable `select_region` slice for file output:

`bamana select_region --bam <input.bam> --region <REGION> --out <output.bam> [--dry-run] [--force] [--prefer-index]`

The command accepts BGZF-compressed BAM input and repeated CLI `--region`
values. It writes BGZF-compressed BAM file output through Bamana's native BGZF
writer. For selected records, the alignment record payload is copied from the
source record's raw BAM bytes; selection does not reserialize alignment fields
or reinterpret auxiliary tags.

When `--prefer-index` is enabled, the command uses a usable, non-stale adjacent
BAI sidecar for native indexed traversal through the M10 chunk planner and
random-access traversal helpers. Missing, stale, unsupported, malformed, or
incomplete index state falls back to the native scanner. Both execution modes
emit selected records in `source_virtual_offset_order` and use
`emit_once_per_source_record`, so repeated regions, overlapping intervals, and
broad BAI bins do not duplicate a physical source record in the output.

The output header follows the M11.4 contract. The full binary reference
dictionary is preserved, including references with no selected records. Textual
header records are retained except for conservative sort metadata changes: an
existing `@HD` has `SO` rewritten to `unknown` and `SS` removed. A
collision-free `@PG` record for `bamana select_region` is appended with
`PN:bamana`, the Bamana version, and `PP` only when the previous program chain
has one unambiguous terminal program.

Dry runs perform selection planning and report counts without writing BAM
output. File-output applied runs report the selected output path, BGZF/BAM
format, selected unique record count, duplicate suppression, execution mode,
index path when used, fallback reason when applicable, and header policy.

The M11.6 implementation intentionally leaves three contract items for later
M11 tasks:

* `--out -` binary BAM stdout routing is rejected until the response/report
  separation is implemented.
* `--region-file` remains syntax-specified by M11.2 but is not yet a public CLI
  flag.
* final output index invalidation/regeneration semantics, governed JSON
  schemas, golden examples, and fixtures remain M11.7 and M11.8 work.

## M11.7 Write-Safety And Index Invalidation

M11.7 completes the file-output safety contract for the current `select_region`
slice. Applied runs continue to write through a temporary BGZF BAM and publish
only after the writer finishes. Existing BAM output paths are refused unless
`--force` is supplied. Same-path input/output rewrites are rejected even with
`--force`; operators must write selected records to a distinct BAM path.

Adjacent output index sidecars are part of the output safety contract. Before
an applied run writes selected BAM output, Bamana checks all conventional BAM
index sidecar names for the requested output:

* `<out>.bai`;
* `<out>.csi`;
* `<out with .bai extension>`;
* `<out with .csi extension>`.

If any of those sidecars already exist and `--force` is not supplied, the
command fails with `output_exists` before writing BAM output. With `--force`,
pre-existing adjacent output index sidecars are removed before selected BAM
output is written. This is intentionally conservative: selected-region output
changes record membership and header provenance, so any existing BAI or CSI
beside the output path cannot be trusted for the new file.

`select_region` does not create a replacement output index in M11.7. The
response reports `output.index_invalidation` with adjacent index candidates,
pre-existing sidecars, removed sidecars, the invalidation action,
`output_index_created: false`, and a regeneration command of the form
`bamana index --input <output.bam>`. Dry runs report the sidecars that would be
removed by an applied forced run but do not remove files.

M11.7 does not add public schemas, golden examples, or fixtures; M11.8 remains
responsible for governing the final `select_region` JSON schema, examples, and
fixture plan.

## M11.8 Public Contracts, Schemas, Examples, And Fixtures

M11.8 adds governed public-contract artifacts for the implemented
`select_region` file-output surface. The command now has:

* `spec/jsonschema/select_region.schema.json`;
* `spec/examples/select_region.success.json`;
* `spec/examples/select_region.failure.json`;
* JSON-output documentation for the `output`, `index_invalidation`,
  `region_scope`, `execution`, and `header` payload blocks;
* CLI contract text for supported options and current deferrals;
* fixture-plan reservations for indexed success, scan fallback,
  duplicate/overlap suppression, output-index sidecar collision, forced sidecar
  removal, and same-path rejection.

The M11.8 schema governs the current file-output behavior only:
`--bam <input.bam>`, repeated CLI `--region <REGION>`, `--out <output.bam>`,
`--dry-run`, `--force`, and `--prefer-index`. Binary stdout output via
`--out -`, public `--region-file`, report sidecar routing, replacement output
index creation, and broader comparator parity remain outside the current
schema until later tasks promote them.

## M11.9 Dependency And Benchmark Guardrails

M11.9 protects selected-region output hot paths from dependency drift and adds
command-level smoke timings for the current file-output slice.

The dependency guardrail explicitly names:

* `select_region_scan_fallback_output`;
* `select_region_indexed_output`;
* `select_region_header_and_index_invalidation`.

Those paths cover the command wrapper, region parser, scan fallback, BAI chunk
planning, random-access traversal, selected-record filtering, duplicate
suppression, raw-record preservation, BGZF BAM file writing, temporary-output
finalization, header provenance, and index-invalidation reporting. Production
direct `noodles` imports remain limited to documented CRAM compatibility
paths.

`scanner_microbench --bamana-bin` now emits two M11 timing rows for
`select_region --bam <input.bam>`:

* `select_region_scan_fallback`, before the generated BAI sidecar exists;
* `select_region_indexed_output`, after `index_bam` has created the BAI
  sidecar.

These are smoke timings over deterministic synthetic BAM fixtures. They
distinguish scan fallback, BAI chunk planning, random-access traversal,
selected-record filtering, duplicate suppression, raw-record preservation,
BGZF BAM file writing, temporary-output finalization, header provenance,
index-invalidation reporting, command startup, and JSON emission. They do not
claim stdout-output evidence, public region-file evidence, replacement
output-index evidence, comparator-parity claims, native CRAM indexed-query
support, biological interpretation, or exhaustive index-offset validation.

## M11.10 Closeout

Milestone 11 is complete as of 2026-05-28. M11.1 through M11.10 are complete,
and the closeout verification passed:

* `cargo test`;
* `cargo test --test contract`;
* `cargo build --bin bamana --bin scanner_microbench`;
* `target/debug/scanner_microbench --profile small --iterations 1 --bamana-bin target/debug/bamana --out /tmp/bamana-m1110-scanner-small.json`;
* `sphinx-build -b html docs/sphinx docs/sphinx/_build/html`;
* `cargo fmt --check`;
* `git diff --check`.

The scanner smoke profile reported both M11 selected-output timing rows as
`1/1` successful: `select_region_scan_fallback` and
`select_region_indexed_output`.

Milestone 11 leaves `select_region` governed for BGZF BAM file output from CLI
`--region` requests. It includes native indexed traversal when a usable
non-stale BAI exists, native scan fallback otherwise, raw-record preservation,
source virtual-offset output ordering, duplicate suppression, conservative
header provenance and sort metadata, output write-safety, adjacent output index
sidecar invalidation, JSON schema/examples, fixture reservations, dependency
guardrails, and benchmark smoke evidence.

Remaining deferrals are explicit: public `--region-file`, binary stdout output
via `--out -`, report sidecar routing, replacement output-index creation, CSI
large-reference behavior, native CRAM indexed queries, biological
interpretation, and broad external comparator parity.

## Ten-Task Outline

1. M11.1 activate scope and freeze the selection command decision.
2. M11.2 specify region-file syntax, validation, ordering, and rejection
   behavior.
3. M11.3 specify selected-record output semantics for stdout, files, and dry
   runs.
4. M11.4 specify header preservation, `@HD` sort-order semantics, and reference
   dictionary behavior.
5. M11.5 specify overlapping-region and duplicate-region record policy.
6. M11.6 implement native selected-record writing from indexed traversal.
7. M11.7 implement write-safety, collision handling, index invalidation, and
   optional regeneration notes.
8. M11.8 update CLI contracts, JSON schemas, examples, fixtures, README, CLI
   docs, and Sphinx docs.
9. M11.9 add dependency-boundary and benchmark guardrails for selection output.
10. M11.10 close the milestone with full tests, contract tests, Sphinx, and
    benchmark smoke evidence.

## Non-Goals

M11 does not imply native CRAM indexed queries, CSI large-reference support,
biological interpretation, pileup/genotyping behavior, or broad external
comparator parity.
