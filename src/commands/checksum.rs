use std::{collections::HashSet, path::PathBuf};

use crate::{
    bam::checksum::{
        ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions, ChecksumPayload,
        compute_checksums,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct ChecksumRequest {
    pub bam: PathBuf,
    pub mode: ChecksumMode,
    pub algorithm: ChecksumAlgorithm,
    pub include_header: bool,
    pub exclude_tags: Vec<String>,
    pub only_primary: bool,
    pub mapped_only: bool,
}

pub fn run(request: ChecksumRequest) -> CommandResponse<ChecksumPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("checksum", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "checksum",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "checksum",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "checksum",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let mut excluded_tags = HashSet::new();
    let mut excluded_tag_strings = Vec::new();
    for tag in &request.exclude_tags {
        let Some(validated) = crate::bam::tags::validate_tag(tag) else {
            return CommandResponse::failure(
                "checksum",
                Some(request.bam.as_path()),
                AppError::InvalidTag {
                    path: request.bam.clone(),
                    tag: tag.clone(),
                },
            );
        };
        excluded_tags.insert(validated);
        excluded_tag_strings.push(tag.clone());
    }

    let options = ChecksumOptions {
        mode: request.mode,
        algorithm: request.algorithm,
        include_header: request.include_header,
        excluded_tags,
        excluded_tag_strings,
        filters: ChecksumFilters {
            only_primary: request.only_primary,
            mapped_only: request.mapped_only,
        },
    };

    match compute_checksums(&request.bam, &options) {
        Ok(payload) => CommandResponse::success("checksum", Some(request.bam.as_path()), payload),
        Err(AppError::ChecksumUncertainty { detail, .. }) => {
            let payload = ChecksumPayload {
                format: "BAM",
                algorithm: None,
                results: None,
                semantic_note: None,
            };
            CommandResponse::failure_with_data(
                "checksum",
                Some(request.bam.as_path()),
                Some(payload),
                AppError::ChecksumUncertainty {
                    path: request.bam.clone(),
                    detail,
                },
            )
        }
        Err(error) => CommandResponse::failure("checksum", Some(request.bam.as_path()), error),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{ChecksumRequest, run};
    use crate::{
        bam::checksum::{ChecksumAlgorithm, ChecksumMode},
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    #[test]
    fn checksum_command_reports_domains_filters_and_excluded_tags() {
        let input = write_temp_file(
            "checksum-command-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(0, 1, "mapped", 0),
                    build_light_record(-1, -1, "unmapped", 0x4),
                ],
            ),
        );

        let response = run(ChecksumRequest {
            bam: input.clone(),
            mode: ChecksumMode::All,
            algorithm: ChecksumAlgorithm::Sha256,
            include_header: true,
            exclude_tags: vec!["NM".to_string()],
            only_primary: true,
            mapped_only: true,
        });

        assert!(response.ok);
        let payload = response.data.expect("payload should be present");
        assert_eq!(payload.algorithm, Some(ChecksumAlgorithm::Sha256));
        let results = payload.results.expect("results should be present");
        assert_eq!(results.len(), 4);
        assert!(
            results
                .iter()
                .any(|result| result.mode == ChecksumMode::Header)
        );
        let raw = results
            .iter()
            .find(|result| result.mode == ChecksumMode::RawRecordOrder)
            .expect("raw record-order result should be present");
        assert_eq!(raw.records_hashed, 1);
        assert!(raw.filters.only_primary);
        assert!(raw.filters.mapped_only);
        assert_eq!(raw.excluded_tags, vec!["NM"]);
        let payload_result = results
            .iter()
            .find(|result| result.mode == ChecksumMode::Payload)
            .expect("payload result should be present");
        assert!(payload_result.header_included);
        assert!(
            payload
                .semantic_note
                .as_deref()
                .is_some_and(|note| note.contains("order-insensitive"))
        );

        fs::remove_file(input).expect("fixture should be removable");
    }
}
