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

Reserved clean roots for the trio:

* `tiny.clean.fastq` should come from a hand-authored tiny FASTQ with stable
  names, stable qualities, and no repeated halves or repeated local blocks
* `tiny.clean.bam` should come from a hand-authored tiny SAM/BAM with a normal
  `@RG`/`@PG` story and no deliberate read-name or tag-schema transition

#### Duplicate fixtures

Preferred process:

* derive them from a known clean source fixture
* create duplication by controlled whole-append or local-block repetition
* keep the resulting file parseable so detection and remediation logic can run

These fixtures should model operator-error duplication, not molecular duplicate
biology.

Reserved duplicate derivations for the trio:

* `tiny.duplicate.fastq.whole_append`: append the clean FASTQ body to itself
  without altering names, sequence, or qualities
* `tiny.duplicate.fastq.local_block`: splice one known clean contiguous record
  block back into the middle of the file
* `tiny.duplicate.bam.local_block`: duplicate a known contiguous BAM record
  block in file order while leaving the header unchanged

Each duplicate fixture should record which source fixture and which block or
range was duplicated so `deduplicate` dry-run and applied outputs can be
reviewed deterministically.

#### Forensic fixtures

Preferred process:

* start from a clean parseable BAM
* inject controlled RG/PG inconsistencies, read-name regime shifts, or layered
  concatenation hallmarks
* preserve parseability unless the fixture is explicitly meant to be invalid

Forensic fixtures must remain distinct from malformed fixtures. A suspicious
file should stay technically consumable where possible.

Reserved forensic derivations for the trio:

* `tiny.forensic.bam.rg_pg_inconsistent`: controlled header edits and RG/body
  mismatch while preserving BAM parseability
* `tiny.forensic.bam.readname_shift`: compose two read-name regimes in one BAM
  without truncation or structural corruption
* `tiny.forensic.bam.concatenated_signature`: layer duplicate-block evidence
  with suspicious provenance metadata and/or read-name regime transitions

Reviewer rule:

* if a forensic fixture stops parsing, it has become an invalid fixture and
  no longer serves the forensic contract it was intended to exercise

#### Invalid fixtures

Preferred process:

* derive from a clean source fixture
* use controlled truncation or mutation
* document the exact intended parse failure

Invalid fixtures should not be used as substitutes for forensic fixtures.

Reserved invalid derivations for the trio:

* `tiny.invalid.fastq.truncated`: truncate the final FASTQ record
* `tiny.invalid.bam.truncated_record`: truncate the BAM alignment stream
* optional later extension: reuse or derive an aux-corruption case only if the
  forensic/tag-inspection surface needs a dedicated malformed-aux negative path

## Trio Test Integration Guidance

The duplication/forensics trio should plug into the contract layer in three
different ways:

### `json_contract.rs`

Use the fixture manifest and expected-example inventory to assert:

* every trio semantic class is represented
* clean, duplicate, forensic, and invalid fixture ids remain stable
* reserved golden output filenames continue to match the command schemas

### `golden_outputs.rs`

Use the reserved expected-output stems to assert:

* clean fixtures stay quiet or no-op as intended
* duplicate fixtures keep stable finding taxonomy and stable dry-run/applied
  removal plans
* forensic fixtures keep stable category, severity, confidence, and assessment
  semantics
* invalid fixtures continue to fail honestly rather than drifting into silent
  success

### `cli_contract.rs`

This layer remains mostly fixture-independent, but representative trio fixture
ids should remain stable enough to support smoke-style invocation examples and
future thin end-to-end CLI checks.

### 5. Consume fixtures

The `consume` fixture family should stay small and should focus on discovery and
policy semantics first.

Recommended roots:

* `tiny.valid.coordinate.bam` for alignment-bearing BAM ingest
* `tiny.valid.sam` for alignment-bearing SAM ingest
* `tiny.valid.cram.explicit_ref` for explicit-reference CRAM ingest
* `tiny.valid.fastq` and `tiny.valid.fastq_gz` for unmapped ingest

