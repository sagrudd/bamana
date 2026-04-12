# Transform Fixtures

This directory is reserved for fixtures that exercise transformation commands:

* `tiny.transforms.source.bam`
* `tiny.transforms.shard1.bam`
* `tiny.transforms.shard2.bam`
* `tiny.transforms.merged.bam`

These fixtures anchor deterministic round-trip and checksum-preservation tests
for `sort`, `merge`, `explode`, and future transform commands.
