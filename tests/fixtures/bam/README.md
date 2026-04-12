# BAM Fixture Categories

The `bam/` tree groups planned fixture binaries and sidecar index files by
intent rather than by command.

Subdirectories:

* `valid/`: baseline valid BAMs for broad command coverage
* `invalid/`: intentionally malformed BAMs for negative-path tests
* `transforms/`: source and derived BAMs for sort/merge/explode workflows
* `tags/`: tag-focused BAMs with deliberate auxiliary-field inventories
* `sorting/`: reserved for future sort-specialized fixtures if needed
* `mapping/`: reserved for future mapping-specialized fixtures if needed
* `indexing/`: BAI and future CSI sidecars, plus indexing-specific variants
