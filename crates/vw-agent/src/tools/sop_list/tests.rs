//! SOP 列表工具测试模块
//!
//! 本模块包含针对 `SopListTool` 的单元测试，验证 SOP（标准操作流程）列表工具的各项功能，
//! 包括完整列表展示、空列表处理、按名称/优先级过滤等场景。
//!
//! # 测试范围
//!
//! - 列出所有已加载的 SOP
//! - 空列表情况的处理
//! - 按名称关键字过滤 SOP
//! - 按优先级过滤 SOP
//! - 过滤无匹配结果的场景
//! - 工具名称和参数 schema 的正确性

use super::super::*;
use crate::app::agent::config::SopConfig;
use crate::app::agent::sop::engine::SopEngine;
use crate::app::agent::sop::types::*;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// 创建测试用的 SOP 对象
///
/// 生成一个简化的 SOP 实例，用于单元测试场景。
/// 该 SOP 包含最基本的配置，仅包含一个手动触发步骤。
///
/// # 参数
///
/// - `name`: SOP 的名称标识符
/// - `priority`: SOP 的执行优先级（如 Critical、Normal 等）
///
/// # 返回值
///
/// 返回配置好的 `Sop` 实例，包含以下默认值：
/// - 版本号：1.0.0
/// - 执行模式：Auto
/// - 触发器：仅 Manual（手动触发）
/// - 步骤：单个测试步骤
/// - 冷却时间：0 秒
/// - 最大并发数：1
fn test_sop(name: &str, priority: SopPriority) -> Sop {
    Sop {
        name: name.into(),
        description: format!("Test SOP: {name}"),
        version: "1.0.0".into(),
        priority,
        execution_mode: SopExecutionMode::Auto,
        triggers: vec![SopTrigger::Manual],
        steps: vec![SopStep {
            number: 1,
            title: "Step one".into(),
            body: "Do it".into(),
            suggested_tools: vec![],
            requires_confirmation: false,
        }],
        cooldown_secs: 0,
        max_concurrent: 1,
        location: None,
    }
}

/// 创建带有指定 SOP 列表的引擎实例
///
/// 初始化一个 `SopEngine` 并预加载指定的 SOP 列表，用于测试场景。
/// 返回的引擎被包装在 `Arc<Mutex<>>` 中，以支持跨线程共享和线程安全访问。
///
/// # 参数
///
/// - `sops`: 要加载到引擎中的 SOP 列表
///
/// # 返回值
///
/// 返回 `Arc<Mutex<SopEngine>>`，可用于创建 `SopListTool` 实例
fn engine_with_sops(sops: Vec<Sop>) -> Arc<Mutex<SopEngine>> {
    let mut engine = SopEngine::new(SopConfig::default());
    engine.set_sops_for_test(sops);
    Arc::new(Mutex::new(engine))
}

/// 测试列出所有 SOP 的功能
///
/// 验证当引擎中存在多个 SOP 时，`SopListTool` 能正确返回所有 SOP 的信息，
/// 输出中应包含所有 SOP 的名称以及总数统计。
#[tokio::test]
async fn list_all_sops() {
    let engine = engine_with_sops(vec![
        test_sop("pump-shutdown", SopPriority::Critical),
        test_sop("daily-check", SopPriority::Normal),
    ]);
    let tool = SopListTool::new(engine);
    let result = tool.execute(json!({})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("pump-shutdown"));
    assert!(result.output.contains("daily-check"));
    assert!(result.output.contains("2 total"));
}

/// 测试空列表场景
///
/// 验证当引擎中没有任何 SOP 时，`SopListTool` 返回友好的提示信息，
/// 而不是错误或空输出。
#[tokio::test]
async fn list_empty() {
    let engine = engine_with_sops(vec![]);
    let tool = SopListTool::new(engine);
    let result = tool.execute(json!({})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("No SOPs loaded"));
}

/// 测试按名称过滤 SOP
///
/// 验证当提供 `filter` 参数时，`SopListTool` 能根据名称关键字
/// 筛选出匹配的 SOP，并排除不匹配的项。
#[tokio::test]
async fn filter_by_name() {
    let engine = engine_with_sops(vec![
        test_sop("pump-shutdown", SopPriority::Critical),
        test_sop("daily-check", SopPriority::Normal),
    ]);
    let tool = SopListTool::new(engine);
    let result = tool.execute(json!({"filter": "pump"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("pump-shutdown"));
    assert!(!result.output.contains("daily-check"));
}

/// 测试按优先级过滤 SOP
///
/// 验证 `filter` 参数支持按优先级关键字（如 "critical"）进行过滤，
/// 返回符合指定优先级的 SOP，排除其他优先级的项。
#[tokio::test]
async fn filter_by_priority() {
    let engine = engine_with_sops(vec![
        test_sop("pump-shutdown", SopPriority::Critical),
        test_sop("daily-check", SopPriority::Normal),
    ]);
    let tool = SopListTool::new(engine);
    let result = tool.execute(json!({"filter": "critical"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("pump-shutdown"));
    assert!(!result.output.contains("daily-check"));
}

/// 测试过滤无匹配结果的场景
///
/// 验证当 `filter` 参数匹配不到任何 SOP 时，`SopListTool` 返回成功状态，
/// 并在输出中给出"No SOPs match"的友好提示。
#[tokio::test]
async fn filter_no_match() {
    let engine = engine_with_sops(vec![test_sop("pump-shutdown", SopPriority::Critical)]);
    let tool = SopListTool::new(engine);
    let result = tool.execute(json!({"filter": "nonexistent"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("No SOPs match"));
}

/// 测试工具名称和参数 schema
///
/// 验证 `SopListTool` 的基础元数据：
/// - 工具名称应为 "sop_list"
/// - 参数 schema 中应包含 "filter" 属性定义
#[test]
fn name_and_schema() {
    let engine = engine_with_sops(vec![]);
    let tool = SopListTool::new(engine);
    assert_eq!(tool.name(), "sop_list");
    assert!(tool.parameters_schema()["properties"]["filter"].is_object());
}

#[tokio::test]
async fn list_rejects_poisoned_engine_lock() {
    let engine = engine_with_sops(vec![test_sop("pump-shutdown", SopPriority::Critical)]);
    let engine_to_poison = engine.clone();
    let panic = std::thread::spawn(move || {
        let _guard = engine_to_poison.lock().unwrap();
        panic!("poison test engine lock");
    })
    .join();
    assert!(panic.is_err());

    let error = SopListTool::new(engine)
        .execute(json!({}))
        .await
        .expect_err("a poisoned engine lock must be rejected");

    assert!(error.to_string().contains("Engine lock poisoned"));
}
