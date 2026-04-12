use std::{
    io::{self, Write},
    process::ExitCode,
};

use serde::Serialize;

pub fn emit_json<T>(response: &T, pretty: bool, ok: bool) -> ExitCode
where
    T: Serialize,
{
    let encoded = if pretty {
        serde_json::to_string_pretty(response)
    } else {
        serde_json::to_string(response)
    };

    match encoded {
        Ok(body) => {
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "{body}").is_ok() {
                if ok {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "failed to serialize response: {error}");
            ExitCode::from(1)
        }
    }
}
