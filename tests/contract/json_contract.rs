use std::collections::BTreeSet;

use serde_json::Value;

use super::{
    command_name_from_example, command_schema_paths, docs_dir, example_paths, fixtures_dir,
    read_utf8, schema_dir, schema_path_for_command, spec_dir,
};
use crate::contract::support::fixture_manifest::load_fixture_manifest;

const M7_MUTATION_FORENSICS_COMMANDS: &[&str] = &[
    "reheader",
    "annotate_rg",
    "inspect_duplication",
    "deduplicate",
    "forensic_inspect",
];

#[test]
fn schema_files_parse_as_json() {
    for path in super::collect_json_files(&schema_dir()) {
        let contents = read_utf8(&path);
        serde_json::from_str::<Value>(&contents)
            .unwrap_or_else(|error| panic!("schema {} did not parse: {error}", path.display()));
    }
}

#[test]
fn example_files_parse_as_json() {
    for path in example_paths() {
        let contents = read_utf8(&path);
        serde_json::from_str::<Value>(&contents)
            .unwrap_or_else(|error| panic!("example {} did not parse: {error}", path.display()));
    }
}

#[test]
fn every_example_has_matching_command_schema() {
    for path in example_paths() {
        let command = command_name_from_example(&path);
        let schema_path = schema_path_for_command(&command);
        assert!(
            schema_path.exists(),
            "example {} has no matching schema {}",
            path.display(),
            schema_path.display()
        );
    }
}

#[test]
fn every_command_schema_has_success_and_failure_examples() {
    let example_commands: BTreeSet<String> = example_paths()
        .into_iter()
        .map(|path| command_name_from_example(&path))
        .collect();

    for schema_path in command_schema_paths() {
        let command = schema_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".schema.json"))
            .unwrap_or_else(|| panic!("invalid schema filename: {}", schema_path.display()));

        assert!(
            example_commands.contains(command),
            "schema {} has no examples",
            schema_path.display()
        );

        let success = spec_dir()
            .join("examples")
            .join(format!("{command}.success.json"));
        let failure = spec_dir()
            .join("examples")
            .join(format!("{command}.failure.json"));

        assert!(
            success.exists(),
            "missing success example {}",
            success.display()
        );
        assert!(
            failure.exists(),
            "missing failure example {}",
            failure.display()
        );
    }
}

