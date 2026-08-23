//! 验证网页工具的状态标题、元数据优先级与空值回退行为。
//! 权限状态优先于错误，驼峰渲染提示优先于蛇形提示和普通数据字段。

use super::ToolPermissionState;
use super::tool_text_key;
use super::web_view::{
    tool_web_view, web_metadata_bool, web_metadata_field, web_metadata_number, web_metadata_text,
    web_string_field, web_tool_title,
};
use crate::app::App;
use crate::app::components::chat_panel::utils::chat_context_target_key;
use iced::widget::text_editor;
use serde_json::json;

// ─── web_tool_title ───────────────────────────────────────────────────

#[test]
fn web_tool_title_running_suffix() {
    assert_eq!(web_tool_title("网页搜索", None, true, false), "网页搜索中");
}

#[test]
fn web_tool_title_approval_required() {
    let title =
        web_tool_title("网页搜索", Some(ToolPermissionState::ApprovalRequired), false, false);
    assert_eq!(title, "网页搜索待批准");
}

#[test]
fn web_tool_title_rejected() {
    let title = web_tool_title("网页搜索", Some(ToolPermissionState::Rejected), false, false);
    assert_eq!(title, "网页搜索已拒绝");
}

#[test]
fn web_tool_title_error_suffix() {
    assert_eq!(web_tool_title("网页搜索", None, false, true), "网页搜索失败");
}

#[test]
fn web_tool_title_base_title() {
    assert_eq!(web_tool_title("网页搜索", None, false, false), "网页搜索");
}

#[test]
fn web_tool_title_permission_takes_precedence_over_error() {
    // permission_state is checked before is_error, so permission title should win
    let title =
        web_tool_title("网页搜索", Some(ToolPermissionState::ApprovalRequired), false, true);
    assert_eq!(title, "网页搜索待批准");
}

// ─── web_metadata_field ───────────────────────────────────────────────

#[test]
fn web_metadata_field_from_render_hint_camel() {
    let value = json!({"renderHint":{"metadata":{"provider":"bing"}}});
    assert_eq!(web_metadata_field(&value, "provider"), Some("bing".to_string()));
}

#[test]
fn web_metadata_field_from_render_hint_snake() {
    let value = json!({"render_hint":{"metadata":{"provider":"google"}}});
    assert_eq!(web_metadata_field(&value, "provider"), Some("google".to_string()));
}

#[test]
fn web_metadata_field_from_data() {
    let value = json!({"data":{"provider":"custom"}});
    assert_eq!(web_metadata_field(&value, "provider"), Some("custom".to_string()));
}

#[test]
fn web_metadata_field_camel_takes_precedence() {
    let value = json!({
        "renderHint":{"metadata":{"provider":"camel"}},
        "render_hint":{"metadata":{"provider":"snake"}},
        "data":{"provider":"data"}
    });
    assert_eq!(web_metadata_field(&value, "provider"), Some("camel".to_string()));
}

#[test]
fn web_metadata_field_snake_takes_precedence_over_data() {
    let value = json!({
        "render_hint":{"metadata":{"provider":"snake"}},
        "data":{"provider":"data"}
    });
    assert_eq!(web_metadata_field(&value, "provider"), Some("snake".to_string()));
}

#[test]
fn web_metadata_field_empty_string_returns_none() {
    let value = json!({"data":{"provider":""}});
    assert_eq!(web_metadata_field(&value, "provider"), None);
}

#[test]
fn web_metadata_field_whitespace_only_returns_none() {
    let value = json!({"data":{"provider":"  "}});
    assert_eq!(web_metadata_field(&value, "provider"), None);
}

#[test]
fn web_metadata_field_non_string_returns_none() {
    let value = json!({"data":{"provider":42}});
    assert_eq!(web_metadata_field(&value, "provider"), None);
}

#[test]
fn web_metadata_field_missing_key_returns_none() {
    let value = json!({"data":{"url":"https://example.test"}});
    assert_eq!(web_metadata_field(&value, "nonexistent"), None);
}

#[test]
fn web_metadata_field_trimmed_value() {
    let value = json!({"data":{"provider":"  bing  "}});
    assert_eq!(web_metadata_field(&value, "provider"), Some("bing".to_string()));
}

