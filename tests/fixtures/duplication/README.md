# Duplication Fixture Family

This directory is reserved for the `inspect_duplication` and `deduplicate`
fixture family.

It intentionally mixes clean, duplicated, and invalid collection fixtures
across FASTQ and BAM so the contract layer can test:

* clean no-duplication baselines
* whole-append duplication
* local contiguous block duplication
* parse-failure handling

Planned fixture IDs include:

* `tiny.clean.fastq`
* `tiny.clean.bam`
* `tiny.duplicate.fastq.whole_append`
* `tiny.duplicate.fastq.local_block`
* `tiny.duplicate.bam.local_block`
* `tiny.invalid.fastq.truncated`
* `tiny.invalid.bam.truncated_record.duplication`
