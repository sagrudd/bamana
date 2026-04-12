use bamana_core::{
    commands::header::{HeaderRequest, run as run_header},
    json::schema::CommandResponse,
};

use crate::cli::BamPathArgs;

pub fn run(args: &BamPathArgs) -> CommandResponse<bamana_core::json::schema::HeaderData> {
    match run_header(HeaderRequest {
        bam: args.bam.clone(),
    }) {
        Ok(data) => CommandResponse::success("header", Some(args.bam.as_path()), data),
        Err(error) => {
            CommandResponse::failure("header", Some(args.bam.as_path()), None, error.into())
        }
    }
}
