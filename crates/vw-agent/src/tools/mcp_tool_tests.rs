//! 验证动态 MCP 工具的标识、规格映射、服务发现及调用结果转换。
//! 同时覆盖文本、结构化及非文本内容，确保服务端失败信息保留调用上下文。

use super::mcp_common::with_server_peer;
use super::mcp_tool::{McpTool, discover_mcp_tool_specs, mcp_tool_id, mcp_tools_to_specs};
use super::{Tool, ToolSpec};
use agent_client_protocol::{HttpHeader, McpServer, McpServerHttp, McpServerStdio};
use rmcp::model::{
    CallToolResult, Content, JsonObject, RawAudioContent, RawContent, Tool as McpToolDefinition,
    ToolAnnotations,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use vw_api_types::tools::ToolResultContentDto;

fn schema(value: serde_json::Value) -> JsonObject {
    match value {
        serde_json::Value::Object(object) => object,
        _ => panic!("test schema must be a JSON object"),
    }
}

fn tool(name: &str, description: &str, input_schema: serde_json::Value) -> McpToolDefinition {
    McpToolDefinition::new(name.to_string(), description.to_string(), schema(input_schema))
}

#[test]
fn mcp_tool_spec_ids_are_stable_and_prefixed() {
    let specs = mcp_tools_to_specs(
        "GitHub Server",
        None,
        &[tool(
            "Search Issues",
            "Search repository issues.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            }),
        )],
        &[],
    )
    .expect("tool spec mapping should succeed");

    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].id, "mcp__github_server__search_issues");
    assert_eq!(specs[0].name, specs[0].id);
    assert_eq!(specs[0].display_name, "GitHub Server: Search Issues");
    assert_eq!(mcp_tool_id("GitHub Server", "Search Issues").unwrap(), specs[0].id);
}

#[test]
fn mcp_tool_new_builds_dynamic_tool_from_definition() {
    let tool = McpTool::new(
        "Docs Server",
        tool("Search Docs", "Search documentation.", json!({ "type": "object" })),
        Some("Use concise answers."),
    )
    .expect("definition should build dynamic MCP tool");

    assert_eq!(tool.name(), "mcp__docs_server__search_docs");
    assert_eq!(
        tool.description(),
        "Search documentation.\n\nServer instructions: Use concise answers."
    );
    assert_eq!(tool.spec().display_name, "Docs Server: Search Docs");
}

#[test]
fn mcp_tool_spec_maps_schema_and_annotations() {
    let input_schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "path": { "type": "string" }
        },
        "required": ["path"]
    });
    let annotations = ToolAnnotations::new().read_only(true).destructive(false).idempotent(true);
    let spec = mcp_tools_to_specs(
        "docs",
        Some("Use for public documentation lookups only.\nBearer abc.def must stay private."),
        &[tool("fetch-page", "Fetch a documentation page.", input_schema.clone())
            .annotate(annotations)],
        &[],
    )
    .expect("tool spec mapping should succeed")
    .pop()
    .expect("spec should exist");

    assert_eq!(spec.input_schema, input_schema);
    assert_eq!(spec.parameters, spec.input_schema);
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
    assert!(spec.description.contains("Fetch a documentation page."));
    assert!(
        spec.description
            .contains("Server instructions: Use for public documentation lookups only.")
    );
    assert!(!spec.description.contains("Bearer abc.def"));
}

#[test]
fn mcp_tool_spec_uses_fallback_description_and_truncates_long_descriptions() {
    let fallback = mcp_tools_to_specs(
        "docs",
        None,
        &[tool("blank", " \n\t ", json!({ "type": "object" }))],
        &[],
    )
    .expect("blank description should map")
    .pop()
    .expect("fallback spec");
    assert_eq!(fallback.description, "Remote MCP tool.");

    let long_description = "a".repeat(1_400);
    let long_instructions = "Use this server for documentation lookups. ".repeat(20);
    let spec = mcp_tools_to_specs(
        "docs",
        Some(&long_instructions),
        &[tool("long", &long_description, json!({ "type": "object" }))],
        &[],
    )
    .expect("long description should map")
    .pop()
    .expect("long spec");

    assert_eq!(spec.description.chars().count(), 1_200);
    assert!(spec.description.ends_with("..."));
}

