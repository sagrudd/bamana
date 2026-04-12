#!/usr/bin/env bash

if [[ -z "${BAMANA_WRAPPER_COMMON_SOURCED:-}" ]]; then
  export BAMANA_WRAPPER_COMMON_SOURCED=1
fi

WRAPPER_CONTRACT_VERSION="1.0.0"

wrapper_die() {
  local message="$1"
  echo "[${WRAPPER_NAME:-wrapper}] ${message}" >&2
  exit 2
}

ensure_parent_dir() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
}

strip_input_suffix() {
  local name
  name="$(basename "$1")"
  for suffix in .fastq.gz .fq.gz .fastq .fq .bam .sam .cram; do
    if [[ "$name" == *"$suffix" ]]; then
      echo "${name%"$suffix"}"
      return 0
    fi
  done
  echo "$name"
}

fraction_token_from_decimal() {
  local value="$1"
  local normalized
  normalized="$(printf '%s' "$value" | tr '.' '_')"
  printf '%s' "${normalized//[^0-9_]/}"
}

write_command_file() {
  local path="$1"
  shift

  ensure_parent_dir "$path"
  {
    echo '#!/usr/bin/env bash'
    echo 'set -euo pipefail'
    local command
    for command in "$@"; do
      printf '%s\n' "$command"
    done
  } >"$path"
  chmod +x "$path"
}

write_command_log() {
  local path="$1"
  shift

  ensure_parent_dir "$path"
  if (( $# == 0 )); then
    : >"$path"
    return 0
  fi

  {
    local command
    for command in "$@"; do
      printf '%s\n' "$command"
    done
  } >"$path"
}

join_notes() {
  if (( ${#NOTES[@]} == 0 )); then
    printf ''
    return 0
  fi
  local joined
  joined="$(printf '%s\n' "${NOTES[@]}" | paste -sd '; ' -)"
  printf '%s' "$joined"
}

emit_wrapper_result() {
  local notes_json='[]'
  local intermediates_json='[]'

  if (( ${#NOTES[@]} > 0 )); then
    notes_json="$(printf '%s\n' "${NOTES[@]}" | jq -R . | jq -s .)"
  fi

  if (( ${#INTERMEDIATE_OUTPUTS[@]} > 0 )); then
    intermediates_json="$(printf '%s\n' "${INTERMEDIATE_OUTPUTS[@]}" | jq -R . | jq -s .)"
  fi

  ensure_parent_dir "$RESULT_OUTPUT"
  jq -n \
    --arg wrapper_contract_version "$WRAPPER_CONTRACT_VERSION" \
    --arg tool "$TOOL_ID" \
    --arg scenario "$SCENARIO" \
    --arg workflow_variant "$WORKFLOW_VARIANT" \
    --arg status "$STATUS" \
    --arg support_status "$SUPPORT_STATUS" \
    --arg semantic_equivalence "$SEMANTIC_EQUIVALENCE" \
    --arg tool_version_command "$TOOL_VERSION_COMMAND" \
    --arg command "$NORMALIZED_COMMAND" \
    --arg command_file "$COMMAND_FILE" \
    --arg command_log "$COMMAND_LOG" \
    --arg output_dir "$OUTPUT_DIR" \
    --arg primary_output "$PRIMARY_OUTPUT" \
    --arg index_output "$INDEX_OUTPUT" \
    --argjson timing_wrapper_compatible true \
    --argjson notes "$notes_json" \
    --argjson intermediates "$intermediates_json" \
    '{
      wrapper_contract_version: $wrapper_contract_version,
      tool: $tool,
      scenario: $scenario,
      workflow_variant: $workflow_variant,
      status: $status,
      support_status: $support_status,
      semantic_equivalence: $semantic_equivalence,
      tool_version_command: $tool_version_command,
      command: (if $command == "" then null else $command end),
      command_file: $command_file,
      command_log: $command_log,
      output_dir: $output_dir,
      output_paths: {
        primary: (if $primary_output == "" then null else $primary_output end),
        index: (if $index_output == "" then null else $index_output end),
        intermediates: $intermediates
      },
      timing_wrapper_compatible: $timing_wrapper_compatible,
      notes: $notes
    }' >"$RESULT_OUTPUT"
}

emit_supported_plan() {
  NORMALIZED_COMMAND="$(printf '%s\n' "${COMMANDS[@]}" | paste -sd ' ; ' -)"
  write_command_file "$COMMAND_FILE" "${COMMANDS[@]}"
  write_command_log "$COMMAND_LOG" "${COMMANDS[@]}"
  emit_wrapper_result
}

emit_unsupported_plan() {
  STATUS="unsupported"
  SUPPORT_STATUS="unsupported"
  SEMANTIC_EQUIVALENCE="unsupported"
  NORMALIZED_COMMAND=""
  PRIMARY_OUTPUT=""
  INDEX_OUTPUT=""
  INTERMEDIATE_OUTPUTS=()
  write_command_file "$COMMAND_FILE" "true"
  write_command_log "$COMMAND_LOG"
  emit_wrapper_result
}

require_value() {
  local flag_name="$1"
  local flag_value="$2"
  if [[ -z "$flag_value" ]]; then
    wrapper_die "missing required value for ${flag_name}"
  fi
}