#[test]
fn web_metadata_field_falls_back_after_unusable_higher_priority_values() {
    let value = json!({
        "renderHint":{"metadata":{"provider":"   "}},
        "render_hint":{"metadata":{"provider":42}},
        "data":{"provider":"fallback"}
    });

    assert_eq!(web_metadata_field(&value, "provider"), Some("fallback".to_string()));
}

// ─── web_metadata_number ──────────────────────────────────────────────

#[test]
fn web_metadata_number_from_render_hint_camel() {
    let value = json!({"renderHint":{"metadata":{"result_count":42}}});
    assert_eq!(web_metadata_number(&value, "result_count"), Some(42));
}

#[test]
fn web_metadata_number_from_render_hint_snake() {
    let value = json!({"render_hint":{"metadata":{"result_count":99}}});
    assert_eq!(web_metadata_number(&value, "result_count"), Some(99));
}

#[test]
fn web_metadata_number_from_data() {
    let value = json!({"data":{"result_count":7}});
    assert_eq!(web_metadata_number(&value, "result_count"), Some(7));
}

#[test]
fn web_metadata_number_missing_returns_none() {
    let value = json!({"data":{"url":"https://example.test"}});
    assert_eq!(web_metadata_number(&value, "result_count"), None);
}

#[test]
fn web_metadata_number_non_number_returns_none() {
    let value = json!({"data":{"result_count":"many"}});
    assert_eq!(web_metadata_number(&value, "result_count"), None);
}

#[test]
fn web_metadata_number_falls_back_after_negative_higher_priority_value() {
    let value = json!({
        "renderHint":{"metadata":{"result_count":-1}},
        "data":{"result_count":7}
    });

    assert_eq!(web_metadata_number(&value, "result_count"), Some(7));
}

// ─── web_metadata_bool ────────────────────────────────────────────────

#[test]
fn web_metadata_bool_from_render_hint_camel() {
    let value = json!({"renderHint":{"metadata":{"truncated":true}}});
    assert!(web_metadata_bool(&value, "truncated"));
}

#[test]
fn web_metadata_bool_from_render_hint_snake() {
    let value = json!({"render_hint":{"metadata":{"truncated":true}}});
    assert!(web_metadata_bool(&value, "truncated"));
}

#[test]
fn web_metadata_bool_from_data() {
    let value = json!({"data":{"truncated":true}});
    assert!(web_metadata_bool(&value, "truncated"));
}

#[test]
fn web_metadata_bool_false_from_data() {
    let value = json!({"data":{"truncated":false}});
    assert!(!web_metadata_bool(&value, "truncated"));
}

#[test]
fn web_metadata_bool_missing_returns_false() {
    let value = json!({"data":{"url":"https://example.test"}});
    assert!(!web_metadata_bool(&value, "truncated"));
}

#[test]
fn web_metadata_bool_non_bool_returns_false() {
    let value = json!({"data":{"truncated":"yes"}});
    assert!(!web_metadata_bool(&value, "truncated"));
}

#[test]
fn web_metadata_bool_false_at_highest_priority_is_authoritative() {
    let value = json!({
        "renderHint":{"metadata":{"truncated":false}},
        "render_hint":{"metadata":{"truncated":true}},
        "data":{"truncated":true}
    });

    assert!(!web_metadata_bool(&value, "truncated"));
}

// ─── web_string_field ─────────────────────────────────────────────────

#[test]
fn web_string_field_from_data() {
    let value = json!({"data":{"url":"https://example.test"}});
    assert_eq!(web_string_field(&value, &["url"]), Some("https://example.test".to_string()));
}

#[test]
fn web_string_field_first_key_wins() {
    let value = json!({"data":{"url":"https://example.test","urls":"https://other.test"}});
    assert_eq!(
        web_string_field(&value, &["url", "urls"]),
        Some("https://example.test".to_string())
    );
}

#[test]
fn web_string_field_empty_string_returns_none() {
    let value = json!({"data":{"url":""}});
    assert_eq!(web_string_field(&value, &["url"]), None);
}

