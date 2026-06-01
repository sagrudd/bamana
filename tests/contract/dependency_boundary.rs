use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{docs_dir, read_utf8, repo_root};

const ALLOWED_PRODUCTION_NOODLES_FILES: &[&str] = &["src/ingest/cram.rs"];
const BENCHMARK_ONLY_EXTERNAL_TOOLS: &[&str] =
    &["samtools", "fastcat", "sambamba", "seqtk", "rasusa"];
const ALLOWED_PRODUCTION_BENCHMARK_TOOL_REFERENCE_FILES: &[&str] = &["src/commands/benchmark.rs"];

const M5_PROOF_COMMAND_HOT_PATHS: &[(&str, &[&str])] = &[
    (
        "verify",
        &[
            "src/commands/verify.rs",
            "src/bam/header.rs",
            "src/bgzf/reader.rs",
        ],
    ),
    (
        "header",
        &[
            "src/commands/header.rs",
            "src/bam/header.rs",
            "src/bgzf/reader.rs",
        ],
    ),
    (
        "subsample",
        &[
            "src/commands/subsample.rs",
            "src/bam/header.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
            "src/bam/write.rs",
            "src/fastq/record.rs",
            "src/fastq/reader.rs",
            "src/fastq/writer.rs",
        ],
    ),
];

const M6_INSPECTION_COMMAND_HOT_PATHS: &[(&str, &[&str])] = &[
    (
        "check_eof",
        &["src/commands/check_eof.rs", "src/bgzf/reader.rs"],
    ),
    (
        "check_sort",
        &[
            "src/commands/check_sort.rs",
            "src/bam/header.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
        ],
    ),
    (
        "check_map",
        &[
            "src/commands/check_map.rs",
            "src/bam/index.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
        ],
    ),
    (
        "summary",
        &[
            "src/commands/summary.rs",
            "src/bam/index.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
            "src/bam/summary.rs",
        ],
    ),
    (
        "check_tag",
        &[
            "src/commands/check_tag.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
            "src/bam/tags.rs",
        ],
    ),
    (
        "validate",
        &[
            "src/commands/validate.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
            "src/bam/tags.rs",
            "src/bam/validate.rs",
        ],
    ),
];

const M7_MUTATION_FORENSICS_HOT_PATHS: &[(&str, &[&str])] = &[
    (
        "reheader",
        &[
            "src/commands/reheader.rs",
            "src/bam/reheader.rs",
            "src/bam/header.rs",
            "src/bam/records.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
        ],
    ),
    (
        "annotate_rg",
        &[
            "src/commands/annotate_rg.rs",
            "src/bam/annotate_rg.rs",
            "src/bam/header.rs",
            "src/bam/records.rs",
            "src/bam/tags.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
        ],
    ),
    (
        "inspect_duplication",
        &[
            "src/commands/inspect_duplication.rs",
            "src/forensics/duplication.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
            "src/bam/tags.rs",
            "src/fastq/reader.rs",
        ],
    ),
    (
        "deduplicate",
        &[
            "src/commands/deduplicate.rs",
            "src/forensics/deduplicate.rs",
            "src/forensics/duplication.rs",
            "src/bam/header.rs",
            "src/bam/record.rs",
            "src/bam/records.rs",
            "src/bam/scan.rs",
            "src/bam/tags.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
            "src/fastq/reader.rs",
            "src/fastq/writer.rs",
        ],
    ),
    (
        "forensic_inspect",
        &[
            "src/commands/forensic_inspect.rs",
            "src/forensics/forensic_inspect.rs",
            "src/forensics/duplication.rs",
            "src/bam/header.rs",
            "src/bam/record.rs",
            "src/bam/scan.rs",
            "src/bam/tags.rs",
        ],
    ),
];

