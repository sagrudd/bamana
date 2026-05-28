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

const M8_TRANSFORM_INGEST_COMMANDS: &[&str] = &["sort", "merge", "explode", "checksum", "consume"];
const M9_INDEX_COMMANDS: &[&str] = &["index", "check_index"];

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
        fixtures_dir()
            .join("expected")
            .join("select_region")
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
fn transform_ingest_command_contracts_have_docs_schemas_examples_and_behavior_notes() {
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let cli_doc = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let sphinx_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_transform_ingest.rst"),
    );

    for command in M8_TRANSFORM_INGEST_COMMANDS {
        assert!(
            schema_path_for_command(command).exists(),
            "M8 command {command} is missing a JSON schema"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.success.json"))
                .exists(),
            "M8 command {command} is missing a canonical success example"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.failure.json"))
                .exists(),
            "M8 command {command} is missing a canonical failure example"
        );
        assert!(
            commands_doc.contains(&format!("## `{command}`")),
            "M8 command {command} is missing from spec/cli/commands.md"
        );
        assert!(
            cli_doc.contains(&format!("`{command}`")),
            "M8 command {command} is missing from docs/cli.md"
        );
        assert!(
            json_doc.contains(&format!("## `{command}`")),
            "M8 command {command} is missing from docs/json-output.md"
        );
        assert!(
            readme.contains(&format!("`{command}`")),
            "M8 command {command} is missing from README.md"
        );
        assert!(
            sphinx_doc.contains(&format!("``{command}``")),
            "M8 command {command} is missing from Sphinx transform/ingest notes"
        );
    }

    for required in [
        "ordering semantics",
        "checksum domains",
        "shard boundaries",
        "ingest mode",
        "dry-run",
        "CRAM reference policy",
        "deferred index behavior",
        "in-memory first-slice",
        "FASTQ.GZI",
        "not imply full BAM validity",
    ] {
        assert!(
            commands_doc.contains(required)
                || cli_doc.contains(required)
                || json_doc.contains(required)
                || readme.contains(required)
                || sphinx_doc.contains(required),
            "M8 command documentation is missing behavior note: {required}"
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
fn mutation_forensics_benchmark_hooks_are_documented_and_schema_governed() {
    let header_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("header_microbench.schema.json"),
    );
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let header_doc = read_utf8(&docs_dir().join("sphinx").join("header_microbenchmarks.rst"));
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let roadmap = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-07-mutation-forensics.md"),
    );

    for (schema_name, schema, doc, command) in [
        ("header_microbench", &header_schema, &header_doc, "reheader"),
        (
            "header_microbench",
            &header_schema,
            &header_doc,
            "annotate_rg",
        ),
        (
            "scanner_microbench",
            &scanner_schema,
            &scanner_doc,
            "inspect_duplication",
        ),
        (
            "scanner_microbench",
            &scanner_schema,
            &scanner_doc,
            "deduplicate",
        ),
        (
            "scanner_microbench",
            &scanner_schema,
            &scanner_doc,
            "forensic_inspect",
        ),
    ] {
        assert!(
            schema.contains(&format!("\"{command}\"")),
            "{schema_name} schema does not include M7 command timing row {command}"
        );
        assert!(
            doc.contains(&format!("``{command}``")),
            "{schema_name} docs do not describe M7 command timing row {command}"
        );
        assert!(
            roadmap.contains(&format!("`{command}`")),
            "M7 roadmap does not record benchmark hook evidence for {command}"
        );
    }

    for required in [
        "process startup",
        "scan cost",
        "rewrite cost",
        "compression cost",
        "checksum verification",
        "dry-run",
        "comparator parity",
    ] {
        assert!(
            header_doc.contains(required)
                || scanner_doc.contains(required)
                || roadmap.contains(required),
            "M7 benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn sort_benchmark_hook_is_documented_and_schema_governed() {
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let roadmap = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );

    assert!(
        scanner_schema.contains("\"sort\""),
        "scanner_microbench schema does not include M8 sort command timing row"
    );
    assert!(
        scanner_doc.contains("``sort``"),
        "scanner_microbench docs do not describe M8 sort command timing row"
    );
    assert!(
        roadmap.contains("`sort` command smoke timing"),
        "M8 roadmap does not record sort benchmark hook evidence"
    );

    for required in [
        "coordinate rewrite",
        "canonical checksum verification",
        "in-memory",
        "BGZF write",
        "smoke timing",
        "comparator parity",
    ] {
        assert!(
            scanner_doc.contains(required) || roadmap.contains(required),
            "M8 sort benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn merge_benchmark_hook_is_documented_and_schema_governed() {
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let roadmap = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );

    assert!(
        scanner_schema.contains("\"merge\""),
        "scanner_microbench schema does not include M8 merge command timing row"
    );
    assert!(
        scanner_doc.contains("``merge``"),
        "scanner_microbench docs do not describe M8 merge command timing row"
    );
    assert!(
        roadmap.contains("`merge` command smoke timing"),
        "M8 roadmap does not record merge benchmark hook evidence"
    );

    for required in [
        "coordinate merge",
        "canonical checksum verification",
        "in-memory",
        "BGZF write",
        "smoke timing",
        "comparator parity",
    ] {
        assert!(
            scanner_doc.contains(required) || roadmap.contains(required),
            "M8 merge benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn checksum_benchmark_hook_is_documented_and_schema_governed() {
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let roadmap = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );

    assert!(
        scanner_schema.contains("\"checksum\""),
        "scanner_microbench schema does not include M8 checksum command timing row"
    );
    assert!(
        scanner_doc.contains("``checksum``"),
        "scanner_microbench docs do not describe M8 checksum command timing row"
    );
    assert!(
        roadmap.contains("`checksum` command smoke timing"),
        "M8 roadmap does not record checksum benchmark hook evidence"
    );

    for required in [
        "raw encounter-order",
        "canonical order-insensitive",
        "header serialization",
        "payload domain",
        "tag exclusion",
        "mapped-only filtering",
        "semantic equivalence",
    ] {
        assert!(
            scanner_doc.contains(required) || roadmap.contains(required),
            "M8 checksum benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn explode_benchmark_hook_is_documented_and_schema_governed() {
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let roadmap = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );

    assert!(
        scanner_schema.contains("\"explode\""),
        "scanner_microbench schema does not include M8 explode command timing row"
    );
    assert!(
        scanner_doc.contains("``explode``"),
        "scanner_microbench docs do not describe M8 explode command timing row"
    );
    assert!(
        roadmap.contains("`explode` command smoke timing"),
        "M8 roadmap does not record explode benchmark hook evidence"
    );

    for required in [
        "scanner-backed BAM",
        "native BGZF",
        "contiguous shard",
        "encounter order",
        "FASTQ.GZI",
        "uniform shard sizes",
        "random-access parallel gzip inflate",
    ] {
        assert!(
            scanner_doc.contains(required) || roadmap.contains(required),
            "M8 explode benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn consume_benchmark_hook_is_documented_and_schema_governed() {
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let roadmap = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );

    assert!(
        scanner_schema.contains("\"consume\""),
        "scanner_microbench schema does not include M8 consume command timing row"
    );
    assert!(
        scanner_doc.contains("``consume``"),
        "scanner_microbench docs do not describe M8 consume command timing row"
    );
    assert!(
        roadmap.contains("`consume` command smoke timing"),
        "M8 roadmap does not record consume benchmark hook evidence"
    );

    for required in [
        "scanner-backed BAM alignment ingest",
        "mixed-format policy",
        "native BGZF",
        "deferred checksum",
        "CRAM reference-policy",
        "post-ingest checksum verification",
    ] {
        assert!(
            scanner_doc.contains(required) || roadmap.contains(required),
            "M8 consume benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn milestone_8_output_safety_contract_is_documented() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );
    let sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_transform_ingest.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for command in ["sort", "merge", "explode", "consume"] {
        assert!(
            sphinx.contains(&format!("``{command}``"))
                && roadmap.contains(&format!("`{command}`"))
                && taskmap.contains(&format!("`{command}`")),
            "M8 output-safety docs do not name writer command: {command}"
        );
    }

    for required in [
        "completed temporary",
        "final rename",
        "reject collisions unless",
        "multi-output preflight",
        "side-effect bounded",
        "checksum/index payloads",
        "work actually performed",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_doc.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || sphinx.contains(required)
                || taskmap.contains(required),
            "M8 output-safety contract is missing: {required}"
        );
    }
}

#[test]
fn milestone_8_dependency_and_benchmark_guardrails_are_documented() {
    let dependency_tests = read_utf8(
        &super::repo_root()
            .join("tests")
            .join("contract")
            .join("dependency_boundary.rs"),
    );
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));
    let scanner_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("bin")
            .join("scanner_microbench.rs"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let benchmark_results = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("README.md"),
    );
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );

    for command in ["sort", "merge", "explode", "checksum", "consume"] {
        assert!(
            dependency_tests.contains(&format!("\"{command}\""))
                && oracle_policy.contains(&format!("`{command}`"))
                && scanner_source.contains(&format!("\"{command}\""))
                && scanner_doc.contains(&format!("``{command}``"))
                && scanner_schema.contains(&format!("\"{command}\"")),
            "M8 dependency and benchmark guardrails do not explicitly name command: {command}"
        );
    }

    for required in [
        "full-record materialization",
        "in-memory sorting cost",
        "merge compatibility",
        "native BGZF compression",
        "checksum-domain",
        "shard planning",
        "ingest normalization",
        "CRAM compatibility behavior",
        "process startup",
        "JSON emission",
        "not comparator parity",
    ] {
        assert!(
            scanner_source.contains(required)
                || scanner_doc.contains(required)
                || benchmark_results.contains(required),
            "M8 benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn milestone_9_dependency_and_benchmark_guardrails_are_documented() {
    let dependency_tests = read_utf8(
        &super::repo_root()
            .join("tests")
            .join("contract")
            .join("dependency_boundary.rs"),
    );
    let oracle_policy = read_utf8(&docs_dir().join("testing-oracles.md"));
    let scanner_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("bin")
            .join("scanner_microbench.rs"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let benchmark_results = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("README.md"),
    );
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m9 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-09-bam-index-random-access.md"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for command in [
        "index",
        "check_index",
        "check_map_indexed",
        "summary_indexed",
        "random_access_substrate",
    ] {
        assert!(
            dependency_tests.contains(&format!("\"{command}\""))
                && oracle_policy.contains("Milestone 9 Index And Random-Access Oracle Boundary"),
            "M9 dependency guardrails do not explicitly name command or substrate: {command}"
        );
    }

    for command in [
        "index_bam",
        "check_index",
        "check_map_indexed",
        "summary_indexed",
    ] {
        assert!(
            scanner_source.contains(&format!("\"{command}\""))
                && scanner_doc.contains(&format!("``{command}``"))
                && scanner_schema.contains(&format!("\"{command}\"")),
            "M9 benchmark hooks do not explicitly name command timing row: {command}"
        );
    }

    for required in [
        "BAM index construction",
        "BAI structural validation",
        "index metadata-backed consumer evidence",
        "scan fallback timings",
        "random-access lookup deferral",
        "process startup",
        "JSON emission",
        "not comparator parity",
    ] {
        assert!(
            scanner_source.contains(required)
                || scanner_doc.contains(required)
                || benchmark_results.contains(required)
                || current.contains(required)
                || m9.contains(required)
                || taskmap.contains(required),
            "M9 benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn milestone_10_dependency_and_benchmark_guardrails_are_documented() {
    let dependency_tests = read_utf8(
        &super::repo_root()
            .join("tests")
            .join("contract")
            .join("dependency_boundary.rs"),
    );
    let scanner_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("bin")
            .join("scanner_microbench.rs"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let benchmark_results = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("README.md"),
    );
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for substrate in [
        "indexed_region_parser",
        "indexed_region_chunk_planning",
        "indexed_region_random_access_traversal",
        "check_map_region_indexed",
        "summary_region_indexed",
        "indexed_region_scan_fallback",
    ] {
        assert!(
            dependency_tests.contains(&format!("\"{substrate}\"")),
            "M10 dependency guardrails do not explicitly name substrate: {substrate}"
        );
    }

    for command in [
        "check_map_region_scan_fallback",
        "summary_region_scan_fallback",
        "check_map_region_indexed",
        "summary_region_indexed",
    ] {
        assert!(
            scanner_source.contains(&format!("\"{command}\""))
                && scanner_doc.contains(&format!("``{command}``"))
                && scanner_schema.contains(&format!("\"{command}\""))
                && benchmark_results.contains(&format!("`{command}`")),
            "M10 benchmark hooks do not explicitly name command timing row: {command}"
        );
    }

    for required in [
        "check_map --region <REGION>",
        "summary --region <REGION>",
        "production direct `noodles` imports remain limited",
        "index lookup",
        "BAI chunk planning",
        "random-access traversal",
        "region filtering",
        "scan fallback",
        "command startup",
        "JSON emission",
        "broad comparator parity",
        "native CRAM indexed queries",
        "biological interpretation",
        "selected-record output",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || scanner_source.contains(required)
                || scanner_doc.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required),
            "M10 benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn milestone_7_closeout_and_m8_activation_docs_are_consistent() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m7 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-07-mutation-forensics.md"),
    );
    let m8 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );
    let m7_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_mutation_forensics.rst"),
    );
    let m8_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_transform_ingest.rst"),
    );
    let sphinx_index = read_utf8(&docs_dir().join("sphinx").join("index.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("current milestone", &current),
        ("M7 roadmap", &m7),
        ("M7 Sphinx", &m7_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 7 is complete")
                || text.contains(
                    "Milestone 7: Native Mutation, Remediation, And Forensics Commands\n\n* status: complete",
                )
                || text.contains("Commands** is complete as of 2026-05-22")
                || text.contains("Status: complete as of 2026-05-22"),
            "{name} does not record Milestone 7 completion"
        );
    }

    for (name, text) in [
        ("README", &readme),
        ("current milestone", &current),
        ("roadmap", &roadmap),
        ("M8 roadmap", &m8),
        ("M8 Sphinx", &m8_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 8 is now the active")
                || text.contains("Milestone 8 is active")
                || text.contains("Milestone 8 is complete")
                || text.contains(
                    "Milestone 8: Native Transform, Checksum, Explode, And Ingest Commands\n\n* status: active",
                )
                || text.contains(
                    "Milestone 8: Native Transform, Checksum, Explode, And Ingest Commands\n\n* status: complete",
                )
                || text.contains("Commands** is active as of 2026-05-22")
                || text.contains("Commands** is complete as of 2026-05-23")
                || text.contains("Status: active as of 2026-05-22")
                || text.contains("Status: complete as of 2026-05-23")
                || text.contains("Milestone 8 became active"),
            "{name} does not record Milestone 8 activation"
        );
    }

    for command in ["sort", "merge", "explode", "checksum", "consume"] {
        assert!(
            current.contains(&format!("`{command}`"))
                && m8_sphinx.contains(&format!("``{command}``")),
            "M8 activation docs do not name command: {command}"
        );
    }

    assert!(
        sphinx_index.contains("native_transform_ingest"),
        "Sphinx index does not include the M8 technical note"
    );
}

#[test]
fn milestone_8_closeout_docs_are_consistent() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m8 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );
    let m8_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_transform_ingest.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M8 roadmap", &m8),
        ("M8 Sphinx", &m8_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 8 is complete")
                || text.contains(
                    "Milestone 8: Native Transform, Checksum, Explode, And Ingest Commands\n\n* status: complete",
                )
                || text.contains("Commands** is complete as of 2026-05-23")
                || text.contains("Status: complete as of 2026-05-23"),
            "{name} does not record Milestone 8 completion"
        );
    }

    for required in [
        "M8.10",
        "scanner_microbench",
        "all command timings reporting `1/1`",
        "activated by M9.1",
    ] {
        assert!(
            current.contains(required) || m8.contains(required) || taskmap.contains(required),
            "M8 closeout evidence is missing: {required}"
        );
    }
    assert!(
        current.contains("Milestone 9 became active")
            || current.contains("Milestone 9 is active")
            || current.contains("Milestone 9: Native BAM Index And Random Access** is complete"),
        "M8 closeout evidence is missing Milestone 9 activation/completion handoff"
    );

    for command in ["sort", "merge", "explode", "checksum", "consume"] {
        assert!(
            current.contains(&format!("`{command}`"))
                && m8.contains(&format!("`{command}`"))
                && m8_sphinx.contains(&format!("``{command}``")),
            "M8 closeout docs do not name command: {command}"
        );
    }
}

