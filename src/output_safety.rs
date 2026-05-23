use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::AppError;

pub(crate) fn remove_stale_temp(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

pub(crate) fn cleanup_temp_outputs(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

pub(crate) fn finalize_completed_output(
    temp_path: &Path,
    output_path: &Path,
    force: bool,
) -> Result<(), AppError> {
    if !temp_path.exists() {
        return Err(AppError::WriteError {
            path: temp_path.to_path_buf(),
            message: "Temporary output was not present at finalization time.".to_string(),
        });
    }
    if output_path.exists() && !force {
        let _ = fs::remove_file(temp_path);
        return Err(AppError::OutputExists {
            path: output_path.to_path_buf(),
        });
    }

    fs::rename(temp_path, output_path).map_err(|error| {
        let _ = fs::remove_file(temp_path);
        AppError::WriteError {
            path: output_path.to_path_buf(),
            message: error.to_string(),
        }
    })
}

pub(crate) fn finalize_completed_outputs(
    temp_paths: &[PathBuf],
    output_paths: &[PathBuf],
    force: bool,
) -> Result<(), AppError> {
    for temp_path in temp_paths {
        if !temp_path.exists() {
            cleanup_temp_outputs(temp_paths);
            return Err(AppError::WriteError {
                path: temp_path.clone(),
                message: "Temporary output was not present at finalization time.".to_string(),
            });
        }
    }
    if !force {
        for output_path in output_paths {
            if output_path.exists() {
                cleanup_temp_outputs(temp_paths);
                return Err(AppError::OutputExists {
                    path: output_path.clone(),
                });
            }
        }
    }

    for (index, (temp_path, output_path)) in temp_paths.iter().zip(output_paths.iter()).enumerate()
    {
        if let Err(error) = fs::rename(temp_path, output_path) {
            cleanup_temp_outputs(&temp_paths[index..]);
            return Err(AppError::WriteError {
                path: output_path.clone(),
                message: error.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{finalize_completed_output, finalize_completed_outputs};

    #[test]
    fn finalize_preserves_existing_output_without_force_and_cleans_temp() {
        let dir = std::env::temp_dir().join(format!(
            "bamana-output-safety-no-force-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("test directory should create");
        let output = dir.join("out.bam");
        let temp = dir.join(".out.bam.tmp");
        fs::write(&output, b"sentinel").expect("sentinel should write");
        fs::write(&temp, b"candidate").expect("temp should write");

        let error = finalize_completed_output(&temp, &output, false)
            .expect_err("existing output should fail without force");

        assert_eq!(error.to_json_error().code, "output_exists");
        assert_eq!(
            fs::read(&output).expect("sentinel should remain"),
            b"sentinel"
        );
        assert!(!temp.exists());

        fs::remove_file(output).expect("sentinel should remove");
        fs::remove_dir(dir).expect("test directory should remove");
    }

    #[test]
    fn finalize_replaces_existing_output_with_force_in_one_step() {
        let dir =
            std::env::temp_dir().join(format!("bamana-output-safety-force-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("test directory should create");
        let output = dir.join("out.bam");
        let temp = dir.join(".out.bam.tmp");
        fs::write(&output, b"sentinel").expect("sentinel should write");
        fs::write(&temp, b"candidate").expect("temp should write");

        finalize_completed_output(&temp, &output, true).expect("force finalize should replace");

        assert_eq!(
            fs::read(&output).expect("candidate should be final"),
            b"candidate"
        );
        assert!(!temp.exists());

        fs::remove_file(output).expect("output should remove");
        fs::remove_dir(dir).expect("test directory should remove");
    }

    #[test]
    fn multi_finalize_preflights_all_temps_before_publishing_any_output() {
        let dir =
            std::env::temp_dir().join(format!("bamana-output-safety-multi-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("test directory should create");
        let temp_a = dir.join(".a.tmp");
        let temp_b = dir.join(".b.tmp");
        let out_a = dir.join("a.bam");
        let out_b = dir.join("b.bam");
        fs::write(&temp_a, b"a").expect("first temp should write");

        let error = finalize_completed_outputs(
            &[temp_a.clone(), temp_b.clone()],
            &[out_a.clone(), out_b.clone()],
            true,
        )
        .expect_err("missing second temp should fail before publishing");

        assert_eq!(error.to_json_error().code, "write_error");
        assert!(!out_a.exists());
        assert!(!out_b.exists());
        assert!(!temp_a.exists());

        fs::remove_dir(dir).expect("test directory should remove");
    }
}
