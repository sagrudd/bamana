use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BamanaError {
    #[error("input path does not exist: {path}")]
    InputNotFound { path: PathBuf },
}
