//! 循环历史记录测试模块
//!
//! 本模块提供针对 Agent 对话历史管理功能的单元测试，
//! 验证历史记录的修剪、增长、清理及在工具调用循环中的完整性。
//!
//! # 测试覆盖范围
//!
//! - 历史记录修剪：当超过配置的最大消息数时自动裁剪
//! - 历史记录完整性：工具调用循环后保持正确的消息序列
//! - 多轮对话增长：验证历史记录随对话轮次正常增长
//! - 历史清理：重置对话并确保系统提示正确重新注入

use crate::app::agent::agent::dispatcher::NativeToolDispatcher;
use crate::app::agent::providers::{ChatMessage, ToolCall};
use vw_config_types::agent::AgentDefinitionConfig;

use super::helpers::{
    EchoTool, ScriptedProvider, build_agent_with, build_agent_with_config, text_response,
    tool_response,
};

/// 测试历史记录在超过最大消息数后是否正确修剪
///
/// 验证当对话历史超过 `max_history_messages` 配置时，
/// Agent 能够自动修剪旧消息，同时保留系统提示。
///
/// # 测试流程
///
/// 1. 设置最大历史消息数为 6
/// 2. 执行 11 轮对话（超过最大值的 5 条）
/// 3. 验证历史长度不超过 `max_history + 1`（系统提示）
/// 4. 确认系统提示始终保留在历史首位
#[tokio::test]
async fn history_trims_after_max_messages() {
    let max_history = 6;
    let mut responses = vec![];
    for _ in 0..max_history + 5 {
        responses.push(text_response("ok"));
    }

    let provider = Box::new(ScriptedProvider::new(responses));
    let config = AgentDefinitionConfig {
        max_history_messages: max_history,
        ..AgentDefinitionConfig::default()
    };

    let mut agent = build_agent_with_config(provider, vec![], config);

    for i in 0..max_history + 5 {
        let _ = agent.turn(&format!("msg {i}")).await.unwrap();
    }

    // 系统提示（1 条）+ 修剪后的消息
    // 总长度不应超过 max_history + 1（系统提示占位）
    assert!(
        agent.history().len() <= max_history + 1,
        "History length {} exceeds max {} + 1 (system)",
        agent.history().len(),
        max_history,
    );

    // 系统提示应始终保留
    let first = &agent.history()[0];
    assert_eq!(first.role, "system");
}

/// 测试工具调用循环后历史记录包含所有预期的消息条目
///
/// 验证当 Agent 执行工具调用循环时，对话历史能够正确记录：
/// - 系统提示
/// - 用户输入
/// - 助手的工具调用请求
/// - 工具执行结果
/// - 助手的最终答案
///
/// # 测试流程
///
/// 1. 配置 Provider 先返回工具调用，再返回文本答案
/// 2. 执行单轮对话触发工具调用
/// 3. 验证历史至少包含 5 条消息（系统、用户、助手、工具、助手）
/// 4. 确认各类角色消息按预期出现
#[tokio::test]
async fn history_contains_all_expected_entries_after_tool_loop() {
    let provider = Box::new(ScriptedProvider::new(vec![
        tool_response(vec![ToolCall {
            id: "tc1".into(),
            name: "echo".into(),
            arguments: r#"{"message": "tool-out"}"#.into(),
        }]),
        text_response("final answer"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(NativeToolDispatcher));

    let _ = agent.turn("test").await.unwrap();

    let history = agent.history();
    assert!(history.len() >= 5, "Expected at least 5 history entries, got {}", history.len());

    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].role, "user");
    assert!(history.iter().any(|m| m.role == "tool"));
    assert!(
        history.iter().any(|m| m.role == "assistant" && m.content == "final answer"),
        "history should contain final assistant answer"
    );
}

/// 测试多轮对话中历史记录持续增长
///
/// 验证在连续多轮对话中，历史记录能够正确累积，
/// 每轮对话后历史长度应增加（用户消息 + 助手回复）。
///
/// # 测试流程
///
/// 1. 配置 Provider 返回 3 个不同的响应
/// 2. 连续执行 3 轮对话
/// 3. 验证每轮响应内容正确
/// 4. 确认历史长度随轮次单调增长
#[tokio::test]
async fn multi_turn_maintains_growing_history() {
    let provider = Box::new(ScriptedProvider::new(vec![
        text_response("response 1"),
        text_response("response 2"),
        text_response("response 3"),
    ]));

    let mut agent = build_agent_with(provider, vec![], Box::new(NativeToolDispatcher));

    let r1 = agent.turn("msg 1").await.unwrap();
    let len_after_1 = agent.history().len();

    let r2 = agent.turn("msg 2").await.unwrap();
    let len_after_2 = agent.history().len();

    let r3 = agent.turn("msg 3").await.unwrap();
    let len_after_3 = agent.history().len();

    assert_eq!(r1, "response 1");
    assert_eq!(r2, "response 2");
    assert_eq!(r3, "response 3");

    // 历史应随每轮对话增长（每轮添加用户 + 助手消息）
    assert!(len_after_2 > len_after_1, "History should grow after turn 2");
    assert!(len_after_3 > len_after_2, "History should grow after turn 3");
}

/// 测试清理历史记录后对话能够正确重置
///
/// 验证 `clear_history()` 方法能够清空对话历史，
/// 并且在下一轮对话时系统提示会被重新注入。
///
/// # 测试流程
///
/// 1. 执行一轮对话产生历史记录
/// 2. 调用 `clear_history()` 清空历史
/// 3. 验证历史已为空
/// 4. 执行新一轮对话
/// 5. 确认系统提示被重新注入到历史首位
#[tokio::test]
async fn clear_history_resets_conversation() {
    let provider =
        Box::new(ScriptedProvider::new(vec![text_response("first"), text_response("second")]));

    let mut agent = build_agent_with(provider, vec![], Box::new(NativeToolDispatcher));

    let _ = agent.turn("hi").await.unwrap();
    assert!(!agent.history().is_empty());

    agent.clear_history();
    assert!(agent.history().is_empty());

    // 下一轮对话应重新注入系统提示
    let _ = agent.turn("hello again").await.unwrap();
    assert!(matches!(
        &agent.history()[0],
        ChatMessage { role, .. } if role == "system"
    ));
}
