# Expected Outputs For `inspect_duplication`

Naming convention:

* `inspect_duplication.<fixture-id>.success.json`
* `inspect_duplication.<fixture-id>.failure.json`

Examples to reserve:

* `inspect_duplication.tiny.clean.fastq.success.json`
* `inspect_duplication.tiny.clean.bam.success.json`
* `inspect_duplication.tiny.duplicate.fastq.whole_append.success.json`
* `inspect_duplication.tiny.duplicate.fastq.local_block.success.json`
* `inspect_duplication.tiny.duplicate.bam.local_block.success.json`
* `inspect_duplication.tiny.forensic.bam.concatenated_signature.success.json`
* `inspect_duplication.tiny.invalid.fastq.truncated.failure.json`
* `inspect_duplication.tiny.invalid.bam.truncated_record.failure.json`

Semantic intent:

* clean fixtures prove absence of suspicious duplicate signatures
* duplicate fixtures prove repeated-block classification and stable range
  reporting
* forensic overlap fixtures prove that duplication evidence can appear inside a
  broader provenance-anomaly case without collapsing the command semantics
* invalid fixtures prove parse-failure behavior
