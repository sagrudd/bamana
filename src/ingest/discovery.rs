use std::{
    collections::BTreeMap,
    fs,
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
    pub candidate_files: Vec<PathBuf>,
    pub discovered_files: Vec<DiscoveredFile>,
    pub skipped_entries: Vec<SkippedEntry>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
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
    let mut directories_scanned = 0_usize;
    let mut candidate_files = Vec::new();
    let mut skipped_entries = Vec::new();

    for path in requested_paths {
        let metadata =
            fs::symlink_metadata(path).map_err(|error| AppError::from_io(path, error))?;
        if metadata.is_file() {
            candidate_files.push(path.clone());
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

    candidate_files.sort_by_cached_key(|path| path.to_string_lossy().into_owned());

    let mut discovered_files = Vec::with_capacity(candidate_files.len());
    for path in &candidate_files {
        let probe = probe_path(path)?;
        discovered_files.push(DiscoveredFile {
            path: path.clone(),
            detected_format: probe.detected_format,
        });
    }

    Ok(DiscoveryResult {
        directories_scanned,
        candidate_files,
        discovered_files,
        skipped_entries,
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
    candidate_files: &mut Vec<PathBuf>,
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
            candidate_files.push(path);
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
