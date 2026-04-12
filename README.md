# Bamana

Bamana is a high-performance Rust toolkit for verification, quality control,
inspection, and transformation of BAM files and related bioinformatics formats.

This repository is structured as a Rust workspace so the command-line interface,
core domain logic, and future format-specific components can evolve with clear
boundaries.

## Workspace Layout

* `crates/bamana` - CLI entry point and command routing
* `crates/bamana-core` - core types, service layer, and reusable logic
* `docs/` - governing and project documentation

## First Implemented Commands

The current milestone implements the first shallow BAM-oriented vertical slice:

* `bamana identify <path>`
* `bamana verify --bam <bamfile>`
* `bamana check_eof --bam <bamfile>`
* `bamana header --bam <bamfile>`

All commands emit structured JSON. The semantics are intentionally narrow:

* `identify` classifies the input format as quickly and deterministically as possible
* `verify` performs shallow BAM verification only
* `check_eof` checks for the canonical BGZF EOF marker only
* `header` extracts BAM header information only

`verify` and `check_eof` do not imply deep validation.

## Development

```bash
cargo fmt
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Current Status

The repository now contains a production-minded first slice with shared JSON
contracts, structured error handling, fast file probing, BGZF EOF inspection,
and initial BAM header parsing. Full BAM validation and deeper transformation
commands will be implemented incrementally under the project charter in
[`docs/project-charter.md`](docs/project-charter.md).
