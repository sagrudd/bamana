nextflow.enable.dsl = 2

process REPORT_RESULTS {
    tag "benchmark-report"
    publishDir "${params.output_dir}/reports", mode: 'copy'

    input:
    path(tidy_csv)
    path(summary_csv)

    output:
    path("reports/benchmark_report.pdf"), emit: report_pdf
    path("reports/benchmark_report_summary.csv"), emit: report_summary_csv

    script:
    """
    set -euo pipefail
    mkdir -p reports
    Rscript "${projectDir}/R/build_benchmark_report.R" \
      --tidy-csv "${tidy_csv}" \
      --summary-csv "${summary_csv}" \
      --output-dir reports
    """
}
