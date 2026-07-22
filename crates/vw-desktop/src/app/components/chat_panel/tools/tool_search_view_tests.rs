use super::tool_search_view::{tool_search_summary, tool_search_view};
use crate::app::App;
use serde_json::json;

#[test]
fn tool_search_summary_prefers_summary_and_truncates_output() {
    assert_eq!(
        tool_search_summary(&json!({"summary":" Found 9 matching tool(s) "})),
        "Found 9 matching tool(s)"
    );
    assert_eq!(tool_search_summary(&json!({"output":"large result"})), "查看详情");
    assert_eq!(tool_search_summary(&json!({})), "查看详情");
}

#[test]
fn tool_search_view_renders_only_tool_search_rows() {
    let app = App::new().0;
    let visible = r#"tool tool_search
{"status":"completed","summary":"Found 9 matching tool(s)","output":"large result"}"#;

    assert!(tool_search_view(&app, 0, 0, visible).is_some());
    assert!(tool_search_view(&app, 0, 1, "tool bash\n{}").is_none());
    assert!(tool_search_view(&app, 0, 2, "tool tool_search\nnot-json").is_none());
    assert!(tool_search_view(&app, 0, 3, "tool tool_search").is_none());
}

#[test]
fn tool_search_summary_truncates_long_unicode_text_and_rejects_blank_summary() {
    let long_summary = "搜".repeat(81);
    let summary = tool_search_summary(&json!({"summary": long_summary}));

    assert_eq!(summary.chars().count(), 81);
    assert_eq!(tool_search_summary(&json!({"summary":"   "})), "查看详情");
}

#[test]
fn tool_search_view_constructs_running_and_denied_rows() {
    let app = App::new().0;
    let running = r#"tool tool_search
{"status":"running","summary":"Searching tools"}"#;
    let denied = r#"tool tool_search
{"status":"denied","summary":"Workspace policy"}"#;

    assert!(tool_search_view(&app, 2, 3, running).is_some());
    assert!(tool_search_view(&app, 2, 4, denied).is_some());
}
