use std::path::PathBuf;

use serde::Serialize;

use crate::{
    bam::header::parse_bam_header_from_native_bgzf,
    error::AppError,
    formats::probe::{Confidence, ContainerKind, DetectedFormat, probe_path},
};

#[derive(Debug)]
pub struct VerifyRequest {
    pub bam: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub detected_format: DetectedFormat,
    pub container: ContainerKind,
    pub is_bam: bool,
    pub shallow_verified: bool,
    pub deep_validated: bool,
    pub confidence: Confidence,
    pub checks_performed: Vec<&'static str>,
    pub semantic_note: String,
}

pub fn run(request: VerifyRequest) -> Result<VerifyResponse, AppError> {
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

    parse_bam_header_from_native_bgzf(&request.bam)?;

    Ok(VerifyResponse {
        detected_format: DetectedFormat::Bam,
        container: ContainerKind::Bgzf,
        is_bam: true,
        shallow_verified: true,
        deep_validated: false,
        confidence: probe.confidence,
        checks_performed: vec![
            "opened_input",
            "confirmed_bgzf_container_header",
            "confirmed_bam_magic",
            "parsed_bam_header",
            "parsed_binary_reference_dictionary",
        ],
        semantic_note: "Header-level verification confirms a BGZF container, BAM magic, and native BAM header parsing. It does not imply alignment-record validation, full BAM body validation, or EOF completeness.".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bgzf::{BGZF_EOF_MARKER, test_support},
        error::AppError,
    };

    use super::{VerifyRequest, run};

    #[test]
    fn verify_accepts_valid_native_bgzf_bam_header() {
        let bytes = test_support::build_bam_file_with_header(
            "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:100\n",
            &[("chr1", 100)],
        );
        let path = test_support::write_temp_file("verify-valid", "bam", &bytes);

        let response = run(VerifyRequest { bam: path.clone() }).expect("valid BAM should verify");

        fs::remove_file(path).expect("fixture should be removed");
        assert!(response.is_bam);
        assert!(response.shallow_verified);
        assert!(!response.deep_validated);
        assert_eq!(
            response.checks_performed,
            vec![
                "opened_input",
                "confirmed_bgzf_container_header",
                "confirmed_bam_magic",
                "parsed_bam_header",
                "parsed_binary_reference_dictionary",
            ]
        );
    }

    #[test]
    fn verify_rejects_non_bgzf_bam_container() {
        let path = test_support::write_temp_file("verify-not-bgzf", "invalid.bam", b"not bgzf");

        let error = run(VerifyRequest { bam: path.clone() }).expect_err("non-BGZF BAM should fail");

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
    fn verify_rejects_bgzf_without_bam_magic_as_header_error() {
        let mut bytes = test_support::build_bgzf_member(b"NOPE");
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = test_support::write_temp_file("verify-no-magic", "invalid.bam", &bytes);

        let error =
            run(VerifyRequest { bam: path.clone() }).expect_err("missing BAM magic should fail");

        fs::remove_file(path).expect("fixture should be removed");
        match error {
            AppError::InvalidHeader { detail, .. } => {
                assert_eq!(detail, "Missing BAM magic in decompressed stream.");
            }
            other => panic!("expected invalid_header error, got {other:?}"),
        }
    }

    #[test]
    fn verify_rejects_negative_header_text_length() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&(-1_i32).to_le_bytes());
        let path = write_bam_payload("verify-negative-l-text", &payload);

        let error =
            run(VerifyRequest { bam: path.clone() }).expect_err("negative l_text should fail");

        fs::remove_file(path).expect("fixture should be removed");
        match error {
            AppError::InvalidHeader { detail, .. } => {
                assert_eq!(detail, "BAM header text length was negative.");
            }
            other => panic!("expected invalid_header error, got {other:?}"),
        }
    }

    #[test]
    fn verify_rejects_truncated_reference_dictionary() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&5_i32.to_le_bytes());
        payload.extend_from_slice(b"chr");
        let path = write_bam_payload("verify-truncated-reference", &payload);

        let error =
            run(VerifyRequest { bam: path.clone() }).expect_err("truncated reference should fail");

        fs::remove_file(path).expect("fixture should be removed");
        match error {
            AppError::TruncatedFile { detail, .. } => {
                assert_eq!(detail, "BAM stream ended while reading reference name.");
            }
            other => panic!("expected truncated_file error, got {other:?}"),
        }
    }

    fn write_bam_payload(name: &str, payload: &[u8]) -> std::path::PathBuf {
        let mut bytes = test_support::build_bgzf_member(payload);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        test_support::write_temp_file(name, "bam", &bytes)
    }
}
