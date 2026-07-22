use super::advanced_view::{
    advanced_base_title, advanced_fallback_body, advanced_metadata_text, advanced_success_body,
    advanced_summary, format_browser_success_body, format_tool_search_body,
    format_verify_plan_execution_body, is_advanced_surface_tool, nested_string,
    result_data_message, tool_advanced_view,
};
use crate::app::App;
use crate::app::state::explicit_advanced_tool_surface_spec;
use serde_json::json;

#[test]
fn advanced_surface_tool_accepts_known_tools_and_mcp_prefixes() {
    assert!(is_advanced_surface_tool("AgentTool"));
    assert!(is_advanced_surface_tool("Agent"));
    assert!(is_advanced_surface_tool("browser_open"));
    assert!(is_advanced_surface_tool("open_browser_page"));
    assert!(is_advanced_surface_tool("tool_search"));
    assert!(is_advanced_surface_tool("verify_plan_execution"));
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
    assert!(
        format_tool_search_body(&json!({
            "result": {"data": {"items": [{"display_name": "alpha"}]}}
        }))
        .is_some()
    );
}

#[test]
fn advanced_base_title_uses_explicit_and_builtin_labels() {
    assert_eq!(advanced_base_title("Agent", None), "AgentTool");
    assert_eq!(advanced_base_title("browser", None), "浏览器");
    assert_eq!(advanced_base_title("browser_open", None), "打开页面");
    assert_eq!(advanced_base_title("read_file", None), "读取");
    assert_eq!(
        advanced_base_title("mcp_linear", explicit_advanced_tool_surface_spec("mcp_linear")),
        "mcp_* 集成"
    );
}

#[test]
fn advanced_summary_prefers_summary_and_formats_inputs() {
    assert_eq!(advanced_summary("AgentTool", &json!({"summary":" ready "})), "ready");
    assert_eq!(
        advanced_summary(
            "AgentTool",
            &json!({"input":"{\"agent\":\"reviewer\",\"prompt\":\"inspect patch\"}"})
        ),
        "reviewer · inspect patch"
    );
    assert_eq!(
        advanced_summary(
            "browser",
            &json!({"input":"{\"action\":\"click\",\"selector\":\"#save\"}"})
        ),
        "click · #save"
    );
    assert_eq!(
        advanced_summary(
            "browser",
            &json!({"input":"{\"action\":\"open\",\"url\":\"local-page\"}"})
        ),
        "open · local-page"
    );
    assert_eq!(advanced_summary("browser", &json!({"input":"not-json"})), "not-json");
    assert_eq!(
        advanced_summary("browser_open", &json!({"input":"{\"url\":\"https://example.test\"}"})),
        "https://example.test"
    );
    assert_eq!(advanced_summary("unknown", &json!({"input":"{}"})), "");
}

#[test]
fn advanced_summary_handles_partial_agent_and_default_browser_inputs() {
    assert_eq!(
        advanced_summary("AgentTool", &json!({"input":"{\"agent\":\"reviewer\"}"})),
        "调用 reviewer"
    );
    assert_eq!(
        advanced_summary("Agent", &json!({"input":"{\"task\":\"inspect patch\"}"})),
        "inspect patch"
    );
    assert_eq!(advanced_summary("Agent", &json!({"input":"{}"})), "");
    assert_eq!(advanced_summary("browser", &json!({"input":"{}"})), "browser");
}

