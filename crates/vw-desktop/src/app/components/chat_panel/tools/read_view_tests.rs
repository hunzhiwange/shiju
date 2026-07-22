use super::read_view::{parse_read_input, read_range_text};
use super::{tool_read_compact_view, tool_read_view, tool_text_key};
use crate::app::App;
use iced::widget::text_editor;

fn app() -> App {
    App::new().0
}

fn visible(tool_name: &str, input: &str) -> String {
    format!("tool {tool_name}\n{}", serde_json::json!({ "input": input }))
}

#[test]
fn parse_read_input_accepts_json_and_plain_paths() {
    assert_eq!(
        parse_read_input(r#"{"filePath":"src/main.rs","offset":2,"limit":5}"#),
        Some(("src/main.rs".to_string(), 2, 5))
    );
    assert_eq!(
        parse_read_input(r#"{"file_path":"docs/readme.md"}"#),
        Some(("docs/readme.md".to_string(), 0, 0))
    );
    assert_eq!(
        parse_read_input(r#"{"path":"file:///tmp/demo.txt","offset":1}"#),
        Some(("tmp/demo.txt".to_string(), 1, 0))
    );
    assert_eq!(parse_read_input("src/lib.rs"), Some(("src/lib.rs".to_string(), 0, 0)));
    assert_eq!(parse_read_input(r#"{"offset":2}"#), None);
    assert_eq!(parse_read_input(r#"{"path":""}"#), None);
}

#[test]
fn parse_read_input_prefers_file_path_keys_and_defaults_invalid_ranges() {
    assert_eq!(
        parse_read_input(
            r#"{"filePath":"first.rs","file_path":"second.rs","path":"third.rs","offset":-1,"limit":"5"}"#,
        ),
        Some(("first.rs".to_string(), 0, 0))
    );
    assert_eq!(
        parse_read_input(r#"{"file_path":"second.rs","path":"third.rs","offset":3.5}"#),
        Some(("second.rs".to_string(), 0, 0))
    );
    assert_eq!(
        parse_read_input(r#"{"path":"third.rs","limit":4}"#),
        Some(("third.rs".to_string(), 0, 4))
    );
}

#[test]
fn parse_read_input_normalizes_wrapped_paths_and_line_anchors() {
    assert_eq!(
        parse_read_input("  [source](`src/lib.rs#L12`)  "),
        Some(("src/lib.rs".to_string(), 0, 0))
    );
    assert_eq!(
        parse_read_input(r#"{"path":"'docs/guide.md#line-8'"}"#),
        Some(("docs/guide.md".to_string(), 0, 0))
    );
    assert_eq!(parse_read_input(r#"  "src/main.rs"  "#), Some(("src/main.rs".to_string(), 0, 0)));
}

#[test]
fn parse_read_input_rejects_malformed_json_and_non_string_paths() {
    assert_eq!(parse_read_input("{"), None);
    assert_eq!(parse_read_input(r#"{"path":42}"#), None);
    assert_eq!(parse_read_input(r#"{"filePath":null}"#), None);
    assert_eq!(parse_read_input("  "), None);
    assert_eq!(parse_read_input("``"), None);
}

#[test]
fn read_range_text_formats_offset_and_limit() {
    assert_eq!(read_range_text(2, 5), Some("offset=2 limit=5 (line 3-7)".to_string()));
    assert_eq!(read_range_text(2, 0), Some("offset=2 (from line 3)".to_string()));
    assert_eq!(read_range_text(0, 5), Some("limit=5 (line 1-5)".to_string()));
    assert_eq!(read_range_text(0, 0), None);
}

#[test]
fn read_views_accept_supported_tools_and_reject_bad_input() {
    let app = app();
    let visible = r#"tool read
{"input":"{\"filePath\":\"src/main.rs\",\"offset\":1,\"limit\":2}"}"#;
    let pdf_visible = r#"tool pdf_read
{"input":"docs/manual.pdf"}"#;

    assert!(tool_read_compact_view(&app, visible).is_some());
    assert!(tool_read_view(&app, 1, 2, visible).is_some());
    assert!(tool_read_view(&app, 1, 3, pdf_visible).is_some());
    assert!(tool_read_view(&app, 1, 4, "tool bash\n{}").is_none());
    assert!(tool_read_view(&app, 1, 5, "tool read\nnot-json").is_none());
    assert!(tool_read_view(&app, 1, 6, "read\n{}").is_none());
}

#[test]
fn read_views_accept_aliases_and_all_range_shapes() {
    let app = app();
    let cases = [
        ("READ", r#"{"path":"src/no-range.rs"}"#),
        ("file_read", r#"{"path":"src/offset.rs","offset":2}"#),
        ("read_file", r#"{"path":"src/limit.rs","limit":3}"#),
        ("pdf_read", r#"{"path":"docs/full.pdf","offset":1,"limit":2}"#),
    ];

    for (tool_name, input) in cases {
        let visible = visible(tool_name, input);
        assert!(tool_read_compact_view(&app, &visible).is_some(), "compact {tool_name}");
        assert!(tool_read_view(&app, 7, 9, &visible).is_some(), "detailed {tool_name}");
    }
}

#[test]
fn tool_read_view_uses_cached_text_and_hovered_detail_slots() {
    let mut app = app();
    let msg_idx = 3;
    let tool_idx = 4;
    app.project_path = Some("/workspace".to_string());
    app.chat_tool_hovered_idx = Some(((msg_idx as u64) << 32) | (tool_idx as u64));
    app.chat_tool_text_editors
        .insert(tool_text_key(msg_idx, tool_idx, 0), text_editor::Content::with_text("cached.rs"));
    app.chat_tool_text_editors.insert(
        tool_text_key(msg_idx, tool_idx, 1),
        text_editor::Content::with_text("cached range"),
    );
    let visible = visible("read", r#"{"path":"src/cached.rs","offset":1,"limit":2}"#);

    assert!(tool_read_view(&app, msg_idx, tool_idx, &visible).is_some());
}

#[test]
fn tool_read_compact_view_rejects_incomplete_or_malformed_calls() {
    let app = app();
    let missing_input = "tool read\n{}";
    let non_string_input = r#"tool read
{"input":{"path":"src/main.rs"}}"#;
    let malformed_input = visible("read", "{");

    assert!(tool_read_compact_view(&app, "tool read").is_none());
    assert!(tool_read_compact_view(&app, "read\n{}").is_none());
    assert!(tool_read_compact_view(&app, "tool bash\n{}").is_none());
    assert!(tool_read_compact_view(&app, "tool read\nnot-json").is_none());
    assert!(tool_read_compact_view(&app, missing_input).is_none());
    assert!(tool_read_compact_view(&app, non_string_input).is_none());
    assert!(tool_read_compact_view(&app, &malformed_input).is_none());
}

#[test]
fn tool_read_view_rejects_incomplete_or_malformed_calls() {
    let app = app();
    let missing_input = "tool read\n{}";
    let non_string_input = r#"tool read
{"input":["src/main.rs"]}"#;
    let empty_input = visible("read", "  ");

    assert!(tool_read_view(&app, 1, 1, "tool read").is_none());
    assert!(tool_read_view(&app, 1, 2, "read\n{}").is_none());
    assert!(tool_read_view(&app, 1, 3, "tool write\n{}").is_none());
    assert!(tool_read_view(&app, 1, 4, "tool read\n[").is_none());
    assert!(tool_read_view(&app, 1, 5, missing_input).is_none());
    assert!(tool_read_view(&app, 1, 6, non_string_input).is_none());
    assert!(tool_read_view(&app, 1, 7, &empty_input).is_none());
}

#[test]
fn read_views_accept_root_path_when_file_name_is_unavailable() {
    let app = app();
    let visible = visible("read", "/");

    assert_eq!(parse_read_input("/"), Some(("/".to_string(), 0, 0)));
    assert!(tool_read_compact_view(&app, &visible).is_some());
    assert!(tool_read_view(&app, 8, 9, &visible).is_some());
}
