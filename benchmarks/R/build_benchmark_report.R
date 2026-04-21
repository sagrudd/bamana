#!/usr/bin/env Rscript

suppressPackageStartupMessages({
  library(dplyr)
  library(grid)
  library(readr)
  library(scales)
  library(stringr)
})

args <- commandArgs(trailingOnly = TRUE)

arg_value <- function(flag, default = NULL) {
  index <- match(flag, args)
  if (is.na(index) || index == length(args)) {
    return(default)
  }
  args[[index + 1]]
}

scenario_label <- function(value) {
  value |>
    str_replace_all("_", " ") |>
    str_to_title()
}

format_seconds <- function(value) {
  ifelse(
    is.na(value),
    "NA",
    ifelse(
      value >= 100,
      sprintf("%.0f", value),
      ifelse(value >= 10, sprintf("%.1f", value), sprintf("%.2f", value))
    )
  )
}

format_gib_per_sec <- function(value) {
  ifelse(is.na(value), "NA", sprintf("%.2f", value / 1024^3))
}

format_mib <- function(value) {
  ifelse(is.na(value), "NA", sprintf("%.2f", value / 1024^2))
}

format_success_ratio <- function(successes, runs) {
  ifelse(is.na(runs) | runs <= 0, "0/0", sprintf("%d/%d", successes, runs))
}

first_non_empty <- function(values) {
  values <- unique(values)
  values <- values[!is.na(values) & nzchar(values)]
  if (length(values) == 0) {
    return(NA_character_)
  }
  values[[1]]
}

draw_text_block <- function(lines, x = 0.06, y_top = 0.86, line_height = 0.05, fontsize = 11) {
  if (length(lines) == 0) {
    return(invisible(NULL))
  }

  for (index in seq_along(lines)) {
    grid.text(
      label = lines[[index]],
      x = unit(x, "npc"),
      y = unit(y_top - ((index - 1) * line_height), "npc"),
      just = c("left", "top"),
      gp = gpar(fontsize = fontsize, col = "#222222")
    )
  }
}

draw_table <- function(data, x = 0.05, y_top = 0.60, width = 0.90, row_height = 0.06) {
  columns <- c("Tool", "Threads", "Median Wall (s)", "Throughput (GiB/s)", "Peak RSS (MiB)", "Success")
  widths <- c(0.24, 0.10, 0.18, 0.20, 0.16, 0.12)
  x_edges <- c(0, cumsum(widths)) * width + x

  for (column_index in seq_along(columns)) {
    cell_x <- x_edges[[column_index]]
    cell_width <- widths[[column_index]] * width
    grid.rect(
      x = unit(cell_x, "npc"),
      y = unit(y_top, "npc"),
      width = unit(cell_width, "npc"),
      height = unit(row_height, "npc"),
      just = c("left", "top"),
      gp = gpar(fill = "#1b3a57", col = "#d9e2ec", lwd = 0.6)
    )
    grid.text(
      label = columns[[column_index]],
      x = unit(cell_x + 0.008, "npc"),
      y = unit(y_top - (row_height / 2), "npc"),
      just = c("left", "center"),
      gp = gpar(fontsize = 10, fontface = "bold", col = "white")
    )
  }

  if (nrow(data) == 0) {
    grid.rect(
      x = unit(x, "npc"),
      y = unit(y_top - row_height, "npc"),
      width = unit(width, "npc"),
      height = unit(row_height, "npc"),
      just = c("left", "top"),
      gp = gpar(fill = "#f7fafc", col = "#d9e2ec", lwd = 0.6)
    )
    grid.text(
      label = "No successful measured runs were available for this slice.",
      x = unit(x + 0.008, "npc"),
      y = unit(y_top - (row_height * 1.5), "npc"),
      just = c("left", "center"),
      gp = gpar(fontsize = 10, col = "#222222")
    )
    return(invisible(NULL))
  }

  for (row_index in seq_len(nrow(data))) {
    row_y <- y_top - (row_height * row_index)
    fill <- if ((row_index %% 2) == 1) "#f7fafc" else "white"
    values <- unname(as.character(data[row_index, ]))

    for (column_index in seq_along(values)) {
      cell_x <- x_edges[[column_index]]
      cell_width <- widths[[column_index]] * width
      grid.rect(
        x = unit(cell_x, "npc"),
        y = unit(row_y, "npc"),
        width = unit(cell_width, "npc"),
        height = unit(row_height, "npc"),
        just = c("left", "top"),
        gp = gpar(fill = fill, col = "#d9e2ec", lwd = 0.6)
      )
      grid.text(
        label = values[[column_index]],
        x = unit(cell_x + 0.008, "npc"),
        y = unit(row_y - (row_height / 2), "npc"),
        just = c("left", "center"),
        gp = gpar(fontsize = 10, col = "#222222")
      )
    }
  }
}

