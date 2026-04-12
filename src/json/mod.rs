use std::{
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

use serde::Serialize;
use serde_json::json;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct JsonError {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse<T>
where
    T: Serialize,
{
    pub ok: bool,
    pub command: String,
    pub path: Option<String>,
    pub data: Option<T>,
    pub error: Option<JsonError>,
}

impl<T> CommandResponse<T>
where
    T: Serialize,
{
    pub fn success(command: &str, path: Option<&Path>, data: T) -> Self {
        Self {
            ok: true,
            command: command.to_string(),
            path: path.map(path_to_string),
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(command: &str, path: Option<&Path>, error: AppError) -> Self {
        Self::failure_with_data(command, path, None, error)
    }

    pub fn failure_with_data(
        command: &str,
        path: Option<&Path>,
        data: Option<T>,
        error: AppError,
    ) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            path: path.map(path_to_string),
            data,
            error: Some(error.to_json_error()),
        }
    }

    pub fn from_result(command: &str, path: Option<&Path>, result: Result<T, AppError>) -> Self {
        match result {
            Ok(data) => Self::success(command, path, data),
            Err(error) => Self::failure(command, path, error),
        }
    }
}

pub fn emit_response<T>(response: &CommandResponse<T>, pretty: bool) -> ExitCode
where
    T: Serialize,
{
    let body = serialize_response(response, pretty);
    let mut stdout = io::stdout().lock();

    if writeln!(stdout, "{body}").is_err() {
        return ExitCode::from(1);
    }

    if response.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn serialize_response<T>(response: &CommandResponse<T>, pretty: bool) -> String
where
    T: Serialize,
{
    let attempt = if pretty {
        serde_json::to_string_pretty(response)
    } else {
        serde_json::to_string(response)
    };

    match attempt {
        Ok(body) => body,
        Err(error) => {
            let fallback_error = AppError::Internal {
                message: format!("failed to serialize response: {error}"),
            }
            .to_json_error();
            let fallback = json!({
                "ok": false,
                "command": "internal",
                "path": null,
                "data": null,
                "error": fallback_error
            });

            if pretty {
                serde_json::to_string_pretty(&fallback)
                    .unwrap_or_else(|_| "{\"ok\":false}".to_string())
            } else {
                serde_json::to_string(&fallback).unwrap_or_else(|_| "{\"ok\":false}".to_string())
            }
        }
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
