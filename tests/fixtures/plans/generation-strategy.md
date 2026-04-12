# Fixture Generation Strategy

This document defines how the tiny synthetic fixture suite should be produced
and regenerated.

## Preferred Source Of Truth

For valid BAM fixtures, prefer a tiny hand-authored SAM source as the human
readable source of truth, then generate BAM deterministically from that source.

Why:

* the SAM source is reviewable in pull requests
* BAM generation can be documented and repeated
* the binary fixture stays intentionally small

## Generation Categories

### 1. Hand-authored valid fixtures

Preferred process:

1. author a tiny SAM with a compact, deterministic header
2. generate BAM deterministically
3. generate BAI where needed
4. capture expected Bamana JSON outputs from the resulting fixture

Valid fixture families should stay deliberately small and should not depend on
large external datasets or random generation.

### 2. Derived invalid fixtures

Malformed fixtures should be produced by controlled mutation of known-good
fixtures rather than by ad hoc hand editing of binary files.

Recommended mutations:

* strip the final 28-byte BGZF EOF marker
* truncate the final record payload
* corrupt an aux field's type or length
* mutate BAI magic bytes
* force a textual `@SQ` mismatch against the binary reference dictionary

These mutations should be scripted and repeatable. Mutation scripts should be
reviewable and should clearly document which bytes or regions are changed.

### 3. Transform-derived fixtures

Fixtures for `sort`, `merge`, and `explode` should be derived from one tiny
source BAM whenever possible.

Recommended model:

* `tiny.transforms.source.bam` is the base
* `tiny.transforms.shard*.bam` are deterministic derived children
* `tiny.transforms.merged.bam` is the deterministic round-trip merge target

Expected checksum comparisons should be documented alongside the derived
fixtures, not inferred later.

### 4. Duplication and forensic fixtures

The duplication/forensics trio needs a separate creation policy because the
fixtures must distinguish clean data, operator-error duplication, suspicious
provenance, and parse failure.

#### Clean fixtures

Preferred process:

* author tiny synthetic FASTQ or SAM roots
* preserve deterministic record/read order
* avoid accidental repeated blocks, header anomalies, or regime shifts

#### Duplicate fixtures

Preferred process:

* derive them from a known clean source fixture
* create duplication by controlled whole-append or local-block repetition
* keep the resulting file parseable so detection and remediation logic can run

These fixtures should model operator-error duplication, not molecular duplicate
biology.

#### Forensic fixtures

Preferred process:

* start from a clean parseable BAM
* inject controlled RG/PG inconsistencies, read-name regime shifts, or layered
  concatenation hallmarks
* preserve parseability unless the fixture is explicitly meant to be invalid

Forensic fixtures must remain distinct from malformed fixtures. A suspicious
file should stay technically consumable where possible.

#### Invalid fixtures

Preferred process:

* derive from a clean source fixture
* use controlled truncation or mutation
* document the exact intended parse failure

Invalid fixtures should not be used as substitutes for forensic fixtures.

## What Should Be Checked In

Preferred first step:

* check in the manifest and plan files now
* check in tiny BAM/BAI binaries only when generation is reproducible
* check in expected JSON outputs once fixture-backed command execution is stable

Avoid checking in generated outputs that cannot be regenerated or explained.

## Bamana-Generated Outputs

Bamana may be used to produce derived expected outputs and transform fixtures,
but only after:

* the source fixture is stable
* the command contract is stable enough to be reviewed
* the regeneration process is documented in the script README

Bamana-generated outputs should not become golden sources accidentally. Their
origin and intended use should be explicit.
