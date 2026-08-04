//! 覆盖预览面板相关行为的回归测试。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::fs;
use std::io::Write;

use super::*;

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
fn safe_preview_reports_binary_files_instead_of_raw_bytes() {
    let file = write_temp_file("preview_binary", &[0x50, 0x4B, 0x03, 0x04, 0x00, 0xFF, 0xAA, 0x00]);
    let path = file.path().to_string_lossy().to_string();

    let (content, truncated) = safe_preview(&path);

    assert!(!truncated);
    assert!(content.contains("二进制文件"));
    assert!(content.contains("不支持文本预览"));
}

#[test]
fn strip_ansi_and_controls_removes_escape_sequences_and_preserves_layout() {
    let input = concat!(
        "plain\x1b[31mred\x1b[0m ",
        "\x1b]0;title\x07after-bel ",
        "\x1b]8;;https://example.invalid\x1b\\link\x1b]8;;\x1b\\ ",
        "\x1bXignored\x00\x08\x7f\n\t中文"
    );

    assert_eq!(strip_ansi_and_controls(input), "plainred after-bel link ignored\n\t中文");
}

#[test]
fn strip_ansi_and_controls_discards_unterminated_and_lone_escape_sequences() {
    assert_eq!(strip_ansi_and_controls("before\x1b[31"), "before");
    assert_eq!(strip_ansi_and_controls("before\x1b]title"), "before");
    assert_eq!(strip_ansi_and_controls("before\x1b"), "before");
}

#[test]
fn wrap_long_lines_counts_characters_and_resets_after_newlines() {
    assert_eq!(wrap_long_lines("甲乙丙丁\nabcdx", 4), "甲乙丙丁\n\nabcd\nx");
    assert_eq!(wrap_long_lines("", 4), "");
}

#[test]
fn truncate_to_char_boundary_preserves_complete_utf8_characters() {
    assert_eq!(truncate_to_char_boundary("a中b", 5), ("a中b", false));
    assert_eq!(truncate_to_char_boundary("a中b", 4), ("a中", true));
    assert_eq!(truncate_to_char_boundary("a中b", 2), ("a", true));
    assert_eq!(truncate_to_char_boundary("中", 0), ("", true));
}

#[test]
fn format_text_preview_content_cleans_controls_and_wraps_long_lines() {
    let content = format!("\x1b[32m{}\x1b[0m\x00", "x".repeat(PREVIEW_MAX_LINE + 1));

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(!truncated);
    assert_eq!(formatted, format!("{}\nx\n", "x".repeat(PREVIEW_MAX_LINE)));
}

#[test]
fn format_text_preview_content_truncates_at_byte_and_character_boundary() {
    let content = format!("{}中", "a".repeat(PREVIEW_MAX_BYTES - 1));

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(truncated);
    assert!(formatted.starts_with("文件较大，预览已截断。\n\n"));
    assert!(!formatted.contains('中'));
}

#[test]
fn format_text_preview_content_limits_output_lines() {
    let content = "line\n".repeat(PREVIEW_MAX_LINES + 1);

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(truncated);
    assert!(formatted.starts_with("文件较大，预览已截断。\n\n"));
    assert_eq!(formatted.lines().filter(|line| *line == "line").count(), PREVIEW_MAX_LINES);
}

#[test]
fn format_text_preview_content_accepts_exact_line_limit_without_truncation() {
    let content = "line\n".repeat(PREVIEW_MAX_LINES);

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(!truncated);
    assert_eq!(formatted.lines().filter(|line| *line == "line").count(), PREVIEW_MAX_LINES);
    assert!(!formatted.starts_with("文件较大，预览已截断。"));
}