#[test]
fn contract_docs_exist() {
    for path in [
        spec_dir().join("README.md"),
        spec_dir().join("cli").join("commands.md"),
        spec_dir().join("cli").join("global-options.md"),
        spec_dir().join("cli").join("exit-codes.md"),
        spec_dir().join("contracts").join("versioning.md"),
        spec_dir().join("contracts").join("compatibility.md"),
        spec_dir().join("contracts").join("naming.md"),
        docs_dir().join("cli.md"),
        docs_dir().join("json-output.md"),
        docs_dir().join("interop-testing.md"),
        docs_dir().join("fixtures.md"),
        fixtures_dir().join("README.md"),
        fixtures_dir().join("manifest.json"),
        fixtures_dir().join("manifest.schema.json"),
        fixtures_dir().join("source").join("README.md"),
        fixtures_dir()
            .join("source")
            .join("tiny.valid.cram.explicit_ref.source.sam"),
        fixtures_dir().join("source").join("tiny.ref.primary.fasta"),
        fixtures_dir()
            .join("source")
            .join("tiny.valid.cram.explicit_ref.provenance.json"),
        fixtures_dir()
            .join("source")
            .join("generate_tiny_cram_fixture.sh"),
        fixtures_dir()
            .join("source")
            .join("generate_tiny_cram_fixture.md"),
        fixtures_dir()
            .join("source")
            .join("generate_tiny_cram_fixture.env.example"),
        fixtures_dir().join("bam").join("README.md"),
        fixtures_dir().join("bam").join("valid").join("README.md"),
        fixtures_dir().join("bam").join("invalid").join("README.md"),
        fixtures_dir()
            .join("bam")
            .join("transforms")
            .join("README.md"),
        fixtures_dir().join("bam").join("tags").join("README.md"),
        fixtures_dir().join("bam").join("sorting").join("README.md"),
        fixtures_dir().join("bam").join("mapping").join("README.md"),
        fixtures_dir()
            .join("bam")
            .join("indexing")
            .join("README.md"),
        fixtures_dir().join("duplication").join("README.md"),
        fixtures_dir().join("cram").join("README.md"),
        fixtures_dir().join("cram").join("valid").join("README.md"),
        fixtures_dir().join("consume").join("README.md"),
        fixtures_dir().join("forensics").join("README.md"),
        fixtures_dir().join("expected").join("README.md"),
        fixtures_dir()
            .join("expected")
            .join("consume")
            .join("README.md"),
        fixtures_dir()
            .join("expected")
            .join("reheader")
            .join("README.md"),
        fixtures_dir()
            .join("expected")
            .join("annotate_rg")
            .join("README.md"),
        fixtures_dir()
            .join("expected")
            .join("inspect_duplication")
            .join("README.md"),
        fixtures_dir()
            .join("expected")
            .join("deduplicate")
            .join("README.md"),
        fixtures_dir()
            .join("expected")
            .join("forensic_inspect")
            .join("README.md"),
        fixtures_dir().join("plans").join("fixture-matrix.md"),
        fixtures_dir().join("plans").join("generation-strategy.md"),
        fixtures_dir().join("plans").join("coverage-map.md"),
        fixtures_dir().join("plans").join("cram-fixtures.md"),
        fixtures_dir()
            .join("plans")
            .join("duplication-forensics.md"),
        fixtures_dir().join("scripts").join("README.md"),
        fixtures_dir()
            .join("scripts")
            .join("generate_valid_fixtures.sh"),
        fixtures_dir()
            .join("scripts")
            .join("mutate_invalid_fixtures.py"),
        fixtures_dir().join("json").join("README.md"),
        fixtures_dir().join("golden").join("README.md"),
    ] {
        assert!(
            path.exists(),
            "missing contract document {}",
            path.display()
        );
    }
}

#[test]
fn mutation_forensics_command_contracts_have_docs_schemas_examples_and_behavior_notes() {
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let cli_doc = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let sphinx_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_mutation_forensics.rst"),
    );

    for command in M7_MUTATION_FORENSICS_COMMANDS {
        assert!(
            schema_path_for_command(command).exists(),
            "M7 command {command} is missing a JSON schema"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.success.json"))
                .exists(),
            "M7 command {command} is missing a canonical success example"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.failure.json"))
                .exists(),
            "M7 command {command} is missing a canonical failure example"
        );
        assert!(
            commands_doc.contains(&format!("## `{command}`")),
            "M7 command {command} is missing from spec/cli/commands.md"
        );
        assert!(
            cli_doc.contains(&format!("`{command}`")),
            "M7 command {command} is missing from docs/cli.md"
        );
        assert!(
            json_doc.contains(&format!("## `{command}`")),
            "M7 command {command} is missing from docs/json-output.md"
        );
        assert!(
            readme.contains(&format!("`{command}`")),
            "M7 command {command} is missing from README.md"
        );
        assert!(
            sphinx_doc.contains(&format!("``{command}``")),
            "M7 command {command} is missing from Sphinx mutation/forensics notes"
        );
    }

    for required in [
        "header-only",
        "per-record `RG:Z` tags",
        "record-level read-group annotation",
        "dry-run",
        "conservative remediation",
        "not a molecular duplicate-marking contract",
        "not a fraud-detection contract",
        "whole-file",
    ] {
        assert!(
            commands_doc.contains(required)
                || cli_doc.contains(required)
                || json_doc.contains(required)
                || readme.contains(required)
                || sphinx_doc.contains(required),
            "M7 command documentation is missing behavior note: {required}"
        );
    }
}

