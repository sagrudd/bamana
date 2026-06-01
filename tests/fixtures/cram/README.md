# CRAM Fixture Companion Set

This directory is reserved for the small CRAM-specific fixture plan used by
`bamana consume` contract tests.

The goal is not to build a broad CRAM corpus. The goal is to keep a very small,
reviewable set that proves three high-risk semantics:

* explicit-reference success
* strict missing-reference failure
* header-compatibility behavior when CRAM is combined with BAM/SAM inputs

Preferred planned artifacts:

* `tiny.valid.cram.explicit_ref.cram`
* `tiny.ref.primary.fasta`
* `tiny.valid.cram.compatible_refdict.cram`
* `tiny.valid.cram.no_external_ref.cram` only if reproducible and honest

M13.5 freezes the fixture boundary as provenance-first. The committed source
SAM and FASTA are present roots; derived CRAM/BAM fixtures remain planned until
regenerated and reviewed; no-external-reference CRAM remains deferred; and CRAI
fixtures, indexed CRAM fixtures, and CRAM random-access oracle outputs are
explicitly deferred.

Companion BAM artifacts for compatibility tests may live under
`tests/fixtures/bam/`, but they should share a documented
`header_compatibility_group` with the CRAM fixture plan.

Important:

* CRAM fixtures must stay explicit about whether they require a reference.
* Missing-reference behavior is a first-class contract, not a generic parse
  failure.
* Planned and deferred CRAM fixtures must be labeled clearly in the manifest.
* `noodles` or external tools may be used only for fixture generation, fixture
  validation, or test-only compatibility checks.
