nextflow.enable.dsl = 2

include { INDEX_RAW_RESULTS } from '../modules/index_raw_results'

workflow COLLECT_RESULTS {
    take:
    raw_json
    raw_tsv
    wrapper_json
    command_file
    command_log

    main:
    indexed = INDEX_RAW_RESULTS(raw_json.map { meta, result_json -> result_json }.collect())

    emit:
    raw_json = raw_json
    raw_tsv = raw_tsv
    wrapper_json = wrapper_json
    command_file = command_file
    command_log = command_log
    inventory_tsv = indexed.inventory_tsv
    inventory_json = indexed.inventory_json
}
