//! 代理循环与文本响应测试模块
//!
//! 本模块提供代理（Agent）核心循环逻辑的集成测试，聚焦于文本响应处理
//! 和工具调用的循环执行机制。主要验证以下能力：
//!
//! - 代理在无工具调用时正确返回文本响应
//! - XML 格式工具调用解析与循环执行
//! - 空文本与 None 文本响应的边界处理
//!
//! # 测试策略
//!
//! 使用 `ScriptedProvider` 模拟模型响应序列，配合 `EchoTool` 验证工具调用路径，
//! 分别测试 `NativeToolDispatcher` 和 `XmlToolDispatcher` 两种分发器实现。

use crate::app::agent::agent::dispatcher::{NativeToolDispatcher, XmlToolDispatcher};
use model_types::ChatResponse;

use super::helpers::{
    EchoTool, ScriptedProvider, build_agent_with, text_response, xml_tool_response,
};

/// 测试：无工具调用时代理返回纯文本响应
///
/// 验证当 Provider 仅返回文本内容而不包含工具调用时，
/// 代理能够正确处理并返回该文本。
///
/// # 测试场景
///
/// 1. 创建仅返回 "Hello world" 文本的模拟 Provider
/// 2. 注册 EchoTool 工具，但 Provider 不调用它
/// 3. 使用 NativeToolDispatcher 分发器
/// 4. 调用代理 turn 方法并验证返回非空文本
///
/// # 预期结果
///
/// - 代理应返回 "Hello world" 文本
/// - 不应触发任何工具调用
#[tokio::test]
async fn turn_returns_text_when_no_tools_called() {
    // 创建预编程的 Provider，仅返回纯文本响应
    let provider = Box::new(ScriptedProvider::new(vec![text_response("Hello world")]));

    // 构建代理实例：注入 Provider、工具列表和原生工具分发器
    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(NativeToolDispatcher));

    // 执行代理 turn 循环，传入用户消息
    let response = agent.turn("hi").await.unwrap();

    // 验证：应返回非空文本（Provider 的原始响应）
    assert!(!response.is_empty(), "Expected non-empty text response from provider");
}

/// 测试：XML 分发器正确解析工具调用并执行循环
///
/// 验证 XmlToolDispatcher 能够从响应中解析 XML 格式的工具调用，
/// 执行工具后将结果反馈给 Provider，并继续循环直到返回最终文本。
///
/// # 测试场景
///
/// 1. 创建返回两次响应的模拟 Provider：
///    - 第一次：包含 XML 格式的 echo 工具调用
///    - 第二次：纯文本 "XML tool completed"
/// 2. 使用 XmlToolDispatcher 分发器
/// 3. 调用代理 turn 方法
///
/// # 循环流程
///
/// 1. Provider 返回工具调用请求 → 分发器解析并执行 EchoTool
/// 2. 工具执行结果作为新消息反馈 → Provider 生成最终响应
/// 3. 返回最终文本给用户
///
/// # 预期结果
///
/// - 代理应成功完成工具调用循环
/// - 返回非空的最终响应文本
#[tokio::test]
async fn xml_dispatcher_parses_and_loops() {
    // 创建预编程的 Provider，先返回工具调用，再返回文本
    let provider = Box::new(ScriptedProvider::new(vec![
        // 第一次响应：触发 echo 工具调用
        xml_tool_response("echo", r#"{"message": "xml-test"}"#),
        // 第二次响应：工具执行完成后的文本
        text_response("XML tool completed"),
    ]));

    // 构建代理实例：使用 XML 工具分发器
    let mut agent =
        build_agent_with(provider, vec![Box::new(EchoTool)], Box::new(XmlToolDispatcher));

    // 执行代理 turn 循环
    let response = agent.turn("test xml").await.unwrap();

    // 验证：应返回非空的最终响应
    assert!(!response.is_empty(), "Expected non-empty response from XML dispatcher");
}

/// 测试：代理正确处理空字符串文本响应
///
/// 验证当 Provider 返回空字符串（Some("")）作为文本时，
/// 代理不会崩溃或异常，而是正常返回空字符串。
///
/// # 边界条件
///
/// - `text: Some(String::new())` 表示显式返回空文本
/// - 无工具调用
/// - 应优雅处理而非 panic
///
/// # 预期结果
///
/// - 代理应返回空字符串
/// - 不应抛出错误或异常
#[tokio::test]
async fn turn_handles_empty_text_response() {
    // 创建返回空字符串的 Provider
    let provider = Box::new(ScriptedProvider::new(vec![ChatResponse {
        text: Some(String::new()), // 显式返回空字符串
        tool_calls: vec![],
        usage: None,
        reasoning_content: None,
    }]));

    // 构建无工具的代理实例
    let mut agent = build_agent_with(provider, vec![], Box::new(NativeToolDispatcher));

    // 执行代理 turn 循环
    let response = agent.turn("hi").await.unwrap();

    // 验证：应返回空字符串
    assert!(response.is_empty());
}

/// 测试：代理正确处理 None 文本响应
///
/// 验证当 Provider 返回 `text: None`（无文本字段）时，
/// 代理能够安全地回退到空字符串，而不会 panic。
///
/// # 边界条件
///
/// - `text: None` 表示响应中无文本内容
/// - 某些模型可能在不稳定情况下返回此类响应
/// - 代理应具有防御性处理逻辑
///
/// # 防御性编程
///
/// 代理内部应将 None 回退为空字符串，确保：
/// - 不会因 unwrap None 而崩溃
/// - 调用方得到一致的 String 类型返回值
///
/// # 预期结果
///
/// - 代理不应 panic
/// - 应返回空字符串作为回退值
#[tokio::test]
async fn turn_handles_none_text_response() {
    // 创建返回 None 文本的 Provider
    let provider = Box::new(ScriptedProvider::new(vec![ChatResponse {
        text: None, // 无文本内容
        tool_calls: vec![],
        usage: None,
        reasoning_content: None,
    }]));

    // 构建无工具的代理实例
    let mut agent = build_agent_with(provider, vec![], Box::new(NativeToolDispatcher));

    // 代理不应崩溃 —— 应回退为空字符串
    let response = agent.turn("hi").await.unwrap();

    // 验证：应返回空字符串（回退值）
    assert!(response.is_empty());
}
