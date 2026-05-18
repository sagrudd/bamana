use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{docs_dir, read_utf8, repo_root};

const ALLOWED_PRODUCTION_NOODLES_FILES: &[&str] = &["src/ingest/cram.rs"];

#[test]
fn production_noodles_usage_stays_inside_cram_compatibility_boundary() {
    let src_dir = repo_root().join("src");
    let mut violations = Vec::new();

    for path in rust_sources(&src_dir) {
        let relative = path
            .strip_prefix(repo_root())
            .unwrap_or_else(|error| panic!("{} is not under repo root: {error}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");
        let allowed = ALLOWED_PRODUCTION_NOODLES_FILES.contains(&relative.as_str());

        for (line_number, line) in read_utf8(&path).lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                continue;
            }

            if contains_direct_noodles_reference(trimmed) && !allowed {
                violations.push(format!("{relative}:{}: {line}", line_number + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "direct production noodles usage is only allowed in {:?}; violations:\n{}",
        ALLOWED_PRODUCTION_NOODLES_FILES,
        violations.join("\n")
    );
}

#[test]
fn native_header_and_verify_paths_do_not_import_noodles() {
    let protected_paths = [
        "src/bam/header.rs",
        "src/bam/reader.rs",
        "src/bgzf/reader.rs",
        "src/commands/header.rs",
        "src/commands/verify.rs",
    ];
    let mut violations = Vec::new();

    for relative in protected_paths {
        let path = repo_root().join(relative);
        for (line_number, line) in read_utf8(&path).lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                continue;
            }

            if contains_direct_noodles_reference(trimmed) {
                violations.push(format!("{relative}:{}: {line}", line_number + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "native BAM header and verify paths must stay noodles-free:\n{}",
        violations.join("\n")
    );
}

#[test]
fn cram_compatibility_exception_is_documented_with_removal_criteria() {
    let policy = read_utf8(&docs_dir().join("dependency-policy.md"));
    let demotion = read_utf8(&docs_dir().join("migration").join("noodles-demotion.md"));

    for required in [
        "src/ingest/cram.rs",
        "strictly within CRAM ingestion",
        "Removal criteria",
        "native CRAM",
    ] {
        assert!(
            policy.contains(required),
            "dependency policy is missing CRAM exception language: {required}"
        );
    }

    for required in [
        "src/ingest/cram.rs",
        "Removal criteria",
        "must not expand",
        "tests/contract/dependency_boundary.rs",
    ] {
        assert!(
            demotion.contains(required),
            "noodles demotion plan is missing guardrail language: {required}"
        );
    }
}

#[test]
fn header_oracle_surface_is_documented_as_test_only() {
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));

    for required in [
        "tests/header_oracle.rs",
        "test-only oracle",
        "production header and verify paths",
        "Malformed-header failure expectations",
    ] {
        assert!(
            oracle_policy.contains(required),
            "testing oracle policy is missing header-oracle boundary language: {required}"
        );
    }
}

fn contains_direct_noodles_reference(line: &str) -> bool {
    line.contains("noodles_") || line.contains("noodles::")
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_sources(dir, &mut files);
    files.sort();
    files
}

fn collect_rust_sources(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read directory {}: {error}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to read directory entry in {}: {error}",
                dir.display()
            )
        });
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}
