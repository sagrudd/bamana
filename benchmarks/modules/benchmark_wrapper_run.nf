nextflow.enable.dsl = 2

process BENCHMARK_WRAPPER_RUN {
    tag "${meta.run_id}"
    publishDir "${params.raw_results_dir}", mode: 'copy', pattern: "*.result.json"
    publishDir "${params.raw_results_dir}", mode: 'copy', pattern: "*.result.tsv"
    publishDir "${params.metadata_dir}", mode: 'copy', pattern: "*.wrapper.json"
    publishDir "${params.metadata_dir}", mode: 'copy', pattern: "*.command.sh"
    publishDir "${params.logs_dir}", mode: 'copy', pattern: "*.command.log"
    publishDir "${params.logs_dir}", mode: 'copy', pattern: "*.stdout.log"
    publishDir "${params.logs_dir}", mode: 'copy', pattern: "*.stderr.log"
    publishDir "${params.logs_dir}", mode: 'copy', pattern: "*.time.tsv"

    input:
    tuple val(meta), path(input_file), path(input_metrics_json), path(input_metrics_tsv)

    output:
    tuple val(meta), path("${meta.run_id}.result.json"), emit: raw_json
    tuple val(meta), path("${meta.run_id}.result.tsv"), emit: raw_tsv
    tuple val(meta), path("${meta.run_id}.wrapper.json"), emit: wrapper_json
    tuple val(meta), path("${meta.run_id}.command.sh"), emit: command_file
    tuple val(meta), path("${meta.run_id}.command.log"), emit: command_log
    tuple val(meta), path("${meta.run_id}.stdout.log"), optional: true, emit: stdout_log
    tuple val(meta), path("${meta.run_id}.stderr.log"), optional: true, emit: stderr_log
    tuple val(meta), path("${meta.run_id}.time.tsv"), optional: true, emit: time_tsv

    script:
    def frameworkVersion = workflow.commitId ?: workflow.runName

    """
    set -euo pipefail

    wrapper_result="${meta.run_id}.wrapper.json"
    command_file="${meta.run_id}.command.sh"
    command_log="${meta.run_id}.command.log"
    timing_output="${meta.run_id}.wrapper.time.json"

    wrapper_cmd=(
      "${meta.wrapper_path}"
      --scenario "${meta.scenario}"
      --workflow-variant "${meta.workflow_variant}"
      --input "${input_file}"
      --output-dir "${PWD}"
      --threads "${meta.threads}"
      --subsample-fraction "${meta.subsample_fraction}"
      --subsample-seed "${meta.subsample_seed}"
      --subsample-mode "${meta.subsample_mode}"
      --sort-order "${meta.sort_order}"
      --result-output "${wrapper_result}"
      --command-file "${command_file}"
      --command-log "${command_log}"
      --timing-output "${timing_output}"
    )

    if [[ "${meta.create_index}" == "true" ]]; then
      wrapper_cmd+=(--create-index)
    fi

    if [[ -n "${meta.wrapper_binary_flag}" && -n "${meta.wrapper_binary_path}" ]]; then
      wrapper_cmd+=("${meta.wrapper_binary_flag}" "${meta.wrapper_binary_path}")
    fi

    "${wrapper_cmd[@]}"

    support_status="$(jq -r '.support_status' "${wrapper_result}")"
    semantic_equivalence="$(jq -r '.semantic_equivalence' "${wrapper_result}")"
    output_target="$(jq -r '.output_paths.primary // ""' "${wrapper_result}")"
    version_cmd="$(jq -r '.tool_version_command' "${wrapper_result}")"
    notes="$(jq -r 'if (.notes | length) == 0 then "" else (.notes | join("; ")) end' "${wrapper_result}")"

    BAMANA_BENCHMARK_FRAMEWORK_VERSION="${frameworkVersion}" \
      run_benchmark.sh \
        --run-id "${meta.run_id}" \
        --tool "${meta.tool}" \
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
