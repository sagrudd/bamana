# Bamana

Bamana is a high-performance Rust toolkit for verification, quality control,
inspection, and transformation of BAM files and related bioinformatics formats.

The current repository contains the first concrete CLI slice for:

* `bamana identify <path>`
* `bamana verify --bam <bamfile>`
* `bamana check_eof --bam <bamfile>`
* `bamana header --bam <bamfile>`
* `bamana check_map --bam <bamfile>`
* `bamana check_sort --bam <bamfile>`
* `bamana check_index --bam <bamfile>`
* `bamana index --bam <bamfile>`

All command output is JSON.

The current semantics are intentionally narrow:

* `identify` determines the most likely file type quickly using extension hints, magic bytes, and shallow text heuristics
* `verify` performs shallow BAM verification only by confirming a BAM-like BGZF container and `BAM\1` magic in the first inflated block
* `check_eof` checks only for the canonical 28-byte BGZF EOF marker
* `header` parses the BAM header only, including the binary reference dictionary and textual SAM-style header records
* `check_map` prefers index-derived mapping summaries when a usable BAI is present and otherwise falls back to scan-based evidence
* `check_sort` combines BAM header declarations with a bounded scan of alignment records to assess coordinate or queryname ordering
* `check_index` inspects adjacent BAM indices for presence, type, shallow syntactic validity, timestamp-based staleness, and apparent usability
* `index` currently validates the BAM and resolves the output index path and format, but reports index creation as not yet implemented in this slice

Neither `verify` nor `check_eof` implies deep validation of the BAM payload.
`header` does not imply that alignment records are readable, that EOF is present, or that the full BAM body is valid.
`check_map` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_sort` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_index` does not imply that every random-access offset is correct or that the BAM and index are semantically matched beyond shallow inspection.
`index` does not imply that BAM index writing has completed unless the response explicitly reports a created output.

## Example Invocations

```bash
cargo run -- identify example.bam
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
```

`header` uses the binary BAM reference section as authoritative for reference
names and lengths, and joins optional fields from textual `@SQ` records into the
structured JSON view when present.

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
