#!/usr/bin/env bash
set -euo pipefail

profile=""
fastq=""
bamana_output=""
comparator_output=""
report=""
workdir=""
threads="1"
container_image=""
bamana_bin=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) profile="$2"; shift 2 ;;
    --fastq) fastq="$2"; shift 2 ;;
    --bamana-output) bamana_output="$2"; shift 2 ;;
    --comparator-output) comparator_output="$2"; shift 2 ;;
    --report) report="$2"; shift 2 ;;
    --workdir) workdir="$2"; shift 2 ;;
    --threads) threads="$2"; shift 2 ;;
    --container-image) container_image="$2"; shift 2 ;;
    --bamana-bin) bamana_bin="$2"; shift 2 ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ "$profile" != "fastq_gz_enumerate" ]]; then
  echo "run_fastq_gz_enumerate_benchmark.sh only supports --profile fastq_gz_enumerate" >&2
  exit 2
fi

for value_name in fastq bamana_output comparator_output report workdir container_image bamana_bin; do
  if [[ -z "${!value_name}" ]]; then
    echo "missing required argument: ${value_name}" >&2
    exit 2
  fi
done

raw_dir="${workdir}/raw"
aggregated_dir="${workdir}/aggregated"
metadata_dir="${workdir}/metadata"
logs_dir="${workdir}/logs"

mkdir -p "${raw_dir}" "${aggregated_dir}" "${metadata_dir}" "${logs_dir}"

/usr/local/bin/print_tool_versions.sh > "${metadata_dir}/tool_versions.tsv"

input_id="$(basename "${fastq}")"
input_id="${input_id%.fastq.gz}"
input_id="${input_id%.fq.gz}"
input_bytes="$(stat -c %s "${fastq}")"
input_count_probe="${metadata_dir}/input_count_probe.json"
input_count_probe_stdout="${logs_dir}/input_count_probe.stdout.log"
input_count_probe_stderr="${logs_dir}/input_count_probe.stderr.log"
if ! "${bamana_bin}" enumerate --input "${fastq}" --json-pretty > "${input_count_probe_stdout}" 2> "${input_count_probe_stderr}"; then
  cp "${input_count_probe_stdout}" "${input_count_probe}"
  echo "bamana enumerate failed while preparing benchmark input metrics" >&2
  echo "see ${input_count_probe_stdout} and ${input_count_probe_stderr}" >&2
  if [[ -s "${input_count_probe_stdout}" ]]; then
    cat "${input_count_probe_stdout}" >&2
  fi
  exit 1
fi
cp "${input_count_probe_stdout}" "${input_count_probe}"
input_records="$(jq -r '.data.records' "${input_count_probe}")"

input_metrics_json="${metadata_dir}/input_metrics.json"
jq -n \
  --arg source_input_id "${input_id}" \
  --arg source_input_path "${fastq}" \
  --arg staged_input_id "${input_id}" \
  --arg staged_input_path "${fastq}" \
  --argjson input_bytes "${input_bytes}" \
  --argjson input_records "${input_records}" \
  '{
    source_input_id: $source_input_id,
    source_input_path: $source_input_path,
    source_input_type: "FASTQ_GZ",
    source_category: "fastq_gz",
    expected_sort_order: "not_applicable",
    has_index: false,
    reference_context: "not_applicable",
    source_owner: "user_supplied",
    sensitivity_level: "unspecified",
    staged_input_id: $staged_input_id,
    staged_input_path: $staged_input_path,
    staging_mode: "direct",
    staging_realization: "bind_mount",
    scenario_materialization: "source_fastq_gz",
    reuse_materialized_inputs: false,
    include_staging_in_timing: false,
    storage_context: "container_bind_mount",
    input_bytes: $input_bytes,
    records_processed: $input_records
  }' > "${input_metrics_json}"

bamana_command_file="${metadata_dir}/fastq_gz_enumerate.bamana.command.sh"
cat > "${bamana_command_file}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
"${bamana_bin}" enumerate --input "${fastq}" --json-pretty > "${bamana_output}"
EOF
chmod +x "${bamana_command_file}"

