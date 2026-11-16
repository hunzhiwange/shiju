//! 验证 MCP 历史单元对连接状态和不同内容块的投影规则。
//! 快照关注面向用户的摘要，结构断言则守住样式、宽度与原始转录契约。

use super::*;
use base64::Engine;
use codex_protocol::mcp::CallToolResult;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";

#[test]
// 当前协议的全部连接阶段都应映射到稳定文案和对应的视觉语气。
fn mcp_inventory_connection_states() {
    use McpServerConnectionStatus as Status;

    let statuses = [
        ("unknown", None),
        ("starting", Some(Status::Starting)),
        ("failed", Some(Status::Failed)),
        ("disabled", Some(Status::Disabled)),
        ("deferred", Some(Status::NotStarted)),
        ("connected-empty", Some(Status::Connected)),
        ("cancelled", Some(Status::Cancelled)),
        ("auth", Some(Status::AuthenticationRequired)),
    ]
    .into_iter()
    .map(|(name, runtime_status)| McpServerStatus {
        name: name.to_string(),
        runtime_status,
        plugin_id: None,
        server_info: None,
        tools: HashMap::new(),
        resources: Vec::new(),
        resource_templates: Vec::new(),
        auth_status: McpAuthStatus::Unknown,
    })
    .collect::<Vec<_>>();
    let cell =
        new_mcp_tools_output_from_statuses(&statuses, McpServerStatusDetail::ToolsAndAuthOnly);
    let rendered = cell
        .display_lines(/*width*/ 100)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(rendered);
    for (name, expected) in [
        ("auth", status_style(StatusTone::Attention)),
        ("cancelled", Style::default().dim()),
        ("connected-empty", status_style(StatusTone::Success)),
        ("deferred", Style::default().dim()),
        ("disabled", Style::default().dim()),
        ("failed", status_style(StatusTone::Failure)),
        ("starting", accent_style()),
        ("unknown", Style::default().dim()),
    ] {
        let row = cell
            .lines
            .iter()
            .find(|line| line.spans.get(1).is_some_and(|span| span.content == name))
            .expect("connection-state row");
        assert_eq!(
            (row.spans[0].style, row.spans[3].style),
            (expected, expected)
        );
    }
}

#[test]
// 缺少运行时状态的旧服务端响应应退回认证字段，同时不覆盖明确的禁用状态。
fn mcp_inventory_older_app_server_authentication() {
    let statuses = serde_json::from_value::<Vec<McpServerStatus>>(json!([
        {
            "name": "legacy-auth",
            "tools": {},
            "resources": [],
            "resourceTemplates": [],
            "authStatus": "notLoggedIn"
        },
        {
            "name": "legacy-healthy",
            "tools": {"lookup": {"name": "lookup", "inputSchema": {"type": "object"}}},
            "resources": [],
            "resourceTemplates": [],
            "authStatus": "oAuth"
        },
        {
            "name": "runtime-disabled",
            "runtimeStatus": "disabled",
            "tools": {},
            "resources": [],
            "resourceTemplates": [],
            "authStatus": "notLoggedIn"
        }
    ]))
    .expect("mixed-version MCP statuses");
    let cell =
        new_mcp_tools_output_from_statuses(&statuses, McpServerStatusDetail::ToolsAndAuthOnly);
    let rendered = cell
        .display_lines(/*width*/ 100)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(rendered);
    let styles = cell
        .lines
        .iter()
        .filter(|line| {
            line.spans
                .get(1)
                .is_some_and(|span| statuses.iter().any(|status| span.content == status.name))
        })
        .map(|line| (line.spans[0].style, line.spans[3].style))
        .collect::<Vec<_>>();
    let attention = status_style(StatusTone::Attention);
    let neutral = Style::default().dim();
    assert_eq!(
        styles,
        vec![
            (attention, attention),
            (neutral, neutral),
            (neutral, neutral)
        ]
    );
}

fn result(content: Vec<Value>) -> CallToolResult {
    // 构造只含内容块的最小工具结果，便于各投影场景共享一致外壳。
    CallToolResult {
        content,
        structured_content: None,
        is_error: None,
        meta: None,
    }
}

