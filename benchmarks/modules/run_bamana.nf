nextflow.enable.dsl = 2

process RUN_BAMANA_BENCHMARK {
    tag "${meta.run_id}"
    publishDir "${params.output_dir}/per_run", mode: 'copy'

    input:
    tuple val(meta), path(input_file), path(input_metrics_json), path(input_metrics_tsv)

    output:
    path("${meta.run_id}.result.tsv"), emit: result_tsv
    path("${meta.run_id}.result.json"), emit: result_json

    script:
    def outputTarget = meta.scenario == 'fastq_ingest_chain' ? "${meta.run_id}.output.bam" : ''
    def supportStatus = 'supported'
    def semanticEquivalence = meta.scenario == 'fastq_ingest_chain' ? 'full' : 'roadmap_blocked'
    def notes = ''
    def command = 'true'

    if (meta.scenario == 'fastq_ingest_chain') {
        command = """\
set -euo pipefail
"${meta.bamana_bin}" consume --input "${input_file}" --out "${outputTarget}" --mode unmapped --force
"""
        notes = 'Bamana fastq ingestion uses consume because the benchmark framework also tracks current ingestion performance before subsample lands.'
    } else {
        supportStatus = 'roadmap_blocked'
        notes = 'Bamana BAM benchmark path is blocked until bamana subsample exists and bamana index supports executable index creation.'
    }

    """
    cat <<'EOF' > command.sh
${command}
EOF
    chmod +x command.sh

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "bamana" \
      --tool-version-cmd "${meta.bamana_bin} --version" \
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
