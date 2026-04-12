use bamana_core::{
    commands::identify::{IdentifyRequest, run as run_identify},
    json::schema::CommandResponse,
};

use crate::cli::IdentifyArgs;

pub fn run(args: &IdentifyArgs) -> CommandResponse<bamana_core::json::schema::IdentifyData> {
    match run_identify(IdentifyRequest {
        path: args.path.clone(),
    }) {
        Ok(data) => CommandResponse::success("identify", Some(args.path.as_path()), data),
        Err(error) => {
            CommandResponse::failure("identify", Some(args.path.as_path()), None, error.into())
        }
    }
}