Recommended derived fixtures:

* `tiny.consume.mixed_alignment_raw` created by composing one alignment-bearing
  source with one raw-read source in a single request
* `tiny.consume.directory_tree` created by arranging supported files,
  unsupported files, and nested directories in a deterministic lexical layout

The first consume fixtures should prove:

* deterministic lexical discovery
* recursive versus non-recursive directory behavior
* mixed-format rejection across alignment/raw-read boundaries
* strict CRAM required-reference behavior
* explicit-reference CRAM success reporting
* dry-run planning semantics

#### CRAM consume fixtures

CRAM fixtures need a more explicit provenance story than BAM/SAM fixtures.

Preferred process:

1. start from a tiny canonical SAM or BAM root with a stable reference
   dictionary
2. version the corresponding tiny reference FASTA in the repository as
   `tiny.ref.primary.fasta`
3. derive `tiny.valid.cram.explicit_ref.cram` deterministically using that
   exact FASTA and a documented one-time toolchain
4. reuse that same CRAM for the `tiny.valid.cram.reference_required` failure
   scenario by omitting `--reference` under `--reference-policy strict`
5. derive `tiny.valid.cram.compatible_refdict.cram` and
   `tiny.valid.bam.compatible_refdict.bam` from the same root or reference
   dictionary group
6. derive `tiny.valid.bam.incompatible_refdict.bam` by changing reference
   dictionary content in a controlled, documented way

Additional guidance:

* If Bamana itself cannot yet write CRAM, an external one-time generation path
  is acceptable, but the exact commands, tool versions, and source fixtures
  must be recorded.
* Do not download arbitrary CRAM files from the internet for contract tests.
* Keep `.crai` generation optional until a concrete consume/index interaction
  requires it.
* Only plan `tiny.valid.cram.no_external_ref` if a deterministic,
  reviewable no-external-reference CRAM can be generated and shown to decode
  conservatively. Otherwise leave it deferred.
* Later malformed or unsupported CRAM scenarios should be added by controlled
  mutation with explicit provenance, not by opaque third-party samples.

#### First provenance package

The first real CRAM package should be anchored by committed source files:

* `tests/fixtures/source/tiny.valid.cram.explicit_ref.source.sam`
* `tests/fixtures/source/tiny.ref.primary.fasta`

These files are now concrete repository content and form the authoritative
provenance root for the first explicit-reference CRAM fixture package.

Recommended deterministic derivation:

1. validate that the SAM `@SQ` dictionary matches the FASTA exactly
2. derive `tests/fixtures/bam/valid/tiny.valid.cram.explicit_ref.source.bam`
   from the source SAM
3. derive `tests/fixtures/cram/valid/tiny.valid.cram.explicit_ref.cram` from
   the same SAM or BAM with the explicit FASTA
4. verify that the CRAM decodes successfully when the explicit FASTA is
   supplied
5. verify that the same CRAM is expected to fail under `consume
   --reference-policy strict` when no `--reference` is supplied

If exact byte-for-byte CRAM reproducibility varies by tool version, document
that explicitly. In that case the governed provenance root remains the source
SAM plus source FASTA, and regenerated binary artifacts should be reviewed with
that limitation in mind.

The current source package is intentionally simple:

* `refA` and `refB` are synthetic, repository-local references
* the SAM contains three mapped reads and one unmapped read
* mapped reads use simple `12M` CIGARs and are easy to verify against the FASTA

Maintainer-facing derivation assets now live at:

* `tests/fixtures/source/generate_tiny_cram_fixture.sh`
* `tests/fixtures/source/generate_tiny_cram_fixture.md`

Those files must remain explicit about:

* the use of external tooling such as `samtools`
* the fact that source SAM + FASTA remain authoritative
* the difference between semantic reproducibility and byte-for-byte stability

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
