use super::advanced_view::*;
use crate::state::explicit_advanced_tool_surface_spec;
use serde_json::json;

#[test]
fn advanced_surface_tool_accepts_known_tools_and_mcp_prefixes() {
    assert!(is_advanced_surface_tool("AgentTool"));
    assert!(is_advanced_surface_tool("Agent"));
    assert!(is_advanced_surface_tool("browser_open"));
    assert!(is_advanced_surface_tool("open_browser_page"));
    assert!(is_advanced_surface_tool("mcp_linear"));
    assert!(!is_advanced_surface_tool("bash"));
}

#[test]
fn nested_string_walks_object_path() {
    let value = json!({"result":{"data":{"message":"done"}}});
    assert_eq!(nested_string(&value, &["result", "data", "message"]), Some("done"));
    assert_eq!(nested_string(&value, &["result", "missing"]), None);
    assert_eq!(nested_string(&json!({"result":{"message":"   "}}), &["result", "message"]), None);
}

#[test]
fn result_data_message_reads_nested_message() {
    assert_eq!(
        result_data_message(&json!({"result":{"data":{"message":"ok"}}})),
        Some("ok".to_string())
    );
}

#[test]
fn advanced_base_title_uses_explicit_and_builtin_labels() {
    assert_eq!(
        advanced_base_title(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "Agent",
            None
        ),
        "AgentTool"
    );
    assert_eq!(
        advanced_base_title(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            None
        ),
        "浏览器"
    );
    assert_eq!(
        advanced_base_title(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser_open",
            None
        ),
        "打开页面"
    );
    assert_eq!(
        advanced_base_title(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "read_file",
            None
        ),
        "读取"
    );
    assert_eq!(
        advanced_base_title(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "mcp_linear",
            explicit_advanced_tool_surface_spec("mcp_linear")
        ),
        "mcp_* 集成"
    );
}

#[test]
fn advanced_summary_prefers_summary_and_formats_inputs() {
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            &json!({"summary":" ready "})
        ),
        "ready"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            &json!({"input":"{\"agent\":\"reviewer\",\"prompt\":\"inspect patch\"}"})
        ),
        "reviewer · inspect patch"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"input":"{\"action\":\"click\",\"selector\":\"#save\"}"})
        ),
        "click · #save"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"input":"{\"action\":\"open\",\"url\":\"local-page\"}"})
        ),
        "open · local-page"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"input":"not-json"})
        ),
        "not-json"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser_open",
            &json!({"input":"{\"url\":\"https://example.test\"}"})
        ),
        "https://example.test"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "unknown",
            &json!({"input":"{}"})
        ),
        ""
    );
}

#[test]
fn advanced_summary_handles_partial_agent_and_default_browser_inputs() {
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            &json!({"input":"{\"agent\":\"reviewer\"}"})
        ),
        "调用 reviewer"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "Agent",
            &json!({"input":"{\"task\":\"inspect patch\"}"})
        ),
        "inspect patch"
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "Agent",
            &json!({"input":"{}"})
        ),
        ""
    );
    assert_eq!(
        advanced_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"input":"{}"})
        ),
        "browser"
    );
}

#[test]
fn advanced_metadata_text_formats_tool_specific_parts() {
    assert_eq!(
        advanced_metadata_text(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            &json!({"result":{"data":{"agent":" reviewer ","session_id":" s1 "}}}),
            None
        ),
        "reviewer · session s1"
    );
    assert_eq!(
        advanced_metadata_text(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({
                "renderHint":{"metadata":{"action":" open ","backend":" playwright "}},
                "result":{"data":{"result":{"url":"local-page"}}}
            }),
            None
        ),
        "open · playwright · local-page"
    );
    assert_eq!(
        advanced_metadata_text(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"result":{"data":{"result":{"title":"Page title"}}}}),
            None
        ),
        "Page title"
    );
    assert_eq!(
        advanced_metadata_text(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser_open",
            &json!({"renderHint":{"metadata":{"browser":"Chrome"}},"result":{"data":{"url":"local-page"}}}),
            None
        ),
        "Chrome · local-page"
    );
    assert_eq!(
        advanced_metadata_text(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "open_browser_page",
            &json!({"renderHint":{"metadata":{"url":"https://example.test"}}}),
            None
        ),
        "https://example.test"
    );
    assert_eq!(
        advanced_metadata_text(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "unknown",
            &json!({}),
            None
        ),
        ""
    );
    assert_eq!(
        advanced_metadata_text(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "mcp_linear",
            &json!({}),
            explicit_advanced_tool_surface_spec("mcp_linear")
        ),
        "状态: planned"
    );
}

