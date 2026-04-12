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
        notes.push("Index presence and shallow structure checks passed.".to_string());
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
            Ok(summary) => (
                Some(true),
                false,
                IndexCompatibility::DetectedButNotSupported,
                Some(format!(
                    "CSI index detected (min_shift={}, depth={}, references={}), but full CSI support is not implemented in this slice.",
                    summary.min_shift, summary.depth, summary.reference_count
                )),
            ),
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
