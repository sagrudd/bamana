nextflow.enable.dsl = 2

process PLOT_RESULTS {
    tag "plot-benchmarks"
    publishDir "${params.output_dir}/figures", mode: 'copy'

    input:
    path(runs_tsv)
    path(summary_tsv)
    path(support_tsv)

    output:
    path("figures/wall_time_by_tool.pdf"), emit: wall_pdf
    path("figures/wall_time_by_tool.png"), emit: wall_png
    path("figures/throughput_by_tool.pdf"), emit: throughput_pdf
    path("figures/throughput_by_tool.png"), emit: throughput_png
    path("figures/memory_by_tool.pdf"), emit: memory_pdf
    path("figures/memory_by_tool.png"), emit: memory_png
    path("figures/replicate_variability.pdf"), emit: variability_pdf
    path("figures/replicate_variability.png"), emit: variability_png
    path("figures/support_status_heatmap.pdf"), emit: support_pdf
    path("figures/support_status_heatmap.png"), emit: support_png

    script:
    """
    set -euo pipefail
    mkdir -p figures
    Rscript "${projectDir}/R/plot_benchmarks.R" \
      --runs-tsv "${runs_tsv}" \
      --summary-tsv "${summary_tsv}" \
      --support-tsv "${support_tsv}" \
      --outdir figures
    """
}
