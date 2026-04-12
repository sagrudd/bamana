use std::path::PathBuf;

use serde::Serialize;

use crate::{
    bam::{
        header::parse_bam_header,
        index::{IndexKind, default_index_output_path},
    },
    cli::IndexFormatArg,
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct IndexRequest {
    pub bam: PathBuf,
    pub out: Option<PathBuf>,
    pub force: bool,
    pub format: Option<IndexFormatArg>,
}

#[derive(Debug, Serialize)]
pub struct IndexCommandPayload {
    pub format: &'static str,
    pub requested_index_kind: IndexKind,
    pub output_index: CreatedIndexInfo,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatedIndexInfo {
    pub path: String,
    pub kind: IndexKind,
    pub created: bool,
    pub overwritten: bool,
}

pub fn run(request: IndexRequest) -> CommandResponse<IndexCommandPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => return CommandResponse::failure("index", Some(request.bam.as_path()), error),
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "index",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "index",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "index",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    if let Err(error) = parse_bam_header(&request.bam) {
        return CommandResponse::failure("index", Some(request.bam.as_path()), error);
    }

    let requested_index_kind = resolve_requested_kind(request.format, request.out.as_deref());
    let output_path = request.out.clone().unwrap_or_else(|| {
        default_index_output_path(&request.bam, requested_index_kind)
            .expect("known index kind should yield a default path")
    });
    let output_exists = output_path.exists();

    let payload = IndexCommandPayload {
        format: "BAM",
        requested_index_kind,
        output_index: CreatedIndexInfo {
            path: output_path.to_string_lossy().into_owned(),
            kind: requested_index_kind,
            created: false,
            overwritten: output_exists && request.force,
        },
        notes: vec![
            "Index command validated the BAM header and resolved the output path.".to_string(),
            "Actual BAM index writing is not implemented in this slice; this command currently establishes the index-creation contract and output selection path.".to_string(),
        ],
    };

    if output_exists && !request.force {
        return CommandResponse::failure_with_data(
            "index",
            Some(request.bam.as_path()),
            Some(payload),
            AppError::OutputExists { path: output_path },
        );
    }

    let detail = match requested_index_kind {
        IndexKind::Bai => "BAI index creation is not implemented in this slice.".to_string(),
        IndexKind::Csi => "CSI index creation is not implemented in this slice.".to_string(),
        IndexKind::Unknown => {
            "Unknown index creation is not implemented in this slice.".to_string()
        }
    };

    CommandResponse::failure_with_data(
        "index",
        Some(request.bam.as_path()),
        Some(payload),
        AppError::Unimplemented {
            path: request.bam.clone(),
            detail,
        },
    )
}

fn resolve_requested_kind(
    format: Option<IndexFormatArg>,
    out: Option<&std::path::Path>,
) -> IndexKind {
    match format {
        Some(IndexFormatArg::Bai) => IndexKind::Bai,
        Some(IndexFormatArg::Csi) => IndexKind::Csi,
        None => match out
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("csi") => IndexKind::Csi,
            Some("bai") => IndexKind::Bai,
            _ => IndexKind::Bai,
        },
    }
}
