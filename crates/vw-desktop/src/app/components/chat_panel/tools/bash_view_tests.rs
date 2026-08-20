use super::bash_view::{bash_command_preview, bash_view_padding, tool_bash_view};
use super::tool_meta::{tool_header_title, tool_status_badge, tool_visual_state};
use crate::app::{App, Message};
use iced::Padding;

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

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
    // "context" is not a recognized shell alias after normalization.
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
fn bash_rows_remove_left_indent() {
    assert_eq!(bash_view_padding("bash"), Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 0.0 });
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
fn bash_tool_header_uses_shared_type_and_status_slots() {
    let app = app();

    for (idx, status) in ["waiting", "running", "completed", "failed", "cancelled"]
        .into_iter()
        .enumerate()
    {
        let visible = format!(
            "tool bash\n{}",
            serde_json::json!({
                "status": status,
                "input": "{\"command\":\"printf 中文\"}",
                "output": "ok",
            })
        );
        keep_element(
            tool_bash_view(&app, 12, idx, &visible)
                .unwrap_or_else(|| panic!("expected {status} bash view")),
        );

        let visual = tool_visual_state(status);
        keep_element(tool_header_title("bash", "运行", visual));
        keep_element(tool_status_badge(visual));
    }
}
