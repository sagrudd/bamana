# Comparator Mismatch Register

M14.6 documents unsupported comparator cases and semantic mismatch reasons.
This file is a benchmark scope-control document: an entry here means the
surface is intentionally not a release-facing comparator claim yet, not that
the command is missing from Bamana.

## How To Read This Register

Evidence status values:

* `documented_no_claim`: no current benchmark comparator claim exists.
* `scaffolded_mismatch`: the workflow matrix names a possible comparator path,
  but semantics are not aligned enough for release-facing evidence.
* `partial_profile_only`: evidence exists only for a named public profile and
  only under that profile's assumptions.

Promotion requirements are the minimum evidence needed before a surface can
move into `benchmarks/comparator_profiles.json` or be cited as release-facing
benchmark evidence.

| Surface | Evidence Status | Semantic Mismatch Reason | Current Boundary | Promotion Requirement |
| --- | --- | --- | --- | --- |
| `fastq` BAM-to-FASTQ export | `documented_no_claim` | External tools differ on FASTQ header policy, auxiliary tag projection, read pairing, methylation tag representation, and quality fallback behavior. | Public contract command with schema/docs only; no benchmark hook or comparator claim. | Dedicated fixture provenance, output equivalence oracle, header/tag policy, and measured profile. |
| `unmap` mapped-BAM stripping | `documented_no_claim` | External tools differ on which mapping fields, mate fields, CIGAR data, reference dictionaries, tags, and sort/index sidecars are preserved or cleared. | Public contract command with schema/docs only; no benchmark hook or comparator claim. | Command-specific BAM fixtures, field-level equivalence contract, sidecar invalidation checks, and measured profile. |
| `identify` format sniffing | `documented_no_claim` | Format-probing heuristics are Bamana-owned and not equivalent to deep validation or external magic-file policy. | CLI/schema/docs only; no comparator claim. | Explicit oracle policy and fixtures for each supported container and ambiguous input class. |
| CRAM consume behavior | `documented_no_claim` | CRAM compatibility is a transitional decode path with explicit reference/cache restrictions; indexed CRAM and CRAI traversal remain unsupported. | Compatibility-only behavior; no CRAM throughput, CRAI, indexed-query, or comparator-parity claim. | Native CRAM strategy, reference/cache fixture plan, CRAI boundary decision, and command-specific benchmark profile. |
| BAM/SAM `consume --mode alignment` | `documented_no_claim` | Alignment normalization semantics include header reconciliation, reference-policy handling, and directory discovery that do not map to a single external comparator command. | Local smoke evidence only through synthetic BAM alignment ingest. | Real alignment fixtures, source provenance, external baseline choice, and output equivalence contract. |
| Mixed-directory ingest | `documented_no_claim` | Directory discovery, skipped files, rejected files, stdin handling, and mixed raw/alignment rejection are Bamana orchestration semantics. | No external comparator claim for mixed input sets. | Discovery fixture matrix and an explicit comparator orchestration policy. |
| BAM `subsample` against `samtools view -s` | `scaffolded_mismatch` | Bamana deterministic identity and random policy are not the same as samtools seeded pseudo-random selection semantics. | Workflow-matrix scaffold and local smoke only, not release-facing comparator parity. | seed/fraction policy alignment, retained-record equivalence rules, and measured profile. |
| FASTQ.GZ `subsample` against `seqtk sample` | `scaffolded_mismatch` | Bamana record identity, gzip output policy, deterministic mode, and malformed-input handling differ from seqtk sampling behavior. | Workflow-matrix scaffold and local smoke only. | FASTQ fixture provenance, seed semantics, output normalization rules, and measured profile. |
| `rasusa` read/alignment downsampling | `scaffolded_mismatch` | Rasusa is coverage-oriented while the current Bamana benchmark contract is fractional record selection. | Explicitly unsupported until a fair strategy is pinned. | coverage-versus-fraction strategy document and matched input/output metrics. |
| Mapped BAM sort/index pipeline | `scaffolded_mismatch` | Bamana BAI creation is partial and CSI writing/full indexed-region parity remain outside the current comparator profile boundary. | Workflow-matrix scaffold only; no release-facing full pipeline claim. | Complete index-writing scope, sorted-output equivalence contract, and archived measured profile. |
| `select_region` indexed output | `documented_no_claim` | Region syntax, overlap filtering, duplicate-chunk handling, output sidecar invalidation, BAI/CSI fallback, and CRAM exclusions are Bamana-owned. | Local smoke only; no comparator parity. | Region fixture suite, external baseline policy, and selected-record equivalence oracle. |
| Mutation commands `reheader` and `annotate_rg` | `documented_no_claim` | Header-only and record-level mutation semantics are intentionally Bamana-specific and include checksum/index-safety evidence. | Local smoke only; no external mutation comparator claim. | External tool choice, mutation-policy equivalence rules, and output oracle. |
| Forensic commands `inspect_duplication`, `deduplicate`, and `forensic_inspect` | `documented_no_claim` | Provenance, fraud-signal, duplicate-block, and remediation semantics are Bamana-owned and are not duplicate-marking parity. | Local smoke only; no Picard/GATK/provenance-tool comparator claim. | Domain-specific oracle policy, fixture labels, and reviewer-approved comparator scope. |

## Public Profile Exceptions

The only M14.6 exceptions are the public profiles already cataloged in
`benchmarks/comparator_profiles.json`:

* `fastq_ingress` is `partial_profile_only` for FASTQ.GZ-to-unmapped-BAM
  timing against `fastcat fastq | samtools import`.
* `fastq_gz_enumerate` is `partial_profile_only` for FASTQ.GZ record-count
  timing against `gzip -cd | awk`.

Those exceptions remain bounded by each profile's
`semantic_equivalence_assumptions`, `unsupported_mismatch_cases`, result
artifacts, and release claim boundary.
