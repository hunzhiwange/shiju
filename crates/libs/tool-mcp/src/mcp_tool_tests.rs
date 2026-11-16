//! 验证依赖应用 Host 类型的动态 MCP 工具构造及调用结果转换。
//! 同时覆盖文本、结构化及非文本内容，确保服务端失败信息保留调用上下文。

use super::mcp_tool::McpTool;
use super::{Tool, ToolSpec};
use rmcp::model::{
    CallToolResult, Content, JsonObject, RawAudioContent, RawContent, Tool as McpToolDefinition,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use vw_api_tools::ToolResultContentDto;

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
fn mcp_tool_new_builds_dynamic_tool_from_definition() {
    let tool = McpTool::new(
        "Docs Server",
        tool("Search Docs", "Search documentation.", json!({ "type": "object" })),
        Some("Use concise answers."),
    )
    .expect("definition should build dynamic MCP tool");

    assert_eq!(tool.name(), "mcp__docs_server__search_docs_7c6e618c831cfc91");
    assert_eq!(
        tool.description(),
        "Search documentation.\n\nServer instructions: Use concise answers."
    );
    assert_eq!(tool.spec().display_name, "Docs Server: Search Docs");
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

fn dynamic_tool(result: CallToolResult, calls: Arc<Mutex<Vec<(String, JsonObject)>>>) -> McpTool {
    McpTool::with_call_handler("docs", "search", dynamic_spec(), move |tool_name, arguments| {
        calls.lock().expect("calls lock").push((tool_name, arguments));
        let result = result.clone();
        async move { Ok(result) }
    })
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
