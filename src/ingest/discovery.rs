use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    error::AppError,
    formats::probe::{DetectedFormat, probe_path},
};

#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    pub recursive: bool,
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub directories_scanned: usize,
    pub discovered_files: Vec<DiscoveredFile>,
    pub skipped_entries: Vec<SkippedEntry>,
    pub staged_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub logical_path: PathBuf,
    pub detected_format: DetectedFormat,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedEntry {
    pub path: String,
    pub detected_format: &'static str,
    pub consumed: bool,
    pub reason: String,
}

pub fn discover_requested_paths(
    requested_paths: &[PathBuf],
    options: &DiscoveryOptions,
) -> Result<DiscoveryResult, AppError> {
    discover_requested_paths_with_reader(requested_paths, options, std::io::stdin().lock())
}

fn discover_requested_paths_with_reader(
    requested_paths: &[PathBuf],
    options: &DiscoveryOptions,
    mut stdin_reader: impl Read,
) -> Result<DiscoveryResult, AppError> {
    let mut directories_scanned = 0_usize;
    let mut candidate_files = Vec::new();
    let mut skipped_entries = Vec::new();
    let mut staged_paths = Vec::new();
    let mut stdin_consumed = false;

    for path in requested_paths {
        if is_stdin_path(path) {
            if stdin_consumed {
                return Err(AppError::InvalidConsumeRequest {
                    path: path.clone(),
                    detail: "STDIN may be requested at most once in a single consume invocation."
                        .to_string(),
                });
            }

            let staged_path = materialize_stream_input(&mut stdin_reader, path)?;
            candidate_files.push((staged_path.clone(), path.clone()));
            staged_paths.push(staged_path);
            stdin_consumed = true;
            continue;
        }

        let metadata =
            fs::symlink_metadata(path).map_err(|error| AppError::from_io(path, error))?;
        if metadata.is_file() {
            if is_cram_index_sidecar(path) {
                return Err(AppError::UnsupportedFormat {
                    path: path.clone(),
                    format: "CRAI sidecars are unsupported/deferred in Milestone 13 and are not valid consume inputs."
                        .to_string(),
                });
            }
            candidate_files.push((path.clone(), path.clone()));
            continue;
        }

        if metadata.is_dir() {
            directories_scanned += 1;
            collect_directory_files(
                path,
                options.recursive,
                &mut directories_scanned,
                &mut candidate_files,
                &mut skipped_entries,
            )
            .map_err(|error| AppError::from_io(path, error))?;
            continue;
        }

        return Err(AppError::UnsupportedDirectoryEntry {
            path: path.clone(),
            detail:
                "Only regular files and directories are supported consume inputs in this slice."
                    .to_string(),
        });
    }

    let mut discovered_files = Vec::with_capacity(candidate_files.len());
    for (path, logical_path) in &candidate_files {
        let probe = probe_path(path)?;
        discovered_files.push(DiscoveredFile {
            path: path.clone(),
            logical_path: logical_path.clone(),
            detected_format: probe.detected_format,
        });
    }

    Ok(DiscoveryResult {
        directories_scanned,
        discovered_files,
        skipped_entries,
        staged_paths,
    })
}

pub fn format_counts(files: &[DiscoveredFile]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for file in files {
        *counts.entry(file.detected_format.to_string()).or_insert(0) += 1;
    }
    counts
}

fn collect_directory_files(
    directory: &Path,
    recursive: bool,
    directories_scanned: &mut usize,
    candidate_files: &mut Vec<(PathBuf, PathBuf)>,
    skipped_entries: &mut Vec<SkippedEntry>,
) -> std::io::Result<()> {
    let mut entries = fs::read_dir(directory)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by_cached_key(|path| path.to_string_lossy().into_owned());

    for path in entries {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            skipped_entries.push(SkippedEntry {
                path: path.to_string_lossy().into_owned(),
                detected_format: "UNSUPPORTED",
                consumed: false,
                reason: "symlink_not_followed".to_string(),
            });
            continue;
        }

        if metadata.is_file() {
            if is_cram_index_sidecar(&path) {
                skipped_entries.push(SkippedEntry {
                    path: path.to_string_lossy().into_owned(),
                    detected_format: "UNSUPPORTED",
                    consumed: false,
                    reason: "cram_index_sidecar_deferred".to_string(),
                });
                continue;
            }
            candidate_files.push((path.clone(), path));
            continue;
        }

        if metadata.is_dir() {
            if recursive {
                *directories_scanned += 1;
                collect_directory_files(
                    &path,
                    recursive,
                    directories_scanned,
                    candidate_files,
                    skipped_entries,
                )?;
            } else {
                skipped_entries.push(SkippedEntry {
                    path: path.to_string_lossy().into_owned(),
                    detected_format: "UNSUPPORTED",
                    consumed: false,
                    reason: "directory_requires_recursive".to_string(),
                });
            }
            continue;
        }

        skipped_entries.push(SkippedEntry {
            path: path.to_string_lossy().into_owned(),
            detected_format: "UNSUPPORTED",
            consumed: false,
            reason: "unsupported_directory_entry".to_string(),
        });
    }

    Ok(())
}

