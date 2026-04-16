#!/usr/bin/env Rscript

suppressPackageStartupMessages({
  library(dplyr)
  library(ggplot2)
  library(jsonlite)
  library(purrr)
  library(readr)
  library(stringr)
  library(tibble)
})

args <- commandArgs(trailingOnly = TRUE)
full_args <- commandArgs(FALSE)
script_file <- sub("^--file=", "", full_args[grepl("^--file=", full_args)])[1]
script_dir <- dirname(normalizePath(script_file, mustWork = FALSE))

arg_value <- function(flag, default = NULL) {
  index <- match(flag, args)
  if (is.na(index) || index == length(args)) {
    return(default)
  }
  args[[index + 1]]
}

read_table_auto <- function(path) {
  if (str_detect(path, "\\.csv$")) {
    return(read_csv(path, show_col_types = FALSE, na = c("", "NA", "null")))
  }
  read_tsv(path, show_col_types = FALSE, na = c("", "NA", "null"))
}

default_registry <- normalizePath(
  file.path(script_dir, "..", "tools", "tool_registry.example.json"),
  mustWork = FALSE
)

runs_path <- arg_value("--runs-csv", arg_value("--runs-tsv", "aggregated/tidy_results.csv"))
tool_registry_path <- arg_value("--tool-registry", default_registry)
outdir <- arg_value("--outdir", "aggregated")
render_plot <- arg_value("--render-plot", "true")

dir.create(outdir, recursive = TRUE, showWarnings = FALSE)

derive_input_type <- function(tool, scenario) {
  if (scenario == "fastq_consume_pipeline") {
    return("FASTQ_GZ")
  }
  if (scenario %in% c("mapped_bam_pipeline", "unmapped_bam_pipeline")) {
    return("BAM")
  }
  if (scenario == "subsample_only" && tool %in% c("seqtk", "fastcat")) {
    return("FASTQ_GZ")
  }
  if (scenario == "subsample_only" && tool == "bamana") {
    return("BAM_or_FASTQ_GZ")
  }
  "BAM"
}

derive_summary_status <- function(status_values) {
  statuses <- unique(status_values)
  if (length(statuses) == 0) {
    return("not_attempted")
  }
  if (all(statuses == "unsupported")) {
    return("unsupported")
  }
  if (all(statuses == "not_attempted")) {
    return("not_attempted")
  }
  if ("mixed_results" %in% statuses) {
    return("mixed_results")
  }
  if ("supported_success" %in% statuses && any(statuses %in% c("supported_failed", "not_attempted", "unsupported"))) {
    return("mixed_results")
  }
  if ("supported_success" %in% statuses) {
    return("supported_success")
  }
  if ("supported_failed" %in% statuses) {
    return("supported_failed")
  }
  "not_attempted"
}

registry <- fromJSON(tool_registry_path, simplifyVector = FALSE)

capabilities <- map_dfr(registry$tools, function(tool_entry) {
  map_dfr(tool_entry$workflow_variants, function(variant) {
    tibble(
      tool = tool_entry$id,
      tool_display_name = tool_entry$display_name,
      scenario = variant$scenario,
      workflow_variant = variant$workflow_variant_id,
      input_type = derive_input_type(tool_entry$id, variant$scenario),
      intended_support = variant$status,
      capability_notes = variant$notes,
      wrapper_path = tool_entry$wrapper$path,
      version_command = tool_entry$version_command
    )
  })
})

runs <- read_table_auto(runs_path) |>
  mutate(
    warmup = as.logical(warmup),
    success = as.logical(success),
    unsupported = as.logical(unsupported),
    failed = as.logical(failed)
  ) |>
  filter(!warmup)

observed <- runs |>
  group_by(tool, scenario, workflow_variant) |>
  summarise(
    tool_version = first(na.omit(tool_version)),
    observed_input_type = first(na.omit(source_input_type)),
    attempted = n() > 0,
    n_runs = n(),
    n_success = sum(status == "success", na.rm = TRUE),
    n_failed = sum(status == "failed", na.rm = TRUE),
    n_unsupported = sum(status == "unsupported", na.rm = TRUE),
    observed_notes = str_c(unique(na.omit(notes)), collapse = " | "),
    .groups = "drop"
  )

