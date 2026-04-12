# Expected Outputs For `deduplicate`

Naming convention:

* dry run:
  `deduplicate.<fixture-id>.dry_run.success.json`
* applied output:
  `deduplicate.<fixture-id>.applied.success.json`
* no-op clean result:
  `deduplicate.<fixture-id>.noop.success.json`
* failures:
  `deduplicate.<fixture-id>.failure.json`

Examples to reserve:

* `deduplicate.tiny.clean.fastq.noop.success.json`
* `deduplicate.tiny.clean.bam.noop.success.json`
* `deduplicate.tiny.duplicate.fastq.whole_append.dry_run.success.json`
* `deduplicate.tiny.duplicate.fastq.whole_append.applied.success.json`
* `deduplicate.tiny.duplicate.fastq.local_block.dry_run.success.json`
* `deduplicate.tiny.duplicate.fastq.local_block.applied.success.json`
* `deduplicate.tiny.duplicate.bam.local_block.dry_run.success.json`
* `deduplicate.tiny.duplicate.bam.local_block.applied.success.json`
* `deduplicate.tiny.invalid.fastq.truncated.failure.json`
* `deduplicate.tiny.invalid.bam.truncated_record.failure.json`
