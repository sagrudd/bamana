# Bamana Interoperability And Contract Testing

The repository includes a contract-test scaffold under `tests/contract/`.

Its purpose is to guard:

* schema parseability
* example-file stability
* command-to-schema coverage
* CLI help surface stability

## Running The Scaffold

```bash
cargo test --test contract
```

## Fixture Layout

The `tests/fixtures/` tree is reserved for:

* tiny valid BAM fixtures
* malformed/truncated BAM fixtures
* golden JSON outputs
* CLI help snapshots

The fixture plan is governed by:

* `tests/fixtures/manifest.json`
* `tests/fixtures/plans/fixture-matrix.md`
* `tests/fixtures/plans/generation-strategy.md`
* `tests/fixtures/plans/coverage-map.md`

The current scaffold is intentionally lightweight. It is designed so future CI
can add:

* real JSON-schema validation
* fixture-backed command execution against the manifest-defined suite
* golden help-output snapshots
* cross-version contract checks

## Updating Golden Files

Golden or canonical examples should be updated only when:

* the contract intentionally changes, or
* documentation/examples were previously wrong

Those updates should be obvious in pull requests and accompanied by schema/doc
changes.
