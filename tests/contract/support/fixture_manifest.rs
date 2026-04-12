use serde::Deserialize;

use crate::contract::{fixtures_dir, read_utf8};

#[derive(Debug, Deserialize)]
pub struct FixtureManifest {
    pub suite_id: String,
    pub version: String,
    pub status: String,
    pub fixtures: Vec<FixtureEntry>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureEntry {
    pub id: String,
    pub path: String,
    pub format: String,
    pub category: String,
    pub validity: String,
    pub status: String,
    pub description: String,
    pub primary_commands: Vec<String>,
    pub secondary_commands: Vec<String>,
    pub generated: bool,
    pub source_fixture_ids: Vec<String>,
    pub regeneration_strategy: String,
    pub expected_artifacts: Vec<String>,
}

pub fn load_fixture_manifest() -> FixtureManifest {
    let path = fixtures_dir().join("manifest.json");
    let contents = read_utf8(&path);
    serde_json::from_str(&contents).unwrap_or_else(|error| {
        panic!(
            "failed to parse fixture manifest {}: {error}",
            path.display()
        )
    })
}
