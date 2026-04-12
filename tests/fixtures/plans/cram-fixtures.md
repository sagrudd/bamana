# CRAM Consume Fixture Plan

This plan defines the minimal CRAM companion set for `bamana consume`.

## Goals

The initial CRAM fixture set exists to make three contract scenarios
executable:

1. explicit-reference success
2. strict missing-reference failure
3. reference-dictionary compatibility versus incompatibility

## Planned Fixture Set

| Fixture ID | Planned Status | Purpose | Expected Contract Outcome |
| --- | --- | --- | --- |
| `tiny.valid.cram.explicit_ref.source_sam` | present | Human-auditable alignment source of truth for the first CRAM package | provenance root for derived BAM and CRAM |
| `tiny.ref.primary` | present | Tiny explicit FASTA used to decode strict-policy CRAM inputs | reference material for explicit-reference success |
| `tiny.valid.cram.explicit_ref.source_bam` | planned | Derived BAM from the source SAM, reserved for review and future compatibility tests | future BAM/CRAM compatibility companion |
| `tiny.valid.cram.explicit_ref` | planned | Primary CRAM fixture that should decode successfully when `--reference tiny.ref.primary.fasta` is supplied | `consume` success with `reference.source_used = explicit_fasta` |
| `tiny.valid.cram.reference_required` | planned | Strict-policy scenario alias for the explicit-ref CRAM when no `--reference` is supplied | `consume` failure with `reference_required` |
| `tiny.valid.cram.compatible_refdict` | planned | Optional next-step CRAM fixture whose decoded header matches an explicit BAM companion | `consume` success when merged with `tiny.valid.bam.compatible_refdict` |
| `tiny.valid.bam.compatible_refdict` | planned | Optional BAM companion with the same reference dictionary as the compatible CRAM | alignment-mode compatibility success |
| `tiny.valid.bam.incompatible_refdict` | planned | Optional BAM companion with a different reference dictionary | `consume` failure with `incompatible_headers` |
| `tiny.valid.cram.no_external_ref` | deferred | Optional no-external-reference CRAM for `allow-embedded` or `auto-conservative` coverage | only added if reproducible and reviewable |

## Notes

* `tiny.valid.cram.reference_required` is expected to reuse the same CRAM bytes
  as `tiny.valid.cram.explicit_ref` when practical. The distinction is the
  invocation and contract outcome, not necessarily a separate binary payload.
* The source SAM and source FASTA are the provenance root. Derived BAM and CRAM
  artifacts should always be reviewed in relation to those two text fixtures.
* Compatibility tests should verify both CRAM classification and conservative
  header checking across alignment-bearing formats.
* `.crai` artifacts are optional for this stage and should only be added when a
  concrete consume or interop test needs them.

## Reserved Expected Outputs

These reserved consume golden outputs should accompany the CRAM companion set:

* `consume.tiny.valid.cram.explicit_ref.success.json`
* `consume.tiny.valid.cram.explicit_ref.reference_required.failure.json`
* `consume.cram_bam.compatible.success.json`
* `consume.cram_bam.incompatible.failure.json`

If `tiny.valid.cram.no_external_ref` becomes real, reserve:

* `consume.cram.no_external_ref.success.json`

## Representative Invocations

* explicit-reference success:
  `bamana consume --mode alignment --input tiny.valid.cram.explicit_ref.cram --reference tiny.ref.primary.fasta --reference-policy strict --out out.bam`
* strict missing-reference failure:
  `bamana consume --mode alignment --input tiny.valid.cram.explicit_ref.cram --reference-policy strict --out out.bam`
* compatible header success:
  `bamana consume --mode alignment --input tiny.valid.cram.compatible_refdict.cram tiny.valid.bam.compatible_refdict.bam --reference tiny.ref.primary.fasta --reference-policy strict --out out.bam`
* incompatible header failure:
  `bamana consume --mode alignment --input tiny.valid.cram.compatible_refdict.cram tiny.valid.bam.incompatible_refdict.bam --reference tiny.ref.primary.fasta --reference-policy strict --out out.bam`

## Test Harness Usage

The contract harness should eventually use this set as follows:

* `json_contract.rs`: validate `consume` output against
  `spec/jsonschema/consume.schema.json`
* `golden_outputs.rs`: compare actual JSON with the reserved expected output
  names above
* `cli_contract.rs`: smoke-check representative `consume` invocations and help
  text references

Critical assertions:

* `reference.policy` is stable
* `reference.source_used` is stable
* strict missing-reference runs fail with `reference_required`
* incompatible CRAM/BAM refdict combinations fail with `incompatible_headers`
