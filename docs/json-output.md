# Bamana JSON Output Contracts

The `spec/jsonschema/` directory contains the machine-readable contract layer
for Bamana command outputs.

## Envelope Model

Every command response follows the same top-level pattern:

* `ok`
* `command`
* `path`
* `data`
* `error`

This repository treats those fields as governed public interface.

## Schema Use

Each command has:

* a command-specific schema file
* canonical success and failure examples under `spec/examples/`
* shared common definitions under `spec/jsonschema/common/`

Consumers should treat the schema and canonical examples together as the output
contract.

## Stability Rules

Breaking output changes require:

* schema update
* example update
* contract-test update
* release-note disclosure

See:

* [spec/contracts/versioning.md](/Users/stephen/Projects/bamana/spec/contracts/versioning.md)
* [spec/contracts/compatibility.md](/Users/stephen/Projects/bamana/spec/contracts/compatibility.md)

## `consume`

The `consume` payload introduces an ingestion-oriented contract layer in
addition to Bamana’s inspection and transformation outputs.

Key concepts:

* requested paths versus discovered files
* deterministic directory traversal reporting
* consumed, skipped, and rejected file lists
* explicit ingest mode (`alignment`, `unmapped`)
* output sort/index/checksum intent
* notes that separate implemented Stage 1 behavior from deferred options

The contract is designed so automation can reason about dry-run discovery
results, mixed-format rejection, and staged normalization behavior without
needing to infer semantics from ad hoc log text.
