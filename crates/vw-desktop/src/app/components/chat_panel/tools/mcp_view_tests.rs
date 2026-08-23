//! 验证 MCP 工具识别、服务端摘要、详情格式化与生命周期状态行。
//! 服务名允许从受支持的嵌套结果读取，但详情只对合法 JSON 做美化输出。

use super::mcp_view::{
    format_mcp_detail_content, is_mcp_tool_name, is_mcp_tool_raw, looks_like_mcp_payload,
    mcp_display_name, mcp_server_name, tool_mcp_view,
};
use super::tool_meta::{tool_header_label, tool_icon};
use crate::app::App;
use crate::app::assets::Icon;
use serde_json::json;

#[test]
fn mcp_tool_names_cover_acp_and_resource_variants() {
    assert!(is_mcp_tool_name("MCP"));
    assert!(is_mcp_tool_name("mcp_linear"));
    assert!(is_mcp_tool_name("list_mcp_resources"));
    assert!(is_mcp_tool_name("list_mcp_resource_templates"));
    assert!(is_mcp_tool_name("read_mcp_resource"));
    assert!(!is_mcp_tool_name("bash"));
    assert_eq!(tool_header_label("MCP"), "MCP");
    assert_eq!(tool_header_label("mcp_linear"), "MCP");
    assert_eq!(tool_icon("MCP"), Icon::Box);
}

#[test]
fn mcp_payload_detection_covers_generic_tools_events() {
    let resources = json!([{
        "server": {"name":"rustcodegraph","transport":"stdio"},
        "resources": []
    }]);
    let capabilities = json!({
        "auth_policy":"skip",
        "auth_methods":[],
        "unsupported_capabilities":[]
    });

    assert!(looks_like_mcp_payload(&resources));
    assert!(looks_like_mcp_payload(&capabilities));
    assert!(is_mcp_tool_raw(&format!("tool tools\n{}", json!({"output": resources.to_string()}))));
    assert!(!is_mcp_tool_raw("tool tools\n{\"output\":\"plain\"}"));
}

#[test]
fn mcp_summary_prefers_server_name_and_tool_fallbacks() {
    let output = r#"[{"server":{"name":"rustcodegraph","transport":"stdio"},"prompts":[]}]"#;
    assert_eq!(mcp_display_name("MCP", &json!({"output": output})), "rustcodegraph");
    assert_eq!(mcp_display_name("mcp_linear", &json!({"input":"{}"})), "linear");
    assert_eq!(mcp_display_name("list_mcp_resources", &json!({"input":"{}"})), "资源列表");
    assert_eq!(mcp_display_name("MCP", &json!({"summary":" connected "})), "connected");
}

#[test]
fn mcp_server_name_walks_supported_result_shapes() {
    assert_eq!(mcp_server_name(&json!([{"server":{"name":"docs"}}])), Some("docs"));
    assert_eq!(
        mcp_server_name(&json!({"result":{"data":{"serverName":"github"}}})),
        Some("github")
    );
    assert_eq!(mcp_server_name(&json!({"server":{"name":" "}})), None);
}

#[test]
fn mcp_detail_content_pretty_prints_json_only() {
    assert_eq!(
        format_mcp_detail_content(r#"[{"server":{"name":"docs"}}]"#),
        "[\n  {\n    \"server\": {\n      \"name\": \"docs\"\n    }\n  }\n]"
    );
    assert_eq!(format_mcp_detail_content(" plain output "), "plain output");
}

#[test]
fn mcp_view_is_single_row_for_valid_mcp_tools() {
    let app = App::new().0;
    let visible = r#"tool MCP
{"status":"completed","output":"[{\"server\":{\"name\":\"rustcodegraph\"},\"prompts\":[]}]"}"#;

    assert!(tool_mcp_view(&app, 0, 0, visible).is_some());
    assert!(tool_mcp_view(&app, 0, 1, "tool mcp_linear\n{}").is_some());
    assert!(
        tool_mcp_view(
            &app,
            0,
            2,
            r#"tool tools
{"output":"[{\"server\":{\"name\":\"rustcodegraph\"},\"resources\":[]}]"}"#
        )
        .is_some()
    );
    assert!(tool_mcp_view(&app, 0, 3, "tool bash\n{}").is_none());
    assert!(tool_mcp_view(&app, 0, 4, "tool MCP\nnot-json").is_none());
}

#[test]
fn mcp_view_builds_running_error_and_denied_rows() {
    let app = App::new().0;

    assert!(tool_mcp_view(&app, 1, 1, "tool mcp_linear\n{\"status\":\"running\"}").is_some());
    assert!(
        tool_mcp_view(
            &app,
            1,
            2,
            "tool mcp_linear\n{\"status\":\"error\",\"error\":\"server failed\"}"
        )
        .is_some()
    );
    assert!(
        tool_mcp_view(
            &app,
            1,
            3,
            "tool mcp_linear\n{\"status\":\"denied\",\"error\":\"permission denied\"}"
        )
        .is_some()
    );
}
