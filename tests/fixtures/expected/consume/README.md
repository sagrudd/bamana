# Expected Outputs For `consume`

Naming convention:

* success:
  `consume.<fixture-id>.success.json`
* failure:
  `consume.<fixture-id>.failure.json`
* dry run:
  `consume.<fixture-id>.dry_run.success.json`

Examples to reserve:

* `consume.tiny.valid.coordinate.dry_run.success.json`
* `consume.tiny.valid.sam.dry_run.success.json`
* `consume.tiny.valid.cram.explicit_ref.success.json`
* `consume.tiny.valid.cram.explicit_ref.reference_required.failure.json`
* `consume.cram_bam.compatible.success.json`
* `consume.cram_bam.incompatible.failure.json`
* `consume.cram.no_external_ref.success.json` only if the corresponding fixture
  is actually present and stable
* `consume.tiny.valid.fastq.dry_run.success.json`
* `consume.tiny.valid.fastq_gz.dry_run.success.json`
* `consume.tiny.consume.mixed_alignment_raw.failure.json`
* `consume.tiny.consume.directory_tree.dry_run.success.json`

Where possible, CRAM-oriented expected outputs should align with the canonical
spec examples:

* [consume.success.alignment.json](/Users/stephen/Projects/bamana/spec/examples/consume.success.alignment.json)
* [consume.failure.reference_required.json](/Users/stephen/Projects/bamana/spec/examples/consume.failure.reference_required.json)
* [consume.failure.incompatible_headers.json](/Users/stephen/Projects/bamana/spec/examples/consume.failure.incompatible_headers.json)

For the first provenance package, the CRAM success and strict missing-reference
failure cases should be anchored by the same derived CRAM plus the committed
source FASTA from `tests/fixtures/source/`.

Concrete placeholder files now reserved for that package:

* [consume.tiny.valid.cram.explicit_ref.success.json](/Users/stephen/Projects/bamana/tests/fixtures/expected/consume/consume.tiny.valid.cram.explicit_ref.success.json)
* [consume.tiny.valid.cram.explicit_ref.reference_required.failure.json](/Users/stephen/Projects/bamana/tests/fixtures/expected/consume/consume.tiny.valid.cram.explicit_ref.reference_required.failure.json)
