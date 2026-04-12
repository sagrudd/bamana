#!/usr/bin/env Rscript

suppressPackageStartupMessages({
  library(dplyr)
  library(jsonlite)
  library(purrr)
  library(readr)
  library(stringr)
  library(tibble)
})

args <- commandArgs(trailingOnly = TRUE)

arg_value <- function(flag, default = NULL) {
  index <- match(flag, args)
  if (is.na(index) || index == length(args)) {
    return(default)
  }
  args[[index + 1]]
}

input_dir <- arg_value("--input-dir", ".")
outdir <- arg_value("--outdir", "aggregated")

dir.create(outdir, recursive = TRUE, showWarnings = FALSE)

result_files <- list.files(
  input_dir,
  pattern = "\\.result\\.tsv$",
  recursive = TRUE,
  full.names = TRUE
)

if (length(result_files) == 0) {
  stop("No .result.tsv files were found for aggregation.")
}

runs <- result_files |>
  map_dfr(~ read_tsv(.x, show_col_types = FALSE, na = c("", "NA", "null")))

runs <- runs |>
  mutate(
    success = as.logical(success),
    warmup_run = as.logical(warmup_run),
    across(
      c(
        input_bytes,
        input_records,
        replicate,
        subsample_fraction,
        subsample_seed,
        threads,
        wall_seconds,
        user_cpu_seconds,
        system_cpu_seconds,
        cpu_seconds,
        max_rss_bytes,
        exit_code,
        output_bytes,
        compression_ratio,
        records_processed
      ),
      as.numeric
    ),
    throughput_records_per_sec = if_else(
      !is.na(wall_seconds) & wall_seconds > 0,
      records_processed / wall_seconds,
      NA_real_
    ),
    throughput_gb_per_sec = if_else(
      !is.na(wall_seconds) & wall_seconds > 0,
      (input_bytes / 1e9) / wall_seconds,
      NA_real_
    )
  )

analysis_runs <- runs |>
  filter(!warmup_run)

summary <- analysis_runs |>
  group_by(
    scenario,
    input_type,
    mapping_state,
    tool,
    workflow_variant,
    semantic_equivalence,
    support_status,
    subsample_fraction,
    subsample_seed,
    subsample_mode,
    threads
  ) |>
  summarise(
    run_count = n(),
    successful_runs = sum(success, na.rm = TRUE),
    median_wall_seconds = median(wall_seconds, na.rm = TRUE),
    iqr_wall_seconds = IQR(wall_seconds, na.rm = TRUE),
    sd_wall_seconds = sd(wall_seconds, na.rm = TRUE),
    median_cpu_seconds = median(cpu_seconds, na.rm = TRUE),
    median_max_rss_bytes = median(max_rss_bytes, na.rm = TRUE),
    median_output_bytes = median(output_bytes, na.rm = TRUE),
    median_records_per_sec = median(throughput_records_per_sec, na.rm = TRUE),
    median_gb_per_sec = median(throughput_gb_per_sec, na.rm = TRUE),
    .groups = "drop"
  )

support_matrix <- analysis_runs |>
  group_by(scenario, input_type, tool, workflow_variant, semantic_equivalence, support_status) |>
  summarise(
    run_count = n(),
    successful_runs = sum(success, na.rm = TRUE),
    notes = str_c(unique(na.omit(notes)), collapse = " | "),
    .groups = "drop"
  )

failures <- analysis_runs |>
  filter(support_status != "completed" | !success)

write_tsv(runs, file.path(outdir, "benchmark_runs.tsv"))
write_json(runs, file.path(outdir, "benchmark_runs.json"), pretty = TRUE, auto_unbox = TRUE)

write_tsv(summary, file.path(outdir, "benchmark_summary.tsv"))
write_json(summary, file.path(outdir, "benchmark_summary.json"), pretty = TRUE, auto_unbox = TRUE)

write_tsv(support_matrix, file.path(outdir, "benchmark_support_matrix.tsv"))
write_json(support_matrix, file.path(outdir, "benchmark_support_matrix.json"), pretty = TRUE, auto_unbox = TRUE)

write_tsv(failures, file.path(outdir, "benchmark_failures.tsv"))
