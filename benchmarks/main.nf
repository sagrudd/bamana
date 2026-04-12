import groovy.json.JsonOutput
import groovy.json.JsonSlurper

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

def canonicalScenarioName(String scenario) {
    switch (scenario) {
        case 'mapped_bam_chain':
            return 'mapped_bam_pipeline'
        case 'unmapped_bam_chain':
            return 'unmapped_bam_pipeline'
        case 'fastq_ingest_chain':
            return 'fastq_consume_pipeline'
        default:
            return scenario
    }
}

def manifestInputType(String category) {
    switch (category) {
        case 'mapped_bam':
        case 'unmapped_bam':
            return 'BAM'
        case 'fastq_gz':
            return 'FASTQ_GZ'
        default:
            error "Unsupported benchmark input type '${category}' in manifest. Expected mapped_bam, unmapped_bam, or fastq_gz."
    }
}

def manifestMappingState(String category, String declaredState) {
    if (declaredState != null && !declaredState.trim().isEmpty()) {
        return declaredState.trim()
    }
    switch (category) {
        case 'mapped_bam':
            return 'mapped'
        case 'unmapped_bam':
            return 'unmapped'
        case 'fastq_gz':
            return 'unknown'
        default:
            return 'unspecified'
    }
}

def defaultScenarioMaterialization(String inputType) {
    if (inputType == 'BAM') {
        return 'source_or_subsampled_bam'
    }
    if (inputType == 'FASTQ_GZ') {
        return 'source_or_subsampled_fastq_gz'
    }
    'source'
}

def defaultExpectedSortOrder(String category) {
    switch (category) {
        case 'mapped_bam':
            return 'coordinate'
        case 'unmapped_bam':
            return 'unsorted'
        case 'fastq_gz':
            return 'not_applicable'
        default:
            return 'unspecified'
    }
}

def defaultAllowedScenarios(String inputType, String mappingState) {
    if (inputType == 'BAM' && mappingState == 'mapped') {
        return ['mapped_bam_pipeline', 'subsample_only']
    }
    if (inputType == 'BAM' && mappingState == 'unmapped') {
        return ['unmapped_bam_pipeline', 'subsample_only']
    }
    if (inputType == 'FASTQ_GZ') {
        return ['fastq_consume_pipeline', 'subsample_only']
    }
    return []
}

def inputTuples(List paths, String inputType, String mappingState) {
    paths.collect { pathString ->
        def inputFile = file(pathString, checkIfExists: true)
        def inputId = slugify(inputFile.getName())
        tuple(
            [
                input_id                  : inputId,
                input_type                : inputType,
                mapping_state             : mappingState,
                source_input_id           : inputId,
                source_input_path         : inputFile.toString(),
                source_input_type         : inputType,
                source_category           : inputType == 'FASTQ_GZ' ? 'fastq_gz' : "${mappingState}_bam",
                description               : '',
                expected_sort_order       : inputType == 'FASTQ_GZ' ? 'not_applicable' : (mappingState == 'mapped' ? 'coordinate' : 'unsorted'),
                has_index                 : false,
                reference_context         : 'unspecified',
                source_owner              : 'user_supplied',
                sensitivity_level         : 'unspecified',
                storage_context           : params.storage_context.toString(),
                staging_mode              : (params.staging_override ?: params.staging_mode).toString(),
                scenario_materialization  : defaultScenarioMaterialization(inputType),
                reuse_materialized_inputs : params.reuse_materialized_inputs as boolean,
                include_staging_in_timing : params.include_staging_in_timing as boolean,
                allowed_benchmark_scenarios: defaultAllowedScenarios(inputType, mappingState),
                notes                     : ''
            ],
            inputFile
        )
    }
}