#[test]
fn milestone_9_activation_baseline_records_index_random_access_scope() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m9 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-09-bam-index-random-access.md"),
    );
    let m9_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_bam_index_random_access.rst"),
    );
    let sphinx_index = read_utf8(&docs_dir().join("sphinx").join("index.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("CLI docs", &cli),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M9 roadmap", &m9),
        ("M9 Sphinx", &m9_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 9 is active")
                || text.contains("Milestone 9 is complete")
                || text.contains("Status: active as of 2026-05-23")
                || text.contains("status: active")
                || text.contains("status: complete")
                || text.contains("Status: complete as of 2026-05-23"),
            "{name} does not record Milestone 9 activation"
        );
    }

    for required in [
        "Milestone 8 closed",
        "VirtualOffset",
        "src/bam/index.rs",
        "src/bgzf/reader.rs",
        "src/bam/scan.rs",
        "src/commands/index.rs",
        "src/commands/check_index.rs",
        "check_map",
        "summary",
        "BAI",
        "CSI",
        "FASTQ.GZI",
        "BAM `index` cannot yet write real CSI",
        "next_record_with_virtual_offsets",
        "build_bai_index_from_bam",
        "typed start/end offsets",
        "BAI chunks",
        "linear-index",
        "random-access",
        "benchmark",
        "fastq",
        "unmap",
    ] {
        assert!(
            current.contains(required)
                || m9.contains(required)
                || m9_sphinx.contains(required)
                || taskmap.contains(required),
            "M9 activation baseline evidence is missing: {required}"
        );
    }

    assert!(
        sphinx_index.contains("native_bam_index_random_access"),
        "Sphinx index does not include the M9 technical note"
    );
}

