use std::path::Path;

use crate::error::AppError;

pub(crate) fn verify_input_sha256(
    path: &Path,
    expected: Option<&str>,
    observed: Option<&str>,
) -> Result<(), AppError> {
    let Some(expected) = expected else {
        return Ok(());
    };
    if observed == Some(expected) {
        return Ok(());
    }
    Err(AppError::ChecksumMismatch {
        path: path.to_path_buf(),
        detail: format!(
            "Input raw SHA-256 {} did not match expected {expected}.",
            observed.unwrap_or("<unavailable>")
        ),
    })
}
