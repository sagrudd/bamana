# Wrapper Examples

This directory documents wrapper examples for benchmark tools.

The current benchmark workflow does not yet invoke standalone wrapper scripts
from this directory. Instead, it uses:

* per-tool Nextflow modules under [../../modules/](/Users/stephen/Projects/bamana/benchmarks/modules)
* the shared timing wrapper [../../bin/run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh)

These examples exist to make the execution contract explicit.

## Bamana Wrapper Example

Scenario: `mapped_bam_pipeline`

Workflow variant: `bamana_subsample_sort_partial_index`

Expected inputs:

* staged BAM input
* threads
* subsample fraction
* subsample mode
* optional seed when `random`

Normalized command path:

```bash
"${BAMANA_BIN}" subsample --input "${INPUT}" --out "${RUN_ID}.subsampled.bam" --fraction "${FRACTION}" --mode "${MODE}" ${SEED_ARG}
"${BAMANA_BIN}" sort --bam "${RUN_ID}.subsampled.bam" --out "${RUN_ID}.sorted.bam"
```

Version command:

```bash
"${BAMANA_BIN}" --version
```

Unsupported behavior:

* if a scenario is not supported, emit `support_status = unsupported` rather
  than a fake failing command

## Samtools Wrapper Example

Scenario: `mapped_bam_pipeline`

Workflow variant: `samtools_view_sort_index`

Normalized command path:

```bash
samtools view -@ "${THREADS}" -s "${SEED}.${FRACTION_TOKEN}" -b "${INPUT}" -o "${RUN_ID}.subsampled.bam"
samtools sort -@ "${THREADS}" -o "${RUN_ID}.sorted.bam" "${RUN_ID}.subsampled.bam"
samtools index -@ "${THREADS}" "${RUN_ID}.sorted.bam"
```

Version command:

```bash
samtools --version
```

## Fastcat Wrapper Example

Scenario: `fastq_consume_pipeline`

Workflow variant: `fastcat_concat_gzip`

Normalized command path:

```bash
fastcat "${INPUT}" | gzip -c > "${RUN_ID}.fastcat.fastq.gz"
```

Version command:

```bash
fastcat --version
```

Unsupported behavior:

* `mapped_bam_pipeline`
* `unmapped_bam_pipeline`
* `subsample_only`

should be represented as unsupported rather than failed.

## Extension Guidance

If standalone shell wrappers are added later, they should preserve the same
contract:

* accept scenario and workflow variant context
* emit a deterministic output target
* expose a version command
* remain compatible with `run_benchmark.sh`
