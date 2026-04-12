nextflow.enable.dsl = 2

include { BENCHMARK_WRAPPER_RUN } from '../modules/benchmark_wrapper_run'

workflow RUN_BENCHMARK_MATRIX {
    take:
    benchmark_attempts

    main:
    executed = BENCHMARK_WRAPPER_RUN(benchmark_attempts)

    emit:
    raw_json = executed.raw_json
    raw_tsv = executed.raw_tsv
    wrapper_json = executed.wrapper_json
    command_file = executed.command_file
    command_log = executed.command_log
    stdout_log = executed.stdout_log
    stderr_log = executed.stderr_log
    time_tsv = executed.time_tsv
}
