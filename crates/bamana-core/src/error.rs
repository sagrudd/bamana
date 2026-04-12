use std::{io, path::Path, path::PathBuf};

use thiserror::Error;

use crate::formats::probe::DetectedFormat;

#[derive(Debug, Error)]
pub enum BamanaError {
    #[error("file not found: {path}")]
    FileNotFound { path: PathBuf },
    #[error("permission denied: {path}")]
    PermissionDenied { path: PathBuf },
    #[error("I/O error for {path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("input is not BAM: {path}")]
    NotBam {
        path: PathBuf,
        detected_format: DetectedFormat,
    },
    #[error("invalid BAM: {path}")]
    InvalidBam { path: PathBuf, detail: String },
    #[error("truncated file: {path}")]
    TruncatedFile { path: PathBuf, detail: String },
    #[error("unknown format: {path}")]
    UnknownFormat { path: PathBuf },
    #[error("unsupported format: {path}")]
    UnsupportedFormat { path: PathBuf, format: String },
    #[error("unimplemented feature: {feature}")]
    Unimplemented { feature: &'static str },
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl BamanaError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::FileNotFound { .. } => "file_not_found",
            Self::PermissionDenied { .. } => "permission_denied",
            Self::Io { .. } => "io_error",
            Self::NotBam { .. } => "not_bam",
            Self::InvalidBam { .. } => "invalid_bam",
            Self::TruncatedFile { .. } => "truncated_file",
            Self::UnknownFormat { .. } => "unknown_format",
            Self::UnsupportedFormat { .. } => "unsupported_format",
            Self::Unimplemented { .. } => "unimplemented",
            Self::Internal { .. } => "internal_error",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::FileNotFound { .. } => "Input file was not found.".to_string(),
            Self::PermissionDenied { .. } => {
                "Permission denied while opening the input file.".to_string()
            }
            Self::Io { .. } => "An I/O error occurred while reading the input file.".to_string(),
            Self::NotBam {
                detected_format, ..
            } => {
                format!("Input is not a BAM file. Detected format: {detected_format}.")
            }
            Self::InvalidBam { .. } => {
                "Input does not satisfy shallow BAM expectations.".to_string()
            }
            Self::TruncatedFile { .. } => "Expected BGZF EOF marker was not found.".to_string(),
            Self::UnknownFormat { .. } => {
                "File format could not be determined confidently.".to_string()
            }
            Self::UnsupportedFormat { format, .. } => {
                format!("Format {format} is not supported by this command.")
            }
            Self::Unimplemented { feature } => {
                format!("Feature is not implemented yet: {feature}.")
            }
            Self::Internal { .. } => "An internal error occurred.".to_string(),
        }
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Io { message, .. } => Some(message.clone()),
            Self::InvalidBam { detail, .. } => Some(detail.clone()),
            Self::TruncatedFile { detail, .. } => Some(detail.clone()),
            Self::UnsupportedFormat { format, .. } => Some(format.clone()),
            Self::Internal { message } => Some(message.clone()),
            _ => None,
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::FileNotFound { .. } => Some("Check the path and rerun the command.".to_string()),
            Self::PermissionDenied { .. } => {
                Some("Ensure the input file is readable by the current user.".to_string())
            }
            Self::Io { .. } => Some(
                "Retry the operation and confirm the file is readable and complete.".to_string(),
            ),
            Self::NotBam { .. } => {
                Some("Run bamana identify on the file to confirm its format.".to_string())
            }
            Self::InvalidBam { .. } => Some(
                "Re-run bamana verify after confirming the file is a valid BGZF-compressed BAM."
                    .to_string(),
            ),
            Self::TruncatedFile { .. } => {
                Some("Re-transfer or regenerate the BAM file, then run bamana verify.".to_string())
            }
            Self::UnknownFormat { .. } => Some(
                "Provide a supported file type or inspect the file contents directly.".to_string(),
            ),
            Self::UnsupportedFormat { .. } => {
                Some("Use a command intended for the detected file format.".to_string())
            }
            Self::Unimplemented { .. } => None,
            Self::Internal { .. } => {
                Some("Capture the failing input and inspect the logs before retrying.".to_string())
            }
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
}
