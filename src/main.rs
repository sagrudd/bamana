mod bam;
mod cli;
mod commands;
mod error;
mod formats;
mod ingest;
mod json;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Commands};
use commands::{
    check_eof::{CheckEofRequest, CheckEofResponse},
    check_index::CheckIndexRequest,
    check_map::{CheckMapPayload, CheckMapRequest},
    check_sort::{CheckSortPayload, CheckSortRequest},
    check_tag::CheckTagRequest,
    checksum::ChecksumRequest,
    consume::ConsumeRequest,
    header::{HeaderRequest, HeaderResponse},
    identify::{IdentifyRequest, IdentifyResponse},
    index::IndexRequest,
    merge::MergeRequest,
    sort::SortRequest,
    summary::SummaryRequest,
    validate::ValidateRequest,
    verify::{VerifyRequest, VerifyResponse},
};
use json::{CommandResponse, emit_response};

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
                sample: args.sample,
                read_group: args.read_group,
                platform: args.platform,
                include_glob: args.include_glob,
                exclude_glob: args.exclude_glob,
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
