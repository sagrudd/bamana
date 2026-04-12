nextflow.enable.dsl = 2

include { STAGE_INPUT } from './modules/stage_input'
include { RUN_BAMANA_BENCHMARK } from './modules/run_bamana'
include { RUN_SAMTOOLS_BENCHMARK } from './modules/run_samtools'
include { RUN_SAMBAMBA_BENCHMARK } from './modules/run_sambamba'
include { RUN_SEQTK_BENCHMARK } from './modules/run_seqtk'
include { RUN_RASUSA_BENCHMARK } from './modules/run_rasusa'
include { RUN_FASTCAT_BENCHMARK } from './modules/run_fastcat'
include { AGGREGATE_RESULTS } from './modules/aggregate_results'
include { PLOT_RESULTS } from './modules/plot_results'

def normalizeList(value) {
    if (value == null) {
        return []
    }
    if (value instanceof Collection) {
        return value.collect { it.toString().trim() }.findAll { !it.isEmpty() }
    }
    def text = value.toString().trim()
    if (text.isEmpty()) {
        return []
    }
    return text.split(',').collect { it.trim() }.findAll { !it.isEmpty() }
}

def slugify(String value) {
    value.replaceAll(/\.fastq\.gz$/, '')
        .replaceAll(/\.fq\.gz$/, '')
        .replaceAll(/\.bam$/, '')
        .replaceAll(/[^A-Za-z0-9._-]+/, '_')
}

def inputTuples(List paths, String inputType, String mappingState) {
    paths.collect { pathString ->
        def inputFile = file(pathString, checkIfExists: true)
        def inputId = slugify(inputFile.getName())
        tuple(
            [
                input_id     : inputId,
                input_type   : inputType,
                mapping_state: mappingState
            ],
            inputFile
        )
    }
}

def workflowVariantFor(String tool, String scenario) {
    def mapping = [
        bamana  : [
            mapped_bam_chain  : 'subsample_sort_partial_index',
            unmapped_bam_chain: 'subsample_only',
            fastq_ingest_chain: 'consume_to_unmapped_bam'
        ],
        samtools: [
            mapped_bam_chain  : 'subsample_sort_index',
            unmapped_bam_chain: 'subsample_only',
            fastq_ingest_chain: 'unsupported'
        ],
        sambamba: [
            mapped_bam_chain  : 'subsample_sort_index',
            unmapped_bam_chain: 'subsample_only',
            fastq_ingest_chain: 'unsupported'
        ],
        seqtk   : [
            mapped_bam_chain  : 'unsupported',
            unmapped_bam_chain: 'unsupported',
            fastq_ingest_chain: 'fractional_fastq_sample'
        ],
        rasusa  : [
            mapped_bam_chain  : 'strategy_required',
            unmapped_bam_chain: 'strategy_required',
            fastq_ingest_chain: 'strategy_required'
        ],
        fastcat : [
            mapped_bam_chain  : 'unsupported',
            unmapped_bam_chain: 'unsupported',
            fastq_ingest_chain: 'fastq_concat_only'
        ]
    ]
    return mapping[tool][scenario]
}

def applicableScenarios(Map meta, List scenarios) {
    scenarios.findAll { scenario ->
        if (scenario == 'mapped_bam_chain') {
            return meta.input_type == 'BAM' && meta.mapping_state == 'mapped'
        }
        if (scenario == 'unmapped_bam_chain') {
            return meta.input_type == 'BAM' && meta.mapping_state == 'unmapped'
        }
        if (scenario == 'fastq_ingest_chain') {
            return meta.input_type == 'FASTQ_GZ'
        }
        return false
    }
}