#[test]
fn milestone_9_index_contracts_and_fixtures_are_frozen() {
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let cli_doc = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let fixtures_doc = read_utf8(&docs_dir().join("fixtures.md"));
    let fixture_matrix = read_utf8(&fixtures_dir().join("plans").join("fixture-matrix.md"));
    let coverage_map = read_utf8(&fixtures_dir().join("plans").join("coverage-map.md"));
    let m9_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_bam_index_random_access.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for command in M9_INDEX_COMMANDS {
        assert!(
            schema_path_for_command(command).exists(),
            "M9 index command {command} is missing a JSON schema"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.success.json"))
                .exists(),
            "M9 index command {command} is missing a canonical success example"
        );
        assert!(
            spec_dir()
                .join("examples")
                .join(format!("{command}.failure.json"))
                .exists(),
            "M9 index command {command} is missing a canonical failure example"
        );
        assert!(
            commands_doc.contains(&format!("## `{command}`")),
            "M9 index command {command} is missing from spec/cli/commands.md"
        );
        assert!(
            cli_doc.contains(&format!("`{command}`")),
            "M9 index command {command} is missing from docs/cli.md"
        );
        assert!(
            json_doc.contains(&format!("## `{command}`")),
            "M9 index command {command} is missing from docs/json-output.md"
        );
        assert!(
            readme.contains(&format!("`{command}`")),
            "M9 index command {command} is missing from README.md"
        );
        assert!(
            m9_sphinx.contains(&format!("``{command}``")),
            "M9 index command {command} is missing from Sphinx M9 notes"
        );
    }

    for required in [
        "CSI writing remains deferred",
        "FASTQ.GZI",
        "overwrite",
        "--force",
        "timestamp based",
        "BAI structural",
        "CSI",
        "detected-but-not-supported",
        "output_index.created",
        "created = false",
        "coordinate-sorted BAM inputs",
    ] {
        assert!(
            commands_doc.contains(required)
                || cli_doc.contains(required)
                || json_doc.contains(required)
                || readme.contains(required)
                || m9_sphinx.contains(required)
                || taskmap.contains(required),
            "M9 index contract docs are missing behavior note: {required}"
        );
    }

    let manifest = load_fixture_manifest();
    for (required_id, expected_format, expected_validity, expected_command) in [
        ("tiny.valid.coordinate.bai", "BAI", "valid", "check_index"),
        ("tiny.invalid.bad_bai", "BAI", "invalid", "check_index"),
        (
            "tiny.invalid.mismatched_reference_count.bai",
            "BAI",
            "invalid",
            "check_index",
        ),
        (
            "tiny.valid.coordinate.stale_bai",
            "BAI",
            "stale",
            "check_index",
        ),
        (
            "tiny.valid.coordinate.csi_header",
            "CSI",
            "unsupported",
            "check_index",
        ),
        ("tiny.invalid.bad_csi", "CSI", "invalid", "check_index"),
        ("tiny.valid.coordinate", "BAM", "valid", "index"),
        (
            "tiny.invalid.unsorted_coordinate",
            "BAM",
            "invalid",
            "index",
        ),
        ("tiny.valid.fastq_gz", "FASTQ.GZ", "valid", "index"),
        ("tiny.valid.fastq_gz.gzi", "GZI", "valid", "index"),
    ] {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.id == required_id)
            .unwrap_or_else(|| panic!("fixture manifest is missing M9 fixture {required_id}"));

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
        assert!(
            fixtures_doc.contains(required_id)
                || fixture_matrix.contains(required_id)
                || coverage_map.contains(required_id)
                || m9_sphinx.contains(required_id),
            "fixture {required_id} is not documented in the M9 fixture plan"
        );
    }
}

#[test]
fn milestone_9_closeout_docs_are_consistent() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m9 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-09-bam-index-random-access.md"),
    );
    let m9_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_bam_index_random_access.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M9 roadmap", &m9),
        ("M9 Sphinx", &m9_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 9 is complete")
                || text.contains(
                    "Milestone 9: Native BAM Index And Random Access\n\n* status: complete"
                )
                || text.contains("Status: complete as of 2026-05-23"),
            "{name} does not record Milestone 9 completion"
        );
    }

    for required in [
        "M9.10",
        "index_bam",
        "check_index",
        "check_map_indexed",
        "summary_indexed",
        "full tests",
        "contract tests",
        "Sphinx",
        "benchmark smoke",
        "CSI writing",
        "deferred beyond M9",
    ] {
        assert!(
            current.contains(required)
                || m9.contains(required)
                || m9_sphinx.contains(required)
                || taskmap.contains(required)
                || readme.contains(required),
            "M9 closeout evidence is missing: {required}"
        );
    }

    for command in ["index", "check_index", "check_map", "summary"] {
        assert!(
            current.contains(&format!("`{command}`"))
                && m9.contains(&format!("`{command}`"))
                && m9_sphinx.contains(&format!("``{command}``")),
            "M9 closeout docs do not name command: {command}"
        );
    }
}

#[test]
fn milestone_10_activation_baseline_records_indexed_region_scope() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let sphinx_index = read_utf8(&docs_dir().join("sphinx").join("index.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("CLI docs", &cli),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M10 roadmap", &m10),
        ("M10 Sphinx", &m10_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 10 is active")
                || text.contains("status: active")
                || text.contains("Status: active as of 2026-05-23")
                || text.contains("Milestone 10 is complete")
                || text.contains("status: complete")
                || text.contains("Status: complete as of 2026-05-23"),
            "{name} does not record Milestone 10 activation or completion"
        );
    }

    for required in [
        "Milestone 9 closeout",
        "M10.1",
        "src/bam/index.rs",
        "src/bgzf/reader.rs",
        "src/bam/scan.rs",
        "src/commands/check_map.rs",
        "src/commands/summary.rs",
        "VirtualOffset",
        "raw_records_in_virtual_range",
        "bai_bin_for_region",
        "valid coordinate BAM/BAI",
        "stale BAI",
        "malformed BAI",
        "mismatched-reference BAI",
        "CSI-header",
        "region syntax",
        "region-file",
        "BAI chunk planning",
        "overlapping-region",
        "multi-reference semantics",
        "indexed-region benchmark",
        "CSI large-reference",
        "benchmark",
        "fastq",
        "unmap",
    ] {
        assert!(
            current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required)
                || readme.contains(required)
                || cli.contains(required),
            "M10 activation baseline evidence is missing: {required}"
        );
    }

    assert!(
        sphinx_index.contains("native_indexed_region_workflows"),
        "Sphinx index does not include the M10 technical note"
    );
}

#[test]
fn milestone_10_closeout_docs_are_consistent() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M10 roadmap", &m10),
        ("M10 Sphinx", &m10_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 10 is complete")
                || text.contains(
                    "Milestone 10: Native Indexed Region Workflows\n\n* status: complete",
                )
                || text.contains("Status: complete as of 2026-05-23"),
            "{name} does not record Milestone 10 completion"
        );
    }

    for required in [
        "M10.10",
        "M10.1 through M10.10",
        "src/bam/region.rs",
        "src/bam/region_plan.rs",
        "src/bam/region_traversal.rs",
        "check_map --region <REGION>",
        "summary --region <REGION>",
        "read-only public",
        "scanner_microbench --bamana-bin",
        "check_map_region_scan_fallback",
        "summary_region_scan_fallback",
        "check_map_region_indexed",
        "summary_region_indexed",
        "1/1",
        "cargo test",
        "cargo test --test contract",
        "Sphinx HTML build",
        "git diff --check",
        "direct `noodles` imports",
        "CRAM compatibility",
        "selected-record output",
    ] {
        assert!(
            readme.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required),
            "M10 closeout evidence is missing: {required}"
        );
    }
}

