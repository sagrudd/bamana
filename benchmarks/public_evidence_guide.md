# Public Benchmark Evidence Guide

M14.8 consolidates the public documentation boundary for Bamana benchmark
evidence. This guide is the short entry point for deciding whether a timing,
profile, row, or command surface can be treated as public comparator evidence.

## Evidence Classes

| Evidence Class | Meaning | Governed Source | Public Claim Boundary |
| --- | --- | --- | --- |
| `public_profile_comparator` | A measured `bamana benchmark --profile` profile with aligned assumptions. | `benchmarks/comparator_profiles.json` | Only `fastq_ingress` and `fastq_gz_enumerate`, only for the runner, input, container image, thread count, result artifacts, semantic assumptions, and unsupported mismatch cases recorded for that profile. |
| `local_smoke` | Repository-local command or substrate timing used as a regression guardrail. | `benchmarks/results/*_microbench.schema.json` and benchmark docs | Useful for detecting regressions; not external comparator parity, biological equivalence, or release performance promise. |
| `scenario_matrix_comparator_scaffold` | A workflow-matrix row where a comparator path exists or is planned but semantics are not promoted. | `benchmarks/tools/workflow_variant_matrix.md` and `benchmarks/command_evidence_matrix.md` | Visible scaffold only; cannot be cited as release-facing comparator evidence. |
| `no_external_comparator_claim` | A command or mode with no current external comparator claim. | `benchmarks/command_evidence_matrix.md` and `benchmarks/comparator_mismatches.md` | Intentional no-claim surface until fixtures, semantic assumptions, schemas, and measured profiles are added. |

## Public Benchmark Profiles

The public `benchmark` command currently exposes two measured comparator
profiles:

* `fastq_ingress`: FASTQ.GZ-to-unmapped-BAM timing for Bamana
  `consume --mode unmapped` versus `fastcat fastq | samtools import`;
* `fastq_gz_enumerate`: FASTQ.GZ record-count timing for Bamana
  `enumerate --input` versus `gzip -cd | awk` line counting.

These are the only current release-facing comparator profiles. Their exact
assumptions and mismatch cases live in `benchmarks/comparator_profiles.json`
and are echoed in the public `benchmark` JSON payload as
`semantic_equivalence_assumptions` and `unsupported_mismatch_cases`.

## Smoke Hooks

Repository-local smoke hooks are regression guardrails:

* `bgzf_microbench`
* `header_microbench`
* `scanner_microbench`
* `fastq_microbench`

Smoke hooks may record command timings for many command families, but those
timings do not imply comparator parity. They are used to keep native BAM, BGZF,
FASTQ, scanner, mutation, index, selected-region, remediation, and forensic
paths observable as implementation evolves.

## Comparator And No-Claim Documents

Use these files together:

* `benchmarks/command_evidence_matrix.md`: command-level classification for
  public-profile comparator, local-smoke, scaffolded-comparator, and no-claim
  surfaces;
* `benchmarks/comparator_profiles.json`: machine-readable aligned comparator
  profile catalog;
* `benchmarks/comparator_mismatches.md`: unsupported comparator and mismatch
  register;
* `benchmarks/schema_stability_manifest.json`: pinned benchmark schema
  inventory enforced by `benchmarks/bin/check_schema_stability.py`.

The public contract commands `benchmark`, `fastq`, and `unmap` must remain
easy to distinguish here: `benchmark` owns governed profile execution,
`fastq` currently has no external comparator claim, and `unmap` currently has no external comparator claim.

## Promotion Rule

A new comparator claim needs all of the following before it can move from
scaffold or no-claim status into public-profile evidence:

* fixture provenance and checksums;
* explicit semantic equivalence assumptions;
* explicit unsupported mismatch cases;
* schema and example coverage for emitted evidence;
* a measured profile or benchmark row with archived artifacts;
* documentation in README, CLI docs, Sphinx docs, roadmap, and taskmap.
