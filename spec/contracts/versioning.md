# Contract Versioning

## Baseline

The initial repository contract baseline is tied to crate version `0.1.0`.

Schemas may also carry metadata such as:

* `x-bamana-command`
* `x-bamana-contract-version`

## What Counts As Breaking

Breaking changes include:

* renaming a command
* removing a command or option
* renaming a JSON field
* changing an enum literal
* changing a field from nullable to required
* changing a field type
* changing a command’s meaning without changing its name
* changing exit-code semantics

Semantic changes are breaking even when the wire shape stays the same.

## Usually Non-Breaking

Usually non-breaking changes include:

* adding optional JSON fields
* adding optional CLI arguments
* adding new warning/info finding codes
* tightening documentation without changing behavior

These changes still require schema/example/doc review.

## Required Process For Breaking Changes

Breaking contract changes require:

* schema update
* example update
* contract-test update
* release-note entry
* explicit compatibility review

## Partial-To-Fuller Implementations

Moving a command from partial implementation to fuller implementation is not
automatically non-breaking.

It is acceptable only when:

* field names remain stable
* semantics remain consistent with prior documentation
* bounded-versus-full evidence distinctions remain explicit
* new guarantees do not silently reinterpret old fields