build_scenario_observations <- function(data) {
  if (nrow(data) == 0) {
    return(c("- No successful measured runs were available for this scenario."))
  }

  ordered <- data |> arrange(median_wall_seconds, tool)
  fastest <- ordered[1, ]
  second <- if (nrow(ordered) >= 2) ordered[2, ] else NULL
  lowest_rss <- ordered |> arrange(median_max_rss_bytes, tool) |> slice(1)
  failed_runs <- sum(ordered$n_failed, na.rm = TRUE)
  unsupported_runs <- sum(ordered$n_unsupported, na.rm = TRUE)

  lines <- c(
    sprintf(
      "- Fastest median wall time: %s at %ss.",
      fastest$tool,
      format_seconds(fastest$median_wall_seconds)
    )
  )

  if (!is.null(second)) {
    slowdown <- ((second$median_wall_seconds - fastest$median_wall_seconds) / fastest$median_wall_seconds) * 100
    lines <- c(
      lines,
      sprintf(
        "- Next best: %s, %.1f%% slower than %s.",
        second$tool,
        slowdown,
        fastest$tool
      )
    )
  }

  lines <- c(
    lines,
    sprintf(
      "- Lowest measured peak RSS: %s at %s MiB.",
      lowest_rss$tool,
      format_mib(lowest_rss$median_max_rss_bytes)
    )
  )

  if (!all(is.na(ordered$median_bytes_per_sec))) {
    throughput_leader <- ordered |>
      arrange(desc(median_bytes_per_sec), tool) |>
      slice(1)
    lines <- c(
      lines,
      sprintf(
        "- Highest byte throughput: %s at %s GiB/s.",
        throughput_leader$tool,
        format_gib_per_sec(throughput_leader$median_bytes_per_sec)
      )
    )
  }

  if (failed_runs > 0 || unsupported_runs > 0) {
    lines <- c(
      lines,
      sprintf(
        "- Non-successful attempts remained visible: %d failed, %d unsupported.",
        failed_runs,
        unsupported_runs
      )
    )
  }

  lines
}

summary_csv <- arg_value("--summary-csv", file.path("aggregated", "tidy_summary.csv"))
tidy_csv <- arg_value("--tidy-csv", file.path("aggregated", "tidy_results.csv"))
output_dir <- arg_value("--output-dir", arg_value("--outdir", "reports"))

dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)

summary <- read_csv(summary_csv, show_col_types = FALSE, na = c("", "NA", "null"))

successful_summary <- summary |>
  filter(n_success > 0, !is.na(median_wall_seconds))

report_rows <- successful_summary |>
  mutate(
    scenario_display = scenario_label(scenario),
    source_display = if_else(
      is.na(source_input_id) | !nzchar(source_input_id),
      "unspecified-input",
      source_input_id
    ),
    tool_display = str_replace_all(tool, "_", " "),
    threads_display = if_else(is.na(threads), "NA", as.character(threads)),
    wall_display = format_seconds(median_wall_seconds),
    throughput_display = format_gib_per_sec(median_bytes_per_sec),
    rss_display = format_mib(median_max_rss_bytes),
    success_display = format_success_ratio(n_success, n_runs)
  ) |>
  arrange(source_display, scenario_display, median_wall_seconds, tool_display)

pages <- report_rows |>
  group_by(source_display, scenario_display) |>
  group_split()

overall_observations <- c(
  sprintf(
    "- This compact PDF suppresses raw byte columns and long note fields to avoid overlapping text."
  ),
  sprintf(
    "- Peak RSS is memory usage, not sequence-read count. For example, 4,161,536 bytes is %.2f MiB; 4,718,592 bytes is %.2f MiB.",
    4161536 / 1024^2,
    4718592 / 1024^2
  )
)

