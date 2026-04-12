nextflow.enable.dsl = 2

process INDEX_RAW_RESULTS {
    tag "index-raw-results"
    publishDir "${params.metadata_dir}", mode: 'copy'

    input:
    path(raw_result_jsons)

    output:
    path("raw_result_inventory.tsv"), emit: inventory_tsv
    path("raw_result_inventory.json"), emit: inventory_json

    script:
    """
    set -euo pipefail

    printf "%s\\n" \
      "run_id\ttool\tscenario\tworkflow_variant\tstatus\tsupport_status\tresult_path" \
      > raw_result_inventory.tsv

    for result_file in ${raw_result_jsons}; do
      published_path="${params.raw_results_dir}/$(basename "$result_file")"
      jq -r '[.run_id, .tool, .scenario, .workflow_variant, .result.status, .result.support_status, input_filename] | @tsv' \
        --arg input_filename "$published_path" \
        "$result_file" >> raw_result_inventory.tsv
    done

    tail -n +2 raw_result_inventory.tsv | jq -R -s '
      split("\\n")
      | map(select(length > 0))
      | map(split("\\t"))
      | map({
          run_id: .[0],
          tool: .[1],
          scenario: .[2],
          workflow_variant: .[3],
          status: .[4],
          support_status: .[5],
          result_path: .[6]
        })
    ' > raw_result_inventory.json
    """
}