support_matrix <- capabilities |>
  full_join(observed, by = c("tool", "scenario", "workflow_variant")) |>
  mutate(
    input_type = coalesce(input_type, observed_input_type, "unknown"),
    intended_support = coalesce(intended_support, "observed_only"),
    attempted = coalesce(attempted, FALSE),
    n_runs = coalesce(n_runs, 0L),
    n_success = coalesce(n_success, 0L),
    n_failed = coalesce(n_failed, 0L),
    n_unsupported = coalesce(n_unsupported, 0L),
    tool_version = coalesce(tool_version, ""),
    support_status = case_when(
      intended_support == "unsupported" ~ "unsupported",
      !attempted ~ "not_attempted",
      n_success > 0 & n_failed > 0 ~ "mixed_results",
      n_success > 0 ~ "supported_success",
      n_failed > 0 ~ "supported_failed",
      n_unsupported > 0 & intended_support != "unsupported" ~ "supported_failed",
      TRUE ~ "not_attempted"
    ),
    notes = str_trim(str_c(capability_notes, observed_notes, sep = " | "), side = "both"),
    notes = str_replace_all(notes, "^\\|\\s*|\\s*\\|$", ""),
    notes = str_replace_all(notes, "\\|\\s*\\|", "|")
  ) |>
  select(
    tool,
    tool_version,
    scenario,
    workflow_variant,
    input_type,
    intended_support,
    attempted,
    n_runs,
    n_success,
    n_failed,
    n_unsupported,
    support_status,
    notes
  ) |>
  arrange(tool, scenario, workflow_variant)

support_summary <- support_matrix |>
  group_by(tool, scenario) |>
  summarise(
    support_status = derive_summary_status(support_status),
    workflow_variants = str_c(workflow_variant, collapse = "; "),
    n_success = sum(n_success, na.rm = TRUE),
    n_failed = sum(n_failed, na.rm = TRUE),
    n_unsupported = sum(n_unsupported, na.rm = TRUE),
    notes = str_c(unique(na.omit(notes)), collapse = " | "),
    .groups = "drop"
  ) |>
  arrange(tool, scenario)

write_csv(support_matrix, file.path(outdir, "support_matrix.csv"))
write_csv(support_summary, file.path(outdir, "support_summary.csv"))

if (tolower(render_plot) == "true") {
  plot_data <- support_summary |>
    mutate(
      tool = factor(tool, levels = unique(tool)),
      scenario = factor(
        scenario,
        levels = c(
          "mapped_bam_pipeline",
          "unmapped_bam_pipeline",
          "fastq_consume_pipeline",
          "subsample_only"
        )
      ),
      support_label = str_replace_all(support_status, "_", "\n")
    )

  support_plot <- ggplot(plot_data, aes(x = scenario, y = tool, fill = support_status)) +
    geom_tile(color = "white") +
    geom_text(aes(label = support_label), size = 3) +
    scale_fill_manual(
      values = c(
        supported_success = "#1b9e77",
        supported_failed = "#d95f02",
        unsupported = "#bdbdbd",
        not_attempted = "#7570b3",
        mixed_results = "#6a3d9a"
      )
    ) +
    labs(
      title = "Benchmark Support Matrix",
      x = "Scenario",
      y = "Tool",
      fill = "Support status"
    ) +
    theme_minimal(base_size = 12) +
    theme(
      legend.position = "bottom",
      panel.grid = element_blank(),
      axis.text.x = element_text(angle = 15, hjust = 1)
    )

  ggsave(file.path(outdir, "support_matrix.pdf"), support_plot, width = 10, height = 6)
  ggsave(file.path(outdir, "support_matrix.png"), support_plot, width = 10, height = 6, dpi = 300)
}
