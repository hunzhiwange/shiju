//! 覆盖预览面板相关行为的回归测试。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::io::Write;

use super::*;
use preview_content::PREVIEW_MAX_BYTES;

fn write_temp_file(name: &str, bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new().prefix(name).tempfile().expect("create temp file");
    file.write_all(bytes).expect("write temp file");
    file.as_file().sync_all().expect("sync temp file");
    file
}

#[test]
fn preview_open_error_allows_utf8_text_files() {
    let file = write_temp_file("preview_text", b"fn main() {}\n");
    let path = file.path().to_string_lossy().to_string();

    assert_eq!(preview_open_error(&path), None);
}

#[test]
fn preview_open_error_blocks_binary_files() {
    let file = write_temp_file("preview_binary", &[0x50, 0x4B, 0x03, 0x04, 0x00, 0xFF, 0xAA, 0x00]);
    let path = file.path().to_string_lossy().to_string();

    assert!(preview_open_error(&path).is_some());
}

#[test]
fn safe_preview_text_cleans_wraps_and_returns_empty_for_missing_files() {
    let input = format!("\x1b[31m{}\x1b[0m\x00", "z".repeat(513));
    let file = write_temp_file("preview_safe_text", input.as_bytes());
    let path = file.path().to_string_lossy().to_string();

    assert_eq!(safe_preview_text(&path), format!("{}\nz\n", "z".repeat(512)));
    assert_eq!(safe_preview_text(&format!("{}.missing", file.path().display())), "");
}

#[test]
fn safe_preview_text_marks_files_over_the_byte_limit_as_truncated() {
    let bytes = vec![b'a'; PREVIEW_MAX_BYTES + 1];
    let file = write_temp_file("preview_large_text", &bytes);
    let path = file.path().to_string_lossy().to_string();

    assert!(safe_preview_text(&path).starts_with("文件较大，预览已截断。\n\n"));
}

#[test]
fn preview_open_error_rejects_unsupported_extensions_before_reading() {
    assert_eq!(
        preview_open_error("missing.ZIP").as_deref(),
        Some("不支持展开该文件：压缩包 文件无法打开")
    );
    assert_eq!(
        preview_open_error("missing.pdf").as_deref(),
        Some("不支持展开该文件：PDF 文件无法打开")
    );
    assert_eq!(preview_open_error("missing.txt"), None);
}
