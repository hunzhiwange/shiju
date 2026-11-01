//! OpenAI 兼容性网关单元测试模块
//!
//! 本模块提供针对 OpenAI 兼容性 API 数据结构的序列化/反序列化测试。
//!
//! # 测试覆盖范围
//!
//! - **请求反序列化**：测试 `ChatCompletionsRequest` 的最小字段和完整字段解析
//! - **响应序列化**：测试 `ChatCompletionsResponse` 和 `ModelsResponse` 的 JSON 输出
//! - **流式响应**：测试 `ChatCompletionsChunk` 的序列化和可选字段省略行为
//! - **辅助函数**：测试时间戳生成和块构造函数
//! - **常量验证**：验证请求体大小限制配置

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试 ChatCompletionsRequest 的最小字段反序列化
    ///
    /// 验证仅包含必需字段（messages）的 JSON 请求能够正确解析，
    /// 可选字段（model、temperature、stream）应保持为 None。
    #[test]
    fn chat_completions_request_deserializes_minimal() {
        // 构造仅包含必需字段的最小 JSON 请求
        let json = r#"{"messages": [{"role": "user", "content": "Hello"}]}"#;

        // 反序列化 JSON 字符串到请求结构
        let req: ChatCompletionsRequest = serde_json::from_str(json).unwrap();

        // 验证可选字段未被设置
        assert!(req.model.is_none());
        assert!(req.temperature.is_none());
        assert!(req.stream.is_none());

        // 验证必需字段正确解析
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.messages[0].content, "Hello");
    }

    /// 测试 ChatCompletionsRequest 的完整字段反序列化
    ///
    /// 验证包含所有字段（model、messages、temperature、stream）的 JSON 请求
    /// 能够正确解析，包括多消息场景。
    #[test]
    fn chat_completions_request_deserializes_full() {
        // 构造包含所有字段的完整 JSON 请求
        let json = r#"{
                "model": "anthropic/claude-sonnet-4",
                "messages": [
                    {"role": "system", "content": "You are helpful"},
                    {"role": "user", "content": "Hi"}
                ],
                "temperature": 0.5,
                "stream": true
            }"#;

        // 反序列化 JSON 字符串到请求结构
        let req: ChatCompletionsRequest = serde_json::from_str(json).unwrap();

        // 验证所有字段正确解析
        assert_eq!(req.model.as_deref(), Some("anthropic/claude-sonnet-4"));
        assert_eq!(req.temperature, Some(0.5));
        assert_eq!(req.stream, Some(true));
        assert_eq!(req.messages.len(), 2);
    }

    #[test]
    fn chat_completions_request_rejects_missing_messages() {
        let error = serde_json::from_str::<ChatCompletionsRequest>(
            r#"{"model":"anthropic/claude-sonnet-4"}"#,
        )
        .expect_err("messages should be required");

        assert!(error.to_string().contains("missing field `messages`"));
    }

    #[test]
    fn chat_completions_request_rejects_message_without_content() {
        let error =
            serde_json::from_str::<ChatCompletionsRequest>(r#"{"messages":[{"role":"user"}]}"#)
                .expect_err("message content should be required");

        assert!(error.to_string().contains("missing field `content`"));
    }

    #[test]
    fn chat_completions_request_rejects_message_without_role() {
        let error =
            serde_json::from_str::<ChatCompletionsRequest>(r#"{"messages":[{"content":"Hello"}]}"#)
                .expect_err("message role should be required");

        assert!(error.to_string().contains("missing field `role`"));
    }

    #[test]
    fn chat_completions_request_accepts_null_optional_fields() {
        let req: ChatCompletionsRequest = serde_json::from_str(
            r#"{
                "model": null,
                "messages": [{"role": "user", "content": "Hello"}],
                "temperature": null,
                "stream": null
            }"#,
        )
        .expect("null optional fields should deserialize");

        assert!(req.model.is_none());
        assert!(req.temperature.is_none());
        assert!(req.stream.is_none());
    }

    #[test]
    fn chat_completions_request_ignores_unknown_compatibility_fields() {
        let req: ChatCompletionsRequest = serde_json::from_str(
            r#"{
                "messages": [{"role": "user", "content": "Hello", "name": "caller"}],
                "max_tokens": 64
            }"#,
        )
        .expect("unknown OpenAI-compatible fields should be ignored");

        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.messages[0].content, "Hello");
    }

    /// 测试 ChatCompletionsResponse 的序列化
    ///
    /// 验证聊天完成响应结构能够正确序列化为 JSON 格式，
    /// 包括所有必需字段（id、object、created、model、choices、usage）。
    #[test]
    fn chat_completions_response_serializes() {
        // 构造完整的响应结构
        let response = ChatCompletionsResponse {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion",
            created: 1_234_567_890,
            model: "test-model".to_string(),
            choices: vec![ChatCompletionsChoice {
                index: 0,
                message: ChatCompletionsResponseMessage {
                    role: "assistant",
                    content: "Hello!".to_string(),
                },
                finish_reason: "stop",
            }],
            usage: ChatCompletionsUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        };

        let json = serde_json::to_value(response).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "created": 1_234_567_890_u64,
                "model": "test-model",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "Hello!"},
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            })
        );
    }

    /// 测试 ModelsResponse 的序列化
    ///
    /// 验证模型列表响应结构能够正确序列化为 JSON 格式，
    /// 符合 OpenAI /v1/models 端点响应格式。
    #[test]
    fn models_response_serializes() {
        // 构造模型列表响应
        let response = ModelsResponse {
            object: "list",
            data: vec![ModelObject {
                id: "anthropic/claude-sonnet-4".to_string(),
                object: "model",
                created: 1_234_567_890,
                owned_by: "vibewindow".to_string(),
            }],
        };

        // 序列化响应结构为 JSON 字符串
        let json = serde_json::to_string(&response).unwrap();

        // 验证 JSON 输出包含所有必需字段
        assert!(json.contains("\"object\":\"list\""));
        assert!(json.contains("anthropic/claude-sonnet-4"));
        assert!(json.contains("vibewindow"));
    }

    #[test]
    fn models_response_serializes_empty_data_array() {
        let response = ModelsResponse { object: "list", data: Vec::new() };

        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json, serde_json::json!({"object": "list", "data": []}));
    }

    /// 测试流式响应块（ChatCompletionsChunk）的序列化
    ///
    /// 验证流式聊天完成块能够正确序列化为 SSE（Server-Sent Events）格式，
    /// 包括 delta 更新中的 role 和 content 字段。
    #[test]
    fn streaming_chunk_serializes() {
        // 构造流式响应块，包含角色和内容增量
        let chunk = ChatCompletionsChunk {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion.chunk",
            created: 1_234_567_890,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta { role: Some("assistant"), content: Some("Hello".to_string()) },
                finish_reason: None,
            }],
        };

        // 序列化块结构为 JSON 字符串
        let json = serde_json::to_string(&chunk).unwrap();

        // 验证 JSON 输出包含流式响应的关键字段
        assert!(json.contains("chat.completion.chunk"));
        assert!(json.contains("Hello"));
        assert!(json.contains("assistant"));
    }

    /// 测试流式响应块省略 None 字段的行为
    ///
    /// 验证当 delta 中的 role 和 content 为 None 时，
    /// 序列化的 JSON 不包含这些字段（遵循 skip_serializing_if 配置）。
    /// 这对于减少流式响应的网络传输开销很重要。
    #[test]
    fn streaming_chunk_omits_none_fields() {
        // 构造所有可选字段都为 None 的块
        let chunk = ChatCompletionsChunk {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion.chunk",
            created: 1_234_567_890,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta { role: None, content: None },
                finish_reason: None,
            }],
        };

        // 序列化块结构为 JSON 字符串
        let json = serde_json::to_string(&chunk).unwrap();

        // 验证 None 字段被正确省略，不在 JSON 输出中出现
        assert!(!json.contains("role"));
        assert!(!json.contains("content"));
    }

    #[test]
    fn streaming_chunk_omits_each_absent_delta_field_independently() {
        let role_only = ChatCompletionsChunk {
            id: "chatcmpl-role".to_string(),
            object: "chat.completion.chunk",
            created: 1_234_567_890,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta { role: Some("assistant"), content: None },
                finish_reason: None,
            }],
        };
        let content_only = ChatCompletionsChunk {
            id: "chatcmpl-content".to_string(),
            object: "chat.completion.chunk",
            created: 1_234_567_890,
            model: "test-model".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta { role: None, content: Some("Hello".to_string()) },
                finish_reason: None,
            }],
        };

        let role_only = serde_json::to_value(role_only).unwrap();
        let content_only = serde_json::to_value(content_only).unwrap();

        assert_eq!(role_only["choices"][0]["delta"], serde_json::json!({"role": "assistant"}));
        assert_eq!(content_only["choices"][0]["delta"], serde_json::json!({"content": "Hello"}));
        assert!(role_only["choices"][0]["finish_reason"].is_null());
        assert!(content_only["choices"][0]["finish_reason"].is_null());
    }

    /// 测试 chunk_bytes 函数生成的流式完成块包含对象类型和 DONE 标记
    ///
    /// 验证构造的流式响应字节包含正确的 object 字段值，
    /// 以及在流结束时包含 "data: [DONE]" 标记。
    /// 这是 OpenAI 兼容流式 API 的标准终止信号。
    #[test]
    fn chunk_bytes_done_emits_json_frame_then_done_marker() {
        // 调用 chunk_bytes 构造包含 DONE 标记的流式响应块
        let bytes = chunk_bytes(
            "chatcmpl-test".to_string(),
            1_234_567_890,
            "test-model".to_string(),
            Some("assistant"),
            Some("failed".to_string()),
            Some("stop"),
            true, // is_done 标记指示流式响应结束
        );

        // 将字节转换为字符串以验证内容
        let s = String::from_utf8(bytes.to_vec()).unwrap();

        let frames = s.split("\n\n").filter(|frame| !frame.is_empty()).collect::<Vec<_>>();
        assert_eq!(frames.len(), 2);
        let json = frames[0].strip_prefix("data: ").expect("first frame should contain data");
        let chunk: serde_json::Value = serde_json::from_str(json).unwrap();

        assert_eq!(chunk["object"], "chat.completion.chunk");
        assert_eq!(chunk["choices"][0]["delta"]["role"], "assistant");
        assert_eq!(chunk["choices"][0]["delta"]["content"], "failed");
        assert_eq!(chunk["choices"][0]["finish_reason"], "stop");
        assert_eq!(frames[1], "data: [DONE]");
    }

    #[test]
    fn chunk_bytes_preserves_escaped_content_in_single_sse_frame() {
        let bytes = chunk_bytes(
            "chatcmpl-test".to_string(),
            1_234_567_890,
            "test-model".to_string(),
            None,
            Some("line one\n\"quoted\"".to_string()),
            None,
            false,
        );
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        let json = text
            .strip_prefix("data: ")
            .and_then(|frame| frame.strip_suffix("\n\n"))
            .expect("chunk should be one SSE data frame");
        let chunk: serde_json::Value = serde_json::from_str(json).unwrap();

        assert_eq!(chunk["choices"][0]["delta"]["content"], "line one\n\"quoted\"");
        assert!(chunk["choices"][0]["delta"].get("role").is_none());
        assert!(chunk["choices"][0]["finish_reason"].is_null());
        assert!(!text.contains("data: [DONE]"));
    }

    /// 验证 unix_timestamp 返回调用边界内的当前秒级时间戳。
    #[test]
    fn unix_timestamp_is_within_call_boundaries() {
        let before =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let ts = unix_timestamp();
        let after =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        assert!((before..=after).contains(&ts));
    }
}
