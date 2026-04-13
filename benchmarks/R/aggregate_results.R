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

`%||%` <- function(lhs, rhs) {
  if (is.null(lhs) || length(lhs) == 0) {
    return(rhs)
  }
  lhs
}

as_note_string <- function(value) {
  if (is.null(value) || length(value) == 0) {
    return("")
  }

  notes <- unique(as.character(value))
  notes <- notes[notes != ""]
  if (length(notes) == 0) {
    return("")
  }

  str_c(notes, collapse = " | ")
}

as_json_string <- function(value) {
  if (is.null(value) || length(value) == 0) {
    return(NA_character_)
  }

  if (is.character(value) && length(value) == 1) {
    return(value)
  }

  toJSON(value, auto_unbox = TRUE, null = "null")
}

first_non_null <- function(...) {
  values <- list(...)
  for (value in values) {
    if (!is.null(value) && length(value) > 0) {
      return(value)
    }
  }
  NULL
}

flatten_result_file <- function(path) {
  raw <- fromJSON(path, simplifyVector = TRUE)

  input <- raw$input %||% list()
  staging <- raw$staging %||% list()
  subsampling <- raw$subsampling %||% list()
  resources <- raw$resources %||% list()
  execution <- raw$execution %||% list()
  result <- raw$result %||% list()

  records_processed <- first_non_null(
    result$records_processed,
    input$records_processed
  )

  wall_seconds <- execution$wall_seconds %||% NA_real_
  input_bytes <- input$input_bytes %||% NA_real_

  throughput_records_per_sec <- result$throughput_records_per_sec %||% NA_real_
  if (is.na(throughput_records_per_sec) &&
      !is.null(records_processed) &&
      !is.na(wall_seconds) &&
      wall_seconds > 0) {
    throughput_records_per_sec <- records_processed / wall_seconds
  }

  throughput_bytes_per_sec <- result$throughput_bytes_per_sec %||% NA_real_
  if (is.na(throughput_bytes_per_sec) &&
      !is.na(input_bytes) &&
      !is.na(wall_seconds) &&
      wall_seconds > 0) {
    throughput_bytes_per_sec <- input_bytes / wall_seconds
  }

  tibble(
    schema_version = raw$schema_version %||% NA_character_,
    run_id = raw$run_id %||% tools::file_path_sans_ext(basename(path)),
    timestamp_utc = raw$timestamp_utc %||% NA_character_,
    benchmark_framework_version = raw$benchmark_framework_version %||% NA_character_,
    scenario = raw$scenario %||% NA_character_,
    workflow_variant = raw$workflow_variant %||% NA_character_,
    tool = raw$tool %||% NA_character_,
    tool_version = raw$tool_version %||% NA_character_,
    source_input_id = input$source_input_id %||% NA_character_,
    source_input_path = input$source_input_path %||% NA_character_,
    source_input_type = input$source_input_type %||% NA_character_,
    source_category = input$source_category %||% NA_character_,
    staged_input_id = input$staged_input_id %||% NA_character_,
    staged_input_path = input$staged_input_path %||% NA_character_,
    input_type = input$input_type %||% NA_character_,
    mapping_state = input$mapped_state %||% NA_character_,
    input_basename = input$input_basename %||% NA_character_,
    expected_sort_order = input$expected_sort_order %||% NA_character_,
    has_index = input$has_index %||% NA,
    reference_context = as_json_string(input$reference_context),
    staging_mode = staging$staging_mode %||% NA_character_,
    staging_realization = staging$staging_realization %||% NA_character_,
    scenario_materialization = staging$scenario_materialization %||% NA_character_,
    reuse_materialized_inputs = staging$reuse_materialized_inputs %||% NA,
    staging_included_in_timing = staging$include_in_timing %||% NA,
    storage_context = staging$storage_context %||% NA_character_,
    input_bytes = input_bytes,
    input_records = input$records_processed %||% NA_real_,
    replicate = execution$replicate %||% NA_integer_,
    warmup = execution$warmup_run %||% NA,
    subsample_enabled = subsampling$enabled %||% FALSE,
    subsample_fraction = subsampling$fraction %||% NA_real_,
    subsample_seed = subsampling$seed %||% NA_real_,
    subsample_mode = subsampling$mode %||% NA_character_,
    threads = resources$threads %||% NA_integer_,
    semantic_equivalence = result$semantic_equivalence %||% NA_character_,
    status = result$status %||% NA_character_,
    support_status = result$support_status %||% NA_character_,
    success = result$success %||% FALSE,
    unsupported = result$unsupported %||% FALSE,
    failed = result$failed %||% FALSE,
    failure_category = result$failure_category %||% NA_character_,
    exit_code = execution$exit_code %||% NA_integer_,
    wall_seconds = wall_seconds,
    user_cpu_seconds = execution$user_cpu_seconds %||% NA_real_,
    system_cpu_seconds = execution$system_cpu_seconds %||% NA_real_,
    cpu_seconds = execution$cpu_seconds %||% NA_real_,
    max_rss_bytes = execution$max_rss_bytes %||% NA_real_,
    output_path = result$output_path %||% NA_character_,
    output_bytes = result$output_bytes %||% NA_real_,
    compression_ratio = result$compression_ratio %||% NA_real_,
    records_processed = records_processed %||% NA_real_,
    throughput_records_per_sec = throughput_records_per_sec,
    throughput_bytes_per_sec = throughput_bytes_per_sec,
    container_image = resources$container_image %||% NA_character_,
    hardware_host_label = resources$hardware_host_label %||% NA_character_,
    memory_target = resources$memory_target %||% NA_character_,
    command_line = execution$command_line %||% NA_character_,
    notes = as_note_string(raw$notes),
    started_at = execution$started_at %||% NA_character_,
    finished_at = execution$finished_at %||% NA_character_
  )
}

