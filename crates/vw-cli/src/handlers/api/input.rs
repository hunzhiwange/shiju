//! 结构化 JSON 输入读取。

use std::io::Read;

use anyhow::{Context, Result};

/// 从文件或 `-` 表示的 stdin 读取一个 JSON 值。
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn read_json_input(source: &str, stdin: &mut impl Read) -> Result<serde_json::Value> {
    let (bytes, description) = if source == "-" {
        let mut bytes = Vec::new();
        stdin.read_to_end(&mut bytes).context("failed to read JSON input from stdin")?;
        (bytes, "stdin".to_string())
    } else {
        let bytes = std::fs::read(source)
            .with_context(|| format!("failed to read JSON input file {source}"))?;
        (bytes, format!("file {source}"))
    };

    serde_json::from_slice(&bytes).with_context(|| format!("invalid JSON input from {description}"))
}
