//! 缓存过长的聊天粘贴文本，并生成可传给 Agent 的文件引用。

use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use std::io::Write as _;

/// 超过此字符数的粘贴文本会保存到系统临时目录。
pub const PASTED_TEXT_FILE_REFERENCE_THRESHOLD_CHARS: usize = 500;

/// 将超过阈值的粘贴文本保存为私有临时文件。
///
/// 返回 `Ok(None)` 表示文本未超过阈值，或当前平台无法创建本地临时文件。
pub fn cache_large_pasted_text(text: &str) -> std::io::Result<Option<PathBuf>> {
    if text.chars().count() <= PASTED_TEXT_FILE_REFERENCE_THRESHOLD_CHARS {
        return Ok(None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut file = tempfile::Builder::new()
            .prefix("vibewindow-paste-")
            .suffix(".txt")
            .tempfile_in(std::env::temp_dir())?;
        file.write_all(text.as_bytes())?;
        let (_file, path) = file.keep().map_err(|error| error.error)?;
        Ok(Some(path))
    }

    #[cfg(target_arch = "wasm32")]
    {
        Ok(None)
    }
}

/// 将本地文件路径编码为聊天请求可识别的文档引用。
pub fn document_reference(path: &Path) -> String {
    format!("[DOCUMENT:{}]", path.display())
}

#[cfg(test)]
#[path = "pasted_text_tests.rs"]
mod tests;