#[test]
fn mcp_tool_spec_rejects_colliding_ids() {
    let error = mcp_tools_to_specs(
        "docs",
        None,
        &[
            tool("fetch-page", "Fetch a documentation page.", json!({ "type": "object" })),
            tool("fetch page", "Fetch a page by alias.", json!({ "type": "object" })),
        ],
        &["mcp__other__tool"],
    )
    .expect_err("normalized tool id collision should be rejected");

    assert!(error.to_string().contains("duplicate MCP tool id 'mcp__docs__fetch_page'"));

    let builtin_error = mcp_tools_to_specs(
        "docs",
        None,
        &[tool("Fetch", "Fetch a page.", json!({ "type": "object" }))],
        &["mcp__docs__fetch"],
    )
    .expect_err("colliding reserved ids should be rejected");

    assert!(
        builtin_error
            .to_string()
            .contains("MCP tool id 'mcp__docs__fetch' collides with an existing tool")
    );
}

#[test]
fn mcp_tool_id_rejects_segments_without_ascii_alphanumeric_characters() {
    let server_error = mcp_tool_id("---", "fetch").expect_err("blank server id segment");
    assert!(server_error.to_string().contains("invalid MCP server name '---'"));

    let tool_error = mcp_tool_id("docs", "你好").expect_err("blank tool id segment");
    assert!(tool_error.to_string().contains("invalid MCP tool name '你好'"));
}

fn dynamic_spec() -> ToolSpec {
    ToolSpec::new(
        "mcp__docs__search",
        "Search docs.",
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "query": { "type": "string" }
            },
            "required": ["query"]
        }),
    )
    .with_display_name("docs: search")
    .with_read_only(true)
    .with_destructive(false)
    .with_concurrency_safe(true)
    .with_requires_user_interaction(false)
    .with_strict(true)
}

// fixture 服务用于验证真实 stdio 握手；其它结果转换测试使用内存处理器保持聚焦。
fn write_mcp_fixture_server(dir: &std::path::Path) -> std::path::PathBuf {
    let script = dir.join("fixture_mcp_server.py");
    std::fs::write(
        &script,
        r#"
import json
import sys

TOOL = {
    "name": "Echo Tool",
    "description": "Echo a message.",
    "inputSchema": {"type": "object", "properties": {"message": {"type": "string"}}},
    "annotations": {"readOnlyHint": True, "destructiveHint": False, "idempotentHint": True}
}

def send(payload):
    sys.stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    if method == "initialize":
        params = message.get("params") or {}
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "fixture-mcp", "version": "1.0.0"},
                "instructions": "Fixture instructions."
            }
        })
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": request_id, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        params = message.get("params") or {}
        arguments = params.get("arguments") or {}
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": "echo:" + str(arguments.get("message", ""))}],
                "structuredContent": {"echoed": arguments.get("message", "")},
                "isError": False
            }
        })
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": {}})
"#,
    )
    .expect("write MCP fixture server");
    script
}

fn fixture_server(script: &std::path::Path) -> McpServer {
    McpServer::Stdio(
        McpServerStdio::new(
            "Fixture Server",
            std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
        )
        .args(vec![script.to_string_lossy().to_string()]),
    )
}

fn dynamic_tool(result: CallToolResult, calls: Arc<Mutex<Vec<(String, JsonObject)>>>) -> McpTool {
    McpTool::with_call_handler("docs", "search", dynamic_spec(), move |tool_name, arguments| {
        calls.lock().expect("calls lock").push((tool_name, arguments));
        let result = result.clone();
        async move { Ok(result) }
    })
}