fn is_stdin_path(path: &Path) -> bool {
    path == Path::new("-")
}

fn is_cram_index_sidecar(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("crai"))
}

fn materialize_stream_input(
    reader: &mut impl Read,
    logical_path: &Path,
) -> Result<PathBuf, AppError> {
    let staged_path = temporary_stdin_path();
    let mut file = fs::File::create(&staged_path).map_err(|error| AppError::WriteError {
        path: logical_path.to_path_buf(),
        message: error.to_string(),
    })?;

    std::io::copy(reader, &mut file).map_err(|error| AppError::Io {
        path: logical_path.to_path_buf(),
        message: error.to_string(),
    })?;
    file.flush().map_err(|error| AppError::WriteError {
        path: logical_path.to_path_buf(),
        message: error.to_string(),
    })?;

    Ok(staged_path)
}

fn temporary_stdin_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        ".bamana-consume-stdin-{}-{nanos}.tmp",
        std::process::id()
    ))
}

pub fn cleanup_staged_paths(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor, path::PathBuf};

    use super::{DiscoveryOptions, cleanup_staged_paths, discover_requested_paths_with_reader};
    use crate::formats::probe::DetectedFormat;

    #[test]
    fn discovers_sam_from_stdin_sentinel() {
        let requested = vec![PathBuf::from("-")];
        let discovery = discover_requested_paths_with_reader(
            &requested,
            &DiscoveryOptions { recursive: false },
            Cursor::new(
                b"@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:10\nread1\t4\t*\t0\t0\t*\t*\t0\t0\tAC\t!!\n",
            ),
        )
        .expect("stdin discovery should succeed");

        assert_eq!(discovery.discovered_files.len(), 1);
        assert_eq!(
            discovery.discovered_files[0].logical_path,
            PathBuf::from("-")
        );
        assert_eq!(
            discovery.discovered_files[0].detected_format,
            DetectedFormat::Sam
        );

        let staged = discovery.staged_paths.clone();
        cleanup_staged_paths(&staged);
        assert!(staged.iter().all(|path| !path.exists()));
    }

    #[test]
    fn rejects_duplicate_stdin_requests() {
        let requested = vec![PathBuf::from("-"), PathBuf::from("-")];
        let error = discover_requested_paths_with_reader(
            &requested,
            &DiscoveryOptions { recursive: false },
            Cursor::new(Vec::<u8>::new()),
        )
        .expect_err("duplicate stdin should fail");

        assert_eq!(error.to_json_error().code, "invalid_consume_mode");
    }

    #[test]
    fn directory_discovery_skips_crai_sidecars_without_probing() {
        let root = std::env::temp_dir().join(format!(
            "bamana-discovery-crai-sidecar-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("fixture dir should create");
        let cram = root.join("sample.cram");
        let crai = root.join("sample.cram.crai");
        fs::write(&cram, b"CRAM").expect("cram hint should write");
        fs::write(&crai, b"not parsed as an index").expect("crai sidecar should write");

        let discovery = discover_requested_paths_with_reader(
            &[root.clone()],
            &DiscoveryOptions { recursive: false },
            Cursor::new(Vec::<u8>::new()),
        )
        .expect("directory discovery should skip CRAI sidecars");

        assert_eq!(discovery.discovered_files.len(), 1);
        assert_eq!(discovery.discovered_files[0].path, cram);
        assert_eq!(discovery.skipped_entries.len(), 1);
        assert_eq!(
            discovery.skipped_entries[0].reason,
            "cram_index_sidecar_deferred"
        );
        assert_eq!(
            discovery.skipped_entries[0].path,
            crai.to_string_lossy().into_owned()
        );

        fs::remove_file(discovery.discovered_files[0].path.clone()).expect("cram should remove");
        fs::remove_file(crai).expect("crai sidecar should remove");
        fs::remove_dir(root).expect("fixture dir should remove");
    }

    #[test]
    fn direct_crai_request_is_rejected_before_probe() {
        let path =
            std::env::temp_dir().join(format!("bamana-direct-crai-{}.crai", std::process::id()));
        fs::write(&path, b"not parsed as an index").expect("crai sidecar should write");

        let error = discover_requested_paths_with_reader(
            std::slice::from_ref(&path),
            &DiscoveryOptions { recursive: false },
            Cursor::new(Vec::<u8>::new()),
        )
        .expect_err("direct CRAI input should be rejected");

        let json_error = error.to_json_error();
        assert_eq!(json_error.code, "unsupported_format");
        assert!(
            json_error
                .detail
                .as_deref()
                .unwrap_or_default()
                .contains("CRAI sidecars are unsupported/deferred")
        );

        fs::remove_file(path).expect("crai sidecar should remove");
    }
}
