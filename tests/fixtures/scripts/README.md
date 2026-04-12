# Fixture Script Stubs

This directory is reserved for deterministic fixture-generation and mutation
entrypoints.

Recommended script split:

* `generate_valid_fixtures.sh`: create tiny valid BAMs and BAI sidecars from
  reviewable source data
* `mutate_invalid_fixtures.py`: derive malformed BAMs and indices from valid
  fixtures via documented byte-level mutations

Rules:

* scripts must be deterministic
* scripts must not fetch external datasets
* scripts must not overwrite checked-in fixtures silently
* scripts should explain which files they generate and why
