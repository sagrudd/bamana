use std::collections::BTreeSet;

use serde_json::Value;

use super::{
    command_name_from_example, command_schema_paths, docs_dir, example_paths, fixtures_dir,
    read_utf8, schema_dir, schema_path_for_command, spec_dir,
};
use crate::contract::support::fixture_manifest::load_fixture_manifest;

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
        fixtures_dir().join("consume").join("README.md"),
        fixtures_dir().join("forensics").join("README.md"),
        fixtures_dir().join("expected").join("README.md"),
        fixtures_dir()
            .join("expected")
            .join("consume")
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
        "tiny.invalid.bam.truncated_record.duplication",
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
