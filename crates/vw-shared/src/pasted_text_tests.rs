//! 粘贴文本内联阈值与临时文件引用格式测试。
//!
//! 原生平台用例同时确认超限内容完整写入系统临时目录。

use std::path::Path;

use super::{
    PASTED_TEXT_FILE_REFERENCE_THRESHOLD_CHARS, cache_large_pasted_text, document_reference,
};

#[test]
fn cache_large_pasted_text_keeps_text_at_or_below_threshold() {
    let text = "x".repeat(PASTED_TEXT_FILE_REFERENCE_THRESHOLD_CHARS);

    assert_eq!(cache_large_pasted_text(&text).unwrap(), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn cache_large_pasted_text_persists_text_above_threshold_in_system_temp_directory() {
    let text = "中".repeat(PASTED_TEXT_FILE_REFERENCE_THRESHOLD_CHARS + 1);
    let path = cache_large_pasted_text(&text).unwrap().expect("large paste should be cached");

    assert_eq!(path.parent(), Some(std::env::temp_dir().as_path()));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn document_reference_uses_document_marker() {
    assert_eq!(document_reference(Path::new("/tmp/pasted.txt")), "[DOCUMENT:/tmp/pasted.txt]");
}
