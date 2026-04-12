use std::path::PathBuf;

use crate::{error::BamanaError, formats::probe::probe_path, json::schema::IdentifyData};

#[derive(Debug)]
pub struct IdentifyRequest {
    pub path: PathBuf,
}

pub fn run(request: IdentifyRequest) -> Result<IdentifyData, BamanaError> {
    let probe = probe_path(&request.path)?;
    Ok(IdentifyData {
        detected_format: probe.detected_format,
        container: probe.container,
        confidence: probe.confidence,
    })
}
