use std::path::PathBuf;

use crate::{
    forensics::deduplicate::{DeduplicateConfig, DeduplicateFailure, DeduplicatePayload, execute},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct DeduplicateRequest {
    pub input: PathBuf,
    pub out: PathBuf,
    pub mode: crate::forensics::deduplicate::DeduplicateMode,
    pub identity_mode: crate::forensics::duplication::DuplicationIdentityMode,
    pub keep_policy: crate::forensics::deduplicate::DeduplicateKeepPolicy,
    pub dry_run: bool,
    pub force: bool,
    pub min_block_size: usize,
    pub verify_checksum: bool,
    pub emit_removed_report: Option<PathBuf>,
    pub sample_records: usize,
    pub full_scan: bool,
    pub reindex: bool,
    pub json_pretty: bool,
}

pub fn run(request: DeduplicateRequest) -> CommandResponse<DeduplicatePayload> {
    let config = DeduplicateConfig {
        input: request.input.clone(),
        out: request.out,
        mode: request.mode,
        identity_mode: request.identity_mode,
        keep_policy: request.keep_policy,
        dry_run: request.dry_run,
        force: request.force,
        min_block_size: request.min_block_size.max(1),
        verify_checksum: request.verify_checksum,
        emit_removed_report: request.emit_removed_report,
        sample_records: request.sample_records.max(1),
        full_scan: request.full_scan,
        reindex: request.reindex,
        json_pretty: request.json_pretty,
    };

    match execute(&config) {
        Ok(payload) => {
            CommandResponse::success("deduplicate", Some(request.input.as_path()), payload)
        }
        Err(DeduplicateFailure { payload, error }) => CommandResponse::failure_with_data(
            "deduplicate",
            Some(request.input.as_path()),
            Some(payload),
            error,
        ),
    }
}