#[test]
fn advanced_metadata_text_formats_tool_specific_parts() {
    assert_eq!(
        advanced_metadata_text(
            "AgentTool",
            &json!({"result":{"data":{"agent":" reviewer ","session_id":" s1 "}}}),
            None
        ),
        "reviewer · session s1"
    );
    assert_eq!(
        advanced_metadata_text(
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
            "browser",
            &json!({"result":{"data":{"result":{"title":"Page title"}}}}),
            None
        ),
        "Page title"
    );
    assert_eq!(
        advanced_metadata_text(
            "browser_open",
            &json!({"renderHint":{"metadata":{"browser":"Chrome"}},"result":{"data":{"url":"local-page"}}}),
            None
        ),
        "Chrome · local-page"
    );
    assert_eq!(
        advanced_metadata_text(
            "open_browser_page",
            &json!({"renderHint":{"metadata":{"url":"https://example.test"}}}),
            None
        ),
        "https://example.test"
    );
    assert_eq!(advanced_metadata_text("unknown", &json!({}), None), "");
    assert_eq!(
        advanced_metadata_text(
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
            "enter_plan_mode",
            &json!({}),
            explicit_advanced_tool_surface_spec("enter_plan_mode")
        ),
        "进入规划模式 已接入当前会话工具面。"
    );
    assert_eq!(
        advanced_fallback_body(
            "mcp_linear",
            &json!({}),
            explicit_advanced_tool_surface_spec("mcp_linear")
        ),
        "mcp_* 集成 当前已明确标记为 planned，桌面端先提供清晰状态，不补完整后端。"
    );
    assert_eq!(
        advanced_fallback_body(
            "tool_search",
            &json!({"error":"backend failed"}),
            explicit_advanced_tool_surface_spec("tool_search")
        ),
        "backend failed"
    );
    assert_eq!(
        advanced_fallback_body("AgentTool", &json!({"result":{"data":{"message":"done"}}}), None),
        "done"
    );
    assert_eq!(advanced_fallback_body("browser", &json!({}), None), "");
}

#[test]
fn advanced_success_body_formats_agent_browser_search_and_plan_results() {
    assert_eq!(advanced_success_body("AgentTool", r#"{"message":" done "}"#, &json!({})), "done");
    assert_eq!(
        advanced_success_body(
            "AgentTool",
            "raw",
            &json!({"result":{"data":{"message":"from result"}}})
        ),
        "from result"
    );
    assert_eq!(advanced_success_body("AgentTool", "raw", &json!({})), "raw");
    assert_eq!(
        advanced_success_body(
            "browser",
            "raw",
            &json!({"result":{"data":{"result":{"message":"loaded"}}}})
        ),
        "loaded"
    );
    assert_eq!(
        advanced_success_body(
            "tool_search",
            "raw",
            &json!({"result":{"data":{"items":[{"id":"tool","reason":"match"}]}}})
        ),
        "- tool: match"
    );
    assert_eq!(
        advanced_success_body(
            "verify_plan_execution",
            "raw",
            &json!({"result":{"data":{"ready":true,"todo_count":2,"pending_count":1,"in_progress_count":0}}})
        ),
        "已满足执行条件。\nTodo: 2，待处理: 1，进行中: 0"
    );
    assert_eq!(advanced_success_body("unknown", "raw", &json!({})), "raw");
}

#[test]
fn format_browser_success_body_covers_open_and_result_shapes() {
    assert_eq!(
        format_browser_success_body(
            "browser_open",
            &json!({"result":{"data":{"browser":"Chrome"}}})
        ),
        Some("已在 Chrome 中打开页面。".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser_open",
            &json!({"result":{"data":{"url":"local-page"}}})
        ),
        Some("已打开 local-page".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "open_browser_page",
            &json!({"result":{"data":{"browser":"Chrome","url":"local-page"}}})
        ),
        Some("已在 Chrome 中打开 local-page".to_string())
    );
    assert_eq!(format_browser_success_body("browser_open", &json!({"result":{"data":{}}})), None);
    assert_eq!(format_browser_success_body("browser", &json!({})), None);
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"text":"hello"}}}})
        ),
        Some("hello".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"path":"/tmp/out"}}}})
        ),
        Some("结果路径: /tmp/out".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"title":"Title","url":"local-page"}}}})
        ),
        Some("Title\nlocal-page".to_string())
    );
    assert_eq!(
        format_browser_success_body("browser", &json!({"result":{"data":{"result":{"value":1}}}})),
        Some("{\n  \"value\": 1\n}".to_string())
    );
}

