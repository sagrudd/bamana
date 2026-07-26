use std::{
    fs,
    path::{Path, PathBuf},
};

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

pub(crate) fn output_matches_input(input: &Path, output: &Path) -> bool {
    if input == output {
        return true;
    }
    let input_canonical = fs::canonicalize(input).ok();
    let output_canonical = fs::canonicalize(output).ok();
    input_canonical.is_some() && input_canonical == output_canonical
}

pub(crate) fn temporary_output_path(output: &Path) -> PathBuf {
    let stem = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-sort-output");
    output.with_file_name(format!(".{stem}.bamana-sort-{}.tmp", std::process::id()))
}
