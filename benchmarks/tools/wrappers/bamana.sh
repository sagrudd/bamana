#!/usr/bin/env bash
set -euo pipefail

WRAPPER_NAME="bamana"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

TOOL_ID="bamana"
SCENARIO=""
WORKFLOW_VARIANT=""
INPUT_PATH=""
OUTPUT_DIR=""
THREADS="1"
SUBSAMPLE_FRACTION=""
SUBSAMPLE_SEED=""
SUBSAMPLE_MODE=""
SORT_ORDER="none"
CREATE_INDEX="false"
RESULT_OUTPUT=""
COMMAND_LOG=""
COMMAND_FILE=""
TIMING_OUTPUT=""
BAMANA_BIN="${BAMANA_BIN:-bamana}"
TOOL_VERSION_COMMAND=""
STATUS="success"
SUPPORT_STATUS="supported"
SEMANTIC_EQUIVALENCE="full"
NORMALIZED_COMMAND=""
PRIMARY_OUTPUT=""
INDEX_OUTPUT=""
declare -a NOTES=()
declare -a COMMANDS=()
declare -a INTERMEDIATE_OUTPUTS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario) SCENARIO="$2"; shift 2 ;;
    --workflow-variant) WORKFLOW_VARIANT="$2"; shift 2 ;;
    --input) INPUT_PATH="$2"; shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    --threads) THREADS="$2"; shift 2 ;;
    --subsample-fraction) SUBSAMPLE_FRACTION="$2"; shift 2 ;;
    --subsample-seed) SUBSAMPLE_SEED="$2"; shift 2 ;;
    --subsample-mode) SUBSAMPLE_MODE="$2"; shift 2 ;;
    --sort-order) SORT_ORDER="$2"; shift 2 ;;
    --create-index) CREATE_INDEX="true"; shift ;;
    --result-output) RESULT_OUTPUT="$2"; shift 2 ;;
    --command-log) COMMAND_LOG="$2"; shift 2 ;;
    --command-file) COMMAND_FILE="$2"; shift 2 ;;
    --timing-output) TIMING_OUTPUT="$2"; shift 2 ;;
    --bamana-bin) BAMANA_BIN="$2"; shift 2 ;;
    --help)
      cat <<'EOF'
Usage: bamana.sh --scenario <id> --workflow-variant <id> --input <path> --output-dir <dir> --result-output <json> --command-file <path> --command-log <path> [options]
EOF
      exit 0
      ;;
    *)
      wrapper_die "unknown argument: $1"
      ;;
  esac
done

require_value --scenario "$SCENARIO"
require_value --workflow-variant "$WORKFLOW_VARIANT"
require_value --input "$INPUT_PATH"
require_value --output-dir "$OUTPUT_DIR"
require_value --result-output "$RESULT_OUTPUT"
require_value --command-log "$COMMAND_LOG"
require_value --command-file "$COMMAND_FILE"

mkdir -p "$OUTPUT_DIR"
TOOL_VERSION_COMMAND="\"${BAMANA_BIN}\" --version"
INPUT_STEM="$(strip_input_suffix "$INPUT_PATH")"
SEED_ARG=()
if [[ "$SUBSAMPLE_MODE" == "random" && -n "$SUBSAMPLE_SEED" ]]; then
  SEED_ARG=(--seed "$SUBSAMPLE_SEED")
fi

