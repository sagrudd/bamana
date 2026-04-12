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

## Development

```bash
cargo fmt
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Current Status

The repository currently contains the initial Rust scaffold and command
architecture. Functional BAM inspection and transformation commands will be
implemented incrementally under the project charter in
[`docs/project-charter.md`](docs/project-charter.md).