#[test]
fn advanced_fallback_body_formats_explicit_and_agent_defaults() {
    assert_eq!(
        advanced_fallback_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "mcp_linear",
            &json!({}),
            explicit_advanced_tool_surface_spec("mcp_linear")
        ),
        "mcp_* 集成 当前已明确标记为 planned，桌面端先提供清晰状态，不补完整后端。"
    );
    assert_eq!(
        advanced_fallback_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            &json!({"result":{"data":{"message":"done"}}}),
            None
        ),
        "done"
    );
    assert_eq!(
        advanced_fallback_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({}),
            None
        ),
        ""
    );
}

#[test]
fn advanced_success_body_formats_agent_browser_and_plan_results() {
    assert_eq!(
        advanced_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            r#"{"message":" done "}"#,
            &json!({})
        ),
        "done"
    );
    assert_eq!(
        advanced_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            "raw",
            &json!({"result":{"data":{"message":"from result"}}})
        ),
        "from result"
    );
    assert_eq!(
        advanced_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "AgentTool",
            "raw",
            &json!({})
        ),
        "raw"
    );
    assert_eq!(
        advanced_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            "raw",
            &json!({"result":{"data":{"result":{"message":"loaded"}}}})
        ),
        "loaded"
    );
    assert_eq!(
        advanced_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "unknown",
            "raw",
            &json!({})
        ),
        "raw"
    );
}

#[test]
fn format_browser_success_body_covers_open_and_result_shapes() {
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser_open",
            &json!({"result":{"data":{"browser":"Chrome"}}})
        ),
        Some("已在 Chrome 中打开页面。".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser_open",
            &json!({"result":{"data":{"url":"local-page"}}})
        ),
        Some("已打开 local-page".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "open_browser_page",
            &json!({"result":{"data":{"browser":"Chrome","url":"local-page"}}})
        ),
        Some("已在 Chrome 中打开 local-page".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser_open",
            &json!({"result":{"data":{}}})
        ),
        None
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({})
        ),
        None
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"result":{"data":{"result":{"text":"hello"}}}})
        ),
        Some("hello".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"result":{"data":{"result":{"path":"/tmp/out"}}}})
        ),
        Some("结果路径: /tmp/out".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"result":{"data":{"result":{"title":"Title","url":"local-page"}}}})
        ),
        Some("Title\nlocal-page".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"result":{"data":{"result":{"value":1}}}})
        ),
        Some("{\n  \"value\": 1\n}".to_string())
    );
}

#[test]
fn format_browser_success_body_uses_single_title_or_url() {
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"result":{"data":{"result":{"title":"Only title"}}}})
        ),
        Some("Only title".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            "browser",
            &json!({"result":{"data":{"result":{"url":"local-page"}}}})
        ),
        Some("local-page".to_string())
    );
    assert_eq!(result_data_message(&json!({"result":{"data":{"message":"   "}}})), None);
}

#[test]
fn compact_browser_success_summary_merges_multiline_snapshot_text() {
    assert_eq!(
        compact_browser_success_summary("Captured page snapshot\n\nsnapshot · browser\n  Get text"),
        "Captured page snapshot snapshot · browser Get text"
    );
}

#[test]
fn desktop_gettext_area_8_browser_success_preserves_raw_url_and_argument_order() {
    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::English);
    let value = serde_json::json!({"result": {"data": {"browser": "RawBrowser", "url": "https://example.invalid/中文/%n/%s"}}});
    let text = format_browser_success_body(i18n, "browser_open", &value).unwrap();
    assert_eq!(text, "Opened https://example.invalid/中文/%n/%s in RawBrowser");
}