def runPlansForInput(Map inputMeta, List tools, List scenarios, int replicateCount, int warmupRuns) {
    def plans = []
    applicableScenarios(inputMeta, scenarios).each { scenario ->
        tools.each { tool ->
            (1..warmupRuns).each { warmup ->
                plans << [
                    run_id            : "${tool}.${scenario}.${inputMeta.input_id}.warmup${warmup}",
                    tool              : tool,
                    scenario          : scenario,
                    workflow_variant  : workflowVariantFor(tool, scenario),
                    replicate         : warmup,
                    warmup_run        : true,
                    input_id          : inputMeta.input_id,
                    input_type        : inputMeta.input_type,
                    mapping_state     : inputMeta.mapping_state,
                    subsample_fraction: params.subsample_fraction,
                    subsample_seed    : params.subsample_seed,
                    subsample_mode    : params.subsample_mode,
                    threads           : params.threads,
                    bamana_bin        : params.bamana_bin
                ]
            }
            (1..replicateCount).each { replicate ->
                plans << [
                    run_id            : "${tool}.${scenario}.${inputMeta.input_id}.rep${replicate}",
                    tool              : tool,
                    scenario          : scenario,
                    workflow_variant  : workflowVariantFor(tool, scenario),
                    replicate         : replicate,
                    warmup_run        : false,
                    input_id          : inputMeta.input_id,
                    input_type        : inputMeta.input_type,
                    mapping_state     : inputMeta.mapping_state,
                    subsample_fraction: params.subsample_fraction,
                    subsample_seed    : params.subsample_seed,
                    subsample_mode    : params.subsample_mode,
                    threads           : params.threads,
                    bamana_bin        : params.bamana_bin
                ]
            }
        }
    }
    plans
}

workflow {
    def tools = normalizeList(params.tools)
    def scenarios = normalizeList(params.scenarios)
    def replicateCount = params.replicate_count as int
    def warmupRuns = params.warmup_runs as int

    def allInputs = []
    allInputs.addAll(inputTuples(normalizeList(params.mapped_bams), 'BAM', 'mapped'))
    allInputs.addAll(inputTuples(normalizeList(params.unmapped_bams), 'BAM', 'unmapped'))
    allInputs.addAll(inputTuples(normalizeList(params.fastq_gzs), 'FASTQ_GZ', 'not_applicable'))

    if (allInputs.isEmpty()) {
        error "No benchmark inputs were provided. Supply mapped_bams, unmapped_bams, and/or fastq_gzs."
    }

    raw_inputs = Channel.fromList(allInputs)
    staged_inputs = STAGE_INPUT(raw_inputs)

    run_matrix = staged_inputs.flatMap { meta, input_file, input_metrics_json, input_metrics_tsv ->
        runPlansForInput(meta, tools, scenarios, replicateCount, warmupRuns).collect { runMeta ->
            tuple(runMeta, input_file, input_metrics_json, input_metrics_tsv)
        }
    }

    bamana_runs = run_matrix.filter { meta, _, _, _ -> meta.tool == 'bamana' }
    samtools_runs = run_matrix.filter { meta, _, _, _ -> meta.tool == 'samtools' }
    sambamba_runs = run_matrix.filter { meta, _, _, _ -> meta.tool == 'sambamba' }
    seqtk_runs = run_matrix.filter { meta, _, _, _ -> meta.tool == 'seqtk' }
    rasusa_runs = run_matrix.filter { meta, _, _, _ -> meta.tool == 'rasusa' }
    fastcat_runs = run_matrix.filter { meta, _, _, _ -> meta.tool == 'fastcat' }

    bamana_results = RUN_BAMANA_BENCHMARK(bamana_runs)
    samtools_results = RUN_SAMTOOLS_BENCHMARK(samtools_runs)
    sambamba_results = RUN_SAMBAMBA_BENCHMARK(sambamba_runs)
    seqtk_results = RUN_SEQTK_BENCHMARK(seqtk_runs)
    rasusa_results = RUN_RASUSA_BENCHMARK(rasusa_runs)
    fastcat_results = RUN_FASTCAT_BENCHMARK(fastcat_runs)

    result_tsvs = bamana_results.result_tsv
        .mix(samtools_results.result_tsv)
        .mix(sambamba_results.result_tsv)
        .mix(seqtk_results.result_tsv)
        .mix(rasusa_results.result_tsv)
        .mix(fastcat_results.result_tsv)

    aggregated = AGGREGATE_RESULTS(result_tsvs.collect())

    if (params.enable_plotting) {
        PLOT_RESULTS(
            aggregated.runs_tsv,
            aggregated.summary_tsv,
            aggregated.support_tsv
        )
    }
}
