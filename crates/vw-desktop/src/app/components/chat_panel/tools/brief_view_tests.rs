use super::brief_view::tool_brief_view;
use super::brief_view::{compact_attachment_path, format_attachment_size, format_sent_at};
use super::tool_text_key;
use crate::app::App;
use iced::widget::text_editor;

#[test]
fn compact_attachment_path_keeps_file_name_for_long_paths() {
    assert_eq!(compact_attachment_path("/tmp/project/src/main.rs"), "src/main.rs");
    assert_eq!(compact_attachment_path("README.md"), "README.md");
    assert_eq!(compact_attachment_path("/"), "/");

    let long_name = format!("/tmp/project/{}", "a".repeat(90));
    let compact = compact_attachment_path(&long_name);
    assert!(compact.starts_with("project/"));
    assert!(compact.ends_with('…'));
    assert_eq!(compact.chars().count(), 73);
}

#[test]
fn format_attachment_size_uses_human_units() {
    assert_eq!(format_attachment_size(512), "512 B");
    assert_eq!(format_attachment_size(1023), "1023 B");
    assert_eq!(format_attachment_size(1024), "1.0 KB");
    assert_eq!(format_attachment_size(2048), "2.0 KB");
    assert_eq!(format_attachment_size(2 * 1024 * 1024), "2.0 MB");
}

#[test]
fn format_sent_at_rejects_empty_values() {
    assert_eq!(format_sent_at(""), None);
    assert_eq!(format_sent_at("not-a-date"), None);
    assert!(format_sent_at("2026-06-12T08:30:00Z").is_some());
}

#[test]
fn brief_view_rejects_malformed_error_and_empty_payloads() {
    let app = App::new().0;

    assert!(tool_brief_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_brief_view(&app, 0, 0, "tool brief").is_none());
    assert!(tool_brief_view(&app, 0, 0, "tool \n{}").is_none());
    assert!(tool_brief_view(&app, 0, 0, "tool brief\nnot-json").is_none());
    assert!(
        tool_brief_view(
            &app,
            0,
            0,
            r#"tool brief
{"status":"error","result":{"data":{"message":"failed"}}}"#
        )
        .is_none()
    );
    assert!(
        tool_brief_view(
            &app,
            0,
            0,
            r#"tool brief
{"status":"denied","result":{"data":{"message":"blocked"}}}"#
        )
        .is_none()
    );
    assert!(
        tool_brief_view(
            &app,
            0,
            0,
            r#"tool brief
{"result":{"data":"not-object"}}"#
        )
        .is_none()
    );
    assert!(
        tool_brief_view(
            &app,
            0,
            0,
            r#"tool brief
{"result":{"data":{"message":"","attachments":[]}}}"#
        )
        .is_none()
    );
}

#[test]
fn brief_view_ignores_invalid_attachment_items() {
    let app = App::new().0;

    assert!(
        tool_brief_view(
            &app,
            0,
            0,
            r#"tool brief
{"data":{"message":"","attachments":"not-array"}}"#
        )
        .is_none()
    );
    assert!(
        tool_brief_view(
            &app,
            0,
            0,
            r#"tool brief
{"data":{"message":"","attachments":[null,{},{"path":"   "},{"path":12}]}}"#
        )
        .is_none()
    );
}

#[test]
fn brief_view_renders_top_level_data_without_hover_or_sent_at() {
    let app = App::new().0;
    let visible = r#"tool brief
{"data":{"message":"  sent from top-level data  ","status":"normal","sentAt":"invalid","attachments":[]}}"#;

    assert!(tool_brief_view(&app, 1, 2, visible).is_some());
}

#[test]
fn brief_view_renders_valid_attachment_after_filtering_invalid_entries() {
    let app = App::new().0;
    let visible = r#"tool brief
{"result":{"data":{"message":"","attachments":[null,{"path":""},{"path":"/tmp/project/report.txt"}]}}}"#;

    assert!(tool_brief_view(&app, 4, 5, visible).is_some());
}

#[test]
fn brief_view_uses_cached_text_editor_when_available() {
    let mut app = App::new().0;
    app.chat_tool_text_editors
        .insert(tool_text_key(6, 7, 0), text_editor::Content::with_text("editor message"));
    let visible = r#"tool brief
{"result":{"data":{"message":"json message","attachments":[]}}}"#;

    assert!(tool_brief_view(&app, 6, 7, visible).is_some());
}

#[test]
fn brief_view_renders_message_and_attachments() {
    let mut app = App::new().0;
    app.chat_tool_hovered_idx = Some((2_u64 << 32) | 1);
    let visible = r#"tool brief
{"result":{"data":{"message":"sent","status":"proactive","sentAt":"2026-06-12T08:30:00Z","attachments":[{"path":"/tmp/project/a.png","size":2048,"isImage":true},{"path":"/tmp/project/readme.md","size":12}]}}}"#;

    assert!(tool_brief_view(&app, 2, 1, visible).is_some());
}

#[test]
fn format_attachment_size_handles_unit_boundaries() {
    assert_eq!(format_attachment_size(1024 * 1024 - 1), "1024.0 KB");
    assert_eq!(format_attachment_size(1024 * 1024), "1.0 MB");
}

#[test]
fn brief_view_defaults_missing_attachment_metadata() {
    let app = App::new().0;
    let visible = r#"tool brief
{"data":{"attachments":[{"path":"notes.txt"}]}}"#;

    assert!(tool_brief_view(&app, 8, 9, visible).is_some());
}
