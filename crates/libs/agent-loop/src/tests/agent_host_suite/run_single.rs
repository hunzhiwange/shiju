//! 单次运行测试模块
//!
//! 本模块包含针对 `Agent::run_single` 方法的集成测试用例。
//! 主要验证代理能够通过 `run_single` 接口正确执行单次交互轮次，
//! 并返回预期的响应结果。

use crate::app::agent::agent::dispatcher::NativeToolDispatcher;

use super::helpers::{ScriptedProvider, build_agent_with, text_response};

/// 测试 `run_single` 方法是否正确委托到底层轮次执行逻辑
///
/// # 测试场景
///
/// 1. 创建一个使用脚本化 Provider 的 Agent 实例
/// 2. Provider 预配置返回固定的文本响应
/// 3. 调用 `run_single` 方法执行单次交互
/// 4. 验证返回的响应非空
///
/// # 预期行为
///
/// - `run_single` 应成功完成并返回非空字符串
/// - 方法内部应正确调用底层轮次执行逻辑
#[tokio::test]
async fn run_single_delegates_to_turn() {
    // 创建脚本化 Provider，预配置返回固定文本响应
    let provider = Box::new(ScriptedProvider::new(vec![text_response("via run_single")]));

    // 构建 Agent 实例：注入 Provider，无工具，使用原生工具分发器
    let mut agent = build_agent_with(provider, vec![], Box::new(NativeToolDispatcher));

    // 执行单次交互并获取响应
    let response = agent.run_single("test").await.unwrap();

    // 验证响应非空
    assert!(!response.is_empty(), "Expected non-empty response from run_single");
}