const M8_TRANSFORM_INGEST_HOT_PATHS: &[(&str, &[&str])] = &[
    (
        "sort",
        &[
            "src/commands/sort.rs",
            "src/bam/sort.rs",
            "src/bam/scan.rs",
            "src/bam/records.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
        ],
    ),
    (
        "merge",
        &[
            "src/commands/merge.rs",
            "src/bam/merge.rs",
            "src/bam/scan.rs",
            "src/bam/records.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
        ],
    ),
    (
        "explode",
        &[
            "src/commands/explode.rs",
            "src/bam/scan.rs",
            "src/bam/records.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
            "src/fastq/gzi.rs",
            "src/fastq/reader.rs",
            "src/fastq/writer.rs",
            "src/ingest/sam.rs",
        ],
    ),
    (
        "checksum",
        &[
            "src/commands/checksum.rs",
            "src/bam/checksum.rs",
            "src/bam/header.rs",
            "src/bam/scan.rs",
            "src/bam/tags.rs",
        ],
    ),
    (
        "consume",
        &[
            "src/commands/consume.rs",
            "src/ingest/consume.rs",
            "src/ingest/discovery.rs",
            "src/ingest/sam.rs",
            "src/bam/scan.rs",
            "src/bam/records.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
            "src/fastq/gzi.rs",
            "src/fastq/reader.rs",
            "src/fastq/writer.rs",
        ],
    ),
];

const M9_INDEX_RANDOM_ACCESS_HOT_PATHS: &[(&str, &[&str])] = &[
    (
        "index",
        &[
            "src/commands/index.rs",
            "src/bam/index.rs",
            "src/bam/scan.rs",
            "src/bgzf/reader.rs",
            "src/bgzf/virtual_offset.rs",
            "src/output_safety.rs",
        ],
    ),
    (
        "check_index",
        &[
            "src/commands/check_index.rs",
            "src/bam/index.rs",
            "src/bam/header.rs",
            "src/bgzf/reader.rs",
        ],
    ),
    (
        "check_map_indexed",
        &[
            "src/commands/check_map.rs",
            "src/bam/index.rs",
            "src/bam/scan.rs",
            "src/bam/record.rs",
        ],
    ),
    (
        "summary_indexed",
        &[
            "src/commands/summary.rs",
            "src/bam/index.rs",
            "src/bam/scan.rs",
            "src/bam/summary.rs",
        ],
    ),
    (
        "random_access_substrate",
        &[
            "src/bgzf/reader.rs",
            "src/bgzf/virtual_offset.rs",
            "src/bam/scan.rs",
            "src/bam/index.rs",
        ],
    ),
];

const M10_INDEXED_REGION_HOT_PATHS: &[(&str, &[&str])] = &[
    ("indexed_region_parser", &["src/bam/region.rs"]),
    (
        "indexed_region_chunk_planning",
        &[
            "src/bam/region.rs",
            "src/bam/region_plan.rs",
            "src/bam/index.rs",
            "src/bgzf/virtual_offset.rs",
        ],
    ),
    (
        "indexed_region_random_access_traversal",
        &[
            "src/bam/region.rs",
            "src/bam/region_plan.rs",
            "src/bam/region_traversal.rs",
            "src/bam/scan.rs",
            "src/bgzf/reader.rs",
            "src/bgzf/virtual_offset.rs",
        ],
    ),
    (
        "check_map_region_indexed",
        &[
            "src/commands/check_map.rs",
            "src/bam/region.rs",
            "src/bam/region_plan.rs",
            "src/bam/region_traversal.rs",
            "src/bam/index.rs",
            "src/bam/scan.rs",
            "src/bgzf/reader.rs",
            "src/bgzf/virtual_offset.rs",
        ],
    ),
    (
        "summary_region_indexed",
        &[
            "src/commands/summary.rs",
            "src/bam/region.rs",
            "src/bam/region_plan.rs",
            "src/bam/region_traversal.rs",
            "src/bam/index.rs",
            "src/bam/scan.rs",
            "src/bam/summary.rs",
            "src/bgzf/reader.rs",
            "src/bgzf/virtual_offset.rs",
        ],
    ),
    (
        "indexed_region_scan_fallback",
        &[
            "src/commands/check_map.rs",
            "src/commands/summary.rs",
            "src/bam/region.rs",
            "src/bam/scan.rs",
            "src/bam/record.rs",
        ],
    ),
];

