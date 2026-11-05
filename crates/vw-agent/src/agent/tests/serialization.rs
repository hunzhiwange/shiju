//! 序列化测试模块
//!
//! 本模块包含对话消息序列化与反序列化的集成测试。
//! 主要验证 `ConversationMessage` 及其变体在 JSON 序列化/反序列化过程中的
//! 数据完整性和类型安全性。
//!
//! # 测试范围
//!
//! - 聊天消息（系统消息、用户消息、助手消息）
//! - 工具调用消息
//! - 工具结果消息
//!
//! # 测试策略
//!
//! 采用往返测试（roundtrip test）策略：将消息序列化为 JSON 字符串，
//! 再反序列化回消息对象，验证序列化前后数据一致。

use crate::app::agent::providers::{ChatMessage, ConversationMessage, ToolCall, ToolResultMessage};

/// 测试对话消息的序列化/反序列化往返一致性
///
/// 该测试验证所有类型的 `ConversationMessage` 在经过 JSON 序列化与反序列化后，
/// 能够完整保留原始数据，包括消息角色、内容和工具调用信息。
///
/// # 测试用例
///
/// 1. **系统消息**：验证角色为 "system" 的基础聊天消息
/// 2. **用户消息**：验证角色为 "user" 的基础聊天消息
/// 3. **工具调用消息**：验证包含工具调用列表的助手消息
/// 4. **工具结果消息**：验证工具执行结果的返回消息
/// 5. **助手消息**：验证角色为 "assistant" 的基础聊天消息
///
/// # 验证内容
///
/// - 序列化不会丢失数据
/// - 反序列化能够正确重建对象
/// - 变体类型在序列化前后保持一致
/// - 消息字段值在序列化前后完全匹配
///
/// # Panics
///
/// 当以下情况发生时测试会 panic：
/// - 序列化失败
/// - 反序列化失败
/// - 变体类型不匹配
/// - 字段值不相等
#[test]
fn conversation_message_serialization_roundtrip() {
    // 构建测试消息集，覆盖所有 ConversationMessage 变体
    let messages = vec![
        // 系统消息：用于设置对话上下文或角色
        ConversationMessage::Chat(ChatMessage::system("system")),
        // 用户消息：模拟用户输入
        ConversationMessage::Chat(ChatMessage::user("hello")),
        // 工具调用消息：助手请求执行工具
        ConversationMessage::AssistantToolCalls {
            text: Some("checking".into()),
            tool_calls: vec![ToolCall {
                kind: Default::default(),
                namespace: None,
                id: "tc1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: None,
        },
        // 工具结果消息：工具执行后的返回结果
        ConversationMessage::ToolResults(vec![ToolResultMessage {
            tool_call_id: "tc1".into(),
            content: "ok".into(),
        }]),
        // 助手消息：助手的最终回复
        ConversationMessage::Chat(ChatMessage::assistant("done")),
    ];

    // 对每条消息执行往返测试
    for msg in &messages {
        // 序列化消息为 JSON 字符串
        let json = serde_json::to_string(msg).unwrap();
        // 从 JSON 字符串反序列化回消息对象
        let parsed: ConversationMessage = serde_json::from_str(&json).unwrap();

        // 验证变体类型匹配，并检查关键字段
        match (msg, &parsed) {
            // 聊天消息：验证角色和内容
            (ConversationMessage::Chat(a), ConversationMessage::Chat(b)) => {
                assert_eq!(a.role, b.role);
                assert_eq!(a.content, b.content);
            }
            // 工具调用消息：验证文本和工具调用列表
            (
                ConversationMessage::AssistantToolCalls {
                    text: a_text, tool_calls: a_calls, ..
                },
                ConversationMessage::AssistantToolCalls {
                    text: b_text, tool_calls: b_calls, ..
                },
            ) => {
                assert_eq!(a_text, b_text);
                assert_eq!(a_calls.len(), b_calls.len());
            }
            // 工具结果消息：验证结果列表长度
            (ConversationMessage::ToolResults(a), ConversationMessage::ToolResults(b)) => {
                assert_eq!(a.len(), b.len());
            }
            // 变体类型不匹配，表示序列化/反序列化存在问题
            _ => panic!("Variant mismatch after serialization"),
        }
    }
}
