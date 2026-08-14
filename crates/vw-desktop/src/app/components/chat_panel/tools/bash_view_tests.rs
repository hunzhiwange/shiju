use super::bash_view::{bash_command_preview, bash_view_padding, tool_bash_view};
use crate::app::App;
use iced::Padding;

fn app() -> App {
    App::new().0
}

#[test]
fn bash_view_rejects_malformed_or_non_bash_payloads() {
    let app = app();

    assert!(tool_bash_view(&app, 0, 0, "tool read\n{}").is_none());
    assert!(tool_bash_view(&app, 0, 0, "bash without prefix").is_none());
    assert!(tool_bash_view(&app, 0, 0, "tool bash\nnot-json").is_none());
}

#[test]
fn bash_view_rejects_unknown_tool_name_after_normalization() {
    let app = app();
    // "context" is not "bash" or "image_info" after normalization
    assert!(tool_bash_view(&app, 0, 0, "tool context\n{\"input\":\"ls\"}").is_none());
}

#[test]
fn bash_view_accepts_plain_command_input() {
    let app = app();
    let visible = r#"tool bash
{"input":"cargo test -p vw-desktop","output":"ok"}"#;

    assert!(tool_bash_view(&app, 1, 2, visible).is_some());
}

#[test]
fn bash_rows_remove_left_indent_without_changing_image_info_padding() {
    assert_eq!(bash_view_padding("bash"), Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 0.0 });
    assert_eq!(
        bash_view_padding("image_info"),
        Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 }
    );
}

#[test]
fn bash_command_preview_is_single_line_and_truncates_overflow() {
    assert_eq!(bash_command_preview("cargo check\n  --all-targets"), "cargo check --all-targets");

    let preview = bash_command_preview(&"x".repeat(100));
    assert!(!preview.contains('\n'));
    assert!(preview.ends_with('…'));
    assert_eq!(preview.chars().count(), 73);
}

#[test]
fn bash_view_empty_input_uses_fallback_title() {
    let app = app();
    let visible = r#"tool bash
{"input":"","output":"ok"}"#;

    assert!(tool_bash_view(&app, 0, 0, visible).is_some());
}

#[test]
fn bash_view_missing_input_field_falls_back() {
    let app = app();
    let visible = r#"tool bash
{"output":"ok"}"#;

    assert!(tool_bash_view(&app, 0, 0, visible).is_some());
}

#[test]
fn bash_view_json_command_extraction() {
    let app = app();
    let visible = r#"tool bash
{"input":"{\"command\":\"cargo build\"}","output":"ok"}"#;

    assert!(tool_bash_view(&app, 0, 0, visible).is_some());
}

#[test]
fn bash_view_json_command_missing_command_field() {
    let app = app();
    let visible = r#"tool bash
{"input":"{\"path\":\"/tmp\"}","output":"ok"}"#;

    assert!(tool_bash_view(&app, 0, 0, visible).is_some());
}

#[test]
fn bash_view_accepts_canonical_name_variants() {
    let app = app();
    // canonical_tool_name normalizes "shell" → "bash", "Shell" → "bash"
    assert!(tool_bash_view(&app, 0, 0, "tool shell\n{\"input\":\"ls\"}").is_some());
    assert!(tool_bash_view(&app, 0, 0, "tool execute\n{\"input\":\"ls\"}").is_some());
    assert!(tool_bash_view(&app, 0, 0, "tool terminal\n{\"input\":\"ls\"}").is_some());
}

#[test]
fn bash_view_accepts_json_command_input_and_hovered_detail_slot() {
    let mut app = app();
    app.chat_tool_hovered_idx = Some((3_u64 << 32) | 4);
    let visible = r#"tool bash
{"input":"{\"command\":\"ls -la\"}","output":"ok"}"#;

    assert!(tool_bash_view(&app, 3, 4, visible).is_some());
}

#[test]
fn image_info_reuses_bash_view_detail_surface() {
    let app = app();
    let visible = r#"tool image_info
{"input":"{\"path\":\"/tmp/image.png\"}","output":"metadata"}"#;

    assert!(tool_bash_view(&app, 0, 1, visible).is_some());
}

#[test]
fn image_info_with_hovered_state() {
    let mut app = app();
    app.chat_tool_hovered_idx = Some((2_u64 << 32) | 7);
    let visible = r#"tool image_info
{"input":"screenshot.png","output":"640x480"}"#;

    assert!(tool_bash_view(&app, 2, 7, visible).is_some());
}

#[test]
fn bash_view_ignores_cached_multiline_text_for_compact_summary() {
    use super::tool_text_key;
    use iced::widget::text_editor;

    let mut app = app();
    app.chat_tool_text_editors.insert(
        tool_text_key(8, 9, 0),
        text_editor::Content::with_text("cached command\nsecond line"),
    );

    assert!(tool_bash_view(&app, 8, 9, "tool bash\n{\"input\":\"echo ignored\"}").is_some());
}

#[test]
fn image_info_empty_input_uses_detail_fallback() {
    let app = app();

    assert!(tool_bash_view(&app, 10, 11, "tool image_info\n{\"input\":\"   \"}").is_some());
}
