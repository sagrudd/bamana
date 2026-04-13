use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    bam::fastq::{FastqExportOptions, export_bam_to_fastq_gz},
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct FastqRequest {
    pub bam: PathBuf,
    pub out: Option<PathBuf>,
    pub threads: usize,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct FastqPayload {
    pub format: &'static str,
    pub output: FastqOutputInfo,
    pub execution: FastqExecutionInfo,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FastqOutputInfo {
    pub path: String,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwritten: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct FastqExecutionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_read: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_written: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads_used: Option<usize>,
}

pub fn run(request: FastqRequest) -> CommandResponse<FastqPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => return CommandResponse::failure("fastq", Some(request.bam.as_path()), error),
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "fastq",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "fastq",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "fastq",
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

    match export_bam_to_fastq_gz(&FastqExportOptions {
        input_path: request.bam.clone(),
        output_path: output_path.clone(),
        threads: request.threads,
        force: request.force,
    }) {
        Ok(execution) => {
            payload.output.written = true;
            payload.output.overwritten = Some(execution.overwritten);
            payload.execution.records_read = Some(execution.records_read);
            payload.execution.records_written = Some(execution.records_written);
            payload.execution.threads_used = Some(execution.threads_used);
            payload.notes.extend(execution.notes);
            CommandResponse::success("fastq", Some(request.bam.as_path()), payload)
        }
        Err(error) => CommandResponse::failure_with_data(
            "fastq",
            Some(request.bam.as_path()),
            Some(payload),
            error,
        ),
    }
}

fn base_payload(output_path: &Path) -> FastqPayload {
    FastqPayload {
        format: "FASTQ.GZ",
        output: FastqOutputInfo {
            path: output_path.to_string_lossy().into_owned(),
            written: false,
            overwritten: None,
        },
        execution: FastqExecutionInfo {
            records_read: None,
            records_written: None,
            threads_used: None,
        },
        notes: Vec::new(),
    }
}

fn default_output_path(input: &Path) -> PathBuf {
    let input_str = input.to_string_lossy();
    if let Some(stripped) = input_str.strip_suffix(".bam") {
        PathBuf::from(format!("{stripped}.fastq.gz"))
    } else {
        PathBuf::from(format!("{input_str}.fastq.gz"))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::default_output_path;

    #[test]
    fn derives_default_fastq_output_path() {
        assert_eq!(
            default_output_path(Path::new("/tmp/example.bam")),
            PathBuf::from("/tmp/example.fastq.gz")
        );
    }
}