#[test]
fn public_contract_commands_have_docs_schemas_and_examples() {
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let cli_doc = read_utf8(&docs_dir().join("cli.md"));

    for command in ["benchmark", "fastq", "unmap"] {
        assert!(
            schema_path_for_command(command).exists(),
            "public command {command} is missing a JSON schema"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.success.json"))
                .exists(),
            "public command {command} is missing a canonical success example"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.failure.json"))
                .exists(),
            "public command {command} is missing a canonical failure example"
        );
        assert!(
            commands_doc.contains(&format!("## `{command}`")),
            "public command {command} is missing from spec/cli/commands.md"
        );
        assert!(
            cli_doc.contains(&format!("`{command}`")),
            "public command {command} is missing from docs/cli.md"
        );
    }
}

#[test]
fn proof_command_contracts_have_docs_schemas_examples_and_behavior_notes() {
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let cli_doc = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));

    for command in ["verify", "header", "subsample"] {
        assert!(
            schema_path_for_command(command).exists(),
            "proof command {command} is missing a JSON schema"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.success.json"))
                .exists(),
            "proof command {command} is missing a canonical success example"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.failure.json"))
                .exists(),
            "proof command {command} is missing a canonical failure example"
        );
        assert!(
            commands_doc.contains(&format!("## `{command}`")),
            "proof command {command} is missing from spec/cli/commands.md"
        );
        assert!(
            cli_doc.contains(&format!("`{command}`")),
            "proof command {command} is missing from docs/cli.md"
        );
        assert!(
            json_doc.contains(&format!("## `{command}`")),
            "proof command {command} is missing from docs/json-output.md"
        );
    }

    for required in [
        "native BAM header parse",
        "Full record-stream validity, EOF presence, or deep validation",
        "native BAM header codec",
        "That alignment records are valid or that the full file body is readable",
        "BAM, FASTQ, or FASTQ.GZ",
        "retained records preserve encounter order",
    ] {
        assert!(
            commands_doc.contains(required),
            "proof-command CLI contract is missing behavior note: {required}"
        );
    }
}

#[test]
fn inspection_command_contracts_have_docs_schemas_examples_and_behavior_notes() {
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let cli_doc = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let readme = read_utf8(&super::repo_root().join("README.md"));

    for command in [
        "check_eof",
        "check_sort",
        "check_map",
        "summary",
        "check_tag",
        "validate",
    ] {
        assert!(
            schema_path_for_command(command).exists(),
            "M6 command {command} is missing a JSON schema"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.success.json"))
                .exists(),
            "M6 command {command} is missing a canonical success example"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.failure.json"))
                .exists(),
            "M6 command {command} is missing a canonical failure example"
        );
        assert!(
            commands_doc.contains(&format!("## `{command}`")),
            "M6 command {command} is missing from spec/cli/commands.md"
        );
        assert!(
            cli_doc.contains(&format!("`{command}`")),
            "M6 command {command} is missing from docs/cli.md"
        );
        assert!(
            json_doc.contains(&format!("## `{command}`")),
            "M6 command {command} is missing from docs/json-output.md"
        );
        assert!(
            readme.contains(&format!("`{command}`")),
            "M6 command {command} is missing from README.md"
        );
    }

    for required in [
        "EOF marker presence or absence",
        "Overall BAM validity or full stream readability",
        "bounded or stricter scan",
        "Mapping evidence from the sources explicitly reported",
        "Only the metrics that correspond to the reported evidence mode",
        "Full-file absence in bounded mode",
        "Biological correctness, reference concordance, or all optional-field semantics",
        "bounded non-observation must not be interpreted as full-file absence",
    ] {
        assert!(
            commands_doc.contains(required)
                || cli_doc.contains(required)
                || json_doc.contains(required)
                || readme.contains(required),
            "M6 command documentation is missing behavior note: {required}"
        );
    }
}

