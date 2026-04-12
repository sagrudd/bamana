mod bam;
mod cli;
mod commands;
mod error;
mod formats;
mod json;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Commands};
use commands::{
    check_eof::{CheckEofRequest, CheckEofResponse},
    header::{HeaderRequest, HeaderResponse},
    identify::{IdentifyRequest, IdentifyResponse},
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
    }
}