#[test]
fn format_browser_success_body_uses_single_title_or_url() {
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"title":"Only title"}}}})
        ),
        Some("Only title".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"url":"local-page"}}}})
        ),
        Some("local-page".to_string())
    );
    assert_eq!(result_data_message(&json!({"result":{"data":{"message":"   "}}})), None);
}

#[test]
fn format_verify_plan_execution_body_covers_blockers_and_empty_state() {
    assert_eq!(
        format_verify_plan_execution_body(
            &json!({"result":{"data":{"blockers":[" missing plan ",""]}}})
        ),
        Some("- missing plan".to_string())
    );
    assert_eq!(
        format_verify_plan_execution_body(
            &json!({"result":{"data":{"ready":false,"blockers":[]}}})
        ),
        None
    );
    assert_eq!(format_verify_plan_execution_body(&json!({})), None);
}

#[test]
fn format_tool_search_body_handles_empty_and_truncated_results() {
    assert_eq!(
        format_tool_search_body(&json!({"result":{"data":{"items":[]}}})),
        Some("未找到匹配工具。".to_string())
    );
    assert!(
        format_tool_search_body(&json!({
            "result": {
                "data": {
                    "count": 8,
                    "items": [
                        {"display_name":"a","reason":"ra"},
                        {"id":"b","reason":"rb"},
                        {"display_name":"c"},
                        {"display_name":"d"},
                        {"display_name":"e"},
                        {"display_name":"f"}
                    ]
                }
            }
        }))
        .expect("tool search body")
        .contains("还有 2 个结果")
    );
    assert!(format_tool_search_body(&json!({"result":{"data":{"items":[42]}}})).is_none());
    assert_eq!(
        format_tool_search_body(&json!({"result":{"data":{"items":[{"reason":"  "}]}}})),
        Some("- unknown: matched".to_string())
    );
    assert!(format_tool_search_body(&json!({"result":{"data":{}}})).is_none());
}

#[test]
fn advanced_view_rejects_unknown_tools_and_invalid_json() {
    let app = App::new().0;

    assert!(tool_advanced_view(&app, 0, 0, "tool browser").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "browser\n{}").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool browser\nnot-json").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool AgentTool\n{}").is_none());
}

#[test]
fn advanced_view_renders_agent_browser_tool_search_and_errors() {
    let mut app = App::new().0;
    app.chat_tool_hovered_idx = Some((4_u64 << 32) | 2);

    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool AgentTool
{"input":"{\"agent\":\"reviewer\",\"prompt\":\"inspect the patch\"}","result":{"data":{"message":"done","agent":"reviewer","session_id":"s1"}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"renderHint":{"metadata":{"action":"open","backend":"playwright"}},"result":{"data":{"result":{"title":"Example","url":"https://example.com"}}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool tool_search
{"result":{"data":{"items":[{"display_name":"Browser","reason":"navigation"}]}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"status":"error","error":"navigation failed"}"#
        )
        .is_some()
    );
}

#[test]
fn advanced_view_renders_running_and_planned_explicit_surfaces() {
    let app = App::new().0;

    assert!(
        tool_advanced_view(
            &app,
            0,
            0,
            r#"tool verify_plan_execution
{"status":"running","input":"{}"}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            0,
            0,
            r#"tool mcp_linear
{"input":"{}"}"#
        )
        .is_some()
    );
}

#[test]
fn advanced_view_renders_empty_body_context_menu_and_browser_open() {
    let mut app = App::new().0;
    app.chat_context_menu_target =
        Some(crate::app::components::chat_panel::utils::chat_context_target_key(1, Some(3)));
    app.chat_context_menu_pos = Some((24.0, 32.0));

    assert!(
        tool_advanced_view(
            &app,
            1,
            3,
            r#"tool browser
{"summary":"metadata only","result":{"data":{"result":{"title":"Only title"}}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            1,
            3,
            r#"tool browser_open
{"output":"opened","result":{"data":{"browser":"Chrome","url":"local-page"}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            1,
            3,
            r#"tool open_browser_page
{"result":{"success":false,"data":{"browser":"Chrome"}},"error":"denied by user"}"#
        )
        .is_some()
    );
}