#[test]
fn post_milestone_10_roadmap_records_m11_through_m15() {
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let milestone_specs = [
        (
            "Milestone 10: Native Indexed Region Workflows",
            "status: complete",
            "roadmap/milestone-10-indexed-region-workflows.md",
        ),
        (
            "Milestone 11: Public Indexed Region Selection And Region Files",
            "status: complete",
            "roadmap/milestone-11-indexed-region-selection.md",
        ),
        (
            "Milestone 12: Extended Index Compatibility",
            "status: active",
            "roadmap/milestone-12-extended-index-compatibility.md",
        ),
        (
            "Milestone 13: Native CRAM Strategy And Compatibility Boundary",
            "status: planned",
            "roadmap/milestone-13-native-cram-strategy.md",
        ),
        (
            "Milestone 14: Interoperability And Benchmark Evidence",
            "status: planned",
            "roadmap/milestone-14-interop-benchmark-evidence.md",
        ),
        (
            "Milestone 15: Release Hardening And Public Contract Freeze",
            "status: planned",
            "roadmap/milestone-15-release-hardening.md",
        ),
    ];

    for (title, status, detail) in milestone_specs {
        assert!(roadmap.contains(title), "roadmap is missing {title}");
        assert!(
            roadmap.contains(status),
            "roadmap is missing {status} for {title}"
        );
        assert!(
            roadmap.contains(detail),
            "roadmap is missing detail link {detail}"
        );

        let detail_text = read_utf8(&docs_dir().join(detail));
        assert!(
            detail_text.contains(title),
            "detail page does not contain title {title}"
        );
        assert!(
            detail_text.contains("Ten-Task Outline") || title.contains("Milestone 10"),
            "planned detail page for {title} does not include a ten-task outline"
        );
    }

    for required in [
        "Milestone 11",
        "complete",
        "does not reopen Milestone 10",
        "selected-record region output",
        "region-file input",
        "header preservation",
        "write-safety",
    ] {
        assert!(
            current.contains(required)
                || roadmap.contains(required)
                || read_utf8(
                    &docs_dir()
                        .join("roadmap")
                        .join("milestone-11-indexed-region-selection.md")
                )
                .contains(required),
            "post-M10 roadmap is missing: {required}"
        );
    }
}

#[test]
fn milestone_12_activation_baseline_records_index_compatibility_scope() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m12 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-12-extended-index-compatibility.md"),
    );
    let m12_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_extended_index_compatibility.rst"),
    );
    let sphinx_index = read_utf8(&docs_dir().join("sphinx").join("index.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("CLI docs", &cli),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M12 roadmap", &m12),
        ("M12 Sphinx", &m12_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 12 is active")
                || text.contains("status: active")
                || text.contains("Status: active as of 2026-05-28")
                || text.contains("Status: complete."),
            "{name} does not record Milestone 12 activation"
        );
    }

    for required in [
        "M12.1",
        "Extended Index Compatibility",
        "does not reopen Milestone 11",
        "BAI detection",
        "structural validation",
        "mapped/unmapped metadata",
        "timestamp-staleness",
        "native BAI writing",
        "coordinate-sorted BAM",
        "check_index",
        "BAI, CSI, GZI, and unknown sidecars",
        "--prefer-csi",
        "CSI",
        "header-only",
        "detected-but-not-supported",
        "index --format csi",
        "explicitly unimplemented",
        "check_map",
        "summary",
        "select_region",
        "BAI-first",
        "native scan fallback",
        "missing, stale, unsupported, malformed, or incomplete",
        "FASTQ.GZI",
        "FASTQ.GZ planning sidecar",
        "not a BAM random-access index",
        "CSI support levels",
        "large-reference thresholds",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m12.contains(required)
                || m12_sphinx.contains(required)
                || taskmap.contains(required),
            "M12 activation baseline is missing: {required}"
        );
    }

    assert!(
        sphinx_index.contains("native_extended_index_compatibility"),
        "Sphinx index does not include the M12 technical note"
    );
}

#[test]
fn milestone_12_2_freezes_csi_support_levels() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_docs = read_utf8(&docs_dir().join("json-output.md"));
    let cli_contract = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m12 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-12-extended-index-compatibility.md"),
    );
    let m12_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_extended_index_compatibility.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let check_index_schema = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("jsonschema")
            .join("check_index.schema.json"),
    );
    let check_index_success = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("examples")
            .join("check_index.success.json"),
    );
    let check_index_failure = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("examples")
            .join("check_index.failure.json"),
    );

    for required in [
        "M12.2",
        "CSI Support-Level Freeze",
        "detect-only",
        "support_level",
        "read_write",
        "detect_only",
        "planning_sidecar",
        "unsupported",
        "absent",
        "index --format csi",
        "unimplemented",
        "check_map",
        "summary",
        "select_region",
        "native scan fallback",
        "CSI bin parsing",
        "CSI writing",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_docs.contains(required)
                || cli_contract.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m12.contains(required)
                || m12_sphinx.contains(required)
                || taskmap.contains(required)
                || check_index_schema.contains(required)
                || check_index_success.contains(required)
                || check_index_failure.contains(required),
            "M12.2 CSI support-level freeze is missing: {required}"
        );
    }

    assert!(
        check_index_schema.contains("\"support_level\"")
            && check_index_schema.contains("\"read_write\"")
            && check_index_schema.contains("\"detect_only\"")
            && check_index_schema.contains("\"planning_sidecar\"")
            && check_index_schema.contains("\"unsupported\"")
            && check_index_schema.contains("\"absent\""),
        "check_index schema does not govern support_level values"
    );
    assert!(
        check_index_success.contains("\"support_level\": \"read_write\"")
            && check_index_failure.contains("\"support_level\": \"absent\""),
        "check_index examples do not include support_level"
    );
}

#[test]
fn milestone_12_3_and_12_4_freeze_large_reference_and_diagnostics() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_docs = read_utf8(&docs_dir().join("json-output.md"));
    let cli_contract = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m12 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-12-extended-index-compatibility.md"),
    );
    let m12_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_extended_index_compatibility.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let check_map_schema = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("jsonschema")
            .join("check_map.schema.json"),
    );
    let check_map_example = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("examples")
            .join("check_map.success.json"),
    );

    for required in [
        "M12.3",
        "M12.4",
        "536,870,912",
        "large-reference",
        "CSI remains detect-only",
        "not a large-reference fallback",
        "diagnostic_status",
        "diagnostic_detail",
        "usable",
        "absent",
        "stale",
        "unsupported",
        "malformed",
        "mismatched_reference",
        "incomplete",
        "disabled",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_docs.contains(required)
                || cli_contract.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m12.contains(required)
                || m12_sphinx.contains(required)
                || taskmap.contains(required)
                || check_map_schema.contains(required)
                || check_map_example.contains(required),
            "M12.3/M12.4 contract evidence is missing: {required}"
        );
    }

    assert!(
        check_map_schema.contains("\"diagnostic_status\"")
            && check_map_schema.contains("\"mismatched_reference\"")
            && check_map_schema.contains("\"diagnostic_detail\""),
        "check_map schema does not govern machine-readable index diagnostics"
    );
    assert!(
        check_map_example.contains("\"diagnostic_status\": \"usable\""),
        "check_map success example does not include the usable diagnostic"
    );
}

#[test]
fn milestone_12_5_extends_index_fixture_plan() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let fixtures_doc = read_utf8(&docs_dir().join("fixtures.md"));
    let fixture_matrix = read_utf8(
        &super::repo_root()
            .join("tests")
            .join("fixtures")
            .join("plans")
            .join("fixture-matrix.md"),
    );
    let coverage_map = read_utf8(
        &super::repo_root()
            .join("tests")
            .join("fixtures")
            .join("plans")
            .join("coverage-map.md"),
    );
    let m12 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-12-extended-index-compatibility.md"),
    );
    let m12_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_extended_index_compatibility.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for required in [
        "M12.5",
        "tiny.invalid.large_reference.bam",
        "tiny.invalid.mismatched_reference_count.csi",
        "tiny.invalid.fastq_gz.bad_gzi",
        "BAI large-reference rejection",
        "CSI reference-count mismatch",
        "malformed FASTQ.GZI",
        "planner-sidecar",
        "support-level vocabulary",
        "diagnostic vocabulary",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || fixtures_doc.contains(required)
                || fixture_matrix.contains(required)
                || coverage_map.contains(required)
                || m12.contains(required)
                || m12_sphinx.contains(required)
                || taskmap.contains(required),
            "M12.5 fixture extension evidence is missing: {required}"
        );
    }

    let manifest = load_fixture_manifest();
    for (required_id, expected_format, expected_validity, expected_command, expected_artifact) in [
        (
            "tiny.invalid.large_reference.bam",
            "BAM",
            "invalid",
            "index",
            "expected/index/tiny.invalid.large_reference.bai.failure.json",
        ),
        (
            "tiny.invalid.mismatched_reference_count.csi",
            "CSI",
            "invalid",
            "check_index",
            "expected/check_index/tiny.invalid.mismatched_reference_count.csi.failure.json",
        ),
        (
            "tiny.invalid.fastq_gz.bad_gzi",
            "GZI",
            "invalid",
            "explode",
            "expected/explode/tiny.invalid.fastq_gz.bad_gzi.failure.json",
        ),
    ] {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.id == required_id)
            .unwrap_or_else(|| panic!("fixture manifest is missing M12.5 fixture {required_id}"));

        assert_eq!(fixture.format, expected_format);
        assert_eq!(fixture.validity, expected_validity);
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
                .any(|artifact| artifact == expected_artifact),
            "fixture {required_id} is missing expected artifact {expected_artifact}"
        );
    }
}

