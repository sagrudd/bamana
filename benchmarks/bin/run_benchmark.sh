#!/usr/bin/env bash
set -euo pipefail

run_id=""
tool=""
version_cmd=""
scenario=""
workflow_variant=""
semantic_equivalence=""
support_status="supported"
input_type=""
mapping_state=""
input_path=""
input_metrics_json=""
replicate=""
warmup_run="false"
subsample_fraction=""
subsample_seed=""
subsample_mode=""
threads=""
container_image=""
output_target=""
command_file=""
notes=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --run-id) run_id="$2"; shift 2 ;;
    --tool) tool="$2"; shift 2 ;;
    --tool-version-cmd) version_cmd="$2"; shift 2 ;;
    --scenario) scenario="$2"; shift 2 ;;
    --workflow-variant) workflow_variant="$2"; shift 2 ;;
    --semantic-equivalence) semantic_equivalence="$2"; shift 2 ;;
    --support-status) support_status="$2"; shift 2 ;;
    --input-type) input_type="$2"; shift 2 ;;
    --mapping-state) mapping_state="$2"; shift 2 ;;
    --input-path) input_path="$2"; shift 2 ;;
    --input-metrics-json) input_metrics_json="$2"; shift 2 ;;
    --replicate) replicate="$2"; shift 2 ;;
    --warmup-run) warmup_run="$2"; shift 2 ;;
    --subsample-fraction) subsample_fraction="$2"; shift 2 ;;
    --subsample-seed) subsample_seed="$2"; shift 2 ;;
    --subsample-mode) subsample_mode="$2"; shift 2 ;;
    --threads) threads="$2"; shift 2 ;;
    --container-image) container_image="$2"; shift 2 ;;
    --output-target) output_target="$2"; shift 2 ;;
    --command-file) command_file="$2"; shift 2 ;;
    --notes) notes="$2"; shift 2 ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$run_id" || -z "$tool" || -z "$scenario" || -z "$input_metrics_json" || -z "$command_file" ]]; then
  echo "run_benchmark.sh requires run_id, tool, scenario, input_metrics_json, and command_file" >&2
  exit 2
fi

timestamp_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

tool_version="$(
  (
    bash -lc "$version_cmd"
  ) 2>/dev/null | head -n 1 | tr '\t' ' ' | tr -d '\r'
)"

input_bytes="$(jq -r '.input_bytes // empty' "$input_metrics_json")"
input_records="$(jq -r '.records_processed // empty' "$input_metrics_json")"
input_basename="$(basename "$input_path")"

stdout_path="${run_id}.stdout.log"
stderr_path="${run_id}.stderr.log"
time_path="${run_id}.time.tsv"
result_tsv="${run_id}.result.tsv"
result_json="${run_id}.result.json"

write_result() {
  local exit_code="$1"
  local success="$2"
  local wall_seconds="$3"
  local user_cpu_seconds="$4"
  local system_cpu_seconds="$5"
  local cpu_seconds="$6"
  local max_rss_bytes="$7"
  local output_bytes="$8"
  local compression_ratio="$9"
  local records_processed="${10}"
  local started_at="${11}"
  local finished_at="${12}"
  local command_line="${13}"
  local combined_notes="${14}"

  printf "%s\n" \
    "benchmark_id	scenario	input_type	mapping_state	input_path	input_basename	input_bytes	input_records	tool	tool_version	workflow_variant	semantic_equivalence	support_status	replicate	warmup_run	subsample_fraction	subsample_seed	subsample_mode	threads	wall_seconds	user_cpu_seconds	system_cpu_seconds	cpu_seconds	max_rss_bytes	exit_code	success	output_path	output_bytes	compression_ratio	records_processed	container_image	command_line	notes	started_at	finished_at" \
    "${run_id}	${scenario}	${input_type}	${mapping_state}	${input_path}	${input_basename}	${input_bytes}	${input_records}	${tool}	${tool_version}	${workflow_variant}	${semantic_equivalence}	${support_status}	${replicate}	${warmup_run}	${subsample_fraction}	${subsample_seed}	${subsample_mode}	${threads}	${wall_seconds}	${user_cpu_seconds}	${system_cpu_seconds}	${cpu_seconds}	${max_rss_bytes}	${exit_code}	${success}	${output_target}	${output_bytes}	${compression_ratio}	${records_processed}	${container_image}	${command_line}	${combined_notes}	${started_at}	${finished_at}" \
    >"${result_tsv}"

  jq -n \
    --arg benchmark_id "$run_id" \
    --arg scenario "$scenario" \
    --arg input_type "$input_type" \
    --arg mapping_state "$mapping_state" \
    --arg input_path "$input_path" \
    --arg input_basename "$input_basename" \
    --argjson input_bytes "${input_bytes:-0}" \
    --argjson input_records "${input_records:-0}" \
    --arg tool "$tool" \
    --arg tool_version "$tool_version" \
    --arg workflow_variant "$workflow_variant" \
    --arg semantic_equivalence "$semantic_equivalence" \
    --arg support_status "$support_status" \
    --argjson replicate "${replicate:-0}" \
    --argjson warmup_run "${warmup_run}" \
    --argjson subsample_fraction "${subsample_fraction:-0}" \
    --argjson subsample_seed "${subsample_seed:-0}" \
    --arg subsample_mode "$subsample_mode" \
    --argjson threads "${threads:-0}" \
    --argjson wall_seconds "${wall_seconds:-0}" \
    --argjson user_cpu_seconds "${user_cpu_seconds:-0}" \
    --argjson system_cpu_seconds "${system_cpu_seconds:-0}" \
    --argjson cpu_seconds "${cpu_seconds:-0}" \
    --argjson max_rss_bytes "${max_rss_bytes:-0}" \
    --argjson exit_code "${exit_code:-0}" \
    --argjson success "${success}" \
    --arg output_path "$output_target" \
    --argjson output_bytes "${output_bytes:-0}" \
    --argjson compression_ratio "${compression_ratio:-0}" \
    --argjson records_processed "${records_processed:-0}" \
    --arg container_image "$container_image" \
    --arg command_line "$command_line" \
    --arg notes "$combined_notes" \
    --arg started_at "$started_at" \
    --arg finished_at "$finished_at" \
    '{
      benchmark_id: $benchmark_id,
      scenario: $scenario,
      input_type: $input_type,
      mapping_state: $mapping_state,
      input_path: $input_path,
      input_basename: $input_basename,
      input_bytes: $input_bytes,
      input_records: $input_records,
      tool: $tool,
      tool_version: $tool_version,
      workflow_variant: $workflow_variant,
      semantic_equivalence: $semantic_equivalence,
      support_status: $support_status,
      replicate: $replicate,
      warmup_run: $warmup_run,
      subsample_fraction: $subsample_fraction,
      subsample_seed: $subsample_seed,
      subsample_mode: $subsample_mode,
      threads: $threads,
      wall_seconds: $wall_seconds,
      user_cpu_seconds: $user_cpu_seconds,
      system_cpu_seconds: $system_cpu_seconds,
      cpu_seconds: $cpu_seconds,
      max_rss_bytes: $max_rss_bytes,
      exit_code: $exit_code,
      success: $success,
      output_path: $output_path,
      output_bytes: $output_bytes,
      compression_ratio: $compression_ratio,
      records_processed: $records_processed,
      container_image: $container_image,
      command_line: $command_line,
      notes: $notes,
      started_at: $started_at,
      finished_at: $finished_at
    }' \
    >"${result_json}"
}