#[test]
fn fixture_manifest_includes_m6_inspection_baseline() {
    let manifest = load_fixture_manifest();

    for (required_id, expected_format, expected_validity, expected_command) in [
        ("tiny.valid.coordinate", "BAM", "valid", "check_eof"),
        ("tiny.valid.coordinate", "BAM", "valid", "check_sort"),
        ("tiny.valid.queryname", "BAM", "valid", "check_sort"),
        (
            "tiny.invalid.unsorted_coordinate",
            "BAM",
            "invalid",
            "check_sort",
        ),
        ("tiny.valid.coordinate", "BAM", "valid", "check_map"),
        ("tiny.valid.unmapped", "BAM", "valid", "check_map"),
        ("tiny.valid.coordinate.bai", "BAI", "valid", "check_map"),
        ("tiny.valid.coordinate.bai", "BAI", "valid", "summary"),
        ("tiny.tags.nm_rg", "BAM", "valid", "check_tag"),
        ("tiny.tags.absent_requested", "BAM", "valid", "check_tag"),
        ("tiny.invalid.bad_aux", "BAM", "invalid", "check_tag"),
        ("tiny.invalid.no_eof", "BAM", "invalid", "check_eof"),
        (
            "tiny.invalid.truncated_record",
            "BAM",
            "invalid",
            "validate",
        ),
        ("tiny.invalid.header_mismatch", "BAM", "invalid", "validate"),
    ] {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.id == required_id)
            .unwrap_or_else(|| {
                panic!("fixture manifest is missing M6 inspection fixture {required_id}")
            });

        assert_eq!(
            fixture.format, expected_format,
            "fixture {required_id} has unexpected format"
        );
        assert_eq!(
            fixture.validity, expected_validity,
            "fixture {required_id} has unexpected validity"
        );
        assert!(
            fixture
                .primary_commands
                .iter()
                .any(|command| command == expected_command)
                || fixture
                    .secondary_commands
                    .iter()
                    .any(|command| command == expected_command),
            "fixture {required_id} is not mapped to {expected_command}"
        );
        assert!(
            fixture
                .expected_artifacts
                .iter()
                .any(|artifact| { artifact.starts_with(&format!("expected/{expected_command}/")) }),
            "fixture {required_id} has no reserved {expected_command} expected artifact"
        );
    }
}

#[test]
fn fixture_manifest_includes_m7_mutation_forensics_baseline() {
    let manifest = load_fixture_manifest();

    for (required_id, expected_format, expected_validity, expected_command) in [
        ("tiny.clean.bam", "BAM", "valid", "reheader"),
        ("tiny.clean.bam", "BAM", "valid", "annotate_rg"),
        ("tiny.clean.bam", "BAM", "valid", "inspect_duplication"),
        ("tiny.clean.bam", "BAM", "valid", "deduplicate"),
        ("tiny.clean.bam", "BAM", "valid", "forensic_inspect"),
        (
            "tiny.duplicate.fastq.whole_append",
            "FASTQ",
            "parseable",
            "inspect_duplication",
        ),
        (
            "tiny.duplicate.fastq.whole_append",
            "FASTQ",
            "parseable",
            "deduplicate",
        ),
        (
            "tiny.duplicate.bam.local_block",
            "BAM",
            "parseable",
            "inspect_duplication",
        ),
        (
            "tiny.duplicate.bam.local_block",
            "BAM",
            "parseable",
            "deduplicate",
        ),
        (
            "tiny.forensic.bam.rg_pg_inconsistent",
            "BAM",
            "parseable",
            "forensic_inspect",
        ),
    ] {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.id == required_id)
            .unwrap_or_else(|| panic!("fixture manifest is missing M7 fixture {required_id}"));

        assert_eq!(
            fixture.format, expected_format,
            "fixture {required_id} has unexpected format"
        );
        assert_eq!(
            fixture.validity, expected_validity,
            "fixture {required_id} has unexpected validity"
        );
        assert!(
            fixture
                .primary_commands
                .iter()
                .any(|command| command == expected_command)
                || fixture
                    .secondary_commands
                    .iter()
                    .any(|command| command == expected_command),
            "fixture {required_id} is not mapped to {expected_command}"
        );
        assert!(
            fixture
                .expected_artifacts
                .iter()
                .any(|artifact| { artifact.starts_with(&format!("expected/{expected_command}/")) }),
            "fixture {required_id} has no reserved {expected_command} expected artifact"
        );
    }
}

