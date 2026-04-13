nextflow.enable.dsl = 2

process RUN_RASUSA_BENCHMARK {
    tag "${meta.run_id}"
    publishDir "${params.output_dir}/per_run", mode: 'copy'

    input:
    tuple val(meta), path(input_file), path(input_metrics_json), path(input_metrics_tsv)

    output:
    path("${meta.run_id}.result.tsv"), emit: result_tsv
    path("${meta.run_id}.result.json"), emit: result_json

    script:
    def notes = 'rasusa is listed explicitly because it is widely used for read and alignment downsampling, but the first benchmark contract keeps it unsupported by default until a fair coverage-or-count strategy is pinned beside the fractional scenario design.'
    """
    cat <<'EOF' > command.sh
true
EOF
    chmod +x command.sh

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "rasusa" \
      --tool-version-cmd "rasusa --version" \
      --scenario "${meta.scenario}" \
      --workflow-variant "${meta.workflow_variant}" \
      --semantic-equivalence "unsupported" \
      --support-status "unsupported" \
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
      --output-target "" \
      --command-file command.sh \
      --notes "${notes}"
    """
}
