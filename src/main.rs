use std::process::ExitCode;

use bamana::{
    cli::{Cli, Commands, ReheaderPlatform},
    commands,
    forensics::duplication::DuplicationScanOptions,
    json::{CommandResponse, emit_response},
};
use clap::Parser;
use commands::{
    annotate_rg::AnnotateRgRequest,
    benchmark::BenchmarkRequest,
    check_eof::{CheckEofRequest, CheckEofResponse},
    check_index::CheckIndexRequest,
    check_map::{CheckMapPayload, CheckMapRequest},
    check_sort::{CheckSortPayload, CheckSortRequest},
    check_tag::CheckTagRequest,
    checksum::ChecksumRequest,
    consume::ConsumeRequest,
    deduplicate::DeduplicateRequest,
    enumerate::EnumerateRequest,
    fastq::FastqRequest,
    forensic_inspect::ForensicInspectRequest,
    header::{HeaderRequest, HeaderResponse},
    identify::{IdentifyRequest, IdentifyResponse},
    index::IndexRequest,
    inspect_duplication::InspectDuplicationRequest,
    merge::MergeRequest,
    reheader::ReheaderRequest,
    sort::SortRequest,
    subsample::SubsampleRequest,
    summary::SummaryRequest,
    unmap::UnmapRequest,
    validate::ValidateRequest,
    verify::{VerifyRequest, VerifyResponse},
};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Identify(args) => {
            let path = args.path;
            let result = commands::identify::run(IdentifyRequest { path: path.clone() });
            let response: CommandResponse<IdentifyResponse> =
                CommandResponse::from_result("identify", Some(path.as_path()), result);
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Benchmark(args) => {
            let fastq = args.fastq;
            let response = commands::benchmark::run(BenchmarkRequest {
                profile: args.profile,
                fastq: fastq.clone(),
                bam: args.bam,
                report: args.report,
                threads: args.threads,
                container_image: args.container_image,
                force: args.force,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Enumerate(args) => {
            let input = args.input;
            let response = commands::enumerate::run(EnumerateRequest {
                input: input.clone(),
                threads: args.threads,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Subsample(args) => {
            let input = args.input;
            let response = commands::subsample::run(SubsampleRequest {
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
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::InspectDuplication(args) => {
            let input = args.input;
            let response = commands::inspect_duplication::run(InspectDuplicationRequest {
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
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Deduplicate(args) => {
            let input = args.input;
            let response = commands::deduplicate::run(DeduplicateRequest {
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
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::ForensicInspect(args) => {
            let scopes = args.resolved_scopes();
            let input = args.input;
            let response = commands::forensic_inspect::run(ForensicInspectRequest {
                input: input.clone(),
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                max_findings: args.max_findings,
                scopes,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::AnnotateRg(args) => {
            let bam = args.bam;
            let response = commands::annotate_rg::run(AnnotateRgRequest {
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
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Consume(args) => {
            let response = commands::consume::run(ConsumeRequest {
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
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Fastq(args) => {
            let bam = args.bam;
            let response = commands::fastq::run(FastqRequest {
                bam: bam.clone(),
                out: args.out,
                threads: args.threads,
                force: args.force,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Checksum(args) => {
            let bam = args.bam;
            let response = commands::checksum::run(ChecksumRequest {
                bam: bam.clone(),
                mode: args.mode,
                algorithm: args.algorithm,
                include_header: args.include_header,
                exclude_tags: args.exclude_tags,
                only_primary: args.only_primary,
                mapped_only: args.mapped_only,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Merge(args) => {
            let response = commands::merge::run(MergeRequest {
                bam: args.bam,
                out: args.out,
                sort: args.sort,
                order: args.order,
                queryname_suborder: args.queryname_suborder,
                create_index: args.create_index,
                verify_checksum: args.verify_checksum,
                threads: args.threads,
                force: args.force,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Reheader(args) => {
            let bam = args.bam;
            let set_platform = args.set_platform.map(|platform| match platform {
                ReheaderPlatform::Ont => "ONT".to_string(),
                ReheaderPlatform::Illumina => "ILLUMINA".to_string(),
                ReheaderPlatform::Pacbio => "PACBIO".to_string(),
                ReheaderPlatform::Unknown => "UNKNOWN".to_string(),
            });
            let response = commands::reheader::run(ReheaderRequest {
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
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Sort(args) => {
            let bam = args.bam;
            let response = commands::sort::run(SortRequest {
                bam: bam.clone(),
                out: args.out,
                order: args.order,
                queryname_suborder: args.queryname_suborder,
                threads: args.threads,
                memory_limit: args.memory_limit,
                create_index: args.create_index,
                verify_checksum: args.verify_checksum,
                force: args.force,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Unmap(args) => {
            let bam = args.bam;
            let response = commands::unmap::run(UnmapRequest {
                bam: bam.clone(),
                out: args.out,
                dry_run: args.dry_run,
                threads: args.threads,
                force: args.force,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Verify(args) => {
            let bam = args.bam;
            let result = commands::verify::run(VerifyRequest { bam: bam.clone() });
            let response: CommandResponse<VerifyResponse> =
                CommandResponse::from_result("verify", Some(bam.as_path()), result);
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::CheckEof(args) => {
            let bam = args.bam;
            let result = commands::check_eof::run(CheckEofRequest { bam: bam.clone() });
            let response: CommandResponse<CheckEofResponse> =
                CommandResponse::from_result("check_eof", Some(bam.as_path()), result);
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Header(args) => {
            let bam = args.bam;
            let result = commands::header::run(HeaderRequest { bam: bam.clone() });
            let response: CommandResponse<HeaderResponse> =
                CommandResponse::from_result("header", Some(bam.as_path()), result);
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::CheckMap(args) => {
            let bam = args.bam;
            let result = commands::check_map::run(CheckMapRequest {
                bam: bam.clone(),
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                prefer_index: args.prefer_index,
            });
            let response: CommandResponse<CheckMapPayload> =
                CommandResponse::from_result("check_map", Some(bam.as_path()), result);
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::CheckIndex(args) => {
            let bam = args.bam;
            let response = commands::check_index::run(CheckIndexRequest {
                bam: bam.clone(),
                require: args.require,
                prefer_csi: args.prefer_csi,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Index(args) => {
            let bam = args.bam;
            let response = commands::index::run(IndexRequest {
                bam: bam.clone(),
                out: args.out,
                force: args.force,
                format: args.format,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Summary(args) => {
            let bam = args.bam;
            let response = commands::summary::run(SummaryRequest {
                bam: bam.clone(),
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                prefer_index: args.prefer_index,
                include_mapq_hist: args.include_mapq_hist,
                include_flags: args.include_flags,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::Validate(args) => {
            let bam = args.bam;
            let response = commands::validate::run(ValidateRequest {
                bam: bam.clone(),
                max_errors: args.max_errors,
                max_warnings: args.max_warnings,
                header_only: args.header_only,
                records: args.records,
                fail_fast: args.fail_fast,
                include_warnings: args.include_warnings,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::CheckTag(args) => {
            let bam = args.bam;
            let response = commands::check_tag::run(CheckTagRequest {
                bam: bam.clone(),
                tag: args.tag,
                sample_records: args.sample_records,
                full_scan: args.full_scan,
                require_type: args.require_type,
                count_hits: args.count_hits,
            });
            emit_response(&response, cli.global.json_pretty)
        }
        Commands::CheckSort(args) => {
            let bam = args.bam;
            let result = commands::check_sort::run(CheckSortRequest {
                bam: bam.clone(),
                sample_records: args.sample_records,
                strict: args.strict,
            });
            let response: CommandResponse<CheckSortPayload> =
                CommandResponse::from_result("check_sort", Some(bam.as_path()), result);
            emit_response(&response, cli.global.json_pretty)
        }
    }
}