case "${SCENARIO}:${WORKFLOW_VARIANT}" in
  mapped_bam_pipeline:bamana_subsample_sort_partial_index|mapped_bam_pipeline:bamana_subsample_sort_index)
    require_value --subsample-fraction "$SUBSAMPLE_FRACTION"
    require_value --subsample-mode "$SUBSAMPLE_MODE"
    SUBSAMPLED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.subsampled.bam"
    SORTED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.sorted.bam"
    PRIMARY_OUTPUT="$SORTED_BAM"
    INTERMEDIATE_OUTPUTS=("$SUBSAMPLED_BAM")
    COMMANDS=(
      "\"${BAMANA_BIN}\" subsample --input \"${INPUT_PATH}\" --out \"${SUBSAMPLED_BAM}\" --fraction ${SUBSAMPLE_FRACTION} --mode ${SUBSAMPLE_MODE} ${SEED_ARG[*]:-} --threads ${THREADS} --force"
      "\"${BAMANA_BIN}\" sort --bam \"${SUBSAMPLED_BAM}\" --out \"${SORTED_BAM}\" --force"
    )
    if [[ "$WORKFLOW_VARIANT" == "bamana_subsample_sort_partial_index" ]]; then
      SEMANTIC_EQUIVALENCE="partial"
      NOTES+=("Mapped BAM benchmarking uses Bamana subsample plus sort. Index creation remains deferred for the partial variant.")
    else
      INDEX_OUTPUT="${SORTED_BAM}.bai"
      COMMANDS+=("\"${BAMANA_BIN}\" index --input \"${SORTED_BAM}\" --out \"${INDEX_OUTPUT}\" --force")
      NOTES+=("The full Bamana subsample-sort-index variant depends on the Bamana index subcommand being available at runtime.")
    fi
    ;;
  unmapped_bam_pipeline:bamana_subsample_only|subsample_only:bamana_subsample_only)
    require_value --subsample-fraction "$SUBSAMPLE_FRACTION"
    require_value --subsample-mode "$SUBSAMPLE_MODE"
    case "$INPUT_PATH" in
      *.fastq.gz|*.fq.gz) PRIMARY_OUTPUT="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.subsampled.fastq.gz" ;;
      *.fastq|*.fq) PRIMARY_OUTPUT="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.subsampled.fastq" ;;
      *) PRIMARY_OUTPUT="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.subsampled.bam" ;;
    esac
    COMMANDS=(
      "\"${BAMANA_BIN}\" subsample --input \"${INPUT_PATH}\" --out \"${PRIMARY_OUTPUT}\" --fraction ${SUBSAMPLE_FRACTION} --mode ${SUBSAMPLE_MODE} ${SEED_ARG[*]:-} --threads ${THREADS} --force"
    )
    NOTES+=("Bamana subsample-only benchmarking uses the native subsample command and preserves encounter order.")
    ;;
  fastq_consume_pipeline:bamana_consume_unmapped_bam|fastq_consume_pipeline:bamana_consume_only)
    PRIMARY_OUTPUT="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.consumed.bam"
    COMMANDS=(
      "\"${BAMANA_BIN}\" consume --input \"${INPUT_PATH}\" --out \"${PRIMARY_OUTPUT}\" --mode unmapped --force"
    )
    NOTES+=("Bamana consume-only benchmarking normalizes FASTQ.GZ input into unmapped BAM.")
    ;;
  fastq_consume_pipeline:bamana_consume_sort)
    CONSUMED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.consumed.bam"
    SORTED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.sorted.bam"
    PRIMARY_OUTPUT="$SORTED_BAM"
    INTERMEDIATE_OUTPUTS=("$CONSUMED_BAM")
    COMMANDS=(
      "\"${BAMANA_BIN}\" consume --input \"${INPUT_PATH}\" --out \"${CONSUMED_BAM}\" --mode unmapped --force"
      "\"${BAMANA_BIN}\" sort --bam \"${CONSUMED_BAM}\" --out \"${SORTED_BAM}\" --force"
    )
    NOTES+=("The Bamana consume-sort variant is scaffolded for future FASTQ normalization benchmarks.")
    ;;
  fastq_consume_pipeline:bamana_consume_sort_index)
    CONSUMED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.consumed.bam"
    SORTED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.sorted.bam"
    PRIMARY_OUTPUT="$SORTED_BAM"
    INDEX_OUTPUT="${SORTED_BAM}.bai"
    INTERMEDIATE_OUTPUTS=("$CONSUMED_BAM")
    COMMANDS=(
      "\"${BAMANA_BIN}\" consume --input \"${INPUT_PATH}\" --out \"${CONSUMED_BAM}\" --mode unmapped --force"
      "\"${BAMANA_BIN}\" sort --bam \"${CONSUMED_BAM}\" --out \"${SORTED_BAM}\" --force"
      "\"${BAMANA_BIN}\" index --input \"${SORTED_BAM}\" --out \"${INDEX_OUTPUT}\" --force"
    )
    NOTES+=("The Bamana consume-sort-index variant is scaffolded and will fail honestly at runtime if the required subcommands are not yet implemented.")
    ;;
  *)
    NOTES+=("Bamana does not support scenario '${SCENARIO}' with workflow variant '${WORKFLOW_VARIANT}' in the current benchmark contract.")
    emit_unsupported_plan
    exit 0
    ;;
esac

if [[ "$CREATE_INDEX" == "true" && -n "$PRIMARY_OUTPUT" && -z "$INDEX_OUTPUT" && "$PRIMARY_OUTPUT" == *.bam ]]; then
  INDEX_OUTPUT="${PRIMARY_OUTPUT}.bai"
  COMMANDS+=("\"${BAMANA_BIN}\" index --input \"${PRIMARY_OUTPUT}\" --out \"${INDEX_OUTPUT}\" --force")
  NOTES+=("Index creation was requested explicitly through the wrapper contract.")
fi

if [[ "$SORT_ORDER" != "none" ]]; then
  NOTES+=("Wrapper sort-order request: ${SORT_ORDER}.")
fi

if [[ -n "$TIMING_OUTPUT" ]]; then
  NOTES+=("Timing output is owned by the outer benchmark wrapper; the planning wrapper records the requested path only.")
fi

emit_supported_plan
