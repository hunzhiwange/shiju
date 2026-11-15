//! CLI 命令的结构化 JSON 输出辅助。

use std::io::{self, Write};

use serde::Serialize;

use acp_types::{OutputErrorParams, OutputFormat, OutputPolicy};

pub fn write_output_error(
    stdout: &mut dyn Write,
    error: &OutputErrorParams,
    policy: &OutputPolicy,
) -> io::Result<()> {
    if !policy.json_strict {
        return Ok(());
    }
    serde_json::to_writer(
        &mut *stdout,
        &serde_json::json!({
            "error": error
        }),
    )
    .map_err(io::Error::other)?;
    stdout.write_all(b"\n")?;
    Ok(())
}

pub fn emit_json_result<T: Serialize>(
    stdout: &mut dyn Write,
    format: OutputFormat,
    payload: &T,
) -> io::Result<bool> {
    if format != OutputFormat::Json {
        return Ok(false);
    }

    serde_json::to_writer(&mut *stdout, payload).map_err(io::Error::other)?;
    stdout.write_all(b"\n")?;
    Ok(true)
}

#[cfg(test)]
#[path = "json_output_tests.rs"]
mod json_output_tests;