#[test]
fn web_string_field_urls_special_handling() {
    let value = json!({"input":"{\"urls\":[\"https://example.test/a\"]}"});
    assert_eq!(web_string_field(&value, &["urls"]), Some("https://example.test/a".to_string()));
}

#[test]
fn web_string_field_urls_special_handling_precedes_data() {
    // Special "urls" handling checks first, but only if value.input exists as JSON string
    let value = json!({
        "input":"{\"urls\":[\"https://from-input.test\"]}",
        "data":{"urls":"https://from-data.test"}
    });
    // The special "urls" branch runs first and finds urls via parsing input
    assert_eq!(web_string_field(&value, &["urls"]), Some("https://from-input.test".to_string()));
}

#[test]
fn web_string_field_fallback_from_tool_input() {
    // When data key is missing, fallback parses tool_input(value) as JSON
    let value = json!({"input":"{\"query\":\"latest docs\"}"});
    // no data.query, so it falls through to the serde_json fallback which parses tool_input
    assert_eq!(web_string_field(&value, &["query"]), Some("latest docs".to_string()));
}

#[test]
fn web_string_field_no_match_returns_none() {
    let value = json!({"data":{"url":"https://example.test"}});
    assert_eq!(web_string_field(&value, &["nonexistent"]), None);
}

#[test]
fn web_string_field_data_trumps_fallback() {
    let value = json!({
        "data":{"query":"from data"},
        "input":"{\"query\":\"from input\"}"
    });
    // data.query is found first
    assert_eq!(web_string_field(&value, &["query"]), Some("from data".to_string()));
}

#[test]
fn web_string_field_malformed_urls_input_falls_back_to_data() {
    let value = json!({
        "input":"not-json",
        "data":{"urls":"https://fallback.test"}
    });

    assert_eq!(web_string_field(&value, &["urls"]), Some("https://fallback.test".to_string()));
}

#[test]
fn web_string_field_empty_urls_array_falls_through_to_later_keys() {
    let value = json!({
        "input":"{\"urls\":[],\"query\":\"fallback query\"}"
    });

    assert_eq!(web_string_field(&value, &["urls", "query"]), Some("fallback query".to_string()));
}

#[test]
fn web_string_field_uses_raw_input_and_skips_blank_candidate() {
    let value = json!({"rawInput":"{\"url\":\"   \",\"query\":\"fallback query\"}"});

    assert_eq!(web_string_field(&value, &["url", "query"]), Some("fallback query".to_string()));
}

// ─── web_metadata_text ────────────────────────────────────────────────

#[test]
fn web_metadata_text_web_fetch_has_url() {
    let value = json!({"data":{"url":"https://example.test"}});
    assert!(web_metadata_text("web_fetch", &value).contains("https://example.test"));
}

#[test]
fn web_metadata_text_web_fetch_all_fields() {
    let value = json!({
        "renderHint":{"metadata":{"provider":"bing","format":"markdown","truncated":true}},
        "data":{"url":"https://example.test"}
    });
    let text = web_metadata_text("web_fetch", &value);
    assert!(text.contains("https://example.test"));
    assert!(text.contains("bing"));
    assert!(text.contains("格式 markdown"));
    assert!(text.contains("已截断"));
}

#[test]
fn web_metadata_text_web_search_has_query_and_results() {
    let value = json!({
        "renderHint":{"metadata":{"provider":"google","result_count":15}},
        "data":{"query":"rust programming"}
    });
    let text = web_metadata_text("web_search", &value);
    assert!(text.contains("rust programming"));
    assert!(text.contains("google"));
    assert!(text.contains("15 条结果"));
}

#[test]
fn web_metadata_text_unknown_tool_returns_empty() {
    let value = json!({"data":{"url":"https://example.test"}});
    assert_eq!(web_metadata_text("bash", &value), "");
}

#[test]
fn web_metadata_text_empty_value_returns_empty() {
    let value = json!({});
    assert_eq!(web_metadata_text("web_fetch", &value), "");
    assert_eq!(web_metadata_text("web_search", &value), "");
}

