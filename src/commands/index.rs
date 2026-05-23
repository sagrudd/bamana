use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    bam::{
        header::parse_bam_header,
        index::{IndexKind, build_bai_index_from_bam, default_index_output_path, write_bai_index},
    },
    cli::IndexFormatArg,
    error::AppError,
    fastq::gzi::{DEFAULT_INTERVAL_PERCENT, build_fastq_gzi},
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
    output_safety::{finalize_completed_output, remove_stale_temp},
};

#[derive(Debug)]
pub struct IndexRequest {
    pub input: PathBuf,
    pub out: Option<PathBuf>,
    pub force: bool,
    pub format: Option<IndexFormatArg>,
}

#[derive(Debug, Serialize)]
pub struct IndexCommandPayload {
    pub format: String,
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
    let probe = match probe_path(&request.input) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("index", Some(request.input.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "index",
            Some(request.input.as_path()),
            AppError::UnknownFormat {
                path: request.input.clone(),
            },
        );
    }

    let requested_index_kind = resolve_requested_kind(
        probe.detected_format,
        request.format,
        request.out.as_deref(),
    );
    let output_path = request.out.clone().unwrap_or_else(|| {
        default_output_path(&request.input, probe.detected_format, requested_index_kind)
            .expect("known index kind should yield a default path")
    });
    let output_exists = output_path.exists();

    let mut payload = IndexCommandPayload {
        format: probe.detected_format.to_string(),
        requested_index_kind,
        output_index: CreatedIndexInfo {
            path: output_path.to_string_lossy().into_owned(),
            kind: requested_index_kind,
            created: false,
            overwritten: output_exists && request.force,
        },
        notes: Vec::new(),
    };

    if output_exists && !request.force {
        payload
            .notes
            .push("Index output path was resolved, but overwrite was not permitted.".to_string());
        return CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            AppError::OutputExists { path: output_path },
        );
    }

    match probe.detected_format {
        DetectedFormat::Bam => handle_bam_index(request, probe.container, payload, &output_path),
        DetectedFormat::FastqGz => handle_fastq_gzi_index(request, payload, &output_path),
        other => CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            AppError::UnsupportedFormat {
                path: request.input.clone(),
                format: format!(
                    "Index currently supports BAM and FASTQ.GZ inputs; detected {other}."
                ),
            },
        ),
    }
}

fn handle_bam_index(
    request: IndexRequest,
    container: ContainerKind,
    mut payload: IndexCommandPayload,
    output_path: &Path,
) -> CommandResponse<IndexCommandPayload> {
    if payload.requested_index_kind == IndexKind::Gzi {
        return CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            AppError::UnsupportedIndex {
                path: request.input.clone(),
                detail: "FASTQ.GZI is only valid for FASTQ.GZ inputs.".to_string(),
            },
        );
    }

    if container != ContainerKind::Bgzf {
        return CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            AppError::InvalidBam {
                path: request.input.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let header = match parse_bam_header(&request.input) {
        Ok(header) => header,
        Err(error) => {
            return CommandResponse::failure_with_data(
                "index",
                Some(request.input.as_path()),
                Some(payload),
                error,
            );
        }
    };

    payload
        .notes
        .push("Index command validated the BAM header and resolved the output path.".to_string());

    if payload.requested_index_kind == IndexKind::Csi {
        return CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            AppError::Unimplemented {
                path: request.input.clone(),
                detail: "CSI index creation is not implemented in this slice.".to_string(),
            },
        );
    }

    if payload.requested_index_kind != IndexKind::Bai {
        let requested_kind = payload.requested_index_kind_label().to_string();
        return CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            AppError::UnsupportedIndex {
                path: request.input.clone(),
                detail: format!("{requested_kind} is not valid for BAM input; use BAI."),
            },
        );
    }

    if let Some(sort_order) = header.header.hd.sort_order.as_deref() {
        if sort_order != "coordinate" {
            return CommandResponse::failure_with_data(
                "index",
                Some(request.input.as_path()),
                Some(payload),
                AppError::InvalidIndex {
                    path: request.input.clone(),
                    detail: format!(
                        "BAM header declares SO:{sort_order}; BAI creation requires coordinate-sorted BAM input."
                    ),
                },
            );
        }
    }

    let temp_path = temporary_output_path(output_path);
    remove_stale_temp(&temp_path);

    match build_bai_index_from_bam(&request.input).and_then(|index| {
        write_bai_index(&temp_path, &index)?;
        finalize_completed_output(&temp_path, output_path, request.force)
    }) {
        Ok(()) => {
            payload.output_index.created = true;
            payload.notes.push(
                "BAI index created successfully from native BAM virtual offsets.".to_string(),
            );
            CommandResponse::success("index", Some(request.input.as_path()), payload)
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            CommandResponse::failure_with_data(
                "index",
                Some(request.input.as_path()),
                Some(payload),
                error,
            )
        }
    }
}

