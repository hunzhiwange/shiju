//! 结构化 JSON 输入读取。

use std::ffi::OsString;
use std::io::Read;

use anyhow::{Context, Result};

/// 从文件、`-` 表示的 stdin 或 `env:NAME` 环境变量读取一个 JSON 值。
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn read_json_input(source: &str, stdin: &mut impl Read) -> Result<serde_json::Value> {
    read_json_input_with_env(source, stdin, |name| std::env::var_os(name))
}

pub(super) fn read_json_input_with_env(
    source: &str,
    stdin: &mut impl Read,
    env_lookup: impl FnOnce(&str) -> Option<OsString>,
) -> Result<serde_json::Value> {
    let (bytes, description) = if source == "-" {
        let mut bytes = Vec::new();
        stdin.read_to_end(&mut bytes).context("failed to read JSON input from stdin")?;
        (bytes, "stdin".to_string())
    } else if let Some(variable) = source.strip_prefix("env:") {
        let variable = variable.trim();
        if variable.is_empty() {
            anyhow::bail!("JSON input environment variable name is empty");
        }
        let value = env_lookup(variable)
            .with_context(|| format!("JSON input environment variable {variable} is not set"))?
            .into_string()
            .map_err(|_| {
                anyhow::anyhow!("JSON input environment variable {variable} is not UTF-8")
            })?;
        (value.into_bytes(), format!("environment variable {variable}"))
    } else {
        let bytes = std::fs::read(source)
            .with_context(|| format!("failed to read JSON input file {source}"))?;
        (bytes, format!("file {source}"))
    };

    serde_json::from_slice(&bytes).with_context(|| format!("invalid JSON input from {description}"))
}
