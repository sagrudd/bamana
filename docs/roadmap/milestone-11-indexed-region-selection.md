# Milestone 11: Public Indexed Region Selection And Region Files

Status: active as of 2026-05-28. Milestone 11 follows the completed Milestone
10 read-only indexed-region evidence surface. M11.1 activates this milestone
without changing CLI behavior.

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
