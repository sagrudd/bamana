use std::{process::ExitCode, time::Instant};

use bamana::{
    cli::{Cli, Commands, ReheaderPlatform},
    commands,
    forensics::duplication::DuplicationScanOptions,
    json::{CommandResponse, emit_response},
};
use clap::Parser;
use serde::Serialize;
use commands::{
    annotate_rg::AnnotateRgRequest,
    benchmark::BenchmarkRequest,
    check_eof::CheckEofRequest,
    check_index::CheckIndexRequest,
    check_map::CheckMapRequest,
    check_sort::CheckSortRequest,
    check_tag::CheckTagRequest,
    checksum::ChecksumRequest,
    consume::ConsumeRequest,
    deduplicate::DeduplicateRequest,
    enumerate::EnumerateRequest,
    fastq::FastqRequest,
    forensic_inspect::ForensicInspectRequest,
    header::HeaderRequest,
    identify::IdentifyRequest,
    index::IndexRequest,
    inspect_duplication::InspectDuplicationRequest,
    merge::MergeRequest,
    reheader::ReheaderRequest,
    sort::SortRequest,
    subsample::SubsampleRequest,
    summary::SummaryRequest,
    unmap::UnmapRequest,
    validate::ValidateRequest,
    verify::VerifyRequest,
};