gzip_command_file="${metadata_dir}/fastq_gz_enumerate.gzip.command.sh"
cat > "${gzip_command_file}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
/usr/bin/gzip -cd "${fastq}" | /usr/bin/awk 'END { printf "%.0f\n", NR / 4 }' > "${comparator_output}"
EOF
chmod +x "${gzip_command_file}"

run_one() {
  local run_id="$1"
  local tool="$2"
  local version_cmd="$3"
  local workflow_variant="$4"
  local semantic_equivalence="$5"
  local output_target="$6"
  local command_file="$7"
  local notes="$8"

  (
    cd "${raw_dir}"
    BAMANA_BENCHMARK_FRAMEWORK_VERSION="fastq_gz_enumerate_cli_v1" \
      /usr/local/bin/run_benchmark.sh \
        --run-id "${run_id}" \
        --tool "${tool}" \
        --tool-version-cmd "${version_cmd}" \
        --scenario "fastq_gz_enumerate" \
        --workflow-variant "${workflow_variant}" \
        --semantic-equivalence "${semantic_equivalence}" \
        --support-status "supported" \
        --input-type "FASTQ_GZ" \
        --mapping-state "unknown" \
        --input-path "${fastq}" \
        --input-metrics-json "${input_metrics_json}" \
        --replicate 1 \
        --warmup-run false \
        --subsample-fraction 0 \
        --subsample-seed 0 \
        --subsample-mode "not_applicable" \
        --threads "${threads}" \
        --container-image "${container_image}" \
        --output-target "${output_target}" \
        --command-file "${command_file}" \
        --notes "${notes}"
  )

  for suffix in stdout.log stderr.log time.tsv failure.log; do
    if [[ -f "${raw_dir}/${run_id}.${suffix}" ]]; then
      mv "${raw_dir}/${run_id}.${suffix}" "${logs_dir}/${run_id}.${suffix}"
    fi
  done
}

run_one \
  "fastq_gz_enumerate.bamana.rep1" \
  "bamana" \
  "printf 'bamana=%s\n' \"\$("${bamana_bin}" --version | head -n 1)\"" \
  "bamana_enumerate" \
  "full" \
  "${bamana_output}" \
  "${bamana_command_file}" \
  "Bamana enumerate counts FASTQ.GZ records and auto-materializes FASTQ.GZI sidecars when absent."

run_one \
  "fastq_gz_enumerate.gzip.rep1" \
  "gzip" \
  "printf 'gzip=%s; awk=%s\n' \"\$(/usr/bin/gzip --version 2>&1 | head -n 1)\" \"\$(/usr/bin/awk --version 2>&1 | head -n 1)\"" \
  "gzip_decompress_line_count" \
  "partial" \
  "${comparator_output}" \
  "${gzip_command_file}" \
  "Comparator path uses gzip decompression plus line counting to derive FASTQ record count."

Rscript /workspace/benchmarks/R/aggregate_results.R \
  --input-dir "${raw_dir}" \
  --output-dir "${aggregated_dir}"

Rscript /workspace/benchmarks/R/build_support_matrix.R \
  --runs-csv "${aggregated_dir}/tidy_results.csv" \
  --outdir "${aggregated_dir}"

Rscript /workspace/benchmarks/R/render_fastq_ingress_report.R \
  --template "/workspace/benchmarks/R/fastq_gz_enumerate_report.Rmd" \
  --tidy-csv "${aggregated_dir}/tidy_results.csv" \
  --summary-csv "${aggregated_dir}/tidy_summary.csv" \
  --support-csv "${aggregated_dir}/support_matrix.csv" \
  --tool-versions "${metadata_dir}/tool_versions.tsv" \
  --fastq "${fastq}" \
  --bamana-output "${bamana_output}" \
  --comparator-output "${comparator_output}" \
  --bamana-output-label "Bamana enumeration output" \
  --comparator-output-label "gzip comparator output" \
  --container-image "${container_image}" \
  --profile "${profile}" \
  --output "${report}"