#[tokio::test]
async fn discover_and_call_configured_mcp_tool_through_stdio_fixture() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let script = write_mcp_fixture_server(temp_dir.path());
    let server = fixture_server(&script);

    let specs = with_server_peer(&server, |peer| {
        Box::pin(async move {
            discover_mcp_tool_specs(peer, "Fixture Server", Some("Fixture instructions."), &[])
                .await
        })
    })
    .await
    .expect("fixture should list tools");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].id, "mcp__fixture_server__echo_tool");

    let tool = McpTool::from_configured_server(server, "Echo Tool", specs[0].clone());
    let result =
        tool.call(json!({ "message": "hello" })).await.expect("fixture tool should execute");

    assert!(result.is_success());
    assert_eq!(result.model_result, json!("echo:hello"));
    assert_eq!(result.data["structured_content"], json!({ "echoed": "hello" }));
}

#[tokio::test]
async fn mcp_tool_call_forwards_arguments() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tool = dynamic_tool(CallToolResult::success(vec![Content::text("ok")]), calls.clone());
    let input = json!({
        "query": "rust mcp",
        "limit": 3
    });

    let result = tool.call(input.clone()).await.expect("call should succeed");

    assert!(result.is_success());
    let calls = calls.lock().expect("calls lock");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "search");
    assert_eq!(serde_json::Value::Object(calls[0].1.clone()), input);
}

#[tokio::test]
async fn mcp_tool_call_maps_text_content_for_model() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tool = dynamic_tool(
        CallToolResult::success(vec![Content::text("first line"), Content::text("second line")]),
        calls,
    );

    let result = tool.call(json!({ "query": "text" })).await.expect("call should succeed");

    assert_eq!(result.model_result, json!("first line\nsecond line"));
    assert_eq!(
        result.content_blocks,
        vec![
            ToolResultContentDto::Text { text: "first line".to_string() },
            ToolResultContentDto::Text { text: "second line".to_string() },
        ]
    );
    assert_eq!(result.data["success"], json!(true));
    assert_eq!(result.data["content"][0]["text"], json!("first line"));
}

#[tokio::test]
async fn mcp_tool_call_preserves_non_text_content_as_json() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let audio = Content::new(
        RawContent::Audio(RawAudioContent {
            data: "UklGRg==".to_string(),
            mime_type: "audio/wav".to_string(),
        }),
        None,
    );
    let tool = dynamic_tool(CallToolResult::success(vec![audio]), calls);

    let result = tool.call(json!({ "query": "audio" })).await.expect("call should succeed");

    assert!(result.is_success());
    assert_eq!(result.model_result[0]["type"], json!("audio"));
    assert_eq!(
        result.content_blocks.first(),
        Some(&ToolResultContentDto::Json {
            value: json!({
                "type": "audio",
                "data": "UklGRg==",
                "mimeType": "audio/wav"
            })
        })
    );
}

#[tokio::test]
async fn mcp_tool_call_uses_structured_content_when_text_is_absent() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let result = CallToolResult {
        content: Vec::new(),
        structured_content: Some(json!({ "answer": 42 })),
        is_error: Some(false),
        meta: None,
    };
    let tool = dynamic_tool(result, calls);

    let result = tool.call(json!({ "query": "structured" })).await.expect("structured result");

    assert!(result.is_success());
    assert_eq!(result.model_result, json!({ "answer": 42 }));
    assert_eq!(result.data["structured_content"], json!({ "answer": 42 }));
}

#[tokio::test]
async fn mcp_tool_call_marks_mcp_error_as_failed() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tool = dynamic_tool(CallToolResult::error(vec![Content::text("bad input")]), calls);

    let result = tool
        .call(json!({ "query": "bad" }))
        .await
        .expect("MCP isError results should remain tool results");

    assert!(!result.is_success());
    assert_eq!(result.model_result, json!("bad input"));
    assert_eq!(result.data["success"], json!(false));
    assert_eq!(result.data["error"], json!("bad input"));
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(false));
}

