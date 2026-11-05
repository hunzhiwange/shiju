//! Dispatcher 模块的单元测试
//!
//! 本模块包含对 `XmlToolDispatcher` 和 `NativeToolDispatcher` 的完整测试套件。
//! 测试覆盖以下核心功能：
//!
//! - 工具调用解析：验证从 AI 响应中正确提取工具调用
//! - 结果格式化：验证工具执行结果的正确格式化输出
//! - 推理内容透传：验证 `reasoning_content` 字段在不同调度器中的处理方式
//! - 消息转换：验证对话历史到 Provider 消息格式的转换
//!
//! ## 测试分类
//!
//! 1. **XML 调度器测试**：测试基于 XML/JSON 混合格式的工具调用解析
//! 2. **原生调度器测试**：测试基于原生 API 工具调用格式的处理
//! 3. **推理内容测试**：测试思维链（reasoning content）的透传与忽略逻辑

use super::*;

/// 测试模块内部容器
///
/// 使用 `#[allow(dead_code)]` 属性以避免编译器对仅用于测试的辅助项发出警告。
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试 XmlToolDispatcher 从响应文本中解析 JSON 格式的工具调用
    ///
    /// ## 测试场景
    /// 响应文本中包含嵌入的 JSON 工具调用：
    /// ` Checking\n{"name":"shell","arguments":{"command":"ls"}}`
    ///
    /// ## 验证点
    /// - 解析出的工具调用数量应为 1
    /// - 工具名称应为 "shell"
    ///
    /// ## 技术细节
    /// 此测试验证 XML 调度器能够从混合文本中提取 JSON 格式的工具调用，
    /// 即使工具调用前有其他文本内容也能正确识别。
    #[test]
    fn xml_dispatcher_parses_tool_calls() {
        // 构造包含 JSON 工具调用的聊天响应
        let response = ChatResponse {
                text: Some(
                    "Checking\n<tool_call>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool_call>"
                        .into(),
                ),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            };

        // 创建 XML 调度器实例
        let dispatcher = XmlToolDispatcher;

        // 解析响应并提取工具调用
        let (_, calls) = dispatcher.parse_response(&response);

        // 断言：应解析出 1 个工具调用
        assert_eq!(calls.len(), 1);

        // 断言：工具名称应为 "shell"
        assert_eq!(calls[0].name, "shell");
    }

    /// 测试 NativeToolDispatcher 的完整往返处理流程
    ///
    /// ## 测试场景
    /// 模拟一个完整的工具调用生命周期：
    /// 1. 从响应中解析原生工具调用
    /// 2. 将工具执行结果格式化为消息
    ///
    /// ## 验证点
    /// - 解析阶段：正确提取工具调用 ID、名称和参数
    /// - 格式化阶段：工具结果消息保留原始工具调用 ID
    ///
    /// ## 技术细节
    /// 原生调度器使用 Provider 返回的结构化 `tool_calls` 字段，
    /// 而非从文本中解析。此测试验证 ID 的完整保留。
    #[test]
    fn native_dispatcher_roundtrip() {
        // 构造包含原生工具调用的响应
        let response = ChatResponse {
            text: Some("ok".into()),
            // 模拟 Provider 返回的工具调用结构
            tool_calls: vec![crate::app::agent::providers::ToolCall {
                kind: Default::default(),
                namespace: None,
                id: "tc1".into(),
                name: "file_read".into(),
                arguments: "{\"path\":\"a.txt\"}".into(),
            }],
            usage: None,
            reasoning_content: None,
        };

        // 创建原生调度器实例
        let dispatcher = NativeToolDispatcher;

        // 步骤 1：解析响应，提取工具调用
        let (_, calls) = dispatcher.parse_response(&response);

        // 断言：应解析出 1 个工具调用
        assert_eq!(calls.len(), 1);

        // 断言：工具调用 ID 应正确保留
        assert_eq!(calls[0].tool_call_id.as_deref(), Some("tc1"));

        // 步骤 2：格式化工具执行结果
        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "file_read".into(),
            output: "hello".into(),
            success: true,
            tool_call_id: Some("tc1".into()),
        }]);

        // 验证格式化结果为 ToolResults 消息类型
        match msg {
            ConversationMessage::ToolResults(results) => {
                // 断言：结果数量应为 1
                assert_eq!(results.len(), 1);
                // 断言：工具调用 ID 应在结果中保留
                assert_eq!(results[0].tool_call_id, "tc1");
            }
            _ => panic!("expected tool results"),
        }
    }

    /// 测试 NativeToolDispatcher 格式化工具结果时保留工具调用 ID
    ///
    /// ## 测试场景
    /// 将带有 `tool_call_id` 的工具执行结果格式化为消息。
    ///
    /// ## 验证点
    /// - 格式化后的消息应为 `ToolResults` 类型
    /// - 工具调用 ID 应在结果中完整保留
    ///
    /// ## 技术细节
    /// 原生调度器使用结构化的 `ToolResults` 消息类型，
    /// 以便 Provider 能够将结果与原始工具调用关联。
    #[test]
    fn native_format_results_keeps_tool_call_id() {
        // 创建原生调度器
        let dispatcher = NativeToolDispatcher;

        // 格式化带有 tool_call_id 的工具结果
        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "shell".into(),
            output: "ok".into(),
            success: true,
            tool_call_id: Some("tc-1".into()),
        }]);

        // 验证消息类型和内容
        match msg {
            ConversationMessage::ToolResults(results) => {
                // 断言：结果数量应为 1
                assert_eq!(results.len(), 1);
                // 断言：工具调用 ID 应保留
                assert_eq!(results[0].tool_call_id, "tc-1");
            }
            _ => panic!("expected ToolResults variant"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 推理内容（reasoning_content）透传测试
    // ═══════════════════════════════════════════════════════════════════════

    /// 测试 NativeToolDispatcher 在消息转换中包含推理内容
    ///
    /// ## 测试场景
    /// 将包含 `reasoning_content` 的 `AssistantToolCalls` 消息转换为 Provider 消息格式。
    ///
    /// ## 验证点
    /// - 转换后的消息数量正确
    /// - 消息角色为 "assistant"
    /// - JSON 载荷中包含 `reasoning_content` 字段
    /// - JSON 载荷中包含 `content` 和 `tool_calls` 字段
    ///
    /// ## 技术细节
    /// 原生调度器将 `AssistantToolCalls` 消息序列化为 JSON 格式，
    /// 其中包含推理内容（思维链），以支持需要显示推理过程的 AI 模型。
    #[test]
    fn native_to_provider_messages_includes_reasoning_content() {
        // 创建原生调度器
        let dispatcher = NativeToolDispatcher;

        // 构造包含推理内容的助手工具调用消息
        let history = vec![ConversationMessage::AssistantToolCalls {
            text: Some("answer".into()),
            tool_calls: vec![crate::app::agent::providers::ToolCall {
                kind: Default::default(),
                namespace: None,
                id: "tc_1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: Some("thinking step".into()),
        }];

        // 执行消息转换
        let messages = dispatcher.to_provider_messages(&history);

        // 断言：应生成 1 条消息
        assert_eq!(messages.len(), 1);

        // 断言：角色应为 assistant
        assert_eq!(messages[0].role, "assistant");

        // 解析 JSON 载荷并验证各字段
        let payload: serde_json::Value = serde_json::from_str(&messages[0].content).unwrap();

        // 断言：推理内容应正确序列化
        assert_eq!(payload["reasoning_content"].as_str(), Some("thinking step"));

        // 断言：主要内容应正确序列化
        assert_eq!(payload["content"].as_str(), Some("answer"));

        // 断言：工具调用应为数组
        assert!(payload["tool_calls"].is_array());
    }

    /// 测试 NativeToolDispatcher 在推理内容为空时省略该字段
    ///
    /// ## 测试场景
    /// 将 `reasoning_content` 为 `None` 的消息转换为 Provider 消息格式。
    ///
    /// ## 验证点
    /// - 消息数量正确
    /// - JSON 载荷中不应包含 `reasoning_content` 字段
    ///
    /// ## 技术细节
    /// 当推理内容不存在时，原生调度器不会在 JSON 载荷中包含该字段，
    /// 以减少消息体积并保持 API 兼容性。
    #[test]
    fn native_to_provider_messages_omits_reasoning_content_when_none() {
        // 创建原生调度器
        let dispatcher = NativeToolDispatcher;

        // 构造不包含推理内容的助手工具调用消息
        let history = vec![ConversationMessage::AssistantToolCalls {
            text: Some("answer".into()),
            tool_calls: vec![crate::app::agent::providers::ToolCall {
                kind: Default::default(),
                namespace: None,
                id: "tc_1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: None,
        }];

        // 执行消息转换
        let messages = dispatcher.to_provider_messages(&history);

        // 断言：应生成 1 条消息
        assert_eq!(messages.len(), 1);

        // 解析 JSON 载荷
        let payload: serde_json::Value = serde_json::from_str(&messages[0].content).unwrap();

        // 断言：不应包含 reasoning_content 字段
        assert!(payload.get("reasoning_content").is_none());
    }

    /// 测试 XmlToolDispatcher 在消息转换中忽略推理内容
    ///
    /// ## 测试场景
    /// 将包含 `reasoning_content` 的消息通过 XML 调度器转换为 Provider 消息格式。
    ///
    /// ## 验证点
    /// - 消息数量正确
    /// - 角色为 "assistant"
    /// - 消息内容仅为原始文本，不包含 JSON 载荷
    /// - 内容中不应包含 "reasoning_content" 字符串
    ///
    /// ## 技术细节
    /// XML 调度器设计为简单的文本透传模式，不处理结构化的推理内容。
    /// 这是因为使用 XML/JSON 混合格式的工具调用场景通常不需要思维链支持。
    #[test]
    fn xml_to_provider_messages_ignores_reasoning_content() {
        // 创建 XML 调度器
        let dispatcher = XmlToolDispatcher;

        // 构造包含推理内容的消息（应被忽略）
        let history = vec![ConversationMessage::AssistantToolCalls {
            text: Some("answer".into()),
            tool_calls: vec![crate::app::agent::providers::ToolCall {
                kind: Default::default(),
                namespace: None,
                id: "tc_1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: Some("should be ignored".into()),
        }];

        // 执行消息转换
        let messages = dispatcher.to_provider_messages(&history);

        // 断言：应生成 1 条消息
        assert_eq!(messages.len(), 1);

        // 断言：角色应为 assistant
        assert_eq!(messages[0].role, "assistant");

        // 断言：内容应为纯文本，而非 JSON 载荷
        assert_eq!(messages[0].content, "answer");

        // 断言：内容中不应包含 reasoning_content 字段
        assert!(!messages[0].content.contains("reasoning_content"));
    }
}
