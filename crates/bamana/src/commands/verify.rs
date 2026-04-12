use bamana_core::{
    commands::verify::{VerifyRequest, run as run_verify},
    json::schema::CommandResponse,
};

use crate::cli::BamPathArgs;

pub fn run(args: &BamPathArgs) -> CommandResponse<bamana_core::json::schema::VerifyData> {
    match run_verify(VerifyRequest {
        bam: args.bam.clone(),
    }) {
        Ok(data) => CommandResponse::success("verify", Some(args.bam.as_path()), data),
        Err(error) => {
            CommandResponse::failure("verify", Some(args.bam.as_path()), None, error.into())
        }
    }
}
