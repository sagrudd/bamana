nextflow.enable.dsl = 2

process RUN_BAMANA_BENCHMARK {
    tag "${meta.run_id}"
    publishDir "${params.output_dir}/per_run", mode: 'copy'

    input:
    tuple val(meta), path(input_file), path(input_metrics_json), path(input_metrics_tsv)

    output:
    path("${meta.run_id}.result.tsv"), emit: result_tsv
    path("${meta.run_id}.result.json"), emit: result_json

    script:
    def outputTarget = ''
    def supportStatus = 'supported'
    def semanticEquivalence = 'full'
    def notes = ''
    def command = 'true'
    def seedArg = meta.subsample_mode == 'random' ? "--seed ${meta.subsample_seed}" : ""

    if (meta.scenario == 'mapped_bam_pipeline') {
        outputTarget = "${meta.run_id}.sorted.bam"
        semanticEquivalence = 'partial'
        command = """\
set -euo pipefail
"${meta.bamana_bin}" subsample --input "${input_file}" --out "${meta.run_id}.subsampled.bam" --fraction ${meta.subsample_fraction} --mode ${meta.subsample_mode} ${seedArg} --force
"${meta.bamana_bin}" sort --bam "${meta.run_id}.subsampled.bam" --out "${outputTarget}" --force
"""
        notes = 'Bamana mapped-BAM benchmarking now includes real subsample plus sort execution. The workflow remains partial because executable BAM index creation is still deferred.'
    } else if (meta.scenario == 'unmapped_bam_pipeline') {
        outputTarget = "${meta.run_id}.subsampled.bam"
        command = """\
set -euo pipefail
"${meta.bamana_bin}" subsample --input "${input_file}" --out "${outputTarget}" --fraction ${meta.subsample_fraction} --mode ${meta.subsample_mode} ${seedArg} --force
"""
        notes = 'Bamana unmapped-BAM benchmarking now exercises the implemented subsample command directly.'
    } else if (meta.scenario == 'fastq_consume_pipeline') {
        outputTarget = "${meta.run_id}.output.bam"
        command = """\
set -euo pipefail
"${meta.bamana_bin}" consume --input "${input_file}" --out "${outputTarget}" --mode unmapped --force
"""
        notes = 'Bamana fastq ingestion uses consume, while fastq subsample benchmarking can now be added in a later variant using the implemented subsample command.'
    } else if (meta.scenario == 'subsample_only') {
        outputTarget = meta.input_type == 'FASTQ_GZ'
            ? "${meta.run_id}.subsampled.fastq.gz"
            : "${meta.run_id}.subsampled.bam"
        command = """\
set -euo pipefail
"${meta.bamana_bin}" subsample --input "${input_file}" --out "${outputTarget}" --fraction ${meta.subsample_fraction} --mode ${meta.subsample_mode} ${seedArg} --force
"""
        notes = 'Bamana subsample-only benchmarking uses the implemented subsample command directly on the staged input.'
    }

    """
    cat <<'EOF' > command.sh
${command}
EOF
    chmod +x command.sh

    run_benchmark.sh \
      --run-id "${meta.run_id}" \
      --tool "bamana" \
      --tool-version-cmd "${meta.bamana_bin} --version" \
      --scenario "${meta.scenario}" \
      --workflow-variant "${meta.workflow_variant}" \
      --semantic-equivalence "${semanticEquivalence}" \
      --support-status "${supportStatus}" \
      --input-type "${meta.input_type}" \
      --mapping-state "${meta.mapping_state}" \
      --input-path "${input_file}" \
      --input-metrics-json "${input_metrics_json}" \
      --replicate "${meta.replicate}" \
      --warmup-run "${meta.warmup_run}" \
      --subsample-fraction "${meta.subsample_fraction}" \
      --subsample-seed "${meta.subsample_seed}" \
      --subsample-mode "${meta.subsample_mode}" \
      --threads "${meta.threads}" \
      --container-image "${params.container_image}" \
      --output-target "${outputTarget}" \
      --command-file command.sh \
      --notes "${notes}"
    """
}
