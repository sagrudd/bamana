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
4. Run Nextflow using `--input_manifest`.

Example:

```bash
python benchmarks/bin/validate_inputs.py --manifest benchmarks/inputs/example_manifest.json --skip-file-checks
cd benchmarks
nextflow run main.nf -profile local_ssd --input_manifest /abs/path/to/benchmark-inputs.json
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
* expected sort order
* index availability
* staging policy hints
* allowed scenarios
* provenance notes such as source owner and sensitivity level

The governing schema is
[manifest.schema.json](/Users/stephen/Projects/bamana/benchmarks/inputs/manifest.schema.json).

## Read-Only Rule

Manifest entries always refer to source data that the benchmark framework must
not mutate or delete.

If a scenario requires a subsampled or otherwise normalized input, the
benchmark framework should create a derived scenario input under
benchmark-managed output paths rather than editing the source file in place.
