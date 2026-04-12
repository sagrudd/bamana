#!/usr/bin/env Rscript

suppressPackageStartupMessages({
  library(ggplot2)
  library(patchwork)
  library(readr)
  library(dplyr)
  library(forcats)
  library(scales)
})

args <- commandArgs(trailingOnly = TRUE)

arg_value <- function(flag, default = NULL) {
  index <- match(flag, args)
  if (is.na(index) || index == length(args)) {
    return(default)
  }
  args[[index + 1]]
}

runs_tsv <- arg_value("--runs-tsv")
summary_tsv <- arg_value("--summary-tsv")
support_tsv <- arg_value("--support-tsv")
outdir <- arg_value("--outdir", "figures")

if (is.null(runs_tsv) || is.null(summary_tsv) || is.null(support_tsv)) {
  stop("plot_benchmarks.R requires --runs-tsv, --summary-tsv, and --support-tsv.")
}

dir.create(outdir, recursive = TRUE, showWarnings = FALSE)

tool_palette <- c(
  bamana = "#1b3a57",
  samtools = "#d95f02",
  sambamba = "#7570b3",
  seqtk = "#1b9e77",
  rasusa = "#66a61e",
  fastcat = "#e7298a"
)

theme_bamana <- function() {
  theme_minimal(base_size = 12) +
    theme(
      legend.position = "bottom",
      strip.text = element_text(face = "bold"),
      panel.grid.minor = element_blank()
    )
}

runs <- read_tsv(runs_tsv, show_col_types = FALSE) |>
  filter(!warmup, status == "success", success) |>
  mutate(tool = fct_inorder(tool))

summary <- read_tsv(summary_tsv, show_col_types = FALSE) |>
  filter(n_success > 0) |>
  mutate(tool = fct_inorder(tool))

support <- read_tsv(support_tsv, show_col_types = FALSE) |>
  mutate(tool = fct_inorder(tool))

wall_plot <- ggplot(runs, aes(x = tool, y = wall_seconds, colour = tool)) +
  geom_boxplot(outlier.shape = NA, width = 0.6) +
  geom_jitter(width = 0.12, alpha = 0.6, size = 2) +
  facet_wrap(~scenario, scales = "free_y") +
  scale_colour_manual(values = tool_palette) +
  labs(
    title = "Wall Time by Tool and Scenario",
    x = "Tool",
    y = "Wall Time (seconds)"
  ) +
  theme_bamana()

throughput_plot <- ggplot(summary, aes(x = tool, y = median_records_per_sec, fill = tool)) +
  geom_col(width = 0.7) +
  facet_wrap(~scenario, scales = "free_y") +
  scale_fill_manual(values = tool_palette) +
  scale_y_continuous(labels = label_number(accuracy = 1)) +
  labs(
    title = "Median Throughput by Tool and Scenario",
    x = "Tool",
    y = "Records per second"
  ) +
  theme_bamana()

memory_plot <- ggplot(summary, aes(x = tool, y = median_max_rss_bytes, fill = tool)) +
  geom_col(width = 0.7) +
  facet_wrap(~scenario, scales = "free_y") +
  scale_fill_manual(values = tool_palette) +
  scale_y_continuous(labels = label_bytes(units = "auto")) +
  labs(
    title = "Median Max RSS by Tool and Scenario",
    x = "Tool",
    y = "Max RSS"
  ) +
  theme_bamana()

variability_plot <- ggplot(runs, aes(x = replicate, y = wall_seconds, colour = tool, group = interaction(tool, scenario))) +
  geom_line(alpha = 0.5) +
  geom_point(size = 2) +
  facet_wrap(~scenario, scales = "free_y") +
  scale_colour_manual(values = tool_palette) +
  labs(
    title = "Replicate Variability",
    x = "Replicate",
    y = "Wall Time (seconds)"
  ) +
  theme_bamana()

support_plot <- ggplot(support, aes(x = scenario, y = tool, fill = status)) +
  geom_tile(color = "white") +
  geom_text(aes(label = status), size = 3) +
  scale_fill_manual(
    values = c(
      success = "#1b9e77",
      failed = "#d95f02",
      unsupported = "#bdbdbd",
      skipped = "#7570b3",
      mixed = "#6a3d9a"
    )
  ) +
  labs(
    title = "Support Status by Tool and Scenario",
    x = "Scenario",
    y = "Tool",
    fill = "Support Status"
  ) +
  theme_bamana()

ggsave(file.path(outdir, "wall_time_by_tool.pdf"), wall_plot, width = 11, height = 7)
ggsave(file.path(outdir, "wall_time_by_tool.png"), wall_plot, width = 11, height = 7, dpi = 300)

ggsave(file.path(outdir, "throughput_by_tool.pdf"), throughput_plot, width = 11, height = 7)
ggsave(file.path(outdir, "throughput_by_tool.png"), throughput_plot, width = 11, height = 7, dpi = 300)

ggsave(file.path(outdir, "memory_by_tool.pdf"), memory_plot, width = 11, height = 7)
ggsave(file.path(outdir, "memory_by_tool.png"), memory_plot, width = 11, height = 7, dpi = 300)

ggsave(file.path(outdir, "replicate_variability.pdf"), variability_plot, width = 11, height = 7)
ggsave(file.path(outdir, "replicate_variability.png"), variability_plot, width = 11, height = 7, dpi = 300)

ggsave(file.path(outdir, "support_status_heatmap.pdf"), support_plot, width = 11, height = 7)
ggsave(file.path(outdir, "support_status_heatmap.png"), support_plot, width = 11, height = 7, dpi = 300)