fn emit_timed_response<T, F>(pretty: bool, build: F) -> ExitCode
where
    T: Serialize,
    F: FnOnce() -> CommandResponse<T>,
{
    let started = Instant::now();
    let response = build().with_analysis_wall_seconds(started.elapsed().as_secs_f64());
    emit_response(&response, pretty)
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Identify(args) => emit_timed_response(cli.global.json_pretty, || {
            let path = args.path;
            let result = commands::identify::run(IdentifyRequest { path: path.clone() });
            CommandResponse::from_result("identify", Some(path.as_path()), result)
        }),
        Commands::Benchmark(args) => emit_timed_response(cli.global.json_pretty, || {
            let fastq = args.fastq;
            commands::benchmark::run(BenchmarkRequest {
                profile: args.profile,
                fastq: fastq.clone(),
                bam: args.bam,
                report: args.report,
                threads: args.threads,
                container_image: args.container_image,
                force: args.force,
            })
        }),
        Commands::Enumerate(args) => emit_timed_response(cli.global.json_pretty, || {
            let input = args.input;
            commands::enumerate::run(EnumerateRequest {
                input: input.clone(),
                threads: args.threads,
            })
        }),
        Commands::Subsample(args) => emit_timed_response(cli.global.json_pretty, || {
            let input = args.input;
            commands::subsample::run(SubsampleRequest {
                input: input.clone(),
                out: args.out,
                fraction: args.fraction,
                mode: args.mode,
                seed: args.seed,
                identity: args.identity,
                dry_run: args.dry_run,
                create_index: args.create_index,
                mapped_only: args.mapped_only,
                primary_only: args.primary_only,
                threads: args.threads,
                force: args.force,
            })
        }),
        Commands::InspectDuplication(args) => emit_timed_response(cli.global.json_pretty, || {
            let input = args.input;
            commands::inspect_duplication::run(InspectDuplicationRequest {
                input: input.clone(),
                options: DuplicationScanOptions {
                    identity_mode: args.identity,
                    min_block_size: args.min_block_size.max(1),
                    max_findings: args.max_findings.max(1),
                    record_limit: if args.full_scan {
                        u64::MAX
                    } else {
                        args.sample_records.max(1) as u64
                    },
                },
            })
        }),
        Commands::Deduplicate(args) => emit_timed_response(cli.global.json_pretty, || {
            let input = args.input;
            commands::deduplicate::run(DeduplicateRequest {
                input: input.clone(),
                out: args.out,
                mode: args.mode,
                identity_mode: args.identity,
                keep_policy: args.keep,
                dry_run: args.dry_run,
                force: args.force,
                min_block_size: args.min_block_size,
                verify_checksum: args.verify_checksum,
                emit_removed_report: args.emit_removed_report,
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                reindex: args.reindex,
                json_pretty: cli.global.json_pretty,
            })
        }),
        Commands::ForensicInspect(args) => emit_timed_response(cli.global.json_pretty, || {
            let scopes = args.resolved_scopes();
            let input = args.input;
            commands::forensic_inspect::run(ForensicInspectRequest {
                input: input.clone(),
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                max_findings: args.max_findings,
                scopes,
            })
        }),
        Commands::AnnotateRg(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::annotate_rg::run(AnnotateRgRequest {
                bam: bam.clone(),
                rg_id: args.rg_id,
                out: args.out,
                only_missing: args.only_missing,
                replace_existing: args.replace_existing,
                fail_on_conflict: args.fail_on_conflict,
                require_header_rg: args.require_header_rg,
                create_header_rg: args.create_header_rg,
                add_header_rg: args.add_header_rg,
                set_header_rg: args.set_header_rg,
                reindex: args.reindex,
                verify_checksum: args.verify_checksum,
                threads: args.threads,
                force: args.force,
                dry_run: args.dry_run,
            })
        }),
        Commands::Consume(args) => emit_timed_response(cli.global.json_pretty, || {
            commands::consume::run(ConsumeRequest {
                input: args.input,
                out: args.out,
                mode: args.mode,
                recursive: args.recursive,
                threads: args.threads,
                force: args.force,
                sort: args.sort,
                create_index: args.create_index,
                verify_checksum: args.verify_checksum,
                dry_run: args.dry_run,
                reference: args.reference,
                reference_cache: args.reference_cache,
                reference_policy: args.reference_policy,
                sample: args.sample,
                read_group: args.read_group,
                platform: args.platform,
                include_glob: args.include_glob,
                exclude_glob: args.exclude_glob,
            })
        }),
        Commands::Fastq(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::fastq::run(FastqRequest {
                bam: bam.clone(),
                out: args.out,
                threads: args.threads,
                force: args.force,
            })
        }),
        Commands::Checksum(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::checksum::run(ChecksumRequest {
                bam: bam.clone(),
                mode: args.mode,
                algorithm: args.algorithm,
                include_header: args.include_header,
                exclude_tags: args.exclude_tags,
                only_primary: args.only_primary,
                mapped_only: args.mapped_only,
            })
        }),
        Commands::Merge(args) => emit_timed_response(cli.global.json_pretty, || {
            commands::merge::run(MergeRequest {
                bam: args.bam,
                out: args.out,
                sort: args.sort,
                order: args.order,
                queryname_suborder: args.queryname_suborder,
                create_index: args.create_index,
                verify_checksum: args.verify_checksum,
                threads: args.threads,
                force: args.force,
            })
        }),
        Commands::Reheader(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            let set_platform = args.set_platform.map(|platform| match platform {
                ReheaderPlatform::Ont => "ONT".to_string(),
                ReheaderPlatform::Illumina => "ILLUMINA".to_string(),
                ReheaderPlatform::Pacbio => "PACBIO".to_string(),
                ReheaderPlatform::Unknown => "UNKNOWN".to_string(),
            });
            commands::reheader::run(ReheaderRequest {
                bam: bam.clone(),
                out: args.out,
                header: args.header,
                add_rg: args.add_rg,
                set_rg: args.set_rg,
                remove_rg: args.remove_rg,
                set_sample: args.set_sample,
                set_platform,
                target_rg: args.target_rg,
                set_pg: args.set_pg,
                add_comment: args.add_comment,
                in_place: args.in_place,
                rewrite_minimized: args.rewrite_minimized,
                safe_rewrite: args.safe_rewrite,
                dry_run: args.dry_run,
                force: args.force,
                reindex: args.reindex,
                verify_checksum: args.verify_checksum,
            })
        }),
        Commands::Sort(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::sort::run(SortRequest {
                bam: bam.clone(),
                out: args.out,
                order: args.order,
                queryname_suborder: args.queryname_suborder,
                threads: args.threads,
                memory_limit: args.memory_limit,
                create_index: args.create_index,
                verify_checksum: args.verify_checksum,
                force: args.force,
            })
        }),
        Commands::Unmap(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::unmap::run(UnmapRequest {
                bam: bam.clone(),
                out: args.out,
                dry_run: args.dry_run,
                threads: args.threads,
                force: args.force,
            })
        }),
        Commands::Verify(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            let result = commands::verify::run(VerifyRequest { bam: bam.clone() });
            CommandResponse::from_result("verify", Some(bam.as_path()), result)
        }),
        Commands::CheckEof(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            let result = commands::check_eof::run(CheckEofRequest { bam: bam.clone() });
            CommandResponse::from_result("check_eof", Some(bam.as_path()), result)
        }),
        Commands::Header(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            let result = commands::header::run(HeaderRequest { bam: bam.clone() });
            CommandResponse::from_result("header", Some(bam.as_path()), result)
        }),
        Commands::CheckMap(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            let result = commands::check_map::run(CheckMapRequest {
                bam: bam.clone(),
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                prefer_index: args.prefer_index,
            });
            CommandResponse::from_result("check_map", Some(bam.as_path()), result)
        }),
        Commands::CheckIndex(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::check_index::run(CheckIndexRequest {
                bam: bam.clone(),
                require: args.require,
                prefer_csi: args.prefer_csi,
            })
        }),
        Commands::Index(args) => emit_timed_response(cli.global.json_pretty, || {
            let input = args.input;
            commands::index::run(IndexRequest {
                input: input.clone(),
                out: args.out,
                force: args.force,
                format: args.format,
            })
        }),
        Commands::Summary(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::summary::run(SummaryRequest {
                bam: bam.clone(),
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                prefer_index: args.prefer_index,
                include_mapq_hist: args.include_mapq_hist,
                include_flags: args.include_flags,
            })
        }),
        Commands::Validate(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::validate::run(ValidateRequest {
                bam: bam.clone(),
                max_errors: args.max_errors,
                max_warnings: args.max_warnings,
                header_only: args.header_only,
                records: args.records,
                fail_fast: args.fail_fast,
                include_warnings: args.include_warnings,
            })
        }),
        Commands::CheckTag(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            commands::check_tag::run(CheckTagRequest {
                bam: bam.clone(),
                tag: args.tag,
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                require_type: args.require_type,
                count_hits: args.count_hits,
            })
        }),
        Commands::CheckSort(args) => emit_timed_response(cli.global.json_pretty, || {
            let bam = args.bam;
            let result = commands::check_sort::run(CheckSortRequest {
                bam: bam.clone(),
                sample_records: args.sample_records,
                strict: args.strict,
            });
            CommandResponse::from_result("check_sort", Some(bam.as_path()), result)
        }),
    }
}
