use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    bam::unmap::{UnmapExecutionOptions, unmap_bam},
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct UnmapRequest {
    pub bam: PathBuf,
    pub out: Option<PathBuf>,
    pub dry_run: bool,
    pub threads: usize,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct UnmapPayload {
    pub format: &'static str,
    pub output: UnmapOutputInfo,
    pub execution: UnmapExecutionInfo,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UnmapOutputInfo {
    pub path: String,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwritten: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct UnmapExecutionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_read: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_written: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping_tags_removed: Option<u64>,
}

pub fn run(request: UnmapRequest) -> CommandResponse<UnmapPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => return CommandResponse::failure("unmap", Some(request.bam.as_path()), error),
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "unmap",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "unmap",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "unmap",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let output_path = request
        .out
        .clone()
        .unwrap_or_else(|| default_output_path(&request.bam));
    let mut payload = base_payload(&output_path);

    match unmap_bam(&UnmapExecutionOptions {
        input_path: request.bam.clone(),
        output_path: output_path.clone(),
        dry_run: request.dry_run,
        threads: request.threads,
        force: request.force,
    }) {
        Ok(execution) => {
            payload.output.written = !request.dry_run;
            payload.output.overwritten = Some(execution.overwritten);
            payload.execution.records_read = Some(execution.records_read);
            payload.execution.records_written = Some(execution.records_written);
            payload.execution.mapping_tags_removed = Some(execution.tags_removed);
            payload.notes.extend(execution.notes);
            CommandResponse::success("unmap", Some(request.bam.as_path()), payload)
        }
        Err(error) => CommandResponse::failure_with_data(
            "unmap",
            Some(request.bam.as_path()),
            Some(payload),
            error,
        ),
    }
}

fn base_payload(output_path: &Path) -> UnmapPayload {
    UnmapPayload {
        format: "BAM",
        output: UnmapOutputInfo {
            path: output_path.to_string_lossy().into_owned(),
            written: false,
            overwritten: None,
        },
        execution: UnmapExecutionInfo {
            records_read: None,
            records_written: None,
            mapping_tags_removed: None,
        },
        notes: Vec::new(),
    }
}

fn default_output_path(input: &Path) -> PathBuf {
    let input_str = input.to_string_lossy();
    if let Some(stripped) = input_str.strip_suffix(".bam") {
        PathBuf::from(format!("{stripped}.unmapped.bam"))
    } else {
        PathBuf::from(format!("{input_str}.unmapped.bam"))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, path::PathBuf};

    use crate::formats::bgzf::test_support::{
        build_bam_file_with_header_and_records, write_temp_file,
    };

    use super::{UnmapRequest, default_output_path, run};

    #[test]
    fn derives_default_output_path() {
        let path = Path::new("/tmp/example.bam");
        assert_eq!(
            default_output_path(path),
            PathBuf::from("/tmp/example.unmapped.bam")
        );
    }

    #[test]
    fn dry_run_reports_without_writing() {
        let input = write_temp_file(
            "unmap-dry-run",
            "bam",
            &build_bam_file_with_header_and_records("@SQ\tSN:chr1\tLN:10\n", &[("chr1", 10)], &[]),
        );
        let output =
            std::env::temp_dir().join(format!("bamana-unmap-dry-run-{}.bam", std::process::id()));

        let response = run(UnmapRequest {
            bam: input.clone(),
            out: Some(output.clone()),
            dry_run: true,
            threads: 1,
            force: true,
        });

        assert!(response.ok);
        assert!(response.data.is_some());
        let payload = response.data.expect("payload should exist");
        assert!(!payload.output.written);
        assert!(!output.exists());

        fs::remove_file(input).expect("fixture should be removable");
    }
}
