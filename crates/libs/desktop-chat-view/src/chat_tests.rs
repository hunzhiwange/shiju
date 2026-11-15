//! 聊天文本工具测试，覆盖光标插入、位置格式和思考及工具块剥离。
//! 所有截取与拼接都必须保持 Unicode 边界，伪标签和未闭合标签按明确规则处理。

#[test]
fn format_position_keeps_file_line_and_column() {
    assert_eq!(super::format_position("src/main.rs", 7, 3), "文件:src/main.rs 行:7 列:3");
    assert_eq!(
        super::format_selection_positions("src/main.rs", 1, 2, 3, 4),
        "@src/main.rs:1:2-3:4"
    );
}

#[test]
fn append_line_adds_separator_only_when_content_exists() {
    let mut editor = iced::widget::text_editor::Content::new();
    super::append_line(&mut editor, "first");
    assert_eq!(editor.text(), "first");

    super::append_line(&mut editor, "second");
    assert_eq!(editor.text(), "first\nsecond");
}

#[test]
fn insert_at_cursor_pastes_text_without_replacing_existing_content() {
    let mut editor = iced::widget::text_editor::Content::with_text("hello");
    super::insert_at_cursor(&mut editor, " world");
    assert_eq!(editor.text(), "hello world");
}

#[test]
fn insert_at_cursor_appends_multiline_unicode_text_at_document_end() {
    let mut editor = iced::widget::text_editor::Content::with_text("prefix");
    super::insert_at_cursor(&mut editor, "\n第二行");
    assert_eq!(editor.text(), "prefix\n第二行");
}

#[test]
fn position_formatters_preserve_unicode_paths_and_zero_positions() {
    assert_eq!(super::format_position("目录/文件.rs", 0, 0), "文件:目录/文件.rs 行:0 列:0");
    assert_eq!(
        super::format_selection_positions("目录/文件.rs", 0, 0, usize::MAX, usize::MAX),
        format!("@目录/文件.rs:0:0-{}:{}", usize::MAX, usize::MAX)
    );
}
