use std::path::PathBuf;

use crate::{
    bam::header::{HeaderPayload, parse_bam_header_from_native_bgzf},
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
};

#[derive(Debug)]
pub struct HeaderRequest {
    pub bam: PathBuf,
}

pub type HeaderResponse = HeaderPayload;

pub fn run(request: HeaderRequest) -> Result<HeaderResponse, AppError> {
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

    parse_bam_header_from_native_bgzf(&request.bam)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bgzf::{BGZF_EOF_MARKER, test_support},
        error::AppError,
    };

    use super::{HeaderRequest, run};

    #[test]
    fn header_command_uses_native_bgzf_streaming_header_path() {
        let header_text = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:100\n";
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&(header_text.len() as i32).to_le_bytes());
        payload.extend_from_slice(header_text.as_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&5_i32.to_le_bytes());
        payload.extend_from_slice(b"chr1\0");
        payload.extend_from_slice(&100_i32.to_le_bytes());

        let split_at = 12;
        let mut bam = test_support::build_bgzf_member(&payload[..split_at]);
        bam.extend_from_slice(&test_support::build_bgzf_member(&payload[split_at..]));
        bam.extend_from_slice(&BGZF_EOF_MARKER);
        let path = test_support::write_temp_file("header-native-bgzf", "bam", &bam);

        let response = run(HeaderRequest { bam: path.clone() })
            .expect("header command should parse a multi-member native BGZF stream");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(response.header.raw_header_text, header_text);
        assert_eq!(response.header.references.len(), 1);
        assert_eq!(response.header.references[0].name, "chr1");
        assert!(response.header.reference_diagnostics.is_empty());
    }

    #[test]
    fn header_command_does_not_validate_alignment_body() {
        let header_text = "@HD\tVN:1.6\n";
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&(header_text.len() as i32).to_le_bytes());
        payload.extend_from_slice(header_text.as_bytes());
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(&(-1_i32).to_le_bytes());
        payload.extend_from_slice(b"malformed-body-that-header-must-not-read");

        let mut bam = test_support::build_bgzf_member(&payload);
        bam.extend_from_slice(&BGZF_EOF_MARKER);
        let path = test_support::write_temp_file("header-body-boundary", "bam", &bam);

        let response = run(HeaderRequest { bam: path.clone() })
            .expect("header command should not validate alignment records");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(response.header.raw_header_text, header_text);
        assert!(response.header.references.is_empty());
        assert!(response.header.reference_diagnostics.is_empty());
    }

    #[test]
    fn header_command_surfaces_native_header_parse_failures() {
        let mut bam = test_support::build_bgzf_member(b"not-bam");
        bam.extend_from_slice(&BGZF_EOF_MARKER);
        let path =
            test_support::write_temp_file("header-native-invalid-magic", "invalid.bam", &bam);

        let error = run(HeaderRequest { bam: path.clone() })
            .expect_err("missing BAM magic should be surfaced as a header parse error");

        fs::remove_file(path).expect("fixture should be removed");
        match error {
            AppError::InvalidHeader { detail, .. } => {
                assert_eq!(detail, "Missing BAM magic in decompressed stream.");
            }
            other => panic!("expected invalid_header error, got {other:?}"),
        }
    }
}
