nextflow.enable.dsl = 2

process RUN_SEQTK_BENCHMARK {
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

    if (meta.scenario == 'fastq_consume_pipeline' || meta.scenario == 'subsample_only') {
        outputTarget = "${meta.run_id}.sampled.fastq.gz"
        command = """\
set -euo pipefail
seqtk sample -s${meta.subsample_seed} "${input_file}" ${meta.subsample_fraction} | gzip -c > "${outputTarget}"
"""
        notes = meta.scenario == 'subsample_only'
            ? 'seqtk is included as a FASTQ-only subsample baseline for the explicit subsample-only scenario.'
            : 'seqtk is included as a FASTQ subsampling baseline only; it does not normalize into BAM in this scenario.'
    } else {
        supportStatus = 'unsupported'
        semanticEquivalence = 'unsupported'
        notes = 'seqtk is not benchmarked for BAM scenarios in this first framework.'
    }

    """
    cat <<'EOF' > command.sh
${command}
EOF
    chmod +x command.sh

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "seqtk" \
      --tool-version-cmd "seqtk 2>&1" \
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
