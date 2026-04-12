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
    warmup = as.logical(warmup),
    success = as.logical(success),
    unsupported = as.logical(unsupported),
    failed = as.logical(failed),
    staging_included_in_timing = as.logical(staging_included_in_timing),
    reuse_materialized_inputs = as.logical(reuse_materialized_inputs),
    has_index = as.logical(has_index),
    subsample_enabled = as.logical(subsample_enabled),
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
    status = coalesce(status, support_status),
    throughput_records_per_sec = if_else(
      is.na(throughput_records_per_sec) & !is.na(wall_seconds) & wall_seconds > 0,
      records_processed / wall_seconds,
      throughput_records_per_sec
    ),
    throughput_bytes_per_sec = if_else(
      is.na(throughput_bytes_per_sec) & !is.na(wall_seconds) & wall_seconds > 0,
      input_bytes / wall_seconds,
      throughput_bytes_per_sec
    )
  )

analysis_runs <- runs |>
  filter(!warmup)

summary <- analysis_runs |>
  group_by(
    schema_version,
    scenario,
    input_type,
    mapping_state,
    tool,
    workflow_variant,
    semantic_equivalence,
    source_input_id,
    source_input_type,
    staged_input_id,
    subsample_fraction,
    subsample_seed,
    subsample_mode,
    threads
  ) |>
  summarise(
    n_runs = n(),
    n_success = sum(status == "success", na.rm = TRUE),
    n_failed = sum(status == "failed", na.rm = TRUE),
    n_unsupported = sum(status == "unsupported", na.rm = TRUE),
    n_skipped = sum(status == "skipped", na.rm = TRUE),
    mean_wall_seconds = if_else(n_success > 0, mean(wall_seconds[status == "success"], na.rm = TRUE), NA_real_),
    median_wall_seconds = if_else(n_success > 0, median(wall_seconds[status == "success"], na.rm = TRUE), NA_real_),
    iqr_wall_seconds = if_else(n_success > 0, IQR(wall_seconds[status == "success"], na.rm = TRUE), NA_real_),
    sd_wall_seconds = if_else(n_success > 1, sd(wall_seconds[status == "success"], na.rm = TRUE), NA_real_),
    median_cpu_seconds = if_else(n_success > 0, median(cpu_seconds[status == "success"], na.rm = TRUE), NA_real_),
    median_max_rss_bytes = if_else(n_success > 0, median(max_rss_bytes[status == "success"], na.rm = TRUE), NA_real_),
    median_output_bytes = if_else(n_success > 0, median(output_bytes[status == "success"], na.rm = TRUE), NA_real_),
    median_records_per_sec = if_else(n_success > 0, median(throughput_records_per_sec[status == "success"], na.rm = TRUE), NA_real_),
    median_bytes_per_sec = if_else(n_success > 0, median(throughput_bytes_per_sec[status == "success"], na.rm = TRUE), NA_real_),
    .groups = "drop"
  )

support_matrix <- analysis_runs |>
  group_by(scenario, input_type, tool, workflow_variant, semantic_equivalence) |>
  summarise(
    n_runs = n(),
    n_success = sum(status == "success", na.rm = TRUE),
    n_failed = sum(status == "failed", na.rm = TRUE),
    n_unsupported = sum(status == "unsupported", na.rm = TRUE),
    n_skipped = sum(status == "skipped", na.rm = TRUE),
    status = case_when(
      n_success > 0 ~ "success",
      n_unsupported == n_runs ~ "unsupported",
      n_failed > 0 ~ "failed",
      n_skipped == n_runs ~ "skipped",
      TRUE ~ "mixed"
    ),
    notes = str_c(unique(na.omit(notes)), collapse = " | "),
    .groups = "drop"
  )

failures <- analysis_runs |>
  filter(status != "success" | !success)

write_tsv(runs, file.path(outdir, "benchmark_runs.tsv"))
write_json(runs, file.path(outdir, "benchmark_runs.json"), pretty = TRUE, auto_unbox = TRUE)

write_tsv(summary, file.path(outdir, "benchmark_summary.tsv"))
write_json(summary, file.path(outdir, "benchmark_summary.json"), pretty = TRUE, auto_unbox = TRUE)

write_tsv(support_matrix, file.path(outdir, "benchmark_support_matrix.tsv"))
write_json(support_matrix, file.path(outdir, "benchmark_support_matrix.json"), pretty = TRUE, auto_unbox = TRUE)

write_tsv(failures, file.path(outdir, "benchmark_failures.tsv"))