#[test]
fn milestone_12_6_wires_detect_only_csi_region_behavior() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_output = read_utf8(&docs_dir().join("json-output.md"));
    let cli_contract = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m12 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-12-extended-index-compatibility.md"),
    );
    let m12_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_extended_index_compatibility.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for required in [
        "M12.6",
        "Read-Only CSI Region Behavior",
        "detect-only CSI",
        "check_map --region",
        "summary --region",
        "native scan fallback",
        "index.kind: CSI",
        "index.diagnostic_status: unsupported",
        "index_derived.kind: CSI",
        "BAI remains the only index kind",
        "usable non-stale BAI",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_output.contains(required)
                || cli_contract.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m12.contains(required)
                || m12_sphinx.contains(required)
                || taskmap.contains(required),
            "M12.6 CSI region behavior evidence is missing: {required}"
        );
    }
}

#[test]
fn milestone_11_activation_baseline_records_selection_scope() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let sphinx_index = read_utf8(&docs_dir().join("sphinx").join("index.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("CLI docs", &cli),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M11 roadmap", &m11),
        ("M11 Sphinx", &m11_sphinx),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 11 is active")
                || text.contains("status: active")
                || text.contains("Status: active as of 2026-05-28")
                || text.contains("Milestone 11 is complete")
                || text.contains("status: complete")
                || text.contains("Status: complete as of 2026-05-28"),
            "{name} does not record Milestone 11 activation"
        );
    }

    for required in [
        "M11.1",
        "select_region",
        "new planned public command",
        "must not overload",
        "check_map",
        "summary",
        "subsample",
        "No new CLI behavior",
        "No public `select_region` CLI synopsis",
        "output semantics",
        "header preservation",
        "record ordering",
        "duplicate-region behavior",
        "region-file syntax",
        "index invalidation",
        "write-safety",
        "src/bam/region.rs",
        "src/bam/region_plan.rs",
        "src/bam/region_traversal.rs",
        "src/bam/write.rs",
        "src/output_safety.rs",
        "native CRAM indexed queries",
        "CSI large-reference support",
        "broad external comparator parity",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || taskmap.contains(required),
            "M11 activation baseline is missing: {required}"
        );
    }

    assert!(
        sphinx_index.contains("native_indexed_region_selection"),
        "Sphinx index does not include the M11 technical note"
    );
    assert!(
        m11.contains("The command is not implemented by M11.1")
            && taskmap.contains("M11.6 Implement Native Selected-Record Writing"),
        "M11.1 must remain recorded as a historical no-CLI activation task"
    );
}

#[test]
fn milestone_11_region_file_contract_is_specified_without_cli_behavior() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let region_source = read_utf8(&super::repo_root().join("src").join("bam").join("region.rs"));

    for required in [
        "M11.2",
        "Region-File Syntax",
        "UTF-8",
        "LF",
        "CRLF",
        "one M10-style region per non-comment line",
        "reference:start-end",
        "1-based closed",
        "0-based half-open",
        "blank lines",
        "first non-whitespace character is",
        "#",
        "Inline comments are not recognized",
        "Request order is preserved",
        "duplicate lines",
        "overlapping intervals",
        "must not sort, merge, or deduplicate",
        "region file path and 1-based line number",
        "empty or comment-only files",
        "unknown references",
        "ambiguous references",
        "zero-length whole-reference",
        "empty intervals",
        "zero coordinates",
        "reversed intervals",
        "out-of-range intervals",
        "non-numeric coordinates",
        "BED-like",
        "0-based half-open",
        "open-ended ranges",
        "comma-separated ranges",
        "strand/name columns",
        "tabular metadata",
        "No JSON schema or example output is introduced by M11.2",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || taskmap.contains(required),
            "M11.2 region-file contract is missing: {required}"
        );
    }

    for required in [
        "Region-file parsing is specified for M11.2",
        "not public CLI behavior until select_region is implemented",
        "provide ordered --region strings instead",
    ] {
        assert!(
            region_source.contains(required),
            "native region-file rejection text is missing: {required}"
        );
    }

    assert!(
        m11.contains("M11.2 specifies the region-file contract")
            && taskmap.contains("M11.6 Implement Native Selected-Record Writing"),
        "M11.2 must remain recorded as syntax-only before the M11.6 implementation task"
    );
}

#[test]
fn milestone_11_selected_record_output_contract_is_specified_without_cli_behavior() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for required in [
        "M11.3",
        "Selected-Record Output",
        "bamana select_region --bam <input.bam>",
        "--out <output.bam|->",
        "--report <report.json>",
        "BAM-only",
        "BGZF-compressed BAM input",
        "BGZF-compressed BAM output",
        "SAM",
        "CRAM",
        "FASTQ",
        "FASTQ.GZ",
        "FASTA",
        "unknown inputs",
        "text",
        "uncompressed BAM",
        "alternate compression",
        "Applied runs require explicit `--out`",
        "JSON report to stdout",
        "--out -",
        "binary BGZF BAM to stdout",
        "requires `--report <report.json>`",
        "reject `--report -`",
        "Human diagnostics must use stderr",
        "Dry runs write no BAM output",
        "dry_run: true",
        "output_created: false",
        "requested output target",
        "region source",
        "intended execution mode",
        "planned rejection or fallback state",
        "command: \"select_region\"",
        "output_format: \"bam\"",
        "compression: \"bgzf\"",
        "normalized region count",
        "selected-record counts",
        "records examined",
        "chunks traversed",
        "fallback reason",
        "biological interpretation",
        "whole-file validation",
        "M11.3 does not add a JSON schema",
        "M11.8 must add",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || taskmap.contains(required),
            "M11.3 selected-record output contract is missing: {required}"
        );
    }

    assert!(
        m11.contains("M11.3 specifies selected-record output semantics")
            && taskmap.contains("M11.6 Implement Native Selected-Record Writing"),
        "M11.3 must remain recorded as output-contract-only before the M11.6 implementation task"
    );
}

#[test]
fn milestone_11_header_sort_contract_is_specified_without_cli_behavior() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for required in [
        "M11.4",
        "Header Preservation",
        "Sort Order",
        "reference dictionary exactly",
        "binary reference dictionary order",
        "names, lengths, and reference indexes",
        "without filtering to selected references",
        "references with no selected records remain",
        "Textual `@SQ` records are preserved",
        "reconcile with the binary reference dictionary",
        "`@RG`",
        "existing `@PG`",
        "`@CO`",
        "unknown SAM-style header records",
        "only permitted selected-output header mutation",
        "append a new `@PG` record",
        "bamana select_region",
        "collision-free `ID`",
        "PN:bamana",
        "Bamana version",
        "set `PP`",
        "program chain is unambiguous",
        "Unrelated header records must not be removed",
        "Sort metadata is conservative",
        "rewrites `SO` to `unknown`",
        "removes `SS`",
        "does not require synthesizing",
        "input `@HD` `SO`/`SS`",
        "output `@HD` `SO`/`SS`",
        "sort-order metadata was downgraded",
        "not guaranteed to preserve whole-file order",
        "M11.4 does not add",
        "M11.8 must add",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || taskmap.contains(required),
            "M11.4 header/sort contract is missing: {required}"
        );
    }

    assert!(
        m11.contains("M11.4 specifies the future selected-output header contract")
            && taskmap.contains("M11.6 Implement Native Selected-Record Writing"),
        "M11.4 must remain recorded as header-contract-only before the M11.6 implementation task"
    );
}

#[test]
fn milestone_11_duplicate_overlap_contract_is_specified_without_cli_behavior() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for required in [
        "M11.5",
        "Duplicate And Overlapping Regions",
        "each physical BAM alignment record at most once",
        "source BAM virtual-offset range",
        "Repeated region strings",
        "repeated region-file lines",
        "overlapping intervals",
        "adjacent BAI chunks",
        "broad bins",
        "must not duplicate",
        "deterministic source order",
        "ascending source BAM virtual-offset order",
        "native BAM encounter order",
        "Region request order",
        "region-file line order",
        "do not control output ordering",
        "be repeated",
        "Multi-reference requests",
        "one source-order output stream",
        "all matched normalized region identifiers",
        "selected BAM output still contains one copy",
        "requested region count",
        "normalized region count",
        "duplicate region request count",
        "overlapping region request count",
        "selected unique record count",
        "duplicate physical records suppressed",
        "source_virtual_offset_order",
        "emit_once_per_source_record",
        "request-order repeated output",
        "per-region BAM blocks",
        "one output file per region",
        "M11.5 does not add schemas, examples, or fixtures",
        "M11.8 must add",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || taskmap.contains(required),
            "M11.5 duplicate/overlap contract is missing: {required}"
        );
    }

    assert!(
        m11.contains("M11.5 specifies record ordering")
            && taskmap.contains("M11.6 Implement Native Selected-Record Writing"),
        "M11.5 must remain recorded as duplicate-policy-only before the M11.6 implementation task"
    );
}

