//! # Agent 工具循环执行测试模块
//!
//! 本模块包含针对 Agent 工具执行循环的集成测试用例，主要验证以下核心功能：
//!
//! - **单工具执行**：验证 Agent 能够正确执行单个工具调用并返回结果
//! - **多步骤工具链**：验证 Agent 能够连续执行多个工具调用形成工具链
//! - **迭代上限保护**：验证 Agent 在达到最大工具迭代次数时能够正确终止并报错
//! - **未知工具处理**：验证 Agent 在遇到未注册工具时能够优雅恢复
//! - **工具失败恢复**：验证 Agent 在工具执行失败或抛出错误时能够继续对话
//! - **Provider 错误传播**：验证底层 Provider 错误能够正确传播到调用方
//! - **混合响应处理**：验证 Agent 能够同时处理文本内容和工具调用
//! - **批处理工具调用**：验证 Agent 能够在一次响应中执行多个工具
//! - **系统提示注入**：验证系统提示在首次对话时正确注入且不重复
//!
//! ## 测试依赖
//!
//! - `ScriptedProvider`：预设响应序列的模拟 Provider，用于控制测试场景
//! - `CountingTool`：计数工具，用于验证工具调用次数
//! - `EchoTool`：回显工具，返回输入的消息内容
//! - `FailingTool`：失败工具，模拟工具执行失败场景
//! - `PanickingTool`：恐慌工具，模拟工具抛出异常场景
//! - `FailingProvider`：失败 Provider，模拟底层服务错误
//! - `NativeToolDispatcher`：原生工具分发器，负责工具的查找和执行
//!
//! ## 测试策略
//!
//! 采用黑盒测试策略，通过模拟 Provider 的响应序列来控制 Agent 的执行路径，
//! 验证 Agent 在各种边界条件下的行为符合预期。

use crate::app::agent::agent::dispatcher::NativeToolDispatcher;
use crate::app::agent::providers::{ChatResponse, ToolCall};
use vw_config_types::agent::AgentDefinitionConfig;

use super::helpers::{
    CountingTool, EchoTool, FailingProvider, FailingTool, PanickingTool, ScriptedProvider,
    build_agent_with, build_agent_with_config, text_response, tool_response,
};

