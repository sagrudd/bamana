# Bamana Exit-Code Contract

Bamana is JSON-first, but exit codes are still part of the public contract.

## Current Policy

* `0`: the command completed successfully and returned `ok: true`
* non-zero: the command returned `ok: false` or the CLI could not complete the
  requested operation

This policy means:

* command-level negative outcomes that are modeled as failures in the JSON
  envelope also return a non-zero exit status
* malformed CLI usage returns a non-zero exit status through clap
* output serialization failures return a non-zero exit status

## Stability Expectations

Changing the meaning of exit codes is a contract change.

Future refinements may introduce differentiated non-zero codes, but only with:

* documentation updates
* contract review
* example/test updates
* release-note disclosure
