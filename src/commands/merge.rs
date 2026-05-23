use std::{collections::HashSet, path::PathBuf};

use serde::Serialize;

use crate::{
    bam::{
        checksum::{
            ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions,
            compute_canonical_digest_for_records, compute_checksums, extract_digest,
        },
        index::IndexKind,
        merge::{MergeExecutionOptions, MergeMode, merge_bams},
        sort::QuerynameSubOrder,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct MergeRequest {
    pub bam: Vec<PathBuf>,
    pub out: PathBuf,
    pub sort: bool,
    pub order: Option<MergeMode>,
    pub queryname_suborder: Option<QuerynameSubOrder>,
    pub create_index: bool,
    pub verify_checksum: bool,
    pub threads: usize,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct MergePayload {
    pub format: &'static str,
    pub inputs: Vec<MergeInputInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<MergeOutputInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge: Option<MergeResultInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<MergeRecordCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<MergeIndexInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_verification: Option<ChecksumVerificationInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MergeInputInfo {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_read: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct MergeOutputInfo {
    pub path: String,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwritten: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MergeResultInfo {
    pub requested_mode: MergeMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_sub_order: Option<QuerynameSubOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produced_mode: Option<MergeMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produced_sub_order: Option<QuerynameSubOrder>,
    pub header_compatibility: HeaderCompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HeaderCompatibility {
    Compatible,
    Incompatible,
}

#[derive(Debug, Serialize)]
pub struct MergeRecordCounts {
    pub records_read: u64,
    pub records_written: u64,
}

#[derive(Debug, Serialize)]
pub struct MergeIndexInfo {
    pub requested: bool,
    pub created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<IndexKind>,
}

#[derive(Debug, Serialize)]
pub struct ChecksumVerificationInfo {
    pub requested: bool,
    pub performed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ChecksumMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<bool>,
}

pub fn run(request: MergeRequest) -> CommandResponse<MergePayload> {
    let requested_mode = match resolve_requested_mode(&request) {
        Ok(mode) => mode,
        Err(error) => {
            return CommandResponse::failure_with_data(
                "merge",
                None,
                Some(base_payload(&request, None)),
                error,
            );
        }
    };

    for input in &request.bam {
        let probe = match probe_path(input) {
            Ok(probe) => probe,
            Err(error) => {
                return CommandResponse::failure_with_data(
                    "merge",
                    None,
                    Some(base_payload(&request, None)),
                    error,
                );
            }
        };

        if probe.detected_format == DetectedFormat::Unknown {
            return CommandResponse::failure_with_data(
                "merge",
                None,
                Some(base_payload(&request, None)),
                AppError::UnknownFormat {
                    path: input.clone(),
                },
            );
        }

        if probe.detected_format != DetectedFormat::Bam {
            return CommandResponse::failure_with_data(
                "merge",
                None,
                Some(base_payload(&request, None)),
                AppError::NotBam {
                    path: input.clone(),
                    detected_format: probe.detected_format,
                },
            );
        }

        if probe.container != ContainerKind::Bgzf {
            return CommandResponse::failure_with_data(
                "merge",
                None,
                Some(base_payload(&request, None)),
                AppError::InvalidBam {
                    path: input.clone(),
                    detail: "Input did not present a BGZF-compatible container header.".to_string(),
                },
            );
        }
    }

    let mut payload = base_payload(&request, Some(requested_mode));

    let merge_result = match merge_bams(&MergeExecutionOptions {
        input_paths: request.bam.clone(),
        output_path: request.out.clone(),
        force: request.force,
        mode: requested_mode,
        queryname_suborder: request.queryname_suborder,
        threads: request.threads,
    }) {
        Ok(result) => result,
        Err(error) => {
            if matches!(error, AppError::IncompatibleHeaders { .. }) {
                if let Some(merge) = payload.merge.as_mut() {
                    merge.header_compatibility = HeaderCompatibility::Incompatible;
                }
            }
            return CommandResponse::failure_with_data(
                "merge",
                None,
                Some(payload),
                map_transform_error(error, request.bam.first()),
            );
        }
    };

    for (input, records_read) in payload
        .inputs
        .iter_mut()
        .zip(&merge_result.per_input_records_read)
    {
        input.records_read = Some(*records_read);
    }
    payload.output = Some(MergeOutputInfo {
        path: request.out.to_string_lossy().into_owned(),
        written: false,
        overwritten: None,
    });
    if let Some(merge) = payload.merge.as_mut() {
        merge.produced_mode = Some(merge_result.produced_mode);
        merge.produced_sub_order = merge_result.produced_sub_order;
    }
    payload.records = Some(MergeRecordCounts {
        records_read: merge_result.per_input_records_read.iter().sum(),
        records_written: merge_result.records_written,
    });
    payload.notes.extend(merge_result.notes);
    payload.index = Some(MergeIndexInfo {
        requested: request.create_index,
        created: false,
        kind: None,
    });
    update_index_reporting(&request, requested_mode, &mut payload);

    if request.verify_checksum {
        match verify_merge_checksum(&merge_result.records_for_checksum, &request.out) {
            Ok((input_digest, output_digest, matched)) => {
                payload.checksum_verification = Some(ChecksumVerificationInfo {
                    requested: true,
                    performed: true,
                    mode: Some(ChecksumMode::CanonicalRecordOrder),
                    input_digest: Some(input_digest.clone()),
                    output_digest: Some(output_digest.clone()),
                    r#match: Some(matched),
                });
                if matched {
                    payload.notes.push(
                        "Canonical multiset checksum verification confirmed content preservation across the merge."
                            .to_string(),
                    );
                } else {
                    return CommandResponse::failure_with_data(
                        "merge",
                        None,
                        Some(payload),
                        AppError::ChecksumMismatch {
                            path: request.out.clone(),
                            detail: "Input multiset checksum differs from output checksum."
                                .to_string(),
                        },
                    );
                }
            }
            Err(error) => {
                return CommandResponse::failure_with_data("merge", None, Some(payload), error);
            }
        }
    }

    if let Some(output) = payload.output.as_mut() {
        output.written = true;
        output.overwritten = Some(merge_result.overwritten);
    }
    CommandResponse::success("merge", None, payload)
}

fn base_payload(request: &MergeRequest, requested_mode: Option<MergeMode>) -> MergePayload {
    MergePayload {
        format: "BAM",
        inputs: request
            .bam
            .iter()
            .map(|path| MergeInputInfo {
                path: path.to_string_lossy().into_owned(),
                records_read: None,
            })
            .collect(),
        output: None,
        merge: requested_mode.map(|mode| MergeResultInfo {
            requested_mode: mode,
            requested_sub_order: request.queryname_suborder,
            produced_mode: None,
            produced_sub_order: None,
            header_compatibility: HeaderCompatibility::Compatible,
        }),
        records: None,
        index: Some(MergeIndexInfo {
            requested: request.create_index,
            created: false,
            kind: None,
        }),
        checksum_verification: Some(ChecksumVerificationInfo {
            requested: request.verify_checksum,
            performed: false,
            mode: None,
            input_digest: None,
            output_digest: None,
            r#match: None,
        }),
        notes: Vec::new(),
    }
}

fn resolve_requested_mode(request: &MergeRequest) -> Result<MergeMode, AppError> {
    match (request.sort, request.order) {
        (true, Some(MergeMode::Input | MergeMode::Queryname)) => Err(AppError::InvalidMergeRequest {
            path: request.out.clone(),
            detail: "--sort is shorthand for --order coordinate and cannot be combined with a conflicting --order value."
                .to_string(),
        }),
        (true, Some(MergeMode::Coordinate)) | (true, None) => Ok(MergeMode::Coordinate),
        (false, Some(mode)) => Ok(mode),
        (false, None) => Ok(MergeMode::Input),
    }
}

fn update_index_reporting(request: &MergeRequest, mode: MergeMode, payload: &mut MergePayload) {
    let Some(index) = payload.index.as_mut() else {
        return;
    };
    if !request.create_index {
        return;
    }

    match mode {
        MergeMode::Coordinate => {
            index.kind = Some(IndexKind::Bai);
            payload.notes.push(
                "Index creation was requested, but BAI writing is not implemented in this slice."
                    .to_string(),
            );
        }
        MergeMode::Input | MergeMode::Queryname => {
            payload.notes.push(
                "Input-order and queryname merge outputs are not suitable for standard coordinate BAM indexing."
                    .to_string(),
            );
        }
    }
}

fn verify_merge_checksum(
    input_records: &[crate::bam::records::RecordLayout],
    output: &std::path::Path,
) -> Result<(String, String, bool), AppError> {
    let input_digest = compute_canonical_digest_for_records(
        input_records,
        ChecksumFilters {
            only_primary: false,
            mapped_only: false,
        },
        &HashSet::new(),
    )
    .map_err(|detail| AppError::ChecksumUncertainty {
        path: output.to_path_buf(),
        detail,
    })?;

    let options = ChecksumOptions {
        mode: ChecksumMode::CanonicalRecordOrder,
        algorithm: ChecksumAlgorithm::Sha256,
        include_header: false,
        excluded_tags: HashSet::new(),
        excluded_tag_strings: Vec::new(),
        filters: ChecksumFilters {
            only_primary: false,
            mapped_only: false,
        },
    };
    let output_digest = extract_digest(
        compute_checksums(output, &options)?,
        ChecksumMode::CanonicalRecordOrder,
    )
    .ok_or_else(|| AppError::ChecksumUncertainty {
        path: output.to_path_buf(),
        detail: "Canonical checksum result was missing from the merged BAM checksum response."
            .to_string(),
    })?;

    Ok((
        input_digest.clone(),
        output_digest.clone(),
        input_digest == output_digest,
    ))
}

fn map_transform_error(error: AppError, input: Option<&PathBuf>) -> AppError {
    match error {
        AppError::InvalidHeader { detail, .. }
        | AppError::InvalidRecord { detail, .. }
        | AppError::TruncatedFile { detail, .. } => AppError::ParseUncertainty {
            path: input.cloned().unwrap_or_default(),
            detail,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{HeaderCompatibility, MergeRequest, run};
    use crate::{
        bam::{index::IndexKind, merge::MergeMode, sort::QuerynameSubOrder},
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    #[test]
    fn merge_reports_checksum_verification_and_index_deferral() {
        let input_a = write_temp_file(
            "merge-command-a",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 3, "read2", 0)],
            ),
        );
        let input_b = write_temp_file(
            "merge-command-b",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "read1", 0)],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-merge-command-output-{}.bam",
            std::process::id()
        ));

        let response = run(MergeRequest {
            bam: vec![input_a.clone(), input_b.clone()],
            out: output.clone(),
            sort: false,
            order: Some(MergeMode::Coordinate),
            queryname_suborder: None,
            create_index: true,
            verify_checksum: true,
            threads: 1,
            force: true,
        });

        assert!(response.ok);
        let payload = response.data.expect("success payload should exist");
        assert!(payload.output.expect("output should be reported").written);
        let merge = payload.merge.expect("merge result should be reported");
        assert_eq!(merge.produced_mode, Some(MergeMode::Coordinate));
        assert_eq!(merge.header_compatibility, HeaderCompatibility::Compatible);
        let records = payload.records.expect("record counts should be reported");
        assert_eq!(records.records_read, 2);
        assert_eq!(records.records_written, 2);
        let index = payload.index.expect("index payload should be present");
        assert!(index.requested);
        assert!(!index.created);
        assert_eq!(index.kind, Some(IndexKind::Bai));
        let checksum = payload
            .checksum_verification
            .expect("checksum verification should be reported");
        assert!(checksum.requested);
        assert!(checksum.performed);
        assert_eq!(checksum.r#match, Some(true));
        assert!(
            payload
                .notes
                .iter()
                .any(|note| note.contains("BAI writing is not implemented"))
        );
        assert!(
            payload
                .notes
                .iter()
                .any(|note| note.contains("Canonical multiset checksum verification"))
        );

        fs::remove_file(input_a).expect("fixture should be removable");
        fs::remove_file(input_b).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn queryname_merge_reports_coordinate_index_unsuitable() {
        let input_a = write_temp_file(
            "merge-command-queryname-a",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 2, "b", 0)],
            ),
        );
        let input_b = write_temp_file(
            "merge-command-queryname-b",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "a", 0)],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-merge-command-queryname-output-{}.bam",
            std::process::id()
        ));

        let response = run(MergeRequest {
            bam: vec![input_a.clone(), input_b.clone()],
            out: output.clone(),
            sort: false,
            order: Some(MergeMode::Queryname),
            queryname_suborder: Some(QuerynameSubOrder::Lexicographical),
            create_index: true,
            verify_checksum: false,
            threads: 1,
            force: true,
        });

        assert!(response.ok);
        let payload = response.data.expect("success payload should exist");
        let merge = payload.merge.expect("merge result should be reported");
        assert_eq!(merge.produced_mode, Some(MergeMode::Queryname));
        assert_eq!(
            merge.produced_sub_order,
            Some(QuerynameSubOrder::Lexicographical)
        );
        let index = payload.index.expect("index payload should be present");
        assert!(index.requested);
        assert!(!index.created);
        assert!(index.kind.is_none());
        assert!(
            payload
                .notes
                .iter()
                .any(|note| note.contains("not suitable for standard coordinate BAM indexing"))
        );

        fs::remove_file(input_a).expect("fixture should be removable");
        fs::remove_file(input_b).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }
}
