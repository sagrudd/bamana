nextflow.enable.dsl = 2

process PLOT_RESULTS {
    tag "plot-benchmarks"
    publishDir "${params.output_dir}/plots", mode: 'copy'

    input:
    path(tidy_csv)
    path(summary_csv)

    output:
    path("plots/wall_time_by_tool.pdf"), emit: wall_pdf
    path("plots/wall_time_by_tool.png"), emit: wall_png

    script:
    """
    set -euo pipefail
    mkdir -p plots
    Rscript "${projectDir}/R/plot_benchmarks.R" \
      --tidy-csv "${tidy_csv}" \
      --summary-csv "${summary_csv}" \
      --output-dir plots
    """
}