const M11_SELECTED_REGION_OUTPUT_HOT_PATHS: &[(&str, &[&str])] = &[
    (
        "select_region_scan_fallback_output",
        &[
            "src/commands/select_region.rs",
            "src/bam/region.rs",
            "src/bam/scan.rs",
            "src/bam/record.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
            "src/output_safety.rs",
        ],
    ),
    (
        "select_region_indexed_output",
        &[
            "src/commands/select_region.rs",
            "src/bam/region.rs",
            "src/bam/region_plan.rs",
            "src/bam/region_traversal.rs",
            "src/bam/index.rs",
            "src/bam/scan.rs",
            "src/bam/record.rs",
            "src/bam/write.rs",
            "src/bgzf/reader.rs",
            "src/bgzf/virtual_offset.rs",
            "src/bgzf/writer.rs",
            "src/output_safety.rs",
        ],
    ),
    (
        "select_region_header_and_index_invalidation",
        &[
            "src/commands/select_region.rs",
            "src/bam/header.rs",
            "src/bam/write.rs",
            "src/output_safety.rs",
        ],
    ),
];

const M12_INDEX_COMPATIBILITY_HOT_PATHS: &[(&str, &[&str])] = &[
    (
        "check_index_support_levels",
        &[
            "src/commands/check_index.rs",
            "src/bam/index.rs",
            "src/bam/header.rs",
        ],
    ),
    (
        "check_map_index_diagnostics",
        &[
            "src/commands/check_map.rs",
            "src/bam/index.rs",
            "src/bam/region.rs",
            "src/bam/scan.rs",
        ],
    ),
    (
        "summary_index_derived_compatibility",
        &[
            "src/commands/summary.rs",
            "src/bam/index.rs",
            "src/bam/region.rs",
            "src/bam/summary.rs",
            "src/bam/scan.rs",
        ],
    ),
    (
        "select_region_input_index_compatibility",
        &[
            "src/commands/select_region.rs",
            "src/bam/index.rs",
            "src/bam/region.rs",
            "src/bam/scan.rs",
            "src/bam/write.rs",
            "src/bgzf/writer.rs",
        ],
    ),
];

const M13_CRAM_GUARDRAIL_PATHS: &[(&str, &[&str])] = &[
    (
        "cram_consume_boundary",
        &[
            "src/commands/consume.rs",
            "src/ingest/consume.rs",
            "src/ingest/discovery.rs",
            "src/ingest/cram.rs",
        ],
    ),
    (
        "non_cram_indexed_surfaces",
        &[
            "src/commands/check_map.rs",
            "src/commands/summary.rs",
            "src/commands/select_region.rs",
            "src/commands/check_index.rs",
            "src/commands/index.rs",
            "src/bam/index.rs",
            "src/bam/region.rs",
            "src/bam/region_plan.rs",
            "src/bam/region_traversal.rs",
        ],
    ),
    (
        "cram_benchmark_guardrail",
        &["src/bin/scanner_microbench.rs"],
    ),
];

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
fn m5_proof_command_hot_paths_do_not_import_noodles() {
    let proof_commands: Vec<_> = M5_PROOF_COMMAND_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        proof_commands,
        ["verify", "header", "subsample"],
        "M5 proof-command dependency boundary must explicitly name verify, header, and subsample"
    );

    let mut violations = Vec::new();

    for (command, paths) in M5_PROOF_COMMAND_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M5 proof-command hot paths must stay noodles-free:\n{}",
        violations.join("\n")
    );
}

