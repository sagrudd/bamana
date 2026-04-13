#!/usr/bin/env Rscript

suppressPackageStartupMessages({
  library(dplyr)
  library(forcats)
  library(ggplot2)
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

tidy_csv <- arg_value("--tidy-csv", file.path("aggregated", "tidy_results.csv"))
summary_csv <- arg_value("--summary-csv", file.path("aggregated", "tidy_summary.csv"))
output_dir <- arg_value("--output-dir", arg_value("--outdir", "plots"))

dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)

tool_palette <- c(
  bamana = "#1b3a57",
  samtools = "#d95f02",
  sambamba = "#7570b3",
  seqtk = "#1b9e77",
  rasusa = "#66a61e",
  fastcat = "#e7298a"
)

label_scenario <- function(value) {
  value |>
    str_replace_all("_", " ") |>
    str_to_title()
}

runs <- read_csv(tidy_csv, show_col_types = FALSE, na = c("", "NA", "null"))
summary <- read_csv(summary_csv, show_col_types = FALSE, na = c("", "NA", "null"))

plot_runs <- runs |>
  mutate(
    warmup = as.logical(warmup),
    success = as.logical(success)
  ) |>
  filter((is.na(warmup) | !warmup), success, status == "success", !is.na(wall_seconds))

if (nrow(plot_runs) == 0) {
  stop("No successful measured runs were available for plotting.")
}

plot_summary <- summary |>
  filter(n_success > 0, !is.na(median_wall_seconds))

tool_levels <- plot_summary |>
  group_by(tool) |>
  summarise(overall_median_wall_seconds = median(median_wall_seconds, na.rm = TRUE), .groups = "drop") |>
  arrange(overall_median_wall_seconds) |>
  pull(tool)

if (length(tool_levels) == 0) {
  tool_levels <- unique(plot_runs$tool)
}

plot_runs <- plot_runs |>
  mutate(
    tool = factor(tool, levels = tool_levels),
    scenario = fct_inorder(scenario)
  )

plot_summary <- plot_summary |>
  mutate(
    tool = factor(tool, levels = tool_levels),
    scenario = factor(scenario, levels = levels(plot_runs$scenario))
  )

wall_plot <- ggplot(plot_runs, aes(x = tool, y = wall_seconds, colour = tool)) +
  geom_point(
    position = position_jitter(width = 0.12, height = 0),
    size = 2.5,
    alpha = 0.85
  ) +
  geom_point(
    data = plot_summary,
    aes(x = tool, y = median_wall_seconds),
    inherit.aes = FALSE,
    colour = "black",
    shape = 18,
    size = 3
  ) +
  facet_wrap(~scenario, scales = "free_y", labeller = as_labeller(label_scenario)) +
  scale_colour_manual(values = tool_palette, drop = FALSE) +
  scale_y_continuous(labels = label_number(accuracy = 0.1)) +
  labs(
    title = "Wall Time by Tool and Scenario",
    subtitle = "Replicate points show successful measured runs; black diamonds show per-group medians.",
    x = "Tool",
    y = "Wall Time (seconds)",
    colour = "Tool",
    caption = "Unsupported and failed combinations are retained in tidy_results.csv and excluded from this timing plot."
  ) +
  theme_minimal(base_size = 12) +
  theme(
    legend.position = "bottom",
    panel.grid.minor = element_blank(),
    strip.text = element_text(face = "bold")
  )

ggsave(
  filename = file.path(output_dir, "wall_time_by_tool.png"),
  plot = wall_plot,
  width = 11,
  height = 7,
  dpi = 300
)

ggsave(
  filename = file.path(output_dir, "wall_time_by_tool.pdf"),
  plot = wall_plot,
  width = 11,
  height = 7
)
