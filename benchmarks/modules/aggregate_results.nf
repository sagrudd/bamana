nextflow.enable.dsl = 2

process AGGREGATE_RESULTS {
    tag "aggregate-results"
    publishDir "${params.output_dir}/aggregated", mode: 'copy'

    input:
    path(result_jsons)

    output:
    path("aggregated/tidy_results.csv"), emit: tidy_csv
    path("aggregated/tidy_summary.csv"), emit: summary_csv

    script:
    """
    set -euo pipefail
    mkdir -p aggregated
    cp ${result_jsons} .
    Rscript "${projectDir}/R/aggregate_results.R" --input-dir . --output-dir aggregated
    """
}
