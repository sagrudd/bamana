nextflow.enable.dsl = 2

process RUN_SAMTOOLS_BENCHMARK {
    tag "${meta.run_id}"
    publishDir "${params.output_dir}/per_run", mode: 'copy'

    input:
    tuple val(meta), path(input_file), path(input_metrics_json), path(input_metrics_tsv)

    output:
    path("${meta.run_id}.result.tsv"), emit: result_tsv
    path("${meta.run_id}.result.json"), emit: result_json

    script:
    def fractionText = meta.subsample_fraction.toString()
    def fractionToken = fractionText.startsWith('0.') ? fractionText.substring(2) : fractionText.replace('.', '')
    def sampleArg = "${meta.subsample_seed}.${fractionToken}"
    def outputTarget = ''
    def supportStatus = 'supported'
    def semanticEquivalence = 'full'
    def notes = ''
    def command = 'true'

    if (meta.scenario == 'mapped_bam_pipeline') {
        outputTarget = "${meta.run_id}.sorted.bam"
        command = """\
set -euo pipefail
samtools view -@ ${meta.threads} -s ${sampleArg} -b "${input_file}" -o "${meta.run_id}.subsampled.bam"
samtools sort -@ ${meta.threads} -o "${outputTarget}" "${meta.run_id}.subsampled.bam"
samtools index -@ ${meta.threads} "${outputTarget}"
"""
        notes = 'samtools is the canonical BAM baseline and uses the natural subsample then sort then index order here.'
    } else if (meta.scenario == 'unmapped_bam_pipeline' || meta.scenario == 'subsample_only') {
        outputTarget = "${meta.run_id}.subsampled.bam"
        command = """\
set -euo pipefail
samtools view -@ ${meta.threads} -s ${sampleArg} -b "${input_file}" -o "${outputTarget}"
"""
        notes = meta.scenario == 'subsample_only'
            ? 'samtools subsample-only benchmarking runs the direct BAM subsampling path without sort or index.'
            : 'Unmapped BAM scenario is benchmarked as subsample only because sort and index are not required for the first comparison.'
    } else {
        supportStatus = 'unsupported'
        semanticEquivalence = 'unsupported'
        notes = 'samtools is not benchmarked for raw FASTQ.GZ consume workflows in this first benchmark contract.'
    }

    """
    cat <<'EOF' > command.sh
${command}
EOF
    chmod +x command.sh

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "samtools" \
      --tool-version-cmd "samtools --version" \
      --scenario "${meta.scenario}" \
      --workflow-variant "${meta.workflow_variant}" \
      --semantic-equivalence "${semanticEquivalence}" \
      --support-status "${supportStatus}" \
      --input-type "${meta.input_type}" \
      --mapping-state "${meta.mapping_state}" \
      --input-path "${input_file}" \
      --input-metrics-json "${input_metrics_json}" \
      --replicate "${meta.replicate}" \
      --warmup-run "${meta.warmup_run}" \
      --subsample-fraction "${meta.subsample_fraction}" \
      --subsample-seed "${meta.subsample_seed}" \
      --subsample-mode "${meta.subsample_mode}" \
      --threads "${meta.threads}" \
      --container-image "${params.container_image}" \
      --output-target "${outputTarget}" \
      --command-file command.sh \
      --notes "${notes}"
    """
}
