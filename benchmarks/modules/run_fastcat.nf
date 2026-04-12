nextflow.enable.dsl = 2

process RUN_FASTCAT_BENCHMARK {
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
    def semanticEquivalence = 'partial'
    def notes = ''
    def command = 'true'

    if (meta.scenario == 'fastq_consume_pipeline') {
        outputTarget = "${meta.run_id}.fastcat.fastq.gz"
        command = """\
set -euo pipefail
fastcat "${input_file}" | gzip -c > "${outputTarget}"
"""
        notes = 'fastcat is the ONT-oriented ingestion and concatenation baseline. Its workflow is intentionally partial versus BAM normalization but central for the project goal of eventually beating fastcat in this space.'
    } else {
        supportStatus = 'unsupported'
        semanticEquivalence = 'unsupported'
        notes = 'fastcat is only benchmarked for FASTQ.GZ ingestion-style scenarios.'
    }

    """
    cat <<'EOF' > command.sh
${command}
EOF
    chmod +x command.sh

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "fastcat" \
      --tool-version-cmd "fastcat --version" \
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