fn temporary_output_path(output_path: &Path) -> PathBuf {
    let mut file_name = output_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "index".to_string());
    file_name.push_str(&format!(".bamana-index-{}.tmp", std::process::id()));
    output_path.with_file_name(file_name)
}

fn handle_fastq_gzi_index(
    request: IndexRequest,
    mut payload: IndexCommandPayload,
    output_path: &Path,
) -> CommandResponse<IndexCommandPayload> {
    if payload.requested_index_kind != IndexKind::Gzi {
        let requested_kind = payload.requested_index_kind_label().to_string();
        return CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            AppError::UnsupportedIndex {
                path: request.input.clone(),
                detail: format!("{requested_kind} is not valid for FASTQ.GZ input; use GZI."),
            },
        );
    }

    match build_fastq_gzi(&request.input, output_path) {
        Ok(summary) => {
            payload.output_index.created = true;
            payload
                .notes
                .push("FASTQ.GZI index created successfully.".to_string());
            payload.notes.push(format!(
                "Checkpoint boundaries were sampled at approximately {:.2}% compressed-offset intervals and pinned to completed FASTQ records.",
                DEFAULT_INTERVAL_PERCENT
            ));
            payload.notes.push(format!(
                "The FASTQ.GZI sidecar stores {} checkpoints including start and end anchors.",
                summary.checkpoints.len()
            ));
            CommandResponse::success("index", Some(request.input.as_path()), payload)
        }
        Err(error) => CommandResponse::failure_with_data(
            "index",
            Some(request.input.as_path()),
            Some(payload),
            error,
        ),
    }
}

fn resolve_requested_kind(
    detected_format: DetectedFormat,
    format: Option<IndexFormatArg>,
    out: Option<&Path>,
) -> IndexKind {
    match format {
        Some(IndexFormatArg::Bai) => IndexKind::Bai,
        Some(IndexFormatArg::Csi) => IndexKind::Csi,
        Some(IndexFormatArg::Gzi) => IndexKind::Gzi,
        None => match out
            .map(|path| path.to_string_lossy().to_ascii_lowercase())
            .as_deref()
        {
            Some(path) if path.ends_with(".csi") => IndexKind::Csi,
            Some(path) if path.ends_with(".bai") => IndexKind::Bai,
            Some(path) if path.ends_with(".gzi") => IndexKind::Gzi,
            _ => match detected_format {
                DetectedFormat::FastqGz => IndexKind::Gzi,
                _ => IndexKind::Bai,
            },
        },
    }
}

fn default_output_path(
    input: &Path,
    detected_format: DetectedFormat,
    kind: IndexKind,
) -> Result<PathBuf, AppError> {
    match detected_format {
        DetectedFormat::FastqGz if kind == IndexKind::Gzi => {
            default_index_output_path(input, IndexKind::Gzi)
        }
        _ => default_index_output_path(input, kind),
    }
}

impl IndexCommandPayload {
    fn requested_index_kind_label(&self) -> &'static str {
        match self.requested_index_kind {
            IndexKind::Bai => "BAI",
            IndexKind::Csi => "CSI",
            IndexKind::Gzi => "GZI",
            IndexKind::Unknown => "UNKNOWN",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        path::PathBuf,
    };

