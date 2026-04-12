# Bamana Specification Layer

The `spec/` tree is the repository-facing contract layer for Bamana. It exists
to make the CLI and JSON surface machine-reviewable, versioned, and suitable
for automation in regulated and operationally demanding environments.

This directory separates:

* machine-readable schemas in `jsonschema/`
* human-readable CLI and contract docs in `cli/` and `contracts/`
* canonical JSON examples in `examples/`

## Contract Intent

Bamana commands are automation-facing interfaces. The schema and example files
in this tree are not informal notes. They are the external contract baseline
that implementation work must preserve unless a deliberate contract change is
reviewed and versioned.

The spec layer is intentionally explicit about what commands prove and what they
do not prove. For example:

* `verify` is shallow only
* `consume` introduces governed ingestion contracts for files and directories
* `check_eof` is EOF-marker only
* `summary` may be bounded or full-scan depending on mode
* `sort` and `merge` do not imply content preservation unless checksum
  verification is explicitly reported
* `validate` does not imply biological or reference-level correctness

## Versioning

The first contract baseline is tied to crate version `0.1.0`.

Contract changes should update:

* the affected schema file under `spec/jsonschema/`
* the relevant example files under `spec/examples/`
* the human-readable contract docs under `spec/cli/`, `spec/contracts/`, and
  `docs/`
* the contract tests under `tests/contract/` when applicable

Breaking changes require an explicit compatibility review. See:

* [spec/contracts/versioning.md](/Users/stephen/Projects/bamana/spec/contracts/versioning.md)
* [spec/contracts/compatibility.md](/Users/stephen/Projects/bamana/spec/contracts/compatibility.md)
* [spec/contracts/naming.md](/Users/stephen/Projects/bamana/spec/contracts/naming.md)

## Running Contract Checks

The current scaffold is designed to be lightweight and CI-friendly.

```bash
cargo test --test contract
```

The contract tests currently enforce:

* schema files and example files are present and parse as JSON
* every command example has a matching schema file
* example files are normalized and stable
* CLI help output includes the documented subcommands and global options

As the repository matures, this scaffold is intended to grow into full schema
validation and fixture-backed interoperability testing without changing the
directory model.
