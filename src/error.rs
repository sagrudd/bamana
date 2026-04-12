use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{formats::probe::DetectedFormat, json::JsonError};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("file not found: {path}")]
    FileNotFound { path: PathBuf },
    #[error("permission denied: {path}")]
    PermissionDenied { path: PathBuf },
    #[error("i/o error for {path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("unable to determine file format: {path}")]
    UnknownFormat { path: PathBuf },
    #[error("input is not BAM: {path}")]
    NotBam {
        path: PathBuf,
        detected_format: DetectedFormat,
    },
    #[error("input does not satisfy shallow BAM expectations: {path}")]
    InvalidBam { path: PathBuf, detail: String },
    #[error("bam header could not be parsed: {path}")]
    InvalidHeader { path: PathBuf, detail: String },
    #[error("bam record could not be parsed: {path}")]
    InvalidRecord { path: PathBuf, detail: String },
    #[error("bam index could not be parsed: {path}")]
    InvalidIndex { path: PathBuf, detail: String },
    #[error("index format is not supported: {path}")]
    UnsupportedIndex { path: PathBuf, detail: String },
    #[error("mapping state could not be determined reliably: {path}")]
    ParseUncertainty { path: PathBuf, detail: String },
    #[error("file is truncated or incomplete: {path}")]
    TruncatedFile { path: PathBuf, detail: String },
    #[error("unsupported format for this command: {path}")]
    UnsupportedFormat { path: PathBuf, format: String },
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl AppError {
    pub fn to_json_error(&self) -> JsonError {
        JsonError {
            code: self.code().to_string(),
            message: self.message(),
            detail: self.detail(),
            hint: self.hint(),
        }
    }

    pub fn from_io(path: &Path, error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::FileNotFound {
                path: path.to_path_buf(),
            },
            io::ErrorKind::PermissionDenied => Self::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => Self::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::FileNotFound { .. } => "file_not_found",
            Self::PermissionDenied { .. } => "permission_denied",
            Self::Io { .. } => "io_error",
            Self::UnknownFormat { .. } => "unknown_format",
            Self::NotBam { .. } => "not_bam",
            Self::InvalidBam { .. } => "invalid_bam",
            Self::InvalidHeader { .. } => "invalid_header",
            Self::InvalidRecord { .. } => "invalid_record",
            Self::InvalidIndex { .. } => "invalid_index",
            Self::UnsupportedIndex { .. } => "unsupported_index",
            Self::ParseUncertainty { .. } => "parse_uncertainty",
            Self::TruncatedFile { .. } => "truncated_file",
            Self::UnsupportedFormat { .. } => "unsupported_format",
            Self::Internal { .. } => "internal_error",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::FileNotFound { .. } => "Input file was not found.".to_string(),
            Self::PermissionDenied { .. } => {
                "Permission denied while opening the input file.".to_string()
            }
            Self::Io { .. } => "An I/O error occurred while reading the input file.".to_string(),
            Self::UnknownFormat { .. } => "Unable to determine file format.".to_string(),
            Self::NotBam { .. } => "Input is not a BAM file.".to_string(),
            Self::InvalidBam { .. } => {
                "Input does not satisfy shallow BAM verification.".to_string()
            }
            Self::InvalidHeader { .. } => "BAM header could not be parsed.".to_string(),
            Self::InvalidRecord { .. } => "BAM record could not be parsed.".to_string(),
            Self::InvalidIndex { .. } => "BAM index could not be parsed.".to_string(),
            Self::UnsupportedIndex { .. } => "Index format is not supported.".to_string(),
            Self::ParseUncertainty { .. } => {
                "Mapping state could not be determined reliably from the available evidence."
                    .to_string()
            }
            Self::TruncatedFile { .. } => "Expected BGZF EOF marker was not found.".to_string(),
            Self::UnsupportedFormat { .. } => {
                "Detected format is not supported by this command.".to_string()
            }
            Self::Internal { .. } => "An internal error occurred.".to_string(),
        }
    }

    fn detail(&self) -> Option<String> {
        match self {
            Self::Io { message, .. } => Some(message.clone()),
            Self::NotBam {
                detected_format, ..
            } => Some(format!("Detected format: {detected_format}.")),
            Self::InvalidBam { detail, .. } => Some(detail.clone()),
            Self::InvalidHeader { detail, .. } => Some(detail.clone()),
            Self::InvalidRecord { detail, .. } => Some(detail.clone()),
            Self::InvalidIndex { detail, .. } => Some(detail.clone()),
            Self::UnsupportedIndex { detail, .. } => Some(detail.clone()),
            Self::ParseUncertainty { detail, .. } => Some(detail.clone()),
            Self::TruncatedFile { detail, .. } => Some(detail.clone()),
            Self::UnsupportedFormat { format, .. } => Some(format.clone()),
            Self::Internal { message } => Some(message.clone()),
            _ => None,
        }
    }

    fn hint(&self) -> Option<String> {
        match self {
            Self::FileNotFound { .. } => Some("Check the path and rerun the command.".to_string()),
            Self::PermissionDenied { .. } => {
                Some("Ensure the input file is readable by the current user.".to_string())
            }
            Self::Io { .. } => {
                Some("Retry the operation and confirm the file is readable.".to_string())
            }
            Self::UnknownFormat { .. } => {
                Some("Inspect the file manually or provide a supported input.".to_string())
            }
            Self::NotBam { .. } => Some(
                "Run bamana identify on the input or provide a BGZF-compressed BAM file."
                    .to_string(),
            ),
            Self::InvalidBam { .. } => {
                Some("Confirm the file is BGZF-compressed BAM and rerun bamana verify.".to_string())
            }
            Self::InvalidHeader { .. } => Some(
                "Run bamana verify to perform shallow BAM checks before parsing the header."
                    .to_string(),
            ),
            Self::InvalidRecord { .. } => Some(
                "The BAM stream could not be sampled safely; rerun bamana verify and inspect the file for truncation or corruption."
                    .to_string(),
            ),
            Self::InvalidIndex { .. } => Some(
                "Use scan mode or regenerate the BAM index before retrying this command."
                    .to_string(),
            ),
            Self::UnsupportedIndex { .. } => Some(
                "Provide a BAI index or rerun the command without relying on index-derived mapping counts."
                    .to_string(),
            ),
            Self::ParseUncertainty { .. } => Some(
                "Run bamana verify and, when available, bamana validate.".to_string(),
            ),
            Self::TruncatedFile { .. } => {
                Some("Re-transfer or regenerate the BAM file, then rerun the command.".to_string())
            }
            Self::UnsupportedFormat { .. } => {
                Some("Use a command intended for the detected format.".to_string())
            }
            Self::Internal { .. } => {
                Some("Inspect logs or rerun the command with the same input.".to_string())
            }
        }
    }
}
