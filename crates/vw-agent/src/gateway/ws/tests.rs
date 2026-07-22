//!
//! WebSocket 网关测试模块
//!
//! 本模块包含 WebSocket 网关相关功能的单元测试，主要测试以下内容：
//! - Bearer 令牌提取逻辑（从 HTTP 头和 WebSocket 协议头中提取）
//! - WebSocket 增量事件解析（工具调用开始、成功、内容块等）
//! - WebSocket 响应清洗逻辑（移除工具调用标签和 JSON 碎片）
//! - WebSocket 响应最终化逻辑（处理空响应的回退策略）
//!

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::tools::{Tool, ToolResult};
    use async_trait::async_trait;
    use axum::http::HeaderValue;
    use vw_api_types::tools::ToolResultContentDto;

    /// 测试 Bearer 令牌提取优先使用 Authorization 头
    ///
    /// 验证当同时存在 Authorization 头和 WebSocket 协议头中的令牌时，
    /// 应优先使用 Authorization 头中的令牌。
    #[test]
    fn extract_ws_bearer_token_prefers_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer from-auth-header"));
        headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("vibewindow.v1, bearer.from-protocol"),
        );

        assert_eq!(extract_ws_bearer_token(&headers).as_deref(), Some("from-auth-header"));
    }

    /// 测试解析工具调用开始事件
    ///
    /// 验证当增量消息包含 "⏳ tool-name: hint" 格式时，
    /// 应正确解析为 ToolCall 事件。
    #[test]
    fn parse_ws_delta_event_maps_tool_start() {
        let delta = format!("{DRAFT_PROGRESS_SENTINEL}⏳ shell: ls -la\n");
        assert_eq!(
            parse_ws_delta_event(&delta),
            Some(WsDeltaEvent::ToolCall {
                name: "shell".to_string(),
                hint: Some("ls -la".to_string()),
            })
        );
    }

    /// 测试解析工具调用成功事件
    ///
    /// 验证当增量消息包含 "✅ tool-name (duration)" 格式时，
    /// 应正确解析为 ToolResult 事件，标记为成功并记录执行时长。
    #[test]
    fn parse_ws_delta_event_maps_tool_success() {
        let delta = format!("{DRAFT_PROGRESS_SENTINEL}✅ shell (2s)\n");
        assert_eq!(
            parse_ws_delta_event(&delta),
            Some(WsDeltaEvent::ToolResult {
                name: "shell".to_string(),
                success: true,
                duration_secs: Some(2),
                tool_call_id: None,
                result: None,
            })
        );
    }

    #[test]
    fn parse_tool_completion_payload_trims_name_and_accepts_missing_duration() {
        assert_eq!(
            parse_tool_completion_payload("  shell command  (15s)  "),
            Some(("shell command".to_string(), Some(15)))
        );
        assert_eq!(
            parse_tool_completion_payload("shell (later)"),
            Some(("shell".to_string(), None))
        );
        assert_eq!(parse_tool_completion_payload("shell 15s"), None);
    }

    #[test]
    fn parse_tool_completion_payload_handles_zero_and_requires_closing_parenthesis() {
        assert_eq!(
            parse_tool_completion_payload("shell (0s)"),
            Some(("shell".to_string(), Some(0)))
        );
        assert_eq!(parse_tool_completion_payload("shell (2s"), None);
    }

    #[test]
    fn parse_ws_delta_event_maps_tool_failure() {
        let delta = format!("{DRAFT_PROGRESS_SENTINEL}❌ shell (3s)");
        assert_eq!(
            parse_ws_delta_event(&delta),
            Some(WsDeltaEvent::ToolResult {
                name: "shell".to_string(),
                success: false,
                duration_secs: Some(3),
                tool_call_id: None,
                result: None,
            })
        );
    }

    #[test]
    fn parse_ws_delta_event_maps_tool_start_without_hint() {
        let delta = format!("{DRAFT_PROGRESS_SENTINEL}⏳ shell");
        assert_eq!(
            parse_ws_delta_event(&delta),
            Some(WsDeltaEvent::ToolCall { name: "shell".to_string(), hint: None })
        );
    }

    #[test]
    fn parse_ws_delta_event_ignores_empty_or_malformed_progress() {
        assert_eq!(parse_ws_delta_event(DRAFT_CLEAR_SENTINEL), None);
        assert_eq!(parse_ws_delta_event(""), None);
        assert_eq!(parse_ws_delta_event(&format!("{DRAFT_PROGRESS_SENTINEL}⏳   ")), None);
        assert_eq!(parse_ws_delta_event(&format!("{DRAFT_PROGRESS_SENTINEL}✅ shell")), None);
        assert_eq!(parse_ws_delta_event(&format!("{DRAFT_PROGRESS_SENTINEL}working")), None);
    }

    /// 测试解析 WebSocket 私有结构化工具结果事件。
    ///
    /// 验证当内部进度流携带 ws 私有 ToolResultDto 事件时，
    /// 网关能够优先恢复 tool_call_id 与结构化结果，并继续映射为 tool_result 事件。
    #[test]
    fn parse_ws_delta_event_maps_structured_tool_result() {
        let delta = format!(
            "{DRAFT_PROGRESS_SENTINEL}{DRAFT_WS_EVENT_SENTINEL}{{\"event\":\"tool_result\",\"name\":\"shell\",\"success\":true,\"duration_secs\":2,\"result\":{{\"tool_use_id\":\"call_1\",\"tool_id\":\"shell\",\"success\":true,\"content\":[{{\"type\":\"text\",\"text\":\"pwd\"}}],\"data\":{{}},\"model_result\":\"pwd\"}}}}\n"
        );
        assert_eq!(
            parse_ws_delta_event(&delta),
            Some(WsDeltaEvent::ToolResult {
                name: "shell".to_string(),
                success: true,
                duration_secs: Some(2),
                tool_call_id: Some("call_1".to_string()),
                result: Some(ToolResultDto {
                    tool_use_id: Some("call_1".to_string()),
                    tool_id: Some("shell".into()),
                    success: Some(true),
                    content: vec![ToolResultContentDto::Text { text: "pwd".to_string() }],
                    data: serde_json::json!({}),
                    model_result: serde_json::json!("pwd"),
                    render_hint: None,
                    permission_request: None,
                    context_updates: Vec::new(),
                    extra_messages: Vec::new(),
                    telemetry: None,
                }),
            })
        );
    }

    #[test]
    fn parse_ws_private_event_uses_result_defaults_and_tool_call_id_fallback() {
        let progress = format!(
            "{DRAFT_WS_EVENT_SENTINEL}{{\"event\":\"tool_result\",\"tool_call_id\":\"call_fallback\",\"result\":{{\"tool_id\":\"file_read\",\"content\":[],\"data\":null,\"model_result\":null}}}}"
        );

        assert_eq!(
            parse_ws_private_event(&progress),
            Some(WsDeltaEvent::ToolResult {
                name: "file_read".to_string(),
                success: false,
                duration_secs: None,
                tool_call_id: Some("call_fallback".to_string()),
                result: Some(ToolResultDto {
                    tool_use_id: None,
                    tool_id: Some("file_read".into()),
                    success: None,
                    content: Vec::new(),
                    data: serde_json::Value::Null,
                    model_result: serde_json::Value::Null,
                    render_hint: None,
                    permission_request: None,
                    context_updates: Vec::new(),
                    extra_messages: Vec::new(),
                    telemetry: None,
                }),
            })
        );
    }

    #[test]
    fn parse_ws_private_event_prefers_structured_result_metadata() {
        let result = ToolResultDto {
            tool_use_id: Some("call_result".to_string()),
            tool_id: Some("file_read".into()),
            success: Some(false),
            content: Vec::new(),
            data: serde_json::Value::Null,
            model_result: serde_json::Value::Null,
            render_hint: None,
            permission_request: None,
            context_updates: Vec::new(),
            extra_messages: Vec::new(),
            telemetry: None,
        };
        let progress = format!(
            "{DRAFT_WS_EVENT_SENTINEL}{}",
            serde_json::json!({
                "event": "tool_result",
                "success": true,
                "duration_secs": 4,
                "tool_call_id": "call_top_level",
                "result": result,
            })
        );

        assert_eq!(
            parse_ws_private_event(&progress),
            Some(WsDeltaEvent::ToolResult {
                name: "file_read".to_string(),
                success: false,
                duration_secs: Some(4),
                tool_call_id: Some("call_result".to_string()),
                result: Some(result),
            })
        );
    }

    #[test]
    fn parse_ws_private_event_falls_back_when_structured_result_is_invalid() {
        let progress = format!(
            "{DRAFT_WS_EVENT_SENTINEL}{}",
            serde_json::json!({
                "event": "tool_result",
                "name": "shell",
                "success": true,
                "tool_call_id": "call_top_level",
                "result": "not-a-tool-result",
            })
        );

        assert_eq!(
            parse_ws_private_event(&progress),
            Some(WsDeltaEvent::ToolResult {
                name: "shell".to_string(),
                success: true,
                duration_secs: None,
                tool_call_id: Some("call_top_level".to_string()),
                result: None,
            })
        );
    }

    #[test]
    fn parse_ws_private_event_rejects_invalid_or_unrelated_payloads() {
        assert_eq!(parse_ws_private_event("plain progress"), None);
        assert_eq!(parse_ws_private_event(&format!("{DRAFT_WS_EVENT_SENTINEL}not-json")), None);
        assert_eq!(parse_ws_private_event(&format!("{DRAFT_WS_EVENT_SENTINEL}{{}}")), None);
        assert_eq!(
            parse_ws_private_event(&format!("{DRAFT_WS_EVENT_SENTINEL}{{\"event\":1}}")),
            None
        );
        assert_eq!(
            parse_ws_private_event(&format!(
                "{DRAFT_WS_EVENT_SENTINEL}{{\"event\":\"tool_call\"}}"
            )),
            None
        );
    }

    /// 测试普通文本被解析为内容块事件
    ///
    /// 验证不匹配特殊格式的普通文本增量消息，
    /// 应被解析为 ContentChunk 事件。
    #[test]
    fn parse_ws_delta_event_treats_plain_text_as_chunk() {
        let delta = "partial response ".to_string();
        assert_eq!(parse_ws_delta_event(&delta), Some(WsDeltaEvent::ContentChunk(delta)));
    }

    #[test]
    fn parse_ws_delta_event_maps_tool_failure_without_parseable_duration() {
        let delta = format!("{DRAFT_PROGRESS_SENTINEL}❌ shell (later)");
        assert_eq!(
            parse_ws_delta_event(&delta),
            Some(WsDeltaEvent::ToolResult {
                name: "shell".to_string(),
                success: false,
                duration_secs: None,
                tool_call_id: None,
                result: None,
            })
        );
    }

    /// 测试从 WebSocket 协议头读取 Bearer 令牌
    ///
    /// 验证当不存在 Authorization 头时，
    /// 应从 Sec-WebSocket-Protocol 头的 "bearer.token" 格式中提取令牌。
    #[test]
    fn extract_ws_bearer_token_reads_websocket_protocol_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("vibewindow.v1, bearer.protocol-token"),
        );

        assert_eq!(extract_ws_bearer_token(&headers).as_deref(), Some("protocol-token"));
    }

    #[test]
    fn extract_ws_bearer_token_uses_first_nonempty_protocol_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static(" , vibewindow.v1, bearer.first-token, bearer.second-token"),
        );

        assert_eq!(extract_ws_bearer_token(&headers).as_deref(), Some("first-token"));
    }

    #[test]
    fn extract_ws_bearer_token_falls_back_after_non_utf8_authorization() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_bytes(b"\xff").expect("non-UTF-8 header value should be valid"),
        );
        headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("vibewindow.v1, bearer.protocol-token"),
        );

        assert_eq!(extract_ws_bearer_token(&headers).as_deref(), Some("protocol-token"));
    }

    #[test]
    fn extract_ws_bearer_token_rejects_non_utf8_protocol_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_bytes(b"\xff").expect("non-UTF-8 header value should be valid"),
        );

        assert!(extract_ws_bearer_token(&headers).is_none());
    }

    #[test]
    fn extract_ws_bearer_token_trims_authorization_and_protocol_values() {
        let mut auth_headers = HeaderMap::new();
        auth_headers.insert(header::AUTHORIZATION, HeaderValue::from_static("  Bearer token-1  "));
        assert_eq!(extract_ws_bearer_token(&auth_headers).as_deref(), Some("token-1"));

        let mut protocol_headers = HeaderMap::new();
        protocol_headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static(" vibewindow.v1 , bearer.token-2 "),
        );
        assert_eq!(extract_ws_bearer_token(&protocol_headers).as_deref(), Some("token-2"));
    }

    /// 测试忽略不含 bearer 值的协议头
    ///
    /// 验证当 Sec-WebSocket-Protocol 头不包含 "bearer." 格式时，
    /// 应返回 None。
    #[test]
    fn extract_ws_bearer_token_ignores_protocol_without_bearer_value() {
        let mut headers = HeaderMap::new();
        headers.insert(header::SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static("vibewindow.v1"));

        assert!(extract_ws_bearer_token(&headers).is_none());
    }

    /// 测试拒绝空令牌
    ///
    /// 验证当 Authorization 头或协议头中包含空令牌时，
    /// 应返回 None 而非空字符串。
    #[test]
    fn extract_ws_bearer_token_rejects_empty_tokens() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer    "));
        headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("vibewindow.v1, bearer."),
        );

        assert!(extract_ws_bearer_token(&headers).is_none());
    }

    /// 模拟调度工具
    ///
    /// 用于测试的 Mock 工具实现，模拟 schedule 工具的行为。
    struct MockScheduleTool;

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Tool for MockScheduleTool {
        fn name(&self) -> &str {
            "schedule"
        }

        fn description(&self) -> &str {
            "模拟调度工具"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string" }
                }
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
        }
    }

    /// 测试清洗 WebSocket 响应时移除工具调用标签
    ///
    /// 验证 sanitize_ws_response 函数能够：
    /// 1. 移除工具调用标签（如 "èrement {"name":"schedule",...}"）
    /// 2. 保留工具调用之外的其他文本
    #[test]
    fn sanitize_ws_response_removes_tool_call_tags() {
        let input = r#"Before
    <tool_call>
    {"name":"schedule","arguments":{"action":"create"}}
    </tool_call>
    After"#;

        let result = sanitize_ws_response(input, &[]);
        let normalized =
            result.lines().filter(|line| !line.trim().is_empty()).collect::<Vec<_>>().join("\n");
        assert_eq!(normalized, "Before\nAfter");
        assert!(!result.contains("<tool_call>"));
        assert!(!result.contains("\"name\":\"schedule\""));
    }

    /// 测试清洗 WebSocket 响应时移除孤立的工具 JSON 碎片
    ///
    /// 验证 sanitize_ws_response 函数能够识别并移除：
    /// 1. 工具调用 JSON 对象（匹配已知工具名称）
    /// 2. 工具结果 JSON 对象
    /// 3. 保留非 JSON 格式的普通文本
    #[test]
    fn sanitize_ws_response_removes_isolated_tool_json_artifacts() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockScheduleTool)];
        let input = r#"{"name":"schedule","parameters":{"action":"create"}}
    {"result":{"status":"scheduled"}}
    Reminder set successfully."#;

        let result = sanitize_ws_response(input, &tools);
        assert_eq!(result, "Reminder set successfully.");
        assert!(!result.contains("\"name\":\"schedule\""));
        assert!(!result.contains("\"result\""));
    }

    #[test]
    fn sanitize_ws_response_returns_error_when_only_malformed_tool_output_remains() {
        let input = r#"<tool_call>
{"name":"schedule","arguments":{"action":"create"}}
</tool_call>"#;

        let result = sanitize_ws_response(input, &[]);
        assert_eq!(
            result,
            "I encountered malformed tool-call output and could not produce a safe reply. Please try again."
        );
    }

    /// 测试最终化 WebSocket 响应时使用静态回退（当无可用内容时）
    ///
    /// 验证 finalize_ws_response 函数的最终回退策略：
    /// 当最终文本为空时，
    /// 应使用预定义的静态回退消息（EMPTY_WS_RESPONSE_FALLBACK）。
    #[test]
    fn finalize_ws_response_uses_static_fallback_when_nothing_available() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockScheduleTool)];
        let history = vec![ChatMessage::system("sys")];

        let result = finalize_ws_response("", &history, &tools);
        assert_eq!(result, EMPTY_WS_RESPONSE_FALLBACK);
    }

    #[test]
    fn finalize_ws_response_uses_static_fallback_for_whitespace_only_response() {
        let history = vec![ChatMessage::system("sys")];

        let result = finalize_ws_response(" \n\t ", &history, &[]);
        assert_eq!(result, EMPTY_WS_RESPONSE_FALLBACK);
    }

    #[test]
    fn finalize_ws_response_returns_sanitized_non_empty_response() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockScheduleTool)];
        let history = vec![ChatMessage::system("sys")];

        let result = finalize_ws_response("hello", &history, &tools);
        assert_eq!(result, "hello");
    }
}
