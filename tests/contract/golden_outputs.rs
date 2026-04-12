use serde_json::Value;

use super::{command_name_from_example, example_paths, read_utf8};

#[test]
fn example_files_have_stable_envelope_shape() {
    for path in example_paths() {
        let contents = read_utf8(&path);
        let parsed: Value = serde_json::from_str(&contents)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
        let object = parsed
            .as_object()
            .unwrap_or_else(|| panic!("example {} is not a JSON object", path.display()));

        for key in ["ok", "command", "path", "data", "error"] {
            assert!(
                object.contains_key(key),
                "example {} is missing top-level key {key}",
                path.display()
            );
        }

        let expected_command = command_name_from_example(&path);
        let command = object
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("example {} has no string command field", path.display()));
        assert_eq!(
            command,
            expected_command,
            "example {} command field does not match filename",
            path.display()
        );

        let is_failure = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(".failure."));
        let ok = object
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| panic!("example {} has no boolean ok field", path.display()));

        if is_failure {
            assert!(
                !ok,
                "failure example {} unexpectedly has ok=true",
                path.display()
            );
            assert!(
                !object.get("error").is_some_and(Value::is_null),
                "failure example {} unexpectedly has null error",
                path.display()
            );
        } else {
            assert!(
                ok,
                "success example {} unexpectedly has ok=false",
                path.display()
            );
            assert!(
                object.get("error").is_some_and(Value::is_null),
                "success example {} unexpectedly has non-null error",
                path.display()
            );
        }
    }
}
