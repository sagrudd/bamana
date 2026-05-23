use std::{fs, path::PathBuf};

use serde::Serialize;

use crate::{
    bam::{
        header::parse_bam_header,
        index::{IndexKind, ResolvedIndex, discover_index_candidates, parse_bai, parse_csi_header},
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct CheckIndexRequest {
    pub bam: PathBuf,
    pub require: bool,
    pub prefer_csi: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckIndexPayload {
    pub format: &'static str,
    pub index: IndexInspection,
    pub candidates: Vec<IndexCandidate>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IndexInspection {
    pub present: bool,
    pub selected_path: Option<String>,
    pub kind: Option<IndexKind>,
    pub usable: bool,
    pub syntactically_valid: Option<bool>,
    pub stale: Option<bool>,
    pub bam_newer_than_index: Option<bool>,
    pub compatibility: IndexCompatibility,
}

#[derive(Debug, Serialize)]
pub struct IndexCandidate {
    pub path: String,
    pub kind: IndexKind,
    pub exists: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexCompatibility {
    Plausible,
    Absent,
    Stale,
    MismatchedOrInvalid,
    DetectedButNotSupported,
}

pub fn run(request: CheckIndexRequest) -> CommandResponse<CheckIndexPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("check_index", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "check_index",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "check_index",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "check_index",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let header = match parse_bam_header(&request.bam) {
        Ok(header) => header,
        Err(error) => {
            return CommandResponse::failure("check_index", Some(request.bam.as_path()), error);
        }
    };
    let bam_references = header.header.references.len();

    let discovered = discover_index_candidates(&request.bam, request.prefer_csi);
    let candidates = discovered
        .iter()
        .map(|candidate| IndexCandidate {
            path: candidate.path.to_string_lossy().into_owned(),
            kind: candidate.kind,
            exists: true,
        })
        .collect::<Vec<_>>();

    let mut notes = Vec::new();

    let Some(selected) = discovered.first() else {
        notes.push("No BAM index was found next to the BAM file.".to_string());
        let payload = CheckIndexPayload {
            format: "BAM",
            index: IndexInspection {
                present: false,
                selected_path: None,
                kind: None,
                usable: false,
                syntactically_valid: None,
                stale: None,
                bam_newer_than_index: None,
                compatibility: IndexCompatibility::Absent,
            },
            candidates,
            notes,
        };

        if request.require {
            return CommandResponse::failure_with_data(
                "check_index",
                Some(request.bam.as_path()),
                Some(payload),
                AppError::MissingIndex {
                    path: request.bam.clone(),
                    detail: None,
                },
            );
        }

        return CommandResponse::success("check_index", Some(request.bam.as_path()), payload);
    };

    let (bam_newer_than_index, stale_note) =
        compare_modification_times(&request.bam, &selected.path);
    if let Some(note) = stale_note {
        notes.push(note);
    }

    let (syntactically_valid, usable, compatibility, validation_note) =
        inspect_selected_index(selected, bam_references);

    if let Some(note) = validation_note {
        notes.push(note);
    }
    if syntactically_valid == Some(true) && usable {
        notes.push("Index presence and BAI structural validation checks passed.".to_string());
    }

    let stale = bam_newer_than_index;
    let compatibility = if stale == Some(true) && compatibility == IndexCompatibility::Plausible {
        IndexCompatibility::Stale
    } else {
        compatibility
    };

    let payload = CheckIndexPayload {
        format: "BAM",
        index: IndexInspection {
            present: true,
            selected_path: Some(selected.path.to_string_lossy().into_owned()),
            kind: Some(selected.kind),
            usable: usable && stale != Some(true),
            syntactically_valid,
            stale,
            bam_newer_than_index,
            compatibility,
        },
        candidates,
        notes,
    };

    if request.require && !payload.index.usable {
        let detail =
            payload.index.selected_path.as_ref().map(|path| {
                format!("Selected index {path} was not usable for fast-path operations.")
            });
        return CommandResponse::failure_with_data(
            "check_index",
            Some(request.bam.as_path()),
            Some(payload),
            AppError::MissingIndex {
                path: request.bam.clone(),
                detail,
            },
        );
    }

    CommandResponse::success("check_index", Some(request.bam.as_path()), payload)
}

fn inspect_selected_index(
    selected: &ResolvedIndex,
    bam_references: usize,
) -> (Option<bool>, bool, IndexCompatibility, Option<String>) {
    match selected.kind {
        IndexKind::Bai => match parse_bai(&selected.path, bam_references) {
            Ok(_) => (Some(true), true, IndexCompatibility::Plausible, None),
            Err(AppError::InvalidIndex { detail, .. }) => (
                Some(false),
                false,
                IndexCompatibility::MismatchedOrInvalid,
                Some(format!("Selected BAI was invalid: {detail}")),
            ),
            Err(error) => (
                Some(false),
                false,
                IndexCompatibility::MismatchedOrInvalid,
                Some(error.to_json_error().message),
            ),
        },
        IndexKind::Csi => match parse_csi_header(&selected.path) {
            Ok(summary) => match usize::try_from(summary.reference_count) {
                Ok(reference_count) if reference_count == bam_references => (
                    Some(true),
                    false,
                    IndexCompatibility::DetectedButNotSupported,
                    Some(format!(
                        "CSI index detected (min_shift={}, depth={}, references={}), but full CSI support is not implemented in this slice.",
                        summary.min_shift, summary.depth, summary.reference_count
                    )),
                ),
                _ => (
                    Some(false),
                    false,
                    IndexCompatibility::MismatchedOrInvalid,
                    Some(format!(
                        "Selected CSI reference count {} does not match BAM header reference count {bam_references}.",
                        summary.reference_count
                    )),
                ),
            },
            Err(AppError::InvalidIndex { detail, .. }) => (
                Some(false),
                false,
                IndexCompatibility::MismatchedOrInvalid,
                Some(format!("Selected CSI was invalid: {detail}")),
            ),
            Err(error) => (
                Some(false),
                false,
                IndexCompatibility::MismatchedOrInvalid,
                Some(error.to_json_error().message),
            ),
        },
        IndexKind::Gzi => (
            Some(false),
            false,
            IndexCompatibility::MismatchedOrInvalid,
            Some("An adjacent FASTQ.GZI sidecar was found, but check_index only inspects BAM BAI/CSI indices.".to_string()),
        ),
        IndexKind::Unknown => (
            Some(false),
            false,
            IndexCompatibility::MismatchedOrInvalid,
            Some("An adjacent index-like file was found, but its magic bytes did not match BAI or CSI.".to_string()),
        ),
    }
}

fn compare_modification_times(
    bam_path: &std::path::Path,
    index_path: &std::path::Path,
) -> (Option<bool>, Option<String>) {
    let bam_modified = fs::metadata(bam_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok());
    let index_modified = fs::metadata(index_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok());

    match (bam_modified, index_modified) {
        (Some(bam), Some(index)) => {
            let bam_newer = bam > index;
            let note = if bam_newer {
                Some("BAM modification time is newer than the selected index; timestamp-based stale detection suggests the index may be outdated.".to_string())
            } else {
                None
            };
            (Some(bam_newer), note)
        }
        _ => (
            None,
            Some("Modification times were unavailable or inconclusive, so stale-index assessment could not be proven from file metadata.".to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, thread, time::Duration};

    use crate::{
        bam::index::test_support::{build_bai_file, build_csi_header},
        bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    use super::{CheckIndexRequest, IndexCompatibility, run};

    #[test]
    fn reports_supported_bai_as_structurally_valid_and_usable() {
        let bam = write_coordinate_bam("check-index-valid-bai");
        let bai = PathBuf::from(format!("{}.bai", bam.to_string_lossy()));
        fs::write(&bai, build_bai_file(&[Some((1, 0))], Some(0))).expect("bai should write");

        let response = run(CheckIndexRequest {
            bam: bam.clone(),
            require: false,
            prefer_csi: false,
        });

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");

        assert!(response.ok);
        let payload = response.data.expect("payload should exist");
        assert!(payload.index.syntactically_valid.unwrap());
        assert!(payload.index.usable);
        assert_eq!(payload.index.compatibility, IndexCompatibility::Plausible);
        assert!(
            payload
                .notes
                .iter()
                .any(|note| note.contains("BAI structural validation"))
        );
    }

    #[test]
    fn reports_stale_bai_as_not_usable() {
        let bam = write_coordinate_bam("check-index-stale-bai");
        let bai = PathBuf::from(format!("{}.bai", bam.to_string_lossy()));
        fs::write(&bai, build_bai_file(&[Some((1, 0))], Some(0))).expect("bai should write");

        thread::sleep(Duration::from_millis(1100));
        fs::OpenOptions::new()
            .append(true)
            .open(&bam)
            .expect("bam should open")
            .set_len(fs::metadata(&bam).expect("metadata").len() + 1)
            .expect("bam mtime should update");

        let response = run(CheckIndexRequest {
            bam: bam.clone(),
            require: false,
            prefer_csi: false,
        });

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");

        assert!(response.ok);
        let payload = response.data.expect("payload should exist");
        assert_eq!(payload.index.stale, Some(true));
        assert!(!payload.index.usable);
        assert_eq!(payload.index.compatibility, IndexCompatibility::Stale);
    }

    #[test]
    fn reports_supported_csi_as_detected_but_not_supported() {
        let bam = write_coordinate_bam("check-index-csi");
        let csi = PathBuf::from(format!("{}.csi", bam.to_string_lossy()));
        fs::write(&csi, build_csi_header(1)).expect("csi should write");

        let response = run(CheckIndexRequest {
            bam: bam.clone(),
            require: false,
            prefer_csi: true,
        });

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(csi).expect("csi should remove");

        assert!(response.ok);
        let payload = response.data.expect("payload should exist");
        assert_eq!(
            payload.index.compatibility,
            IndexCompatibility::DetectedButNotSupported
        );
        assert_eq!(payload.index.syntactically_valid, Some(true));
        assert!(!payload.index.usable);
    }

    #[test]
    fn reports_csi_reference_mismatch_as_invalid() {
        let bam = write_coordinate_bam("check-index-csi-mismatch");
        let csi = PathBuf::from(format!("{}.csi", bam.to_string_lossy()));
        fs::write(&csi, build_csi_header(2)).expect("csi should write");

        let response = run(CheckIndexRequest {
            bam: bam.clone(),
            require: false,
            prefer_csi: true,
        });

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(csi).expect("csi should remove");

        assert!(response.ok);
        let payload = response.data.expect("payload should exist");
        assert_eq!(
            payload.index.compatibility,
            IndexCompatibility::MismatchedOrInvalid
        );
        assert_eq!(payload.index.syntactically_valid, Some(false));
        assert!(!payload.index.usable);
        assert!(
            payload
                .notes
                .iter()
                .any(|note| note.contains("does not match BAM header reference count"))
        );
    }

    #[test]
    fn reports_unknown_adjacent_index_kind_as_invalid() {
        let bam = write_coordinate_bam("check-index-unknown");
        let bai = PathBuf::from(format!("{}.bai", bam.to_string_lossy()));
        fs::write(&bai, b"NOPE").expect("unknown index should write");

        let response = run(CheckIndexRequest {
            bam: bam.clone(),
            require: false,
            prefer_csi: false,
        });

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("unknown index should remove");

        assert!(response.ok);
        let payload = response.data.expect("payload should exist");
        assert_eq!(
            payload.index.compatibility,
            IndexCompatibility::MismatchedOrInvalid
        );
        assert_eq!(payload.index.syntactically_valid, Some(false));
        assert!(!payload.index.usable);
    }

    fn write_coordinate_bam(prefix: &str) -> PathBuf {
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[build_light_record(0, 5, "read1", 0)],
        );
        write_temp_file(prefix, "bam", &bytes)
    }
}
