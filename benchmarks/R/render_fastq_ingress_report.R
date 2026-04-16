args <- commandArgs(trailingOnly = TRUE)

parse_args <- function(args) {
  values <- list()
  index <- 1
  while (index <= length(args)) {
    key <- args[[index]]
    if (!startsWith(key, "--")) {
      stop(sprintf("unexpected positional argument: %s", key), call. = FALSE)
    }
    if (index == length(args)) {
      stop(sprintf("missing value for %s", key), call. = FALSE)
    }
    values[[substring(key, 3)]] <- args[[index + 1]]
    index <- index + 2
  }
  values
}

required <- c(
  "template",
  "tidy-csv",
  "summary-csv",
  "support-csv",
  "tool-versions",
  "fastq",
  "bamana-output",
  "comparator-output",
  "bamana-output-label",
  "comparator-output-label",
  "container-image",
  "profile",
  "output"
)

values <- parse_args(args)
missing <- required[!required %in% names(values)]
if (length(missing) > 0) {
  stop(sprintf("missing required arguments: %s", paste(missing, collapse = ", ")), call. = FALSE)
}

output_path <- normalizePath(values[["output"]], mustWork = FALSE)
output_dir <- dirname(output_path)
dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)

rmarkdown::render(
  input = values[["template"]],
  output_file = basename(output_path),
  output_dir = output_dir,
  params = list(
    tidy_csv = values[["tidy-csv"]],
    summary_csv = values[["summary-csv"]],
    support_csv = values[["support-csv"]],
    tool_versions = values[["tool-versions"]],
    fastq = values[["fastq"]],
    bamana_output = values[["bamana-output"]],
    comparator_output = values[["comparator-output"]],
    bamana_output_label = values[["bamana-output-label"]],
    comparator_output_label = values[["comparator-output-label"]],
    container_image = values[["container-image"]],
    profile = values[["profile"]]
  ),
  envir = new.env(parent = globalenv())
)