if (nrow(successful_summary) > 0) {
  global_fastest <- successful_summary |>
    arrange(median_wall_seconds, tool) |>
    slice(1)
  overall_observations <- c(
    overall_observations,
    sprintf(
      "- Fastest successful slice in this run set: %s / %s / %s at %ss.",
      first_non_empty(global_fastest$source_input_id),
      scenario_label(first_non_empty(global_fastest$scenario)),
      first_non_empty(global_fastest$tool),
      format_seconds(global_fastest$median_wall_seconds)
    )
  )
}

failed_runs <- sum(summary$n_failed, na.rm = TRUE)
unsupported_runs <- sum(summary$n_unsupported, na.rm = TRUE)
if (failed_runs > 0 || unsupported_runs > 0) {
  overall_observations <- c(
    overall_observations,
    sprintf(
      "- Summary rows retain non-successful attempts: %d failed and %d unsupported across successful groups.",
      failed_runs,
      unsupported_runs
    )
  )
}

pdf(file.path(output_dir, "benchmark_report.pdf"), width = 11, height = 8.5, onefile = TRUE)

grid.newpage()
grid.rect(gp = gpar(fill = "white", col = NA))
grid.text(
  "Bamana Benchmark Summary",
  x = unit(0.05, "npc"),
  y = unit(0.95, "npc"),
  just = c("left", "top"),
  gp = gpar(fontsize = 22, fontface = "bold", col = "#111111")
)
grid.text(
  "Compact report with reduced metric tables and explicit unit labels.",
  x = unit(0.05, "npc"),
  y = unit(0.91, "npc"),
  just = c("left", "top"),
  gp = gpar(fontsize = 11, col = "#444444")
)
draw_text_block(overall_observations, y_top = 0.84, line_height = 0.06, fontsize = 11)

glossary_lines <- c(
  "- Median Wall (s): median elapsed runtime across successful measured runs.",
  "- Throughput (GiB/s): median input-byte throughput derived from successful runs.",
  "- Peak RSS (MiB): median peak resident memory; this is the field that previously appeared as raw bytes."
)

grid.text(
  "Metric Glossary",
  x = unit(0.05, "npc"),
  y = unit(0.50, "npc"),
  just = c("left", "top"),
  gp = gpar(fontsize = 15, fontface = "bold", col = "#111111")
)
draw_text_block(glossary_lines, y_top = 0.45, line_height = 0.055, fontsize = 11)

if (length(pages) == 0) {
  grid.text(
    "No successful measured runs were available to summarize.",
    x = unit(0.05, "npc"),
    y = unit(0.28, "npc"),
    just = c("left", "top"),
    gp = gpar(fontsize = 12, col = "#222222")
  )
} else {
  for (page in pages) {
    source_display <- page$source_display[[1]]
    scenario_display <- page$scenario_display[[1]]
    observations <- build_scenario_observations(page)
    table_rows <- page |>
      transmute(
        Tool = tool_display,
        Threads = threads_display,
        `Median Wall (s)` = wall_display,
        `Throughput (GiB/s)` = throughput_display,
        `Peak RSS (MiB)` = rss_display,
        Success = success_display
      )

    grid.newpage()
    grid.rect(gp = gpar(fill = "white", col = NA))
    grid.text(
      sprintf("%s: %s", source_display, scenario_display),
      x = unit(0.05, "npc"),
      y = unit(0.95, "npc"),
      just = c("left", "top"),
      gp = gpar(fontsize = 20, fontface = "bold", col = "#111111")
    )
    grid.text(
      "Only the core performance metrics are retained here so the PDF remains readable.",
      x = unit(0.05, "npc"),
      y = unit(0.91, "npc"),
      just = c("left", "top"),
      gp = gpar(fontsize = 10.5, col = "#444444")
    )
    draw_text_block(observations, y_top = 0.84, line_height = 0.055, fontsize = 11)
    draw_table(table_rows, y_top = 0.52)
  }
}

dev.off()

write_csv(
  report_rows |>
    transmute(
      source_input_id = source_display,
      scenario = scenario_display,
      tool = tool_display,
      threads = threads_display,
      median_wall_seconds = wall_display,
      median_bytes_per_sec_gib = throughput_display,
      median_peak_rss_mib = rss_display,
      success_runs = success_display
    ),
  file.path(output_dir, "benchmark_report_summary.csv"),
  na = ""
)
