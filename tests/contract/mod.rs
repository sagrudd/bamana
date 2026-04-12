use std::{
    fs,
    path::{Path, PathBuf},
};

pub mod cli_contract;
pub mod golden_outputs;
pub mod json_contract;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn spec_dir() -> PathBuf {
    repo_root().join("spec")
}

pub fn schema_dir() -> PathBuf {
    spec_dir().join("jsonschema")
}

pub fn examples_dir() -> PathBuf {
    spec_dir().join("examples")
}

pub fn docs_dir() -> PathBuf {
    repo_root().join("docs")
}

pub fn fixtures_dir() -> PathBuf {
    repo_root().join("tests").join("fixtures")
}

pub fn command_schema_paths() -> Vec<PathBuf> {
    let mut paths = collect_json_files(&schema_dir());
    paths.retain(|path| path.parent() != Some(schema_dir().join("common").as_path()));
    paths.sort();
    paths
}

pub fn example_paths() -> Vec<PathBuf> {
    let mut paths = collect_json_files(&examples_dir());
    paths.sort();
    paths
}

pub fn collect_json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_json_files_recursive(dir, &mut files);
    files
}

fn collect_json_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("failed to read directory {}: {error}", dir.display());
    });

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to read directory entry in {}: {error}",
                dir.display()
            );
        });
        let path = entry.path();
        if path.is_dir() {
            collect_json_files_recursive(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
}

pub fn command_name_from_example(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split('.').next())
        .unwrap_or_else(|| panic!("invalid example filename: {}", path.display()))
        .to_owned()
}

pub fn schema_path_for_command(command: &str) -> PathBuf {
    schema_dir().join(format!("{command}.schema.json"))
}

pub fn read_utf8(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}