started_at="$(timestamp_utc)"
finished_at="$started_at"
command_line="$(tr '\n' ' ' <"$command_file" | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//')"

if [[ "$support_status" != "supported" ]]; then
  combined_notes="$notes"
  write_result 0 false "" "" "" "" "" "" "" "${input_records:-0}" "$started_at" "$finished_at" "$command_line" "$combined_notes"
  exit 0
fi

set +e
/usr/bin/time \
  -f $'wall_seconds\t%e\nuser_cpu_seconds\t%U\nsystem_cpu_seconds\t%S\nmax_rss_kb\t%M' \
  -o "$time_path" \
  bash "$command_file" >"$stdout_path" 2>"$stderr_path"
exit_code="$?"
set -e
finished_at="$(timestamp_utc)"

wall_seconds="$(awk -F '\t' '$1=="wall_seconds"{print $2}' "$time_path")"
user_cpu_seconds="$(awk -F '\t' '$1=="user_cpu_seconds"{print $2}' "$time_path")"
system_cpu_seconds="$(awk -F '\t' '$1=="system_cpu_seconds"{print $2}' "$time_path")"
max_rss_kb="$(awk -F '\t' '$1=="max_rss_kb"{print $2}' "$time_path")"
cpu_seconds="$(awk -v u="${user_cpu_seconds:-0}" -v s="${system_cpu_seconds:-0}" 'BEGIN { printf "%.6f", (u + s) }')"
max_rss_bytes="$(awk -v kb="${max_rss_kb:-0}" 'BEGIN { printf "%.0f", (kb * 1024) }')"

success=false
support_status_final="failed"
if [[ "$exit_code" -eq 0 ]]; then
  success=true
  support_status_final="completed"
fi
support_status="$support_status_final"

output_bytes=""
compression_ratio=""
if [[ -n "$output_target" && -e "$output_target" ]]; then
  output_bytes="$(du -sb "$output_target" | awk '{print $1}')"
  compression_ratio="$(awk -v in_b="${input_bytes:-0}" -v out_b="${output_bytes:-0}" 'BEGIN { if (in_b > 0) printf "%.6f", (out_b / in_b); else print "" }')"
fi

combined_notes="$notes"
if [[ "$success" != "true" ]]; then
  if [[ -n "$combined_notes" ]]; then
    combined_notes="${combined_notes}; command failed with exit code ${exit_code}"
  else
    combined_notes="command failed with exit code ${exit_code}"
  fi
fi

write_result \
  "$exit_code" \
  "$success" \
  "${wall_seconds:-}" \
  "${user_cpu_seconds:-}" \
  "${system_cpu_seconds:-}" \
  "${cpu_seconds:-}" \
  "${max_rss_bytes:-}" \
  "${output_bytes:-}" \
  "${compression_ratio:-}" \
  "${input_records:-0}" \
  "$started_at" \
  "$finished_at" \
  "$command_line" \
  "$combined_notes"

exit 0
