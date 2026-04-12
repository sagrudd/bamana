nextflow.enable.dsl = 2

process AGGREGATE_RESULTS {
    tag "aggregate-results"
    publishDir "${params.output_dir}/summary", mode: 'copy'

    input:
    path(result_tsvs)

    output:
    path("aggregated/benchmark_runs.tsv"), emit: runs_tsv
    path("aggregated/benchmark_runs.json"), emit: runs_json
    path("aggregated/benchmark_summary.tsv"), emit: summary_tsv
    path("aggregated/benchmark_summary.json"), emit: summary_json
    path("aggregated/benchmark_support_matrix.tsv"), emit: support_tsv
    path("aggregated/benchmark_support_matrix.json"), emit: support_json
    path("aggregated/benchmark_failures.tsv"), emit: failures_tsv

    script:
    """
    set -euo pipefail
    mkdir -p aggregated
    Rscript "${projectDir}/R/aggregate_results.R" --input-dir . --outdir aggregated
    """
}