#[test]
fn fixture_manifest_includes_m5_subsample_baseline() {
    let manifest = load_fixture_manifest();

    for (required_id, expected_format, expected_validity) in [
        ("tiny.clean.bam", "BAM", "valid"),
        ("tiny.clean.fastq", "FASTQ", "valid"),
        ("tiny.valid.fastq_gz", "FASTQ.GZ", "valid"),
        ("tiny.invalid.fastq.truncated", "FASTQ", "invalid"),
        ("tiny.invalid.bam.truncated_record", "BAM", "invalid"),
    ] {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.id == required_id)
            .unwrap_or_else(|| {
                panic!("fixture manifest is missing M5 subsample fixture {required_id}")
            });

        assert_eq!(
            fixture.format, expected_format,
            "fixture {required_id} has unexpected format"
        );
        assert_eq!(
            fixture.validity, expected_validity,
            "fixture {required_id} has unexpected validity"
        );
        assert!(
            fixture
                .primary_commands
                .iter()
                .any(|command| command == "subsample")
                || fixture
                    .secondary_commands
                    .iter()
                    .any(|command| command == "subsample"),
            "fixture {required_id} is not mapped to subsample"
        );
        assert!(
            fixture
                .expected_artifacts
                .iter()
                .any(|artifact| artifact.starts_with("expected/subsample/")),
            "fixture {required_id} has no reserved subsample expected artifact"
        );
    }
}

#[test]
fn subsample_failure_examples_cover_m5_failure_taxonomy() {
    let examples = [
        (
            "subsample.failure.unsupported_format.json",
            "unsupported_format",
        ),
        (
            "subsample.failure.invalid_fraction.json",
            "invalid_fraction",
        ),
        (
            "subsample.failure.invalid_filter_combination.json",
            "unsupported_input_for_command",
        ),
    ];

    for (file_name, expected_code) in examples {
        let path = spec_dir().join("examples").join(file_name);
        let contents = read_utf8(&path);
        let value = serde_json::from_str::<Value>(&contents)
            .unwrap_or_else(|error| panic!("example {} did not parse: {error}", path.display()));
        assert_eq!(
            value
                .get("command")
                .and_then(Value::as_str)
                .expect("example should include command"),
            "subsample"
        );
        assert_eq!(
            value
                .pointer("/error/code")
                .and_then(Value::as_str)
                .expect("example should include error.code"),
            expected_code,
            "example {file_name} should cover {expected_code}"
        );
    }
}

#[test]
fn proof_command_benchmark_schemas_include_m5_command_timings() {
    let header_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("header_microbench.schema.json"),
    );
    let bgzf_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("bgzf_microbench.schema.json"),
    );
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let fastq_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("fastq_microbench.schema.json"),
    );

    for (schema_name, schema, command) in [
        ("header_microbench", &header_schema, "verify"),
        ("header_microbench", &header_schema, "header"),
        ("bgzf_microbench", &bgzf_schema, "check_eof"),
        ("scanner_microbench", &scanner_schema, "summary"),
        ("scanner_microbench", &scanner_schema, "subsample_bam"),
        ("scanner_microbench", &scanner_schema, "check_sort"),
        ("scanner_microbench", &scanner_schema, "check_map"),
        ("scanner_microbench", &scanner_schema, "check_tag"),
        ("scanner_microbench", &scanner_schema, "validate"),
        ("fastq_microbench", &fastq_schema, "subsample_fastq"),
        ("fastq_microbench", &fastq_schema, "subsample_fastq_gz"),
    ] {
        assert!(
            schema.contains(&format!("\"{command}\"")),
            "{schema_name} schema does not include command timing row {command}"
        );
    }
}

