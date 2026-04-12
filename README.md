# Bamana

Bamana is a high-performance Rust toolkit for verification, quality control,
inspection, and transformation of BAM files and related bioinformatics formats.

The current repository contains the first concrete CLI slice for:

* `bamana identify <path>`
* `bamana verify --bam <bamfile>`
* `bamana check_eof --bam <bamfile>`
* `bamana header --bam <bamfile>`

All command output is JSON.

The current semantics are intentionally narrow:

* `identify` determines the most likely file type quickly using extension hints, magic bytes, and shallow text heuristics
* `verify` performs shallow BAM verification only by confirming a BAM-like BGZF container and `BAM\1` magic in the first inflated block
* `check_eof` checks only for the canonical 28-byte BGZF EOF marker
* `header` parses the BAM header only, including the binary reference dictionary and textual SAM-style header records

Neither `verify` nor `check_eof` implies deep validation of the BAM payload.
`header` does not imply that alignment records are readable, that EOF is present, or that the full BAM body is valid.

## Example Invocations

```bash
cargo run -- identify example.bam
cargo run -- verify --bam example.bam
cargo run -- check_eof --bam example.bam
cargo run -- header --bam example.bam
```

`header` uses the binary BAM reference section as authoritative for reference
names and lengths, and joins optional fields from textual `@SQ` records into the
structured JSON view when present.

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
