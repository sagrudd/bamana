nextflow.enable.dsl = 2

process RUN_SAMBAMBA_BENCHMARK {
    tag "${meta.run_id}"
    publishDir "${params.output_dir}/per_run", mode: 'copy'

    input:
    tuple val(meta), path(input_file), path(input_metrics_json), path(input_metrics_tsv)

    output:
    path("${meta.run_id}.result.tsv"), emit: result_tsv
    path("${meta.run_id}.result.json"), emit: result_json

    script:
    def outputTarget = ''
    def supportStatus = 'supported'
    def semanticEquivalence = 'full'
    def notes = ''
    def command = 'true'

    if (meta.scenario == 'mapped_bam_pipeline') {
        outputTarget = "${meta.run_id}.sorted.bam"
        command = """\
set -euo pipefail
sambamba view -t ${meta.threads} --subsampling-seed=${meta.subsample_seed} -s ${meta.subsample_fraction} -f bam -o "${meta.run_id}.subsampled.bam" "${input_file}"
sambamba sort -t ${meta.threads} -o "${outputTarget}" "${meta.run_id}.subsampled.bam"
sambamba index -t ${meta.threads} "${outputTarget}"
"""
        notes = 'sambamba is included as an additional BAM-oriented comparator with a broadly comparable mapped BAM workflow.'
    } else if (meta.scenario == 'unmapped_bam_pipeline' || meta.scenario == 'subsample_only') {
        outputTarget = "${meta.run_id}.subsampled.bam"
        command = """\
set -euo pipefail
sambamba view -t ${meta.threads} --subsampling-seed=${meta.subsample_seed} -s ${meta.subsample_fraction} -f bam -o "${outputTarget}" "${input_file}"
"""
        notes = meta.scenario == 'subsample_only'
            ? 'sambamba subsample-only benchmarking uses the direct BAM subsampling path without sort or index.'
            : 'Unmapped BAM scenario is benchmarked as subsample only for sambamba as well.'
    } else {
        supportStatus = 'unsupported'
        semanticEquivalence = 'unsupported'
        notes = 'sambamba is not benchmarked for raw FASTQ.GZ consume workflows in this first benchmark contract.'
    }

    """
    cat <<'EOF' > command.sh
${command}
EOF
    chmod +x command.sh

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "sambamba" \
      --tool-version-cmd "sambamba --version" \
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