success_metric <- function(values, statuses, fn, require_n = 1L) {
  success_values <- values[statuses == "success"]
  success_values <- success_values[!is.na(success_values)]

  if (length(success_values) < require_n) {
    return(NA_real_)
  }

  fn(success_values)
}

input_dir <- arg_value("--input-dir", "raw")
output_dir <- arg_value("--output-dir", arg_value("--outdir", "aggregated"))

dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)

result_files <- list.files(
  input_dir,
  pattern = "\\.result\\.json$",
  recursive = TRUE,
  full.names = TRUE
)

if (length(result_files) == 0) {
  stop("No .result.json files were found for aggregation.")
}

runs <- result_files |>
  sort() |>
  map_dfr(flatten_result_file) |>
  mutate(
    across(
      c(
        input_bytes,
        input_records,
        replicate,
        subsample_fraction,
        subsample_seed,
        threads,
        exit_code,
        wall_seconds,
        user_cpu_seconds,
        system_cpu_seconds,
        cpu_seconds,
        max_rss_bytes,
        output_bytes,
        compression_ratio,
        records_processed,
        throughput_records_per_sec,
        throughput_bytes_per_sec
      ),
      as.numeric
    ),
    across(
      c(
        has_index,
        warmup,
        subsample_enabled,
        reuse_materialized_inputs,
        staging_included_in_timing,
        success,
        unsupported,
        failed
      ),
      as.logical
    )
  )

analysis_runs <- runs |>
  filter(is.na(warmup) | !warmup)

summary <- analysis_runs |>
  group_by(
    scenario,
    workflow_variant,
    tool,
    source_input_id,
    source_input_type,
    threads,
    subsample_mode,
    subsample_fraction,
    subsample_seed
  ) |>
  summarise(
    n_runs = n(),
    n_success = sum(status == "success", na.rm = TRUE),
    n_failed = sum(status == "failed", na.rm = TRUE),
    n_unsupported = sum(status == "unsupported", na.rm = TRUE),
    median_wall_seconds = success_metric(wall_seconds, status, median),
    mean_wall_seconds = success_metric(wall_seconds, status, mean),
    sd_wall_seconds = success_metric(wall_seconds, status, sd, require_n = 2L),
    median_max_rss_bytes = success_metric(max_rss_bytes, status, median),
    median_records_per_sec = success_metric(throughput_records_per_sec, status, median),
    median_bytes_per_sec = success_metric(throughput_bytes_per_sec, status, median),
    notes = str_c(unique(na.omit(notes[nzchar(notes)])), collapse = " | "),
    .groups = "drop"
  ) |>
  mutate(notes = if_else(notes == "", NA_character_, notes))

write_csv(runs, file.path(output_dir, "tidy_results.csv"), na = "")
write_csv(summary, file.path(output_dir, "tidy_summary.csv"), na = "")