#[test]
fn web_metadata_text_fetch_webpage() {
    let value = json!({"data":{"url":"https://example.test"}});
    let text = web_metadata_text("fetch_webpage", &value);
    assert!(text.contains("https://example.test"));
}

#[test]
fn web_metadata_text_http_request() {
    let value = json!({"data":{"url":"https://api.example.test/data"}});
    let text = web_metadata_text("http_request", &value);
    assert!(text.contains("https://api.example.test/data"));
}

#[test]
fn web_metadata_text_join_parts_with_separator() {
    let value = json!({
        "data":{"url":"https://example.test"},
        "renderHint":{"metadata":{"provider":"custom"}}
    });
    let text = web_metadata_text("web_fetch", &value);
    assert_eq!(text, "https://example.test · custom");
}

// ─── tool_web_view ────────────────────────────────────────────────────

fn app() -> App {
    App::new().0
}

#[test]
fn tool_web_view_rejects_non_web_tool() {
    let app = app();
    assert!(tool_web_view(&app, 0, 0, "tool bash\n{\"output\":\"ok\"}").is_none());
}

#[test]
fn tool_web_view_rejects_malformed_header() {
    let app = app();
    // No "tool " prefix
    assert!(tool_web_view(&app, 0, 0, "web_fetch\n{\"output\":\"ok\"}").is_none());
    // No newline
    assert!(tool_web_view(&app, 0, 0, "tool web_fetch").is_none());
}

#[test]
fn tool_web_view_rejects_invalid_json() {
    let app = app();
    assert!(tool_web_view(&app, 0, 0, "tool web_fetch\nnot-json").is_none());
}

#[test]
fn tool_web_view_returns_some_for_valid_fetch() {
    let app = app();
    let visible = r#"tool web_fetch
{"output":"web content here"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_returns_some_for_webfetch_alias() {
    let app = app();
    let visible = r#"tool webfetch
{"output":"web content"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_returns_some_for_web_search() {
    let app = app();
    let visible = r#"tool web_search
{"output":"search results here","summary":"search complete"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_returns_some_for_running_status() {
    let app = app();
    let visible = r#"tool web_fetch
{"status":"running"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_returns_some_for_error_status() {
    let app = app();
    let visible = r#"tool web_fetch
{"status":"error","output":"connection failed"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_returns_some_for_denied_status() {
    let app = app();
    let visible = r#"tool web_fetch
{"status":"denied","output":"permission denied"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_returns_none_for_empty_body_not_running() {
    let app = app();
    // No status, no output, no summary → empty body, not running → None
    let visible = r#"tool web_fetch
{}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_none());
}

#[test]
fn tool_web_view_accepts_fetch_webpage() {
    let app = app();
    let visible = r#"tool fetch_webpage
{"output":"page content"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_accepts_http_request() {
    let app = app();
    let visible = r#"tool http_request
{"output":"api response"}"#;
    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_renders_request_with_url_without_output() {
    let app = app();
    let visible = r#"tool web_fetch
{"data":{"url":"https://example.test"}}"#;

    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_renders_summary_only_completed_result() {
    let app = app();
    let visible = r#"tool web_search
{"summary":"3 relevant results"}"#;

    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_renders_error_field_when_output_is_absent() {
    let app = app();
    let visible = r#"tool web_fetch
{"status":"error","error":"request timed out"}"#;

    assert!(tool_web_view(&app, 0, 0, visible).is_some());
}

#[test]
fn tool_web_view_renders_hovered_context_menu_with_cached_body_editor() {
    let mut app = app();
    let msg_idx = 7;
    let tool_idx = 9;
    app.chat_tool_hovered_idx = Some((msg_idx as u64) << 32 | tool_idx as u64);
    app.chat_context_menu_target = Some(chat_context_target_key(msg_idx, Some(tool_idx)));
    app.chat_context_menu_pos = Some((24.0, 48.0));
    app.chat_tool_text_editors.insert(
        tool_text_key(msg_idx, tool_idx, 0),
        text_editor::Content::with_text("cached web body"),
    );
    let visible = r#"tool web_fetch
{"output":"network body","data":{"url":"https://example.test"}}"#;

    assert!(tool_web_view(&app, msg_idx, tool_idx, visible).is_some());
}
