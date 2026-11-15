//! 稳定 JSON 与 JSONL 输出。

use std::io::Write;

use anyhow::Result;
use serde::Serialize;

use super::ApiOutputFormat;

pub(super) fn write_json(
    writer: &mut impl Write,
    value: &impl Serialize,
    format: ApiOutputFormat,
) -> Result<()> {
    match format {
        ApiOutputFormat::Json => {
            serde_json::to_writer_pretty(&mut *writer, value)?;
            writer.write_all(b"\n")?;
        }
        ApiOutputFormat::Jsonl => write_jsonl_item(writer, value)?,
    }
    Ok(())
}

pub(super) fn write_jsonl_item(writer: &mut impl Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    Ok(())
}

/// 原样写出文本响应，不附加或移除换行。
pub(super) fn write_text(writer: &mut impl Write, value: &str) -> Result<()> {
    writer.write_all(value.as_bytes())?;
    Ok(())
}
