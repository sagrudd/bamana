# Bamana

Bamana is a high-performance Rust toolkit for verification, quality control,
inspection, and transformation of BAM files and related bioinformatics formats.

The current repository contains the first concrete CLI slice for:

* `bamana identify <path>`
* `bamana verify --bam <bamfile>`
* `bamana check_eof --bam <bamfile>`

All command output is JSON.

The current semantics are intentionally narrow:

* `identify` determines the most likely file type quickly using extension hints, magic bytes, and shallow text heuristics
* `verify` performs shallow BAM verification only by confirming a BAM-like BGZF container and `BAM\1` magic in the first inflated block
* `check_eof` checks only for the canonical 28-byte BGZF EOF marker

Neither `verify` nor `check_eof` implies deep validation of the BAM payload.

## Example Invocations

```bash
cargo run -- identify example.bam
cargo run -- verify --bam example.bam
cargo run -- check_eof --bam example.bam
```

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