#[test]
// 文本、媒体、资源及未知块在各种宽度下都应使用各自的稳定降级表示。
fn projected_content_preserves_width_dependent_rendering() {
    let text = "{\"result\": [1, 2, 3], \"text\": \"long output 🦀\"}";
    let malformed = json!({"type": "image", "data": PNG});
    let invalid_metadata =
        json!({"type": "text", "text": "not a valid block", "annotations": {"priority": "high"}});
    let unknown = json!({"type": "future_block", "text": "unknown output 🦀"});
    let projected = McpToolResult::new(
        result(vec![
            json!({"type": "text", "text": text}),
            json!({"type": "image", "mimeType": "image/png", "data": PNG}),
            json!({"type": "audio", "mimeType": "audio/wav", "data": "audio data"}),
            json!({"type": "resource", "resource": {"uri": "file:///text.txt", "text": "resource body"}}),
            json!({"type": "resource", "resource": {"uri": "file:///blob.bin", "blob": "binary data"}}),
            json!({"type": "resource_link", "uri": "file:///linked.txt", "name": "linked"}),
            malformed.clone(),
            invalid_metadata.clone(),
            unknown.clone(),
        ]),
        McpResultKind::Standard,
    );

    for width in [1, 8, 40, 120, RAW_TOOL_OUTPUT_WIDTH] {
        let format_text =
            |text: &str| format_and_truncate_tool_result(text, TOOL_CALL_MAX_LINES, width);
        assert_eq!(
            projected
                .content
                .iter()
                .map(|block| block.render(width))
                .collect::<Vec<_>>(),
            vec![
                format_text(text),
                "<image content>".to_string(),
                "<audio content>".to_string(),
                "embedded resource: file:///text.txt".to_string(),
                "embedded resource: file:///blob.bin".to_string(),
                "link: file:///linked.txt".to_string(),
                format_text(&malformed.to_string()),
                format_text(&invalid_metadata.to_string()),
                format_text(&unknown.to_string()),
            ],
        );
    }
}

#[test]
// 展示占位符不代表图片可用；只有完整解码成功的图片才能设置能力标记。
fn projected_image_marker_still_requires_a_complete_image() {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(PNG)
        .expect("decode PNG fixture");
    let truncated = base64::engine::general_purpose::STANDARD.encode(&bytes[..33]);
    let invalid = json!({"type": "image", "mimeType": "image/png", "data": truncated});
    let valid = json!({"type": "image", "mimeType": "image/png", "data": format!("data:image/png;base64,{PNG}")});

    let projected = McpToolResult::new(result(vec![invalid.clone()]), McpResultKind::Standard);
    assert!(!projected.has_image);
    assert_eq!(projected.content[0].render(/*width*/ 80), "<image content>");

    let projected = McpToolResult::new(result(vec![invalid, valid]), McpResultKind::Standard);
    assert!(projected.has_image);
}

#[test]
// 代码模式需保留非文本块携带的文本字段，展示摘要与原始转录各遵循自身格式。
fn code_mode_preserves_text_fields_on_nontext_and_unknown_blocks() {
    let mut cell = new_active_mcp_tool_call(
        "call-code-mode".to_string(),
        McpInvocation {
            server: "node_repl".to_string(),
            tool: "js".to_string(),
            arguments: Some(json!({"title": "Inspect results"})),
        },
        /*animations_enabled*/ false,
    );
    let unknown =
        json!({"type": "future_block", "text": "Script completed\nOutput:\nunknown-side output"});
    cell.complete(
        Duration::ZERO,
        Ok(result(vec![
            json!({"type": "image", "mimeType": "image/png", "data": PNG, "text": "Script completed\nOutput:\nimage-side output"}),
            unknown.clone(),
        ])),
    );

    let display = cell
        .display_lines(/*width*/ 200)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let transcript = cell
        .transcript_lines(/*width*/ 200)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(format!("history:\n{display}\n\ntranscript:\n{transcript}"), @r#"
    history:
    • Called Inspect results
      └ image-side output
        unknown-side output

    transcript:
    • Called node_repl.js({"title":"Inspect results"})
      └ Script completed
        Output:
        image-side output
        Script completed
        Output:
        unknown-side output
    "#);
    assert_eq!(
        cell.raw_lines(),
        vec![
            Line::from("Called node_repl.js({\"title\":\"Inspect results\"})"),
            Line::from("<image content>"),
            Line::from(format_and_truncate_tool_result(
                &unknown.to_string(),
                TOOL_CALL_MAX_LINES,
                RAW_TOOL_OUTPUT_WIDTH,
            )),
        ],
    );
}