#[test]
fn milestone_11_selected_record_writing_is_implemented_natively() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let public_commands = read_utf8(&docs_dir().join("sphinx").join("public_commands.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let cli_source = read_utf8(&super::repo_root().join("src").join("cli.rs"));
    let command_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("commands")
            .join("select_region.rs"),
    );

    for required in [
        "M11.6",
        "Native Selected-Record Writing",
        "select_region",
        "--bam",
        "--region",
        "--out",
        "--dry-run",
        "--force",
        "--prefer-index",
        "BGZF-compressed BAM",
        "raw BAM record bytes",
        "native indexed traversal",
        "native scan",
        "source_virtual_offset_order",
        "emit_once_per_source_record",
        "SO:unknown",
        "@PG",
        "--out -",
        "--region-file",
        "M11.7",
        "M11.8",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || public_commands.contains(required)
                || taskmap.contains(required),
            "M11.6 selected-record writing docs are missing: {required}"
        );
    }

    for token in [
        "SelectRegion(SelectRegionArgs)",
        "pub struct SelectRegionArgs",
        "long = \"bam\"",
        "long = \"out\"",
        "long = \"region\"",
        "long = \"dry-run\"",
        "long = \"force\"",
        "long = \"prefer-index\"",
    ] {
        assert!(
            cli_source.contains(token),
            "M11.6 CLI implementation is missing token: {token}"
        );
    }

    for token in [
        "traverse_planned_region_chunks",
        "BgzfWriter",
        "writer.write_all(&record.raw_record)",
        "serialize_bam_header_payload",
        "finalize_completed_output",
        "SelectRegionExecutionMode::Indexed",
        "SelectRegionExecutionMode::ScanFallback",
        "source_virtual_offset_order",
        "emit_once_per_source_record",
    ] {
        assert!(
            command_source.contains(token),
            "M11.6 native selected-record writer is missing token: {token}"
        );
    }

    assert!(
        !command_source.contains("noodles"),
        "select_region production command path must stay free of direct noodles imports"
    );
}

#[test]
fn milestone_11_write_safety_and_index_invalidation_are_implemented() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let public_commands = read_utf8(&docs_dir().join("sphinx").join("public_commands.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let command_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("commands")
            .join("select_region.rs"),
    );

    for required in [
        "M11.7",
        "Write-Safety",
        "Index Invalidation",
        "same-path input/output",
        "Existing BAM output paths",
        "Adjacent output index sidecars",
        "<out>.bai",
        "<out>.csi",
        "output_exists",
        "removed before",
        "output.index_invalidation",
        "adjacent_index_paths",
        "preexisting_index_paths",
        "removed_index_paths",
        "invalidation_action",
        "output_index_created: false",
        "bamana index --input <output.bam>",
        "Dry runs",
        "M11.8",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || public_commands.contains(required)
                || taskmap.contains(required),
            "M11.7 write-safety/index-invalidation docs are missing: {required}"
        );
    }

    for token in [
        "SelectRegionIndexInvalidation",
        "prepare_output_index_invalidation",
        "output_index_candidate_paths",
        "existing_paths_are_same_file",
        "UnsupportedInputForCommand",
        "OutputExists",
        "fs::remove_file(path)",
        "removed_existing_index_sidecars",
        "would_remove_existing_index_sidecars_on_apply",
        "output_index_created: false",
        "bamana index --input",
        "existing_output_index_sidecar_requires_force",
        "force_removes_existing_output_index_sidecar_before_writing",
        "dry_run_reports_index_invalidation_without_removing_sidecar",
        "same_path_input_output_is_rejected",
    ] {
        assert!(
            command_source.contains(token),
            "M11.7 select_region implementation is missing token: {token}"
        );
    }
}

#[test]
fn milestone_11_selection_contract_artifacts_are_governed() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let spec = read_utf8(
        &super::repo_root()
            .join("spec")
            .join("cli")
            .join("commands.md"),
    );
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let public_commands = read_utf8(&docs_dir().join("sphinx").join("public_commands.rst"));
    let fixtures_doc = read_utf8(&docs_dir().join("fixtures.md"));
    let coverage_map = read_utf8(&fixtures_dir().join("plans").join("coverage-map.md"));
    let expected_readme = read_utf8(
        &fixtures_dir()
            .join("expected")
            .join("select_region")
            .join("README.md"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    let schema_path = schema_path_for_command("select_region");
    let success_example_path = spec_dir()
        .join("examples")
        .join("select_region.success.json");
    let failure_example_path = spec_dir()
        .join("examples")
        .join("select_region.failure.json");

    assert!(
        schema_path.exists(),
        "M11.8 must ship select_region.schema.json"
    );
    assert!(
        success_example_path.exists(),
        "M11.8 must ship select_region.success.json"
    );
    assert!(
        failure_example_path.exists(),
        "M11.8 must ship select_region.failure.json"
    );

    for required in [
        "M11.8",
        "select_region",
        "select_region.schema.json",
        "select_region.success.json",
        "select_region.failure.json",
        "output.index_invalidation",
        "region_scope",
        "execution",
        "header",
        "indexed",
        "scan_fallback",
        "emit_once_per_source_record",
        "source_virtual_offset_order",
        "--out -",
        "--region-file",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_doc.contains(required)
                || spec.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || public_commands.contains(required)
                || fixtures_doc.contains(required)
                || coverage_map.contains(required)
                || expected_readme.contains(required)
                || taskmap.contains(required),
            "M11.8 select_region contract docs are missing: {required}"
        );
    }

    let schema = read_utf8(&schema_path);
    for token in [
        "x-bamana-command",
        "select_region",
        "indexInvalidation",
        "normalizedRegion",
        "output_index_created",
        "emit_once_per_source_record",
        "source_virtual_offset_order",
        "indexed",
        "scan_fallback",
    ] {
        assert!(
            schema.contains(token),
            "M11.8 select_region schema is missing token: {token}"
        );
    }

    for path in [&success_example_path, &failure_example_path] {
        let example: Value = serde_json::from_str(&read_utf8(path))
            .unwrap_or_else(|error| panic!("example {} did not parse: {error}", path.display()));
        assert_eq!(
            example.pointer("/command").and_then(Value::as_str),
            Some("select_region"),
            "example {} must be bound to select_region",
            path.display()
        );
    }

    let manifest = load_fixture_manifest();
    let fixture = manifest
        .fixtures
        .iter()
        .find(|fixture| fixture.id == "tiny.select_region.coordinate")
        .unwrap_or_else(|| {
            panic!("fixture manifest is missing planned M11.8 select_region fixture")
        });

    assert_eq!(fixture.format, "BAM");
    assert!(
        fixture
            .primary_commands
            .iter()
            .any(|command| command == "select_region"),
        "planned select_region fixture must list select_region as a primary command"
    );
    assert!(
        fixture
            .expected_artifacts
            .iter()
            .all(|artifact| artifact.starts_with("expected/select_region/")),
        "planned select_region fixture expected artifacts must live under expected/select_region/"
    );

    for planned in [
        "indexed selected-output success",
        "scan fallback",
        "duplicate/overlap suppression",
        "output-index sidecar collision",
        "forced sidecar removal",
        "same-path rejection",
    ] {
        assert!(
            coverage_map.contains(planned) || expected_readme.contains(planned),
            "M11.8 select_region fixture plan is missing scenario: {planned}"
        );
    }
}

