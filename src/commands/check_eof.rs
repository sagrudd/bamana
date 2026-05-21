use std::path::PathBuf;

use serde::Serialize;

use crate::{
    error::AppError,
    formats::{
        bgzf,
        probe::{ContainerKind, DetectedFormat, probe_path},
    },
};

#[derive(Debug)]
pub struct CheckEofRequest {
    pub bam: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct CheckEofResponse {
    pub detected_format: DetectedFormat,
    pub bgzf_eof_present: bool,
    pub complete: bool,
    pub semantic_note: String,
}

pub fn run(request: CheckEofRequest) -> Result<CheckEofResponse, AppError> {
    let probe = probe_path(&request.bam)?;

    if probe.detected_format == DetectedFormat::Unknown {
        return Err(AppError::UnknownFormat { path: request.bam });
    }

    if probe.detected_format != DetectedFormat::Bam {
        return Err(AppError::NotBam {
            path: request.bam,
            detected_format: probe.detected_format,
        });
    }

    if probe.container != ContainerKind::Bgzf {
        return Err(AppError::InvalidBam {
            path: request.bam,
            detail: "Input did not present a BGZF-compatible container header.".to_string(),
        });
    }

    let bgzf_eof_present = bgzf::has_bgzf_eof(&request.bam)?;
    if !bgzf_eof_present {
        return Err(AppError::TruncatedFile {
            path: request.bam,
            detail: "Expected BGZF EOF marker was not found.".to_string(),
        });
    }

    Ok(CheckEofResponse {
        detected_format: DetectedFormat::Bam,
        bgzf_eof_present: true,
        complete: true,
        semantic_note: "EOF marker presence indicates tail completeness only and does not imply full BAM validity.".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bgzf::{BGZF_EOF_MARKER, test_support},
        error::AppError,
    };

    use super::{CheckEofRequest, run};

    #[test]
    fn check_eof_accepts_valid_bgzf_bam_with_canonical_eof_marker() {
        let bytes = test_support::build_bam_file_with_header(
            "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:100\n",
            &[("chr1", 100)],
        );
        let path = test_support::write_temp_file("check-eof-valid", "bam", &bytes);

        let response = run(CheckEofRequest { bam: path.clone() })
            .expect("valid BGZF BAM should report EOF presence");

        fs::remove_file(path).expect("fixture should be removed");
        assert!(response.bgzf_eof_present);
        assert!(response.complete);
        assert_eq!(
            response.semantic_note,
            "EOF marker presence indicates tail completeness only and does not imply full BAM validity."
        );
    }

    #[test]
    fn check_eof_rejects_bam_without_canonical_eof_marker() {
        let mut bytes = test_support::build_bam_file();
        bytes.truncate(bytes.len() - BGZF_EOF_MARKER.len());
        let path = test_support::write_temp_file("check-eof-missing", "bam", &bytes);

        let error = run(CheckEofRequest { bam: path.clone() })
            .expect_err("missing EOF marker should fail check_eof");

        fs::remove_file(path).expect("fixture should be removed");
        match error {
            AppError::TruncatedFile { detail, .. } => {
                assert_eq!(detail, "Expected BGZF EOF marker was not found.");
            }
            other => panic!("expected truncated_file error, got {other:?}"),
        }
    }

    #[test]
    fn check_eof_rejects_tail_too_short_for_canonical_eof_marker() {
        let path = test_support::write_temp_file(
            "check-eof-short-tail",
            "truncated.bam",
            &BGZF_EOF_MARKER[..18],
        );

        let error = run(CheckEofRequest { bam: path.clone() })
            .expect_err("short file should fail EOF check");

        fs::remove_file(path).expect("fixture should be removed");
        match error {
            AppError::TruncatedFile { detail, .. } => {
                assert_eq!(
                    detail,
                    "File is smaller than the canonical 28-byte BGZF EOF marker."
                );
            }
            other => panic!("expected truncated_file error, got {other:?}"),
        }
    }

    #[test]
    fn check_eof_rejects_non_bgzf_bam_container() {
        let path = test_support::write_temp_file("check-eof-not-bgzf", "not-bgzf.bam", b"not bgzf");

        let error = run(CheckEofRequest { bam: path.clone() })
            .expect_err("non-BGZF input should fail check_eof");

        fs::remove_file(path).expect("fixture should be removed");
        match error {
            AppError::InvalidBam { detail, .. } => {
                assert_eq!(
                    detail,
                    "Input did not present a BGZF-compatible container header."
                );
            }
            other => panic!("expected invalid_bam error, got {other:?}"),
        }
    }

    #[test]
    fn check_eof_does_not_validate_bam_header_or_alignment_records() {
        let mut bytes = test_support::build_bgzf_member(b"not-bam-header");
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path =
            test_support::write_temp_file("check-eof-invalid-bam-body", "invalid-body.bam", &bytes);

        let response = run(CheckEofRequest { bam: path.clone() })
            .expect("check_eof should only require BGZF EOF evidence");

        fs::remove_file(path).expect("fixture should be removed");
        assert!(response.bgzf_eof_present);
        assert!(response.complete);
    }
}
