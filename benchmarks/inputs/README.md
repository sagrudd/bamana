# Benchmark Inputs

This directory contains manifest scaffolding and documentation for large
benchmark inputs. It does not contain the large benchmark datasets themselves.

## What Belongs Here

Tracked items:

* input manifest schema
* example manifests
* documentation for users supplying benchmark data

Untracked items:

* whole-genome BAMs
* large FASTQ.GZ collections
* external indices
* raw sequencing collections

## Preferred Workflow

1. Place large benchmark source inputs outside the repository.
2. Create a manifest describing them.
3. Validate the manifest locally.
4. Create or copy a params JSON file for the run.
5. Run Nextflow using `-params-file`.

Example:

```bash
python benchmarks/bin/validate_inputs.py --manifest benchmarks/inputs/example_manifest.json --skip-file-checks
nextflow run benchmarks/main.nf -profile local_ssd -params-file /abs/path/to/benchmark-run.json
```

## Input Categories

First-slice supported categories:

* `mapped_bam`
* `unmapped_bam`
* `fastq_gz`

These categories are intentionally operational rather than biological. They are
used to decide which benchmark scenarios are allowed and what staging guidance
applies.

## Manifest Expectations

Each manifest entry should provide:

* a stable `id`
* an absolute `path`
* a benchmark `type`
* mapped-state context
* compression
* index path when present
* expected sort order
* index availability
* staging policy hints
* allowed scenarios
* provenance notes such as source owner and sensitivity level
* fixture provenance metadata when the input is generated, derived, selected,
  or used as comparator evidence

The governing schema is
[manifest.schema.json](/Users/stephen/Projects/bamana/benchmarks/inputs/manifest.schema.json).
M14.4 adds the governed `fixture_provenance` object documented in
[fixture_provenance.md](/Users/stephen/Projects/bamana/benchmarks/inputs/fixture_provenance.md).
It records source kind, source description, source URI, `derived_from`,
generation command, generation environment, expected semantic scope, review
boundary, checksum policy, and reproducibility notes. The metadata is required
for new generated, derived, or comparator fixtures before release-facing
benchmark claims are promoted.

Ready-to-edit params examples live in
[../params.examples/](/Users/stephen/Projects/bamana/benchmarks/params.examples).

Typical workflow:

1. edit the manifest so `inputs` contains your datasets
2. copy one of the example params JSON files
3. point `input_manifest` at your edited manifest
4. set `dataset_ids` to the datasets you want in this run
5. set `scenarios`, `tools`, `replicates`, and `output_dir`

## Read-Only Rule

Manifest entries always refer to source data that the benchmark framework must
not mutate or delete.

If a scenario requires a subsampled or otherwise normalized input, the
benchmark framework should create a derived scenario input under
benchmark-managed output paths rather than editing the source file in place.
