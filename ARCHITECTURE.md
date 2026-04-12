# Bamana Architecture

The canonical architecture guide is
[docs/architecture.md](/Users/stephen/Projects/bamana/docs/architecture.md).

Current project rule:

* Bamana-native BGZF, BAM, FASTQ, sampling, ingest, and forensic modules own
  performance-critical execution paths.
* `noodles` and similar general-purpose bioinformatics crates are demoted to
  compatibility, testing, oracle, or transitional roles.
* CRAM remains a staged compatibility slice and is not the model for BAM/FASTQ
  hot-path architecture.

Supporting documents:

* [docs/dependency-policy.md](/Users/stephen/Projects/bamana/docs/dependency-policy.md)
* [docs/performance-core.md](/Users/stephen/Projects/bamana/docs/performance-core.md)
* [docs/migration/noodles-demotion.md](/Users/stephen/Projects/bamana/docs/migration/noodles-demotion.md)
* [docs/testing-oracles.md](/Users/stephen/Projects/bamana/docs/testing-oracles.md)