#[test]
fn fixture_manifest_includes_duplication_and_forensics_trio() {
    let manifest = load_fixture_manifest();
    let ids: BTreeSet<String> = manifest
        .fixtures
        .into_iter()
        .map(|fixture| fixture.id)
        .collect();

    for required_id in [
        "tiny.clean.fastq",
        "tiny.clean.bam",
        "tiny.duplicate.fastq.whole_append",
        "tiny.duplicate.fastq.local_block",
        "tiny.duplicate.bam.local_block",
        "tiny.forensic.bam.rg_pg_inconsistent",
        "tiny.forensic.bam.readname_shift",
        "tiny.forensic.bam.concatenated_signature",
        "tiny.invalid.fastq.truncated",
        "tiny.invalid.bam.truncated_record",
    ] {
        assert!(
            ids.contains(required_id),
            "fixture manifest is missing required trio fixture {required_id}"
        );
    }
}

#[test]
fn fixture_manifest_includes_consume_fixture_plan() {
    let manifest = load_fixture_manifest();
    let ids: BTreeSet<String> = manifest
        .fixtures
        .into_iter()
        .map(|fixture| fixture.id)
        .collect();

    for required_id in [
        "tiny.valid.sam",
        "tiny.valid.cram.explicit_ref.source_sam",
        "tiny.valid.cram.explicit_ref.source_bam",
        "tiny.ref.primary",
        "tiny.valid.cram.explicit_ref",
        "tiny.valid.cram.reference_required",
        "tiny.valid.cram.compatible_refdict",
        "tiny.valid.bam.compatible_refdict",
        "tiny.valid.bam.incompatible_refdict",
        "tiny.valid.fastq",
        "tiny.valid.fastq_gz",
        "tiny.consume.mixed_alignment_raw",
        "tiny.consume.directory_tree",
    ] {
        assert!(
            ids.contains(required_id),
            "fixture manifest is missing planned consume fixture {required_id}"
        );
    }
}

#[test]
fn fixture_manifest_parses_and_has_unique_ids() {
    let manifest = load_fixture_manifest();
    assert_eq!(manifest.suite_id, "bamana-tiny-fixtures");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.status, "planning");
    assert!(
        !manifest.fixtures.is_empty(),
        "fixture manifest should list planned fixtures"
    );

    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();

    for fixture in manifest.fixtures {
        assert!(
            ids.insert(fixture.id.clone()),
            "duplicate fixture id {}",
            fixture.id
        );
        assert!(
            paths.insert(fixture.path.clone()),
            "duplicate fixture path {}",
            fixture.path
        );
        assert!(
            !fixture.description.is_empty(),
            "fixture {} is missing a description",
            fixture.id
        );
        assert!(
            !fixture.primary_commands.is_empty(),
            "fixture {} is missing primary commands",
            fixture.id
        );
        assert!(
            !fixture.format.is_empty(),
            "fixture {} is missing format metadata",
            fixture.id
        );
        assert!(
            !fixture.category.is_empty(),
            "fixture {} is missing category metadata",
            fixture.id
        );
        assert!(
            !fixture.validity.is_empty(),
            "fixture {} is missing validity metadata",
            fixture.id
        );
        assert!(
            !fixture.status.is_empty(),
            "fixture {} is missing status metadata",
            fixture.id
        );
        assert!(
            !fixture.regeneration_strategy.is_empty(),
            "fixture {} is missing regeneration strategy metadata",
            fixture.id
        );
        assert!(
            !fixture.secondary_commands.is_empty() || !fixture.primary_commands.is_empty(),
            "fixture {} should have command mappings",
            fixture.id
        );
        assert!(
            fixture.generated || !fixture.source_fixture_ids.is_empty(),
            "fixture {} should document whether it is generated or derived",
            fixture.id
        );
        assert!(
            fixture
                .expected_artifacts
                .iter()
                .all(|artifact| artifact.starts_with("expected/")),
            "fixture {} has non-expected artifact paths",
            fixture.id
        );
    }
}
