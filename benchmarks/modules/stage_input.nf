nextflow.enable.dsl = 2

process STAGE_INPUT {
    tag "${meta.input_id}"
    publishDir "${params.output_dir}/input_metadata", mode: 'copy', pattern: "*.input_metrics.*"

    input:
    tuple val(meta), path(input_file)

    output:
    tuple val(meta), path("${meta.input_id}.staged*"), path("${meta.input_id}.input_metrics.json"), path("${meta.input_id}.input_metrics.tsv")

    script:
    def inputType = meta.input_type
    def mappingState = meta.mapping_state
    def sourceInputId = meta.source_input_id ?: meta.input_id
    def sourceInputPath = meta.source_input_path ?: input_file.toString()
    def sourceInputType = meta.source_input_type ?: inputType
    def sourceCategory = meta.source_category ?: 'unspecified'
    def description = meta.description ?: ''
    def expectedSortOrder = meta.expected_sort_order ?: 'unspecified'
    def hasIndex = meta.has_index ? 'true' : 'false'
    def referenceContext = meta.reference_context ?: 'unspecified'
    def sourceOwner = meta.source_owner ?: 'unspecified'
    def sensitivityLevel = meta.sensitivity_level ?: 'unspecified'
    def storageContext = meta.storage_context ?: params.storage_context
    def stagingMode = meta.staging_mode ?: params.staging_mode
    def scenarioMaterialization = meta.scenario_materialization ?: 'source'
    def reuseMaterializedInputs = meta.reuse_materialized_inputs ? 'true' : 'false'
    def includeStagingInTiming = meta.include_staging_in_timing ? 'true' : 'false'
    def notes = meta.notes ?: ''
    """
    set -euo pipefail

    staged_path="${meta.input_id}.staged"
    case "${input_file}" in
      *.fastq.gz) staged_path="${staged_path}.fastq.gz" ;;
      *.fq.gz) staged_path="${staged_path}.fq.gz" ;;
      *.fastq) staged_path="${staged_path}.fastq" ;;
      *.fq) staged_path="${staged_path}.fq" ;;
      *.bam) staged_path="${staged_path}.bam" ;;
    esac

    case "${stagingMode}" in
      direct|symlink|stream)
        ln -sf "${input_file}" "\${staged_path}"
        staging_realization="link"
        ;;
      hardlink)
        ln "${input_file}" "\${staged_path}" 2>/dev/null || cp -p "${input_file}" "\${staged_path}"
        staging_realization="hardlink_or_copy_fallback"
        ;;
      copy|scratch_copy)
        cp -p "${input_file}" "\${staged_path}"
        staging_realization="copy"
        ;;
      *)
        echo "unsupported staging mode: ${stagingMode}" >&2
        exit 2
        ;;
    esac

    input_bytes=\$(stat -c %s "${input_file}")

    if [[ "${inputType}" == "BAM" ]]; then
      records_processed=\$(samtools view -c "${input_file}")
    elif [[ "${inputType}" == "FASTQ_GZ" ]]; then
      records_processed=\$(gzip -dc "${input_file}" | awk 'END { printf "%.0f", NR / 4 }')
    else
      records_processed=0
    fi

    jq -n \
      --arg input_id "${meta.input_id}" \
      --arg input_type "${inputType}" \
      --arg mapping_state "${mappingState}" \
      --arg input_path "\${staged_path}" \
      --arg input_basename "${input_file.getFileName()}" \
      --arg source_input_id "${sourceInputId}" \
      --arg source_input_path "${sourceInputPath}" \
      --arg source_input_type "${sourceInputType}" \
      --arg source_category "${sourceCategory}" \
      --arg description "${description}" \
      --arg expected_sort_order "${expectedSortOrder}" \
      --argjson has_index ${hasIndex} \
      --arg reference_context "${referenceContext}" \
      --arg source_owner "${sourceOwner}" \
      --arg sensitivity_level "${sensitivityLevel}" \
      --arg staged_input_id "${meta.input_id}" \
      --arg staged_input_path "\${staged_path}" \
      --arg staging_mode "${stagingMode}" \
      --arg staging_realization "\${staging_realization}" \
      --arg storage_context "${storageContext}" \
      --arg scenario_materialization "${scenarioMaterialization}" \
      --argjson reuse_materialized_inputs ${reuseMaterializedInputs} \
      --argjson include_staging_in_timing ${includeStagingInTiming} \
      --arg notes "${notes}" \
      --argjson input_bytes "\${input_bytes}" \
      --argjson records_processed "\${records_processed}" \
      '{
        input_id: \$input_id,
        input_type: \$input_type,
        mapping_state: \$mapping_state,
        input_path: \$input_path,
        input_basename: \$input_basename,
        source_input_id: \$source_input_id,
        source_input_path: \$source_input_path,
        source_input_type: \$source_input_type,
        source_category: \$source_category,
        description: \$description,
        expected_sort_order: \$expected_sort_order,
        has_index: \$has_index,
        reference_context: \$reference_context,
        source_owner: \$source_owner,
        sensitivity_level: \$sensitivity_level,
        staged_input_id: \$staged_input_id,
        staged_input_path: \$staged_input_path,
        staging_mode: \$staging_mode,
        staging_realization: \$staging_realization,
        storage_context: \$storage_context,
        scenario_materialization: \$scenario_materialization,
        reuse_materialized_inputs: \$reuse_materialized_inputs,
        include_staging_in_timing: \$include_staging_in_timing,
        input_bytes: \$input_bytes,
        records_processed: \$records_processed,
        notes: \$notes
      }' \
      > "${meta.input_id}.input_metrics.json"

    printf "%s\\n" \
      "input_id\tinput_type\tmapping_state\tinput_path\tinput_basename\tsource_input_id\tsource_input_path\tsource_input_type\tsource_category\texpected_sort_order\thas_index\treference_context\tsource_owner\tsensitivity_level\tstaged_input_id\tstaged_input_path\tstaging_mode\tstaging_realization\tstorage_context\tscenario_materialization\treuse_materialized_inputs\tinclude_staging_in_timing\tinput_bytes\trecords_processed\tnotes" \
      "${meta.input_id}\t${inputType}\t${mappingState}\t\${staged_path}\t${input_file.getFileName()}\t${sourceInputId}\t${sourceInputPath}\t${sourceInputType}\t${sourceCategory}\t${expectedSortOrder}\t${hasIndex}\t${referenceContext}\t${sourceOwner}\t${sensitivityLevel}\t${meta.input_id}\t\${staged_path}\t${stagingMode}\t\${staging_realization}\t${storageContext}\t${scenarioMaterialization}\t${reuseMaterializedInputs}\t${includeStagingInTiming}\t\${input_bytes}\t\${records_processed}\t${notes}" \
      > "${meta.input_id}.input_metrics.tsv"
    """
}