#[test]
fn milestone_11_dependency_and_benchmark_guardrails_are_documented() {
    let dependency_tests = read_utf8(
        &super::repo_root()
            .join("tests")
            .join("contract")
            .join("dependency_boundary.rs"),
    );
    let scanner_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("bin")
            .join("scanner_microbench.rs"),
    );
    let scanner_doc = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("scanner_microbenchmarks.rst"),
    );
    let scanner_schema = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("scanner_microbench.schema.json"),
    );
    let benchmark_results = read_utf8(
        &super::repo_root()
            .join("benchmarks")
            .join("results")
            .join("README.md"),
    );
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for substrate in [
        "select_region_scan_fallback_output",
        "select_region_indexed_output",
        "select_region_header_and_index_invalidation",
    ] {
        assert!(
            dependency_tests.contains(&format!("\"{substrate}\"")),
            "M11 dependency guardrails do not explicitly name substrate: {substrate}"
        );
    }

    for command in [
        "select_region_scan_fallback",
        "select_region_indexed_output",
    ] {
        assert!(
            scanner_source.contains(&format!("\"{command}\""))
                && scanner_doc.contains(&format!("``{command}``"))
                && scanner_schema.contains(&format!("\"{command}\""))
                && benchmark_results.contains(&format!("`{command}`")),
            "M11 benchmark hooks do not explicitly name command timing row: {command}"
        );
    }

    for required in [
        "select_region --bam <input.bam>",
        "production direct `noodles` imports remain limited",
        "scan fallback",
        "BAI chunk planning",
        "random-access traversal",
        "selected-record filtering",
        "duplicate suppression",
        "raw-record preservation",
        "BGZF BAM file writing",
        "temporary-output finalization",
        "header provenance",
        "index-invalidation reporting",
        "command startup",
        "JSON emission",
        "stdout-output evidence",
        "public region-file evidence",
        "replacement output-index evidence",
        "comparator-parity claims",
        "native CRAM indexed-query support",
        "biological interpretation",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || scanner_source.contains(required)
                || scanner_doc.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || taskmap.contains(required),
            "M11 benchmark interpretation notes are missing: {required}"
        );
    }
}

#[test]
fn milestone_11_closeout_docs_are_consistent() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let roadmap = read_utf8(&docs_dir().join("roadmap.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m11 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-11-indexed-region-selection.md"),
    );
    let m11_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_selection.rst"),
    );
    let public_commands = read_utf8(&docs_dir().join("sphinx").join("public_commands.rst"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for (name, text) in [
        ("README", &readme),
        ("CLI docs", &cli),
        ("roadmap", &roadmap),
        ("current milestone", &current),
        ("M11 roadmap", &m11),
        ("M11 Sphinx", &m11_sphinx),
        ("public commands", &public_commands),
        ("task map", &taskmap),
    ] {
        assert!(
            text.contains("Milestone 11 is complete")
                || text.contains(
                    "Milestone 11: Public Indexed Region Selection And Region Files\n\n* status: complete",
                )
                || text.contains("Status: complete as of 2026-05-28")
                || text.contains("Status: complete."),
            "{name} does not record Milestone 11 completion"
        );
    }

    for required in [
        "M11.10",
        "M11.1 through M11.10",
        "select_region",
        "BGZF BAM file output",
        "CLI `--region`",
        "native indexed traversal",
        "native scan fallback",
        "raw-record preservation",
        "source virtual-offset",
        "duplicate suppression",
        "header provenance",
        "index-invalidation",
        "spec/jsonschema/select_region.schema.json",
        "scanner_microbench --profile small --iterations 1 --bamana-bin",
        "select_region_scan_fallback",
        "select_region_indexed_output",
        "1/1",
        "cargo test",
        "cargo test --test contract",
        "cargo build --bin bamana --bin scanner_microbench",
        "Sphinx HTML",
        "cargo fmt --check",
        "git diff --check",
        "public `--region-file`",
        "`--out -`",
        "replacement output-index creation",
        "CSI large-reference behavior",
        "native CRAM indexed queries",
        "broad comparator parity",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || roadmap.contains(required)
                || current.contains(required)
                || m11.contains(required)
                || m11_sphinx.contains(required)
                || public_commands.contains(required)
                || taskmap.contains(required),
            "M11 closeout evidence is missing: {required}"
        );
    }
}

#[test]
fn milestone_10_region_syntax_contract_is_documented_and_native() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let region_source = read_utf8(&super::repo_root().join("src").join("bam").join("region.rs"));
    let bam_mod = read_utf8(&super::repo_root().join("src").join("bam").join("mod.rs"));
    let error_source = read_utf8(&super::repo_root().join("src").join("error.rs"));

    for required in [
        "src/bam/region.rs",
        "`reference`",
        "`reference:start-end`",
        "1-based closed",
        "0-based half-open",
        "preserve request order",
        "not merged",
        "deduplicated",
        "duplicate reference names",
        "unknown references",
        "zero coordinates",
        "reversed intervals",
        "out-of-range",
        "Region-file",
        "explicitly deferred",
        "no public command behavior change",
        "benchmark",
        "fastq",
        "unmap",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required),
            "M10.2 region syntax documentation is missing: {required}"
        );
    }

    for required in [
        "pub mod region",
        "pub struct NormalizedRegion",
        "pub struct NormalizedRegionSet",
        "pub fn normalize_region_string",
        "pub fn normalize_region_strings",
        "pub fn reject_region_file_request",
        "RegionInputKind",
        "input_1_based_closed_output_0_based_half_open",
        "preserve_request_order_without_merging_or_deduplication",
        "InvalidRegion",
        "\"invalid_region\"",
    ] {
        assert!(
            region_source.contains(required)
                || bam_mod.contains(required)
                || error_source.contains(required),
            "M10.2 native region source is missing: {required}"
        );
    }
}

#[test]
fn milestone_10_region_workflow_contracts_and_fixture_plans_are_frozen() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let fixture_doc = read_utf8(&docs_dir().join("fixtures.md"));
    let fixture_matrix = read_utf8(&fixtures_dir().join("plans").join("fixture-matrix.md"));
    let coverage_map = read_utf8(&fixtures_dir().join("plans").join("coverage-map.md"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let check_map_schema = read_utf8(&schema_path_for_command("check_map"));
    let summary_schema = read_utf8(&schema_path_for_command("summary"));
    let check_map_region_example = read_utf8(
        &spec_dir()
            .join("examples")
            .join("check_map.region.success.json"),
    );
    let summary_region_example = read_utf8(
        &spec_dir()
            .join("examples")
            .join("summary.region.success.json"),
    );

    for required in [
        "check_map --region <REGION>",
        "summary --region <REGION>",
        "region_scope",
        "cli_regions",
        "input_1_based_closed_output_0_based_half_open",
        "preserve_request_order_without_merging_or_deduplication",
        "indexed",
        "scan_fallback",
        "rejected",
        "standalone indexed selection command",
        "region files remain deferred",
        "whole-file mapping evidence",
        "full-file totals",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_doc.contains(required)
                || commands_doc.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required)
                || check_map_schema.contains(required)
                || summary_schema.contains(required)
                || check_map_region_example.contains(required)
                || summary_region_example.contains(required),
            "M10.3 region workflow contract is missing: {required}"
        );
    }

    for scenario in [
        "single-region indexed success",
        "multi-region indexed success",
        "overlapping-region request-order behavior",
        "unknown-reference rejection",
        "empty-region rejection",
        "stale-index scan fallback",
        "missing-index scan fallback",
        "unsupported-index scan fallback",
    ] {
        assert!(
            fixture_doc.contains(scenario)
                || fixture_matrix.contains(scenario)
                || coverage_map.contains(scenario)
                || taskmap.contains(scenario),
            "M10.3 fixture plan is missing scenario: {scenario}"
        );
    }

    for example in [&check_map_region_example, &summary_region_example] {
        assert!(example.contains("\"region_scope\""));
        assert!(example.contains("\"execution\": \"indexed\""));
        assert!(example.contains("\"original\": \"chr1:100-200\""));
    }
}

#[test]
fn milestone_10_indexed_selection_surface_is_explicitly_deferred() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let fixture_doc = read_utf8(&docs_dir().join("fixtures.md"));
    let coverage_map = read_utf8(&fixtures_dir().join("plans").join("coverage-map.md"));
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let cli_source = read_utf8(&super::repo_root().join("src").join("cli.rs"));

    for required in [
        "M10.8",
        "public indexed region selection command",
        "defer",
        "read-only evidence",
        "no command claims",
        "No public synopsis",
        "output semantics",
        "header preservation",
        "record ordering",
        "duplicate-region behavior",
        "index invalidation",
        "write-safety",
        "check_map --region <REGION>",
        "summary --region <REGION>",
        "Region files",
        "future selection work",
        "M10.2",
        "M10.4",
        "M10.5",
        "M10.6",
        "M10.7",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_doc.contains(required)
                || commands_doc.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || fixture_doc.contains(required)
                || coverage_map.contains(required)
                || taskmap.contains(required),
            "M10.8 indexed selection decision is missing: {required}"
        );
    }

    assert!(cli_source.contains("CheckMap"));
    assert!(cli_source.contains("Summary"));
    assert!(
        m10.contains("M10.8 deliberately defers a public indexed region selection command")
            && taskmap.contains("M11.6 Implement Native Selected-Record Writing"),
        "M10.8 must remain documented as a historical selected-output deferral"
    );
}