#[test]
fn scanner_and_migrated_hot_paths_do_not_import_noodles() {
    let protected_paths = [
        "src/bam/record.rs",
        "src/bam/scan.rs",
        "src/bam/tags.rs",
        "src/bam/summary.rs",
        "src/bam/validate.rs",
        "src/commands/check_map.rs",
        "src/commands/check_sort.rs",
        "src/commands/check_tag.rs",
        "src/commands/summary.rs",
        "src/forensics/duplication.rs",
        "src/forensics/forensic_inspect.rs",
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
        "native scanner and migrated BAM record hot paths must stay noodles-free:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m6_inspection_command_hot_paths_do_not_import_noodles() {
    let inspection_commands: Vec<_> = M6_INSPECTION_COMMAND_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        inspection_commands,
        [
            "check_eof",
            "check_sort",
            "check_map",
            "summary",
            "check_tag",
            "validate"
        ],
        "M6 dependency boundary must explicitly name the six inspection and validation commands"
    );

    let mut violations = Vec::new();

    for (command, paths) in M6_INSPECTION_COMMAND_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M6 inspection and validation hot paths must stay noodles-free:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m7_mutation_forensics_hot_paths_do_not_import_noodles() {
    let mutation_forensics_commands: Vec<_> = M7_MUTATION_FORENSICS_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        mutation_forensics_commands,
        [
            "reheader",
            "annotate_rg",
            "inspect_duplication",
            "deduplicate",
            "forensic_inspect"
        ],
        "M7 dependency boundary must explicitly name the five mutation, remediation, and forensics commands"
    );

    let mut violations = Vec::new();

    for (command, paths) in M7_MUTATION_FORENSICS_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M7 mutation, remediation, and forensics hot paths must stay noodles-free:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m8_transform_ingest_hot_paths_do_not_import_noodles() {
    let transform_ingest_commands: Vec<_> = M8_TRANSFORM_INGEST_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        transform_ingest_commands,
        ["sort", "merge", "explode", "checksum", "consume"],
        "M8 dependency boundary must explicitly name the five transform, checksum, explode, and ingest commands"
    );

    let mut violations = Vec::new();

    for (command, paths) in M8_TRANSFORM_INGEST_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M8 transform, checksum, explode, and ingest hot paths must stay noodles-free outside documented CRAM compatibility:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m9_index_random_access_hot_paths_do_not_import_noodles() {
    let index_random_access_paths: Vec<_> = M9_INDEX_RANDOM_ACCESS_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        index_random_access_paths,
        [
            "index",
            "check_index",
            "check_map_indexed",
            "summary_indexed",
            "random_access_substrate"
        ],
        "M9 dependency boundary must explicitly name index, check_index, indexed check_map, indexed summary, and the random-access substrate"
    );

    let mut violations = Vec::new();

    for (command, paths) in M9_INDEX_RANDOM_ACCESS_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M9 BAM index, indexed-consumer, and random-access substrate paths must stay noodles-free outside documented CRAM compatibility:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m10_indexed_region_hot_paths_do_not_import_noodles() {
    let indexed_region_paths: Vec<_> = M10_INDEXED_REGION_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        indexed_region_paths,
        [
            "indexed_region_parser",
            "indexed_region_chunk_planning",
            "indexed_region_random_access_traversal",
            "check_map_region_indexed",
            "summary_region_indexed",
            "indexed_region_scan_fallback"
        ],
        "M10 dependency boundary must explicitly name indexed-region parsing, planning, random-access traversal, command wiring, and scan fallback"
    );

    let mut violations = Vec::new();

    for (command, paths) in M10_INDEXED_REGION_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M10 indexed-region planning and traversal paths must stay noodles-free outside documented CRAM compatibility:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m11_selected_region_output_hot_paths_do_not_import_noodles() {
    let selected_region_paths: Vec<_> = M11_SELECTED_REGION_OUTPUT_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        selected_region_paths,
        [
            "select_region_scan_fallback_output",
            "select_region_indexed_output",
            "select_region_header_and_index_invalidation"
        ],
        "M11 dependency boundary must explicitly name selected-region scan fallback, indexed output, and header/index-invalidation paths"
    );

    let mut violations = Vec::new();

    for (command, paths) in M11_SELECTED_REGION_OUTPUT_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M11 selected-region output hot paths must stay noodles-free outside documented CRAM compatibility:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m12_index_compatibility_hot_paths_do_not_import_noodles() {
    let index_compatibility_paths: Vec<_> = M12_INDEX_COMPATIBILITY_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        index_compatibility_paths,
        [
            "check_index_support_levels",
            "check_map_index_diagnostics",
            "summary_index_derived_compatibility",
            "select_region_input_index_compatibility"
        ],
        "M12 dependency boundary must explicitly name support levels, diagnostics, summary index-derived evidence, and selected-region input-index compatibility"
    );

    let mut violations = Vec::new();

    for (command, paths) in M12_INDEX_COMPATIBILITY_HOT_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) {
                    violations.push(format!("{command}: {relative}:{}: {line}", line_number + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M12 index compatibility hot paths must stay noodles-free outside documented CRAM compatibility:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m13_cram_guardrail_paths_keep_noodles_confined_to_cram_ingest() {
    let guardrail_paths: Vec<_> = M13_CRAM_GUARDRAIL_PATHS
        .iter()
        .map(|(command, _paths)| *command)
        .collect();
    assert_eq!(
        guardrail_paths,
        [
            "cram_consume_boundary",
            "non_cram_indexed_surfaces",
            "cram_benchmark_guardrail"
        ],
        "M13 dependency boundary must explicitly name CRAM consume, non-CRAM indexed surfaces, and benchmark guardrails"
    );

    let mut violations = Vec::new();

    for (guardrail, paths) in M13_CRAM_GUARDRAIL_PATHS {
        for relative in *paths {
            let path = repo_root().join(relative);
            let allowed = ALLOWED_PRODUCTION_NOODLES_FILES.contains(relative);
            for (line_number, line) in read_utf8(&path).lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }

                if contains_direct_noodles_reference(trimmed) && !allowed {
                    violations.push(format!(
                        "{guardrail}: {relative}:{}: {line}",
                        line_number + 1
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "M13 CRAM guardrail paths must keep direct production noodles usage confined to src/ingest/cram.rs:\n{}",
        violations.join("\n")
    );
}

#[test]
fn m14_benchmark_only_external_tools_stay_out_of_native_hot_paths() {
    assert_eq!(
        BENCHMARK_ONLY_EXTERNAL_TOOLS,
        ["samtools", "fastcat", "sambamba", "seqtk", "rasusa"],
        "M14.9 must explicitly name the benchmark-only external comparator tools"
    );

    let src_dir = repo_root().join("src");
    let mut violations = Vec::new();

    for path in rust_sources(&src_dir) {
        let relative = path
            .strip_prefix(repo_root())
            .unwrap_or_else(|error| panic!("{} is not under repo root: {error}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWED_PRODUCTION_BENCHMARK_TOOL_REFERENCE_FILES.contains(&relative.as_str()) {
            continue;
        }

        for (line_number, line) in read_utf8(&path).lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                continue;
            }

            if contains_benchmark_tool_reference(trimmed) {
                violations.push(format!("{relative}:{}: {line}", line_number + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "benchmark-only external comparator tools must stay out of Bamana-native production hot paths; only {:?} may reference them:\n{}",
        ALLOWED_PRODUCTION_BENCHMARK_TOOL_REFERENCE_FILES,
        violations.join("\n")
    );
}

#[test]
fn m14_benchmark_only_external_tool_boundary_is_documented() {
    let dependency_policy = read_utf8(&docs_dir().join("dependency-policy.md"));
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));
    let public_evidence_guide = read_utf8(
        &repo_root()
            .join("benchmarks")
            .join("public_evidence_guide.md"),
    );
    let m14 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-14-interop-benchmark-evidence.md"),
    );
    let taskmap = read_utf8(&repo_root().join("taskmap.md"));
    let combined = [
        dependency_policy.as_str(),
        oracle_policy.as_str(),
        public_evidence_guide.as_str(),
        m14.as_str(),
        taskmap.as_str(),
    ]
    .join("\n");

    for required in [
        "M14.9",
        "benchmark-only external tools",
        "samtools",
        "fastcat",
        "sambamba",
        "seqtk",
        "rasusa",
        "wrappers, comparators, fixtures, or oracle aids",
        "src/commands/benchmark.rs",
        "Bamana-native production hot paths",
        "tests/contract/dependency_boundary.rs",
        "do not enter Bamana-native production hot paths",
    ] {
        assert!(
            combined.contains(required),
            "M14.9 benchmark-only dependency boundary is missing documentation token: {required}"
        );
    }
}

#[test]
fn fastq_hot_paths_do_not_import_external_bio_parser_crates() {
    let protected_paths = [
        "src/fastq/mod.rs",
        "src/fastq/record.rs",
        "src/fastq/reader.rs",
        "src/fastq/writer.rs",
        "src/fastq/gzip.rs",
        "src/fastq/gzi.rs",
        "src/fastq/unmapped.rs",
        "src/commands/enumerate.rs",
        "src/commands/subsample.rs",
        "src/commands/explode.rs",
        "src/ingest/consume.rs",
        "src/forensics/duplication.rs",
        "src/forensics/deduplicate.rs",
    ];
    let mut violations = Vec::new();

    for relative in protected_paths {
        let path = repo_root().join(relative);
        for (line_number, line) in read_utf8(&path).lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                continue;
            }

            if contains_external_bio_parser_reference(trimmed) {
                violations.push(format!("{relative}:{}: {line}", line_number + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "native FASTQ hot paths must stay free of external generic bioinformatics parser crates:\n{}",
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

#[test]
fn scanner_oracle_surface_is_documented_as_native_first() {
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));

    for required in [
        "Native Scanner Oracle Boundary",
        "Malformed-record failure expectations",
        "RecordLayout",
        "production scanner and migrated record hot paths",
    ] {
        assert!(
            oracle_policy.contains(required),
            "testing oracle policy is missing scanner-oracle boundary language: {required}"
        );
    }
}

#[test]
fn fastq_oracle_surface_is_documented_as_native_first() {
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));

    for required in [
        "Native FASTQ Oracle Boundary",
        "Malformed FASTQ and FASTQ.GZ failure expectations",
        "production FASTQ parser, writer, command-consumer, and FASTQ.GZI paths",
        "src/fastq",
    ] {
        assert!(
            oracle_policy.contains(required),
            "testing oracle policy is missing FASTQ-oracle boundary language: {required}"
        );
    }
}

#[test]
fn m5_proof_command_oracle_policy_is_documented() {
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));

    for required in [
        "Milestone 5 Proof-Command Oracle Boundary",
        "`verify`, `header`, and `subsample`",
        "production proof-command paths",
        "tests/header_oracle.rs",
        "BAM-side `subsample`",
        "FASTQ-side `subsample`",
    ] {
        assert!(
            oracle_policy.contains(required),
            "testing oracle policy is missing M5 proof-command boundary language: {required}"
        );
    }
}

#[test]
fn m7_mutation_forensics_oracle_policy_is_documented() {
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));

    for required in [
        "Milestone 7 Mutation And Forensics Oracle Boundary",
        "production mutation, remediation, and forensics paths",
        "BAM-side `deduplicate`",
        "FASTQ-side `inspect_duplication` and `deduplicate`",
        "Malformed mutation and forensics failure expectations",
    ] {
        assert!(
            oracle_policy.contains(required),
            "testing oracle policy is missing M7 mutation/forensics boundary language: {required}"
        );
    }

    for command in M7_MUTATION_FORENSICS_HOT_PATHS
        .iter()
        .map(|(command, _paths)| *command)
    {
        assert!(
            oracle_policy.contains(&format!("`{command}`")),
            "testing oracle policy is missing M7 command name: {command}"
        );
    }
}

fn contains_direct_noodles_reference(line: &str) -> bool {
    line.contains("noodles_") || line.contains("noodles::")
}

fn contains_external_bio_parser_reference(line: &str) -> bool {
    contains_direct_noodles_reference(line)
        || line.contains("bio::")
        || line.contains("needletail::")
        || line.contains("seq_io::")
        || line.contains("rust_htslib::")
}

fn contains_benchmark_tool_reference(line: &str) -> bool {
    BENCHMARK_ONLY_EXTERNAL_TOOLS
        .iter()
        .any(|tool| line.contains(tool))
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