/// 测试 Agent 执行单个工具后正常返回
///
/// 验证 Agent 在接收到包含单个工具调用的响应时，能够：
/// 1. 正确识别工具调用请求
/// 2. 调用对应的工具执行器
/// 3. 将工具执行结果反馈给 Provider
/// 4. 返回最终的文本响应
///
/// # 测试场景
///
/// - Provider 首先返回一个 echo 工具调用
/// - Agent 执行 echo 工具，参数为 "hello from tool"
/// - Provider 接收工具结果后返回最终文本 "I ran the tool"
///
/// # 断言
///
/// - 最终响应非空，表明工具执行流程正常完成
#[tokio::test]
async fn turn_executes_single_tool_then_returns() {
    let provider = Box::new(ScriptedProvider::new(vec![
        tool_response(vec![ToolCall {
            id: "tc1".into(),
            name: "echo".into(),
            arguments: r#"{"message": "hello from tool"}"#.into(),
        }]),
        text_response("I ran the tool"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(NativeToolDispatcher));

    let response = agent.turn("run echo").await.unwrap();
    assert!(!response.is_empty(), "Expected non-empty response after tool execution");
}

/// 测试 Agent 处理多步骤工具链
///
/// 验证 Agent 能够连续执行多次工具调用，形成完整的工具链执行流程。
/// 这模拟了真实场景中 Agent 需要多次调用工具才能完成任务的场景。
///
/// # 测试场景
///
/// - Provider 返回 3 个连续的 counter 工具调用
/// - 每次工具调用后，Agent 继续请求下一个响应
/// - 最终 Provider 返回文本响应 "Done after 3 calls"
///
/// # 断言
///
/// - 最终响应非空，表明多步骤流程正常完成
/// - counter 工具被恰好调用了 3 次
#[tokio::test]
async fn turn_handles_multi_step_tool_chain() {
    let (counting_tool, count) = CountingTool::new();

    let provider = Box::new(ScriptedProvider::new(vec![
        tool_response(vec![ToolCall {
            id: "tc1".into(),
            name: "counter".into(),
            arguments: "{}".into(),
        }]),
        tool_response(vec![ToolCall {
            id: "tc2".into(),
            name: "counter".into(),
            arguments: "{}".into(),
        }]),
        tool_response(vec![ToolCall {
            id: "tc3".into(),
            name: "counter".into(),
            arguments: "{}".into(),
        }]),
        text_response("Done after 3 calls"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(counting_tool)], Box::new(NativeToolDispatcher));

    let response = agent.turn("count 3 times").await.unwrap();
    assert!(!response.is_empty(), "Expected non-empty response after multi-step chain");
    assert_eq!(*count.lock().unwrap(), 3);
}

/// 测试 Agent 在达到最大迭代次数时终止执行
///
/// 验证 Agent 的安全机制：当工具调用陷入无限循环时，
/// Agent 能够在达到配置的最大迭代次数后强制终止并返回错误。
///
/// # 测试场景
///
/// - 配置 max_iterations 为 3
/// - Provider 返回 8 个（超过限制）连续的工具调用响应
/// - Agent 应在执行 3 次工具调用后停止并报错
///
/// # 断言
///
/// - turn() 返回错误而非无限循环
/// - 错误消息包含 "maximum tool iterations" 关键词
#[tokio::test]
async fn turn_bails_out_at_max_iterations() {
    // 创建超过 max_iterations 限制的工具调用响应序列
    let max_iters = 3;
    let mut responses = Vec::new();
    for i in 0..max_iters + 5 {
        responses.push(tool_response(vec![ToolCall {
            id: format!("tc{i}"),
            name: "echo".into(),
            arguments: r#"{"message": "loop"}"#.into(),
        }]));
    }

    let provider = Box::new(ScriptedProvider::new(responses));

    let config =
        AgentDefinitionConfig { max_iterations: max_iters, ..AgentDefinitionConfig::default() };

    let mut agent = build_agent_with_config(provider, vec![Box::new(EchoTool)], config);

    let result = agent.turn("infinite loop").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("maximum tool iterations"), "Expected max iterations error, got: {err}");
}

/// 测试 Agent 优雅处理未知工具调用
///
/// 验证 Agent 在遇到未注册的工具调用时，能够：
/// 1. 将错误信息作为工具结果返回给 Provider
/// 2. Provider 能够根据错误信息调整后续响应
/// 3. 对话流程能够正常继续
///
/// # 测试场景
///
/// - Provider 请求调用 "nonexistent_tool"
/// - Agent 找不到该工具，返回包含 "Unknown tool" 的错误结果
/// - Provider 接收错误后返回恢复性文本响应
///
/// # 断言
///
/// - 最终响应非空，表明对话流程正常恢复
/// - 历史记录中存在包含 "Unknown tool" 的工具结果消息
#[tokio::test]
async fn turn_handles_unknown_tool_gracefully() {
    let provider = Box::new(ScriptedProvider::new(vec![
        tool_response(vec![ToolCall {
            id: "tc1".into(),
            name: "nonexistent_tool".into(),
            arguments: "{}".into(),
        }]),
        text_response("I couldn't find that tool"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(NativeToolDispatcher));

    let response = agent.turn("use nonexistent").await.unwrap();
    assert!(!response.is_empty(), "Expected non-empty response after unknown tool recovery");

    // 验证历史记录中包含 "Unknown tool" 的工具结果消息
    let has_tool_result = agent
        .history()
        .iter()
        .any(|msg| msg.role == "tool" && msg.content.contains("Unknown tool"));
    assert!(has_tool_result, "Expected tool result with 'Unknown tool' message");
}

/// 测试 Agent 从工具执行失败中恢复
///
/// 验证 Agent 在工具返回失败结果（非 panic）时，能够继续对话流程。
/// 这模拟了工具执行业务逻辑失败但不影响 Agent 整体运行的情况。
///
/// # 测试场景
///
/// - Provider 请求调用 "fail" 工具
/// - FailingTool 返回失败结果
/// - Provider 接收失败结果后返回恢复性文本响应
///
/// # 断言
///
/// - 最终响应非空，表明 Agent 能够从工具失败中恢复
#[tokio::test]
async fn turn_recovers_from_tool_failure() {
    let provider = Box::new(ScriptedProvider::new(vec![
        tool_response(vec![ToolCall {
            id: "tc1".into(),
            name: "fail".into(),
            arguments: "{}".into(),
        }]),
        text_response("Tool failed but I recovered"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(FailingTool)], Box::new(NativeToolDispatcher));

    let response = agent.turn("try failing tool").await.unwrap();
    assert!(!response.is_empty(), "Expected non-empty response after tool failure recovery");
}

/// 测试 Agent 从工具错误中恢复
///
/// 验证 Agent 在工具执行过程中抛出异常（panic）时，能够捕获错误并继续对话。
/// 这测试了 Agent 对工具异常的容错能力。
///
/// # 测试场景
///
/// - Provider 请求调用 "panicker" 工具
/// - PanickingTool 在执行时抛出 panic
/// - Agent 捕获 panic 并将其转换为错误结果返回给 Provider
/// - Provider 接收错误后返回恢复性文本响应
///
/// # 断言
///
/// - 最终响应非空，表明 Agent 能够从工具异常中恢复
#[tokio::test]
async fn turn_recovers_from_tool_error() {
    let provider = Box::new(ScriptedProvider::new(vec![
        tool_response(vec![ToolCall {
            id: "tc1".into(),
            name: "panicker".into(),
            arguments: "{}".into(),
        }]),
        text_response("I recovered from the error"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(PanickingTool)], Box::new(NativeToolDispatcher));

    let response = agent.turn("try panicking").await.unwrap();
    assert!(!response.is_empty(), "Expected non-empty response after tool error recovery");
}

/// 测试 Provider 错误能够正确传播
///
/// 验证底层 Provider 在发生错误时，错误能够正确传播到 Agent 调用方，
/// 而不是被 Agent 内部捕获或忽略。
///
/// # 测试场景
///
/// - 使用 FailingProvider 作为底层 Provider
/// - FailingProvider 在任何请求下都返回错误
/// - Agent 的 turn() 方法应该返回该错误
///
/// # 断言
///
/// - turn() 返回错误，而非成功
#[tokio::test]
async fn turn_propagates_provider_error() {
    let mut agent =
        build_agent_with(Box::new(FailingProvider), vec![], Box::new(NativeToolDispatcher));

    let result = agent.turn("hello").await;
    assert!(result.is_err(), "Expected provider error to propagate");
}

/// 测试 Agent 在工具调用同时保留文本内容
///
/// 验证 Agent 在 Provider 返回混合响应（同时包含文本内容和工具调用）时，
/// 能够正确处理两者：执行工具调用的同时保留文本内容到对话历史。
///
/// # 测试场景
///
/// - Provider 返回一个同时包含文本 "Let me check..." 和工具调用的响应
/// - Agent 应该同时记录文本内容并执行工具
/// - Provider 接收工具结果后返回最终文本响应
///
/// # 断言
///
/// - 最终响应非空
/// - 对话历史中包含中间文本 "Let me check..."
#[tokio::test]
async fn turn_preserves_text_alongside_tool_calls() {
    let provider = Box::new(ScriptedProvider::new(vec![
        ChatResponse {
            text: Some("Let me check...".into()),
            tool_calls: vec![ToolCall {
                id: "tc1".into(),
                name: "echo".into(),
                arguments: r#"{"message": "hi"}"#.into(),
            }],
            usage: None,
            reasoning_content: None,
        },
        text_response("Here are the results"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(NativeToolDispatcher));

    let response = agent.turn("check something").await.unwrap();
    assert!(!response.is_empty(), "Expected non-empty final response after mixed text+tool");

    // 中间文本应该被保留在对话历史中
    let has_intermediate = agent
        .history()
        .iter()
        .any(|msg| msg.role == "assistant" && msg.content.contains("Let me check"));
    assert!(has_intermediate, "Intermediate text should be in history");
}

/// 测试 Agent 处理单次响应中的多个工具调用
///
/// 验证 Agent 能够在单次 Provider 响应中执行多个工具调用，
/// 并将所有工具结果一次性返回给 Provider。
///
/// # 测试场景
///
/// - Provider 返回包含 3 个 counter 工具调用的单个响应
/// - Agent 应该依次执行这 3 个工具调用
/// - 所有工具执行完成后，将结果发送给 Provider
///
/// # 断言
///
/// - 最终响应非空
/// - counter 工具被恰好调用了 3 次
#[tokio::test]
async fn turn_handles_multiple_tools_in_one_response() {
    let (counting_tool, count) = CountingTool::new();

    let provider = Box::new(ScriptedProvider::new(vec![
        tool_response(vec![
            ToolCall { id: "tc1".into(), name: "counter".into(), arguments: "{}".into() },
            ToolCall { id: "tc2".into(), name: "counter".into(), arguments: "{}".into() },
            ToolCall { id: "tc3".into(), name: "counter".into(), arguments: "{}".into() },
        ]),
        text_response("All 3 done"),
    ]));

    let mut agent =
        build_agent_with(provider, vec![Box::new(counting_tool)], Box::new(NativeToolDispatcher));

    let response = agent.turn("batch").await.unwrap();
    assert!(!response.is_empty(), "Expected non-empty response after multi-tool batch");
    assert_eq!(*count.lock().unwrap(), 3, "All 3 tools should have been called");
}

/// 测试系统提示在首次对话时注入
///
/// 验证 Agent 在首次调用 turn() 时，会自动在对话历史开头注入系统提示消息。
/// 系统提示用于定义 Agent 的行为规范和能力边界。
///
/// # 测试场景
///
/// - 创建新的 Agent 实例，历史记录初始为空
/// - 调用 turn() 进行首次对话
/// - 检查对话历史的第一条消息是否为系统提示
///
/// # 断言
///
/// - 首次调用 turn() 前历史记录为空
/// - 首次调用后历史记录的第一条消息角色为 "system"
#[tokio::test]
async fn system_prompt_injected_on_first_turn() {
    let provider = Box::new(ScriptedProvider::new(vec![text_response("ok")]));
    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(NativeToolDispatcher));

    assert!(agent.history().is_empty(), "History should start empty");

    let _ = agent.turn("hi").await.unwrap();

    // 历史记录的第一条消息应该是系统提示
    let first = &agent.history()[0];
    assert_eq!(first.role, "system", "First history entry should be system prompt");
}

/// 测试系统提示在多次对话中不重复注入
///
/// 验证 Agent 在后续对话中不会重复注入系统提示消息。
/// 系统提示应该只出现在对话历史的开头，且仅出现一次。
///
/// # 测试场景
///
/// - 创建新的 Agent 实例
/// - 连续调用两次 turn() 进行对话
/// - 检查对话历史中系统提示的出现次数
///
/// # 断言
///
/// - 对话历史中恰好只有一条系统提示消息
#[tokio::test]
async fn system_prompt_not_duplicated_on_second_turn() {
    let provider =
        Box::new(ScriptedProvider::new(vec![text_response("first"), text_response("second")]));
    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(NativeToolDispatcher));

    let _ = agent.turn("hi").await.unwrap();
    let _ = agent.turn("hello again").await.unwrap();

    let system_count = agent.history().iter().filter(|msg| msg.role == "system").count();
    assert_eq!(system_count, 1, "System prompt should appear exactly once");
}