#[test]
fn milestone_10_chunk_planning_contract_is_documented_and_native() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let bam_mod = read_utf8(&super::repo_root().join("src").join("bam").join("mod.rs"));
    let index_source = read_utf8(&super::repo_root().join("src").join("bam").join("index.rs"));
    let planner_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("bam")
            .join("region_plan.rs"),
    );

    for required in [
        "src/bam/region_plan.rs",
        "bai_bins_for_region",
        "validated `BaiIndex`",
        "candidate BAI bins",
        "candidate chunks",
        "coalesced chunks",
        "index path",
        "index kind",
        "reference name",
        "reference index",
        "requested region strings",
        "unsupported index kinds",
        "stale BAI",
        "reference-count incompatibility",
        "impossible virtual-offset chunks",
        "No-hit intervals",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required),
            "M10.4 chunk planning documentation is missing: {required}"
        );
    }

    for required in [
        "pub mod region_plan",
        "pub fn bai_bins_for_region",
        "pub struct RegionChunkPlan",
        "pub struct ReferenceChunkPlan",
        "pub fn plan_bai_region_chunks",
        "candidate_bins",
        "candidate_chunks",
        "coalesced_chunks",
        "UnsupportedIndex",
        "MissingIndex",
        "InvalidIndex",
    ] {
        assert!(
            bam_mod.contains(required)
                || index_source.contains(required)
                || planner_source.contains(required),
            "M10.4 native chunk-planning source is missing: {required}"
        );
    }
}

#[test]
fn milestone_10_region_traversal_contract_is_documented_and_native() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let bam_mod = read_utf8(&super::repo_root().join("src").join("bam").join("mod.rs"));
    let traversal_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("bam")
            .join("region_traversal.rs"),
    );
    let scan_source = read_utf8(&super::repo_root().join("src").join("bam").join("scan.rs"));

    for required in [
        "src/bam/region_traversal.rs",
        "traverse_planned_region_chunks",
        "raw_records_in_virtual_range",
        "VirtualOffset",
        "normalized intervals",
        "overlap",
        "deduplicate by virtual-offset",
        "matched region strings",
        "NativeScanRequired",
        "scan fallback",
        "internal traversal baseline",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required),
            "M10.5 region traversal documentation is missing: {required}"
        );
    }

    for required in [
        "pub mod region_traversal",
        "pub struct RegionTraversalResult",
        "pub struct RegionMatchedRecord",
        "pub enum RegionFallback",
        "NativeScanRequired",
        "pub fn traverse_planned_region_chunks",
        "pub fn scan_fallback_required",
        "raw_records_in_virtual_range",
        "virtual_offsets.start.packed()",
        "matching_regions",
        "merge_region_matches",
        "does_not_double_count_overlapping_region_chunks",
    ] {
        assert!(
            bam_mod.contains(required)
                || traversal_source.contains(required)
                || scan_source.contains(required),
            "M10.5 native traversal source is missing: {required}"
        );
    }
}

#[test]
fn milestone_10_check_map_region_contract_is_documented_and_native() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let check_map_schema = read_utf8(&schema_path_for_command("check_map"));
    let check_map_region_example = read_utf8(
        &spec_dir()
            .join("examples")
            .join("check_map.region.success.json"),
    );
    let cli_source = read_utf8(&super::repo_root().join("src").join("cli.rs"));
    let check_map_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("commands")
            .join("check_map.rs"),
    );
    let index_source = read_utf8(&super::repo_root().join("src").join("bam").join("index.rs"));

    for required in [
        "check_map --region <REGION>",
        "region_scope.execution",
        "indexed",
        "scan_fallback",
        "index_path",
        "chunks_traversed",
        "raw_records_seen",
        "duplicate_records_suppressed",
        "fallback_mode",
        "native_scan_required",
        "scan_records_limit",
        "region_records_examined",
        "region_mapped_records_observed",
        "region_unmapped_records_observed",
        "invalid_region",
        "summary --region",
        "region files",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_doc.contains(required)
                || commands_doc.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required)
                || check_map_schema.contains(required)
                || check_map_region_example.contains(required),
            "M10.6 check_map region documentation or schema is missing: {required}"
        );
    }

    for required in [
        "pub regions: Vec<String>",
        "long = \"region\"",
        "normalize_region_strings",
        "plan_bai_region_chunks",
        "traverse_planned_region_chunks",
        "RegionScope",
        "RegionExecution",
        "parse_bai_index",
        "scan_region_mapping_records",
        "region_request_uses_indexed_traversal_when_bai_is_usable",
        "region_request_stale_bai_falls_back_to_scan",
    ] {
        assert!(
            cli_source.contains(required)
                || check_map_source.contains(required)
                || index_source.contains(required),
            "M10.6 native check_map region source is missing: {required}"
        );
    }
}

#[test]
fn milestone_10_summary_region_contract_is_documented_and_native() {
    let readme = read_utf8(&super::repo_root().join("README.md"));
    let cli = read_utf8(&docs_dir().join("cli.md"));
    let json_doc = read_utf8(&docs_dir().join("json-output.md"));
    let commands_doc = read_utf8(&spec_dir().join("cli").join("commands.md"));
    let current = read_utf8(&docs_dir().join("roadmap").join("current_milestone.md"));
    let m10 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-10-indexed-region-workflows.md"),
    );
    let m10_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_indexed_region_workflows.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));
    let summary_schema = read_utf8(&schema_path_for_command("summary"));
    let summary_region_example = read_utf8(
        &spec_dir()
            .join("examples")
            .join("summary.region.success.json"),
    );
    let cli_source = read_utf8(&super::repo_root().join("src").join("cli.rs"));
    let summary_source = read_utf8(
        &super::repo_root()
            .join("src")
            .join("commands")
            .join("summary.rs"),
    );

    for required in [
        "summary --region <REGION>",
        "region_scope.execution",
        "indexed",
        "scan_fallback",
        "index_path",
        "chunks_traversed",
        "raw_records_seen",
        "duplicate_records_suppressed",
        "fallback_mode",
        "native_scan_required",
        "scan_records_limit",
        "fractions_observed",
        "flag_categories",
        "whole-file BAI totals are intentionally omitted",
        "invalid_region",
        "region files",
    ] {
        assert!(
            readme.contains(required)
                || cli.contains(required)
                || json_doc.contains(required)
                || commands_doc.contains(required)
                || current.contains(required)
                || m10.contains(required)
                || m10_sphinx.contains(required)
                || taskmap.contains(required)
                || summary_schema.contains(required)
                || summary_region_example.contains(required),
            "M10.7 summary region documentation or schema is missing: {required}"
        );
    }

    for required in [
        "pub regions: Vec<String>",
        "long = \"region\"",
        "normalize_region_strings",
        "plan_bai_region_chunks",
        "traverse_planned_region_chunks",
        "RegionScope",
        "RegionExecution",
        "scan_region_summary_records",
        "region_summary_uses_indexed_traversal_when_bai_is_usable",
        "region_summary_stale_bai_falls_back_to_scan",
    ] {
        assert!(
            cli_source.contains(required) || summary_source.contains(required),
            "M10.7 native summary region source is missing: {required}"
        );
    }
}

#[test]
fn milestone_8_activation_baseline_records_command_evidence_and_gaps() {
    let m8 = read_utf8(
        &docs_dir()
            .join("roadmap")
            .join("milestone-08-transform-ingest.md"),
    );
    let m8_sphinx = read_utf8(
        &docs_dir()
            .join("sphinx")
            .join("native_transform_ingest.rst"),
    );
    let taskmap = read_utf8(&super::repo_root().join("taskmap.md"));

    for command in ["sort", "merge", "explode", "checksum", "consume"] {
        assert!(
            m8.contains(&format!("`{command}`"))
                && taskmap.contains(&format!("`{command}`"))
                && m8_sphinx.contains(&format!("``{command}``")),
            "M8 baseline docs do not name command: {command}"
        );
    }

    for required in [
        "M8.1 Baseline Audit",
        "JSON schemas",
        "canonical examples",
        "fixture reservations",
        "BamReader::open",
        "parse_bam_header_from_reader",
        "read_next_record_layout",
        "in-memory first-slice",
        "consume --verify-checksum",
        "FASTQ.GZI",
        "dependency-boundary tests",
        "command-level benchmark smoke hooks",
    ] {
        assert!(
            m8.contains(required) || taskmap.contains(required) || m8_sphinx.contains(required),
            "M8 baseline evidence is missing: {required}"
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
fn fixture_manifest_includes_m8_transform_ingest_baseline() {
    let manifest = load_fixture_manifest();
    let ids: BTreeSet<String> = manifest
        .fixtures
        .into_iter()
        .map(|fixture| fixture.id)
        .collect();

    for required_id in [
        "tiny.valid.queryname",
        "tiny.tags.nm_rg",
        "tiny.transforms.source",
        "tiny.transforms.shard1",
        "tiny.transforms.shard2",
        "tiny.transforms.merged",
        "tiny.valid.fastq_gz",
        "tiny.consume.mixed_alignment_raw",
        "tiny.consume.directory_tree",
    ] {
        assert!(
            ids.contains(required_id),
            "fixture manifest is missing required M8 transform/ingest fixture {required_id}"
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
