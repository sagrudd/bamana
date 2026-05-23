# Milestone 13: Native CRAM Strategy And Compatibility Boundary

Status: planned. Milestone 13 is the first explicit post-M10 checkpoint for
CRAM strategy.

## Goal

Decide whether Bamana promotes any native CRAM substrate or keeps CRAM as a
documented compatibility boundary. The milestone should reduce ambiguity around
reference policy, cache policy, indexed CRAM queries, and allowed production
dependency exceptions.

## Ten-Task Outline

1. M13.1 activate scope and audit current CRAM ingestion/reference-policy
   behavior.
2. M13.2 decide native CRAM promotion, compatibility-only continuation, or
   explicit deferral.
3. M13.3 freeze reference discovery, explicit FASTA, embedded-reference, and
   cache policy semantics.
4. M13.4 define whether CRAM indexed queries are unsupported, transitional, or
   native work.
5. M13.5 add CRAM fixtures and oracle boundaries for the chosen decision.
6. M13.6 implement only the chosen CRAM behavior with documented dependency
   boundaries.
7. M13.7 update `consume` and inspection command contracts where CRAM behavior
   changes.
8. M13.8 update schemas, examples, README, CLI docs, Sphinx docs, and roadmap
   notes.
9. M13.9 add CRAM dependency-boundary and benchmark guardrails.
10. M13.10 close the milestone with full verification and residual risk notes.

## Non-Goals

M13 does not weaken Bamana-native BAM, BGZF, FASTQ, sampling, ingest, or
forensic hot-path ownership. `noodles` remains limited to documented CRAM
compatibility, tests, fixtures, compatibility checks, and oracle validation
unless a later contract changes that explicitly.