    use flate2::{Compression, write::GzEncoder};

    use crate::{
        bam::index::parse_bai,
        bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
        cli::IndexFormatArg,
    };

    use super::{IndexRequest, run};

    #[test]
    fn indexes_fastq_gz_to_default_fastq_gzi_path() {
        let input = std::env::temp_dir().join(format!(
            "bamana-index-fastq-gz-{}.fastq.gz",
            std::process::id()
        ));
        let output = input.with_extension("gzi");
        let file = fs::File::create(&input).expect("fixture should create");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(b"@read1\nACGT\n+\n!!!!\n@read2\nTTAA\n+\n####\n")
            .expect("fixture should write");
        encoder.finish().expect("gzip should finish");

        let response = run(IndexRequest {
            input: input.clone(),
            out: None,
            force: false,
            format: None,
        });

        assert!(response.ok);
        let payload = response.data.expect("payload should exist");
        assert_eq!(payload.format, "FASTQ.GZ");
        assert_eq!(payload.output_index.path, output.to_string_lossy());
        assert!(payload.output_index.created);

        let mut magic = [0_u8; 8];
        fs::File::open(&output)
            .expect("index should exist")
            .read_exact(&mut magic)
            .expect("index should read");

        fs::remove_file(input).expect("fixture should remove");
        fs::remove_file(output).expect("index should remove");

        assert_eq!(&magic, b"FQGZI\0\0\x02");
    }

    #[test]
    fn indexes_coordinate_bam_to_bai_path() {
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[
                build_light_record(0, 5, "read1", 0),
                build_light_record(0, 20_000, "read2", 0),
            ],
        );
        let input = write_temp_file("index-bam-coordinate", "bam", &bytes);
        let output = PathBuf::from(format!("{}.bai", input.to_string_lossy()));

        let response = run(IndexRequest {
            input: input.clone(),
            out: None,
            force: false,
            format: None,
        });

        assert!(response.ok);
        let payload = response.data.expect("payload should exist");
        assert_eq!(payload.format, "BAM");
        assert_eq!(payload.output_index.path, output.to_string_lossy());
        assert!(payload.output_index.created);
        assert!(!payload.output_index.overwritten);

        let summary = parse_bai(&output, 1).expect("written bai should parse");
        assert_eq!(
            summary.reference_summaries[0]
                .as_ref()
                .map(|summary| summary.mapped_reads),
            Some(2)
        );

        fs::remove_file(input).expect("fixture should remove");
        fs::remove_file(output).expect("index should remove");
    }

    #[test]
    fn rejects_bai_creation_for_queryname_header() {
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:queryname\n@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[build_light_record(0, 5, "read1", 0)],
        );
        let input = write_temp_file("index-bam-queryname", "bam", &bytes);
        let output = PathBuf::from(format!("{}.bai", input.to_string_lossy()));

        let response = run(IndexRequest {
            input: input.clone(),
            out: None,
            force: false,
            format: Some(IndexFormatArg::Bai),
        });

        fs::remove_file(input).expect("fixture should remove");

        assert!(!response.ok);
        assert!(!output.exists());
        let error = response.error.expect("error should exist");
        assert_eq!(error.code, "invalid_index");
        assert!(
            error
                .detail
                .is_some_and(|detail| detail.contains("SO:queryname"))
        );
    }

    #[test]
    fn rejects_bam_with_gzi_request() {
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[build_light_record(0, 0, "read1", 0)],
        );
        let input = write_temp_file("index-bam", "bam", &bytes);

        let response = run(IndexRequest {
            input: input.clone(),
            out: None,
            force: false,
            format: Some(crate::cli::IndexFormatArg::Gzi),
        });
        fs::remove_file(input).expect("fixture should remove");

        assert!(!response.ok);
        let error = response.error.expect("error should exist");
        assert_eq!(error.code, "unsupported_index");
    }
}
