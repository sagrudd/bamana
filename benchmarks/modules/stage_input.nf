nextflow.enable.dsl = 2

process STAGE_INPUT {
    tag "${meta.input_id}"
    publishDir "${params.output_dir}/input_metadata", mode: 'copy', pattern: "*.input_metrics.*"

    input:
    tuple val(meta), path(input_file)

    output:
    tuple val(meta), path(input_file), path("${meta.input_id}.input_metrics.json"), path("${meta.input_id}.input_metrics.tsv")

    script:
    def inputType = meta.input_type
    def mappingState = meta.mapping_state
    """
    set -euo pipefail

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
      --arg input_path "${input_file}" \
      --arg input_basename "${input_file.getFileName()}" \
      --argjson input_bytes "\${input_bytes}" \
      --argjson records_processed "\${records_processed}" \
      '{
        input_id: \$input_id,
        input_type: \$input_type,
        mapping_state: \$mapping_state,
        input_path: \$input_path,
        input_basename: \$input_basename,
        input_bytes: \$input_bytes,
        records_processed: \$records_processed
      }' \
      > "${meta.input_id}.input_metrics.json"

    printf "%s\\n" \
      "input_id\tinput_type\tmapping_state\tinput_path\tinput_basename\tinput_bytes\trecords_processed" \
      "${meta.input_id}\t${inputType}\t${mappingState}\t${input_file}\t${input_file.getFileName()}\t\${input_bytes}\t\${records_processed}" \
      > "${meta.input_id}.input_metrics.tsv"
    """
}
