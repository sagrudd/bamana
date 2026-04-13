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
    def wrapperPath = "${projectDir}/benchmarks/tools/wrappers/fastcat.sh"

    """
    wrapper_result="${meta.run_id}.wrapper.json"
    command_file="${meta.run_id}.command.sh"
    command_log="${meta.run_id}.command.log"
    timing_output="${meta.run_id}.wrapper.time.json"

    "${wrapperPath}" \
      --scenario "${meta.scenario}" \
      --workflow-variant "${meta.workflow_variant}" \
      --input "${input_file}" \
      --output-dir "${PWD}" \
      --threads "${meta.threads}" \
      --subsample-fraction "${meta.subsample_fraction}" \
      --subsample-seed "${meta.subsample_seed}" \
      --subsample-mode "${meta.subsample_mode}" \
      --sort-order "none" \
      --result-output "${wrapper_result}" \
      --command-file "${command_file}" \
      --command-log "${command_log}" \
      --timing-output "${timing_output}"

    support_status="$(jq -r '.support_status' "${wrapper_result}")"
    semantic_equivalence="$(jq -r '.semantic_equivalence' "${wrapper_result}")"
    output_target="$(jq -r '.output_paths.primary // ""' "${wrapper_result}")"
    version_cmd="$(jq -r '.tool_version_command' "${wrapper_result}")"
    notes="$(jq -r 'if (.notes | length) == 0 then "" else (.notes | join("; ")) end' "${wrapper_result}")"

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "fastcat" \
      --tool-version-cmd "${version_cmd}" \
      --scenario "${meta.scenario}" \
      --workflow-variant "${meta.workflow_variant}" \
      --semantic-equivalence "${semantic_equivalence}" \
      --support-status "${support_status}" \
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
      --output-target "${output_target}" \
      --command-file "${command_file}" \
      --notes "${notes}"
    """
}