#[tokio::test]
async fn mcp_tool_call_formats_structured_error_as_pretty_json() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let result = CallToolResult {
        content: Vec::new(),
        structured_content: Some(json!({ "code": "bad_input", "field": "query" })),
        is_error: Some(true),
        meta: None,
    };
    let tool = dynamic_tool(result, calls);

    let result = tool.call(json!({ "query": "bad" })).await.expect("structured error result");

    assert!(!result.is_success());
    assert_eq!(result.model_result, json!({ "code": "bad_input", "field": "query" }));
    assert_eq!(
        result.error_text().as_deref(),
        Some("{\n  \"code\": \"bad_input\",\n  \"field\": \"query\"\n}")
    );
}

#[test]
fn mcp_tool_from_spec_exposes_dynamic_tool_contract() {
    let spec = dynamic_spec();
    let tool = McpTool::from_spec("docs", "search", spec.clone());

    assert_eq!(tool.name(), spec.id);
    assert_eq!(tool.description(), spec.description);
    assert_eq!(tool.parameters_schema(), spec.input_schema);
    assert_eq!(
        serde_json::to_value(tool.spec()).expect("tool spec json"),
        serde_json::to_value(spec).expect("expected spec json")
    );
    assert!(tool.is_read_only());
    assert!(tool.is_concurrency_safe());
}

#[tokio::test]
async fn mcp_tool_from_spec_default_handler_reports_missing_configured_server() {
    let tool = McpTool::from_spec("missing-docs", "search", dynamic_spec());

    let error = tool.call(json!({})).await.unwrap_err();

    assert_eq!(error.to_string(), "failed to call MCP tool 'missing-docs::search'");
}

#[tokio::test]
async fn mcp_tool_from_configured_server_uses_server_bound_handler() {
    let server = McpServer::Http(
        McpServerHttp::new("bad-http", "https://example.com/mcp")
            .headers(vec![HttpHeader::new("invalid header", "value")]),
    );
    let tool = McpTool::from_configured_server(server, "search", dynamic_spec());

    let error = tool.call(json!({})).await.unwrap_err();

    assert_eq!(error.to_string(), "failed to call MCP tool 'bad-http::search'");
    assert!(format!("{error:#}").contains("failed to build HTTP MCP client for 'bad-http'"));
}

#[tokio::test]
async fn mcp_tool_call_rejects_non_object_input_without_invoking_handler() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tool = dynamic_tool(CallToolResult::success(vec![Content::text("unused")]), calls.clone());

    let error = tool.call(json!(["not", "an", "object"])).await.unwrap_err();

    assert_eq!(error.to_string(), "MCP tool input must be a JSON object");
    assert!(calls.lock().expect("calls lock").is_empty());
}

#[tokio::test]
async fn mcp_tool_call_adds_server_and_tool_context_to_handler_error() {
    let tool = McpTool::with_call_handler(
        "docs",
        "search",
        dynamic_spec(),
        |_tool_name, _arguments| async { Err(anyhow::anyhow!("transport closed")) },
    );

    let error = tool.call(json!({})).await.unwrap_err();

    assert_eq!(error.to_string(), "failed to call MCP tool 'docs::search'");
    assert_eq!(error.chain().nth(1).map(ToString::to_string).as_deref(), Some("transport closed"));
}

#[tokio::test]
async fn mcp_tool_execute_and_render_hint_preserve_failure_details() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tool = dynamic_tool(CallToolResult::error(vec![Content::text("denied")]), calls);

    let result = tool.execute(json!({})).await.expect("MCP error remains a tool result");
    assert!(!result.success);
    assert_eq!(result.output, "denied");
    assert_eq!(result.error.as_deref(), Some("denied"));

    let call_result = super::traits::ToolCallResult {
        model_result: json!("denied"),
        data: json!({ "error": "denied" }),
        ..super::traits::ToolCallResult::default()
    };
    let hint = tool.render_hint(&call_result).expect("fallback render hint");
    assert_eq!(hint.title.as_deref(), Some("docs: search"));
    assert_eq!(hint.kind.as_deref(), Some("mcp_tool"));
    assert_eq!(hint.summary.as_deref(), Some("denied"));
    assert_eq!(hint.metadata["server"], "docs");
    assert_eq!(hint.metadata["tool"], "search");
}
