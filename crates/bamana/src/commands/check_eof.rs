use bamana_core::{
    commands::check_eof::{CheckEofRequest, run as run_check_eof},
    error::BamanaError,
    json::schema::CommandResponse,
};

use crate::cli::BamPathArgs;

pub fn run(args: &BamPathArgs) -> CommandResponse<bamana_core::json::schema::CheckEofData> {
    match run_check_eof(CheckEofRequest {
        bam: args.bam.clone(),
    }) {
        Ok(data) if data.complete => {
            CommandResponse::success("check_eof", Some(args.bam.as_path()), data)
        }
        Ok(data) => CommandResponse::failure(
            "check_eof",
            Some(args.bam.as_path()),
            Some(data),
            BamanaError::TruncatedFile {
                path: args.bam.clone(),
                detail: "Expected BGZF EOF marker was not found.".to_string(),
            }
            .into(),
        ),
        Err(error) => {
            CommandResponse::failure("check_eof", Some(args.bam.as_path()), None, error.into())
        }
    }
}
