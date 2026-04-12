#!/usr/bin/env bash
set -euo pipefail

WRAPPER_NAME="fastcat"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

TOOL_ID="fastcat"
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
FASTCAT_BIN="${FASTCAT_BIN:-fastcat}"
TOOL_VERSION_COMMAND=""
STATUS="success"
SUPPORT_STATUS="supported"
SEMANTIC_EQUIVALENCE="partial"
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
    --fastcat-bin) FASTCAT_BIN="$2"; shift 2 ;;
    --help)
      cat <<'EOF'
Usage: fastcat.sh --scenario <id> --workflow-variant <id> --input <path> --output-dir <dir> --result-output <json> --command-file <path> --command-log <path> [options]
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
TOOL_VERSION_COMMAND="\"${FASTCAT_BIN}\" --version"
INPUT_STEM="$(strip_input_suffix "$INPUT_PATH")"

case "${SCENARIO}:${WORKFLOW_VARIANT}" in
  fastq_consume_pipeline:fastcat_concat_gzip|fastq_consume_pipeline:fastcat_consume_only|fastq_consume_pipeline:fastcat_fastq_concat_only)
    PRIMARY_OUTPUT="${OUTPUT_DIR}/${INPUT_STEM}.${WORKFLOW_VARIANT}.fastq.gz"
    COMMANDS=(
      "\"${FASTCAT_BIN}\" \"${INPUT_PATH}\" | gzip -c > \"${PRIMARY_OUTPUT}\""
    )
    NOTES+=("fastcat is benchmarked in FASTQ ingestion and concatenation space rather than BAM sort-index scenarios.")
    ;;
  *)
    NOTES+=("fastcat is not a comparator for scenario '${SCENARIO}' with workflow variant '${WORKFLOW_VARIANT}' in the current benchmark contract.")
    emit_unsupported_plan
    exit 0
    ;;
esac

if [[ "$CREATE_INDEX" == "true" ]]; then
  NOTES+=("Index creation was requested but ignored because fastcat does not emit BAM targets in this wrapper.")
fi

if [[ "$SORT_ORDER" != "none" ]]; then
  NOTES+=("Sort-order request '${SORT_ORDER}' is ignored because fastcat benchmarks operate in FASTQ ingestion space.")
fi

if [[ -n "$TIMING_OUTPUT" ]]; then
  NOTES+=("Timing output is owned by the outer benchmark wrapper; the planning wrapper records the requested path only.")
fi

emit_supported_plan