def manifestTuples(def manifestPath) {
    if (manifestPath == null || manifestPath.toString().trim().isEmpty()) {
        return []
    }

    def manifestFile = file(manifestPath.toString(), checkIfExists: true)
    def manifest = new JsonSlurper().parseText(manifestFile.text)
    def entries = manifest.inputs instanceof Collection
        ? manifest.inputs
        : (manifest.entries instanceof Collection ? manifest.entries : [])

    entries.collect { entry ->
        def category = entry.type.toString()
        def inputType = manifestInputType(category)
        def mappingState = manifestMappingState(category, entry.mapped_state?.toString())
        def inputFile = file(entry.path.toString(), checkIfExists: true)
        def allowedScenarios = normalizeList(entry.allowed_benchmark_scenarios).collect { canonicalScenarioName(it) }
        if (allowedScenarios.isEmpty()) {
            allowedScenarios = defaultAllowedScenarios(inputType, mappingState)
        }
        def referenceContext = entry.reference_context != null
            ? JsonOutput.toJson(entry.reference_context)
            : 'null'

        tuple(
            [
                input_id                  : entry.id.toString(),
                input_type                : inputType,
                mapping_state             : mappingState,
                source_input_id           : entry.id.toString(),
                source_input_path         : inputFile.toString(),
                source_input_type         : inputType,
                source_category           : category,
                description               : entry.description?.toString() ?: '',
                expected_sort_order       : entry.expected_sort_order?.toString() ?: defaultExpectedSortOrder(category),
                has_index                 : (entry.has_index ?: false) as boolean,
                reference_context         : referenceContext,
                source_owner              : entry.source_owner?.toString() ?: 'unspecified',
                sensitivity_level         : entry.sensitivity_level?.toString() ?: 'unspecified',
                storage_context           : entry.storage_context?.toString() ?: params.storage_context.toString(),
                staging_mode              : (params.staging_override ?: entry.staging_policy?.mode ?: params.staging_mode).toString(),
                scenario_materialization  : entry.scenario_materialization?.toString() ?: defaultScenarioMaterialization(inputType),
                reuse_materialized_inputs : entry.reuse_materialized_inputs != null ? entry.reuse_materialized_inputs as boolean : params.reuse_materialized_inputs as boolean,
                include_staging_in_timing : entry.include_staging_in_timing != null ? entry.include_staging_in_timing as boolean : params.include_staging_in_timing as boolean,
                allowed_benchmark_scenarios: allowedScenarios,
                notes                     : entry.notes?.toString() ?: ''
            ],
            inputFile
        )
    }
}

def workflowVariantFor(String tool, String scenario) {
    def mapping = [
        bamana  : [
            mapped_bam_pipeline   : 'bamana_subsample_sort_partial_index',
            unmapped_bam_pipeline : 'bamana_subsample_only',
            fastq_consume_pipeline: 'bamana_consume_unmapped_bam',
            subsample_only        : 'bamana_subsample_only'
        ],
        samtools: [
            mapped_bam_pipeline   : 'samtools_view_sort_index',
            unmapped_bam_pipeline : 'samtools_view_subsample_only',
            fastq_consume_pipeline: 'unsupported',
            subsample_only        : 'samtools_view_subsample_only'
        ],
        sambamba: [
            mapped_bam_pipeline   : 'sambamba_view_sort_index',
            unmapped_bam_pipeline : 'sambamba_view_subsample_only',
            fastq_consume_pipeline: 'unsupported',
            subsample_only        : 'sambamba_view_subsample_only'
        ],
        seqtk   : [
            mapped_bam_pipeline   : 'unsupported',
            unmapped_bam_pipeline : 'unsupported',
            fastq_consume_pipeline: 'seqtk_sample_gzip',
            subsample_only        : 'seqtk_sample_gzip'
        ],
        rasusa  : [
            mapped_bam_pipeline   : 'rasusa_strategy_required',
            unmapped_bam_pipeline : 'rasusa_strategy_required',
            fastq_consume_pipeline: 'rasusa_strategy_required',
            subsample_only        : 'rasusa_strategy_required'
        ],
        fastcat : [
            mapped_bam_pipeline   : 'unsupported',
            unmapped_bam_pipeline : 'unsupported',
            fastq_consume_pipeline: 'fastcat_concat_gzip',
            subsample_only        : 'unsupported'
        ]
    ]
    return mapping[tool][scenario]
}

