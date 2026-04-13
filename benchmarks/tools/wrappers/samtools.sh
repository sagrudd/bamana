#!/usr/bin/env bash
set -euo pipefail

WRAPPER_NAME="samtools"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

TOOL_ID="samtools"
SCENARIO=""
WORKFLOW_VARIANT=""
INPUT_PATH=""
OUTPUT_DIR=""
THREADS="1"
SUBSAMPLE_FRACTION=""
SUBSAMPLE_SEED=""
SUBSAMPLE_MODE=""
SORT_ORDER="coordinate"
CREATE_INDEX="false"
RESULT_OUTPUT=""
COMMAND_LOG=""
COMMAND_FILE=""
TIMING_OUTPUT=""
SAMTOOLS_BIN="${SAMTOOLS_BIN:-samtools}"
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
    --samtools-bin) SAMTOOLS_BIN="$2"; shift 2 ;;
    --help)
      cat <<'EOF'
Usage: samtools.sh --scenario <id> --workflow-variant <id> --input <path> --output-dir <dir> --result-output <json> --command-file <path> --command-log <path> [options]
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
TOOL_VERSION_COMMAND="\"${SAMTOOLS_BIN}\" --version"
INPUT_STEM="$(strip_input_suffix "$INPUT_PATH")"

fraction_text="${SUBSAMPLE_FRACTION:-}"
fraction_token=""
sample_arg=""
if [[ -n "$fraction_text" ]]; then
  fraction_token="$(printf '%s' "$fraction_text" | tr -d '. ')"
fi
if [[ -n "$SUBSAMPLE_SEED" && -n "$fraction_token" ]]; then
  sample_arg="${SUBSAMPLE_SEED}.${fraction_token}"
fi

if [[ "${SUBSAMPLE_MODE:-}" == "deterministic" ]]; then
  SEMANTIC_EQUIVALENCE="partial"
  NOTES+=("samtools uses seeded pseudo-random selection via view -s; deterministic benchmark mode is therefore only partially equivalent.")
fi

case "${SCENARIO}:${WORKFLOW_VARIANT}" in
  mapped_bam_pipeline:samtools_view_sort_index)
    require_value --subsample-fraction "$SUBSAMPLE_FRACTION"
    require_value --subsample-seed "$SUBSAMPLE_SEED"
    SUBSAMPLED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.subsampled.bam"
    SORTED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.sorted.bam"
    PRIMARY_OUTPUT="$SORTED_BAM"
    INDEX_OUTPUT="${SORTED_BAM}.bai"
    INTERMEDIATE_OUTPUTS=("$SUBSAMPLED_BAM")
    COMMANDS=(
      "\"${SAMTOOLS_BIN}\" view -@ ${THREADS} -s ${sample_arg} -b \"${INPUT_PATH}\" -o \"${SUBSAMPLED_BAM}\""
      "\"${SAMTOOLS_BIN}\" sort -@ ${THREADS} -o \"${SORTED_BAM}\" \"${SUBSAMPLED_BAM}\""
      "\"${SAMTOOLS_BIN}\" index -@ ${THREADS} \"${SORTED_BAM}\""
    )
    NOTES+=("samtools is the canonical BAM baseline and uses the natural view-sort-index path for mapped BAM benchmarking.")
    ;;
  mapped_bam_pipeline:samtools_sort_index)
    SORTED_BAM="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.sorted.bam"
    PRIMARY_OUTPUT="$SORTED_BAM"
    INDEX_OUTPUT="${SORTED_BAM}.bai"
    COMMANDS=(
      "\"${SAMTOOLS_BIN}\" sort -@ ${THREADS} -o \"${SORTED_BAM}\" \"${INPUT_PATH}\""
      "\"${SAMTOOLS_BIN}\" index -@ ${THREADS} \"${SORTED_BAM}\""
    )
    NOTES+=("The samtools sort-index variant assumes the input is already the scenario-specific benchmarking input.")
    ;;
  unmapped_bam_pipeline:samtools_view_subsample_only|unmapped_bam_pipeline:samtools_subsample_only|subsample_only:samtools_view_subsample_only|subsample_only:samtools_subsample_only)
    require_value --subsample-fraction "$SUBSAMPLE_FRACTION"
    require_value --subsample-seed "$SUBSAMPLE_SEED"
    PRIMARY_OUTPUT="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.subsampled.bam"
    COMMANDS=(
      "\"${SAMTOOLS_BIN}\" view -@ ${THREADS} -s ${sample_arg} -b \"${INPUT_PATH}\" -o \"${PRIMARY_OUTPUT}\""
    )
    NOTES+=("samtools subsample-only benchmarking uses view -s without downstream sort or index.")
    ;;
  *)
    NOTES+=("samtools does not support scenario '${SCENARIO}' with workflow variant '${WORKFLOW_VARIANT}' in the current benchmark contract.")
    emit_unsupported_plan
    exit 0
    ;;
esac

if [[ "$CREATE_INDEX" == "true" && -n "$PRIMARY_OUTPUT" && -z "$INDEX_OUTPUT" && "$PRIMARY_OUTPUT" == *.bam ]]; then
  INDEX_OUTPUT="${PRIMARY_OUTPUT}.bai"
  COMMANDS+=("\"${SAMTOOLS_BIN}\" index -@ ${THREADS} \"${PRIMARY_OUTPUT}\"")
  NOTES+=("Index creation was requested explicitly through the wrapper contract.")
fi

if [[ "$SORT_ORDER" != "none" ]]; then
  NOTES+=("Wrapper sort-order request: ${SORT_ORDER}.")
fi

if [[ -n "$TIMING_OUTPUT" ]]; then
  NOTES+=("Timing output is owned by the outer benchmark wrapper; the planning wrapper records the requested path only.")
fi

emit_supported_plan
