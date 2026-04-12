use std::path::PathBuf;

use serde::Serialize;

use crate::error::BamanaError;

#[derive(Debug)]
pub struct IdentifyRequest {
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct IdentifyReport {
    pub path: PathBuf,
    pub kind: &'static str,
    pub status: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Default)]
pub struct IdentifyService;

impl IdentifyService {
    pub fn identify(&self, request: IdentifyRequest) -> Result<IdentifyReport, BamanaError> {
        if !request.path.exists() {
            return Err(BamanaError::InputNotFound { path: request.path });
        }

        Ok(IdentifyReport {
            path: request.path,
            kind: "unknown",
            status: "scaffold",
            message: "File identification is not implemented yet; this command shape is stable enough for early development.",
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{IdentifyRequest, IdentifyService};

    #[test]
    fn identify_returns_scaffold_report_for_existing_path() {
        let service = IdentifyService;
        let report = service
            .identify(IdentifyRequest {
                path: PathBuf::from("."),
            })
            .expect("existing path should be accepted");

        assert_eq!(report.kind, "unknown");
        assert_eq!(report.status, "scaffold");
    }

    #[test]
    fn identify_rejects_missing_path() {
        let service = IdentifyService;
        let result = service.identify(IdentifyRequest {
            path: PathBuf::from("this-path-should-not-exist-for-bamana-tests"),
        });

        assert!(result.is_err());
    }
}