def applicableScenarios(Map meta, List scenarios) {
    scenarios.findAll { scenario ->
        scenario = canonicalScenarioName(scenario)
        if (meta.allowed_benchmark_scenarios instanceof Collection && !meta.allowed_benchmark_scenarios.isEmpty()) {
            if (!meta.allowed_benchmark_scenarios.contains(scenario)) {
                return false
            }
        }
        if (scenario == 'mapped_bam_pipeline') {
            return meta.input_type == 'BAM' && meta.mapping_state == 'mapped'
        }
        if (scenario == 'unmapped_bam_pipeline') {
            return meta.input_type == 'BAM' && meta.mapping_state == 'unmapped'
        }
        if (scenario == 'fastq_consume_pipeline') {
            return meta.input_type == 'FASTQ_GZ'
        }
        if (scenario == 'subsample_only') {
            return meta.input_type == 'BAM' || meta.input_type == 'FASTQ_GZ'
        }
        return false
    }
}

def shouldIncludeToolScenario(String tool, Map inputMeta, String scenario, boolean includeUnsupportedRows) {
    if (tool == 'fastcat' && !(params.fastcat_enabled as boolean)) {
        return false
    }

    if (includeUnsupportedRows) {
        return true
    }

    if (tool == 'rasusa') {
        return false
    }

    if (scenario == 'fastq_consume_pipeline') {
        return tool in ['bamana', 'seqtk', 'fastcat']
    }

    if (scenario == 'subsample_only') {
        if (inputMeta.input_type == 'BAM') {
            return tool in ['bamana', 'samtools', 'sambamba']
        }
        if (inputMeta.input_type == 'FASTQ_GZ') {
            return tool in ['bamana', 'seqtk']
        }
    }

    return tool in ['bamana', 'samtools', 'sambamba']
}

def runPlansForInput(Map inputMeta, List tools, List scenarios, int replicateCount, int warmupRuns) {
    def plans = []
    def includeUnsupportedRows = params.include_unsupported_matrix_rows as boolean
    applicableScenarios(inputMeta, scenarios).each { scenario ->
        tools.each { tool ->
            if (shouldIncludeToolScenario(tool, inputMeta, scenario, includeUnsupportedRows)) {
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
    }
    plans
}

workflow {
    def tools = normalizeList(params.tools)
    if (!(params.fastcat_enabled as boolean)) {
        tools = tools.findAll { it != 'fastcat' }
    }
    def scenarios = normalizeList(params.scenarios).collect { canonicalScenarioName(it) }
    def requestedDatasetIds = normalizeList(params.dataset_ids)
    def replicateCount = (params.replicates ?: params.replicate_count) as int
    def warmupRuns = params.warmup_runs as int

    if (!requestedDatasetIds.isEmpty() && (params.input_manifest == null || params.input_manifest.toString().trim().isEmpty())) {
        error "dataset_ids requires input_manifest. Supply a manifest or remove dataset_ids."
    }

    def allInputs = []
    allInputs.addAll(manifestTuples(params.input_manifest))
    allInputs.addAll(inputTuples(normalizeList(params.mapped_bams), 'BAM', 'mapped'))
    allInputs.addAll(inputTuples(normalizeList(params.unmapped_bams), 'BAM', 'unmapped'))
    allInputs.addAll(inputTuples(normalizeList(params.fastq_gzs), 'FASTQ_GZ', 'unknown'))

    if (!requestedDatasetIds.isEmpty()) {
        def availableIds = allInputs.collect { meta, _ -> meta.input_id }
        def missingIds = requestedDatasetIds.findAll { !availableIds.contains(it) }
        if (!missingIds.isEmpty()) {
            error "Requested dataset_ids were not present in the manifest: ${missingIds.join(', ')}"
        }
        allInputs = allInputs.findAll { meta, _ -> requestedDatasetIds.contains(meta.input_id) }
    }

    if (allInputs.isEmpty()) {
        error "No benchmark inputs were provided. Supply input_manifest, mapped_bams, unmapped_bams, and/or fastq_gzs."
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

    if ((params.enable_plots ?: params.enable_plotting) as boolean) {
        PLOT_RESULTS(
            aggregated.runs_tsv,
            aggregated.summary_tsv,
            aggregated.support_tsv
        )
    }
}
