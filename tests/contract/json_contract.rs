use std::collections::BTreeSet;

use serde_json::Value;

use super::{
    command_name_from_example, command_schema_paths, docs_dir, example_paths, fixtures_dir,
    read_utf8, schema_dir, schema_path_for_command, spec_dir,
};

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
        fixtures_dir().join("README.md"),
        fixtures_dir().join("bam").join("README.md"),
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
