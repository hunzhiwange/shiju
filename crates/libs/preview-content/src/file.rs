//! 受限预览文件读取与类型识别；显示消息由调用方提供。

use crate::{PREVIEW_MAX_BYTES, format_text_preview_content, strip_ansi_and_controls, wrap_long_lines};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedPreviewKind {
    Archive,
    Pdf,
}

pub enum FilePreview {
    Image,
    Unsupported(UnsupportedPreviewKind),
    Binary,
    Text { content: String, truncated: bool },
}

pub fn unsupported_preview_kind(path: &str) -> Option<UnsupportedPreviewKind> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz") {
        Some(UnsupportedPreviewKind::Archive)
    } else if ext == "pdf" {
        Some(UnsupportedPreviewKind::Pdf)
    } else {
        None
    }
}

fn read_preview_sample(path: &str) -> Option<Vec<u8>> {
    use std::fs::File;
    use std::io::{Read, Take};

    const SAMPLE_BYTES: u64 = 4096;

    let file = File::open(path).ok()?;
    let mut sample = Vec::new();
    let mut take: Take<File> = file.take(SAMPLE_BYTES);
    take.read_to_end(&mut sample).ok()?;
    Some(sample)
}

pub fn file_looks_like_text(path: &str) -> bool {
    let Some(sample) = read_preview_sample(path) else {
        return true;
    };

    std::str::from_utf8(&sample).is_ok()
}

pub fn read_preview_text(path: &str) -> (String, bool) {
    /// 最大读取字节数（约 300KB）
    const MAX_BYTES: usize = 300_000;
    /// 每行最大字符数
    const MAX_LINE: usize = 512;
    /// 最大行数
    const MAX_LINES: usize = 5000;

    let mut truncated = false;
    let mut buf: Vec<u8> = Vec::new();

    // 尝试读取文件（限制字节数）
    let read_ok = (|| {
        use std::fs::File;
        use std::io::{BufReader, Read};

        let f = File::open(path)?;

        // 检查文件大小，标记是否需要截断
        if let Ok(m) = f.metadata()
            && m.len() > MAX_BYTES as u64
        {
            truncated = true;
        }

        let reader = BufReader::new(f);
        let mut take = reader.take(MAX_BYTES as u64);
        take.read_to_end(&mut buf)?;

        Ok::<(), std::io::Error>(())
    })()
    .is_ok();

    // 转换为字符串
    let content = if read_ok {
        String::from_utf8_lossy(&buf).to_string()
    } else {
        // 回退：尝试直接读取整个文件
        std::fs::read_to_string(path).unwrap_or_else(|_| String::new())
    };

    // 清理 ANSI 转义序列和控制字符
    let cleaned = strip_ansi_and_controls(&content);

    // 长行自动换行
    let wrapped = wrap_long_lines(&cleaned, MAX_LINE);

    // 限制行数
    let limited = {
        let mut out = String::new();
        for (line_index, line) in wrapped.lines().enumerate() {
            if line_index >= MAX_LINES {
                truncated = true;
                break;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    };

    (limited, truncated)
}

pub fn read_file_preview(path: &str) -> FilePreview {
    let mut truncated = false;

    // 获取文件扩展名（转小写）
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // 检测文件类型
    let is_image = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp");
    let is_svg = ext.as_str() == "svg";
    // 处理图片文件
    if is_image || is_svg {
        return FilePreview::Image;
    }

    if let Some(kind) = unsupported_preview_kind(path) {
        return FilePreview::Unsupported(kind);
    }

    if !file_looks_like_text(path) {
        return FilePreview::Binary;
    }

    let mut buf: Vec<u8> = Vec::new();

    // 尝试读取文件（限制字节数）
    let read_ok = (|| {
        use std::fs::File;
        use std::io::{BufReader, Read};

        let f = File::open(path)?;

        // 检查文件大小，标记是否需要截断
        if let Ok(m) = f.metadata()
            && m.len() > PREVIEW_MAX_BYTES as u64
        {
            truncated = true;
        }

        let reader = BufReader::new(f);
        let mut take = reader.take(PREVIEW_MAX_BYTES as u64);
        take.read_to_end(&mut buf)?;

        Ok::<(), std::io::Error>(())
    })()
    .is_ok();

    // 转换为字符串
    let content = if read_ok {
        String::from_utf8_lossy(&buf).to_string()
    } else {
        // 回退：尝试直接读取整个文件
        std::fs::read_to_string(path).unwrap_or_else(|_| String::new())
    };

    let (formatted, content_truncated) = format_text_preview_content(&content);
    FilePreview::Text { content: formatted, truncated: truncated || content_truncated }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