#[test]
fn unsupported_preview_kind_is_case_insensitive_and_rejects_archives_and_pdf() {
    for extension in ["zip", "TAR", "gz", "rar", "7z", "bz2", "xz"] {
        let path = format!("artifact.{extension}");
        assert_eq!(unsupported_preview_kind(&path), Some("压缩包"));
    }
    assert_eq!(unsupported_preview_kind("document.PDF"), Some("PDF"));
    assert_eq!(unsupported_preview_kind("notes.txt"), None);
    assert_eq!(unsupported_preview_kind("README"), None);
}

#[test]
fn read_preview_sample_is_bounded_and_missing_files_are_treated_as_text() {
    let bytes = vec![b'a'; 5000];
    let file = write_temp_file("preview_sample", &bytes);
    let path = file.path().to_string_lossy().to_string();
    let missing = format!("{}.missing", file.path().display());

    assert_eq!(read_preview_sample(&path).map(|sample| sample.len()), Some(4096));
    assert!(file_looks_like_text(&missing));
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

#[test]
fn safe_preview_classifies_images_archives_and_pdf_without_reading_them() {
    assert_eq!(
        safe_preview("assets/photo.PNG"),
        ("文件: photo.PNG\n类型: 图片\n".to_string(), false)
    );
    assert_eq!(
        safe_preview("assets/icon.svg"),
        ("文件: icon.svg\n类型: 图片\n".to_string(), false)
    );
    assert_eq!(
        safe_preview("archives/source.tar"),
        ("文件: source.tar\n类型: 压缩包\n不支持文本预览\n".to_string(), false)
    );
    assert_eq!(
        safe_preview("manual.pdf"),
        ("文件: manual.pdf\n类型: PDF\n不支持文本预览\n".to_string(), false)
    );
}

#[test]
fn safe_preview_missing_plain_text_path_returns_empty_untruncated_content() {
    assert_eq!(safe_preview("missing.txt"), (String::new(), false));
}

#[test]
fn safe_preview_formats_text_and_reports_large_files() {
    let temp = tempfile::tempdir().expect("create temp directory");
    let text_path = temp.path().join("notes.txt");
    let large_path = temp.path().join("large.txt");
    fs::write(&text_path, b"hello\x1b[31m world\x1b[0m\x00").expect("write text preview");
    fs::write(&large_path, vec![b'x'; PREVIEW_MAX_BYTES + 1]).expect("write large preview");

    assert_eq!(safe_preview(&text_path.to_string_lossy()), ("hello world\n".to_string(), false));
    let (large_content, large_truncated) = safe_preview(&large_path.to_string_lossy());
    assert!(large_truncated);
    assert_eq!(large_content.bytes().filter(|byte| *byte == b'x').count(), PREVIEW_MAX_BYTES);
}

#[test]
fn compute_search_matches_skips_empty_terms_and_sorts_byte_positions() {
    let terms = vec!["文".to_string(), "ana".to_string(), String::new(), "ban".to_string()];

    assert_eq!(
        compute_search_matches("banana 文文\nban", &terms),
        vec![(0, 0, 3), (0, 1, 3), (0, 7, 3), (0, 10, 3), (1, 0, 3)]
    );
    assert!(compute_search_matches("anything", &[]).is_empty());
}

#[test]
fn path_to_file_uri_encodes_spaces() {
    assert_eq!(path_to_file_uri(std::path::Path::new("/tmp/a b.rs")), "file:///tmp/a%20b.rs");
}

#[test]
fn preview_tab_debug_omits_content_and_editor_state() {
    let tab = PreviewTab {
        path: "/tmp/example.rs".to_string(),
        title: "example.rs".to_string(),
        content: "secret content".to_string(),
        is_dirty: true,
        truncated: true,
        auto_save_revision: 7,
        editor: Editor::new("secret editor content", "rust"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: Some("rust"),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: Some("file:///tmp/example.rs".to_string()),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: Some("rust".to_string()),
    };

    let debug = format!("{tab:?}");

    assert_eq!(debug, "PreviewTab { path: \"/tmp/example.rs\", title: \"example.rs\" }");
    assert!(!debug.contains("secret"));
}
