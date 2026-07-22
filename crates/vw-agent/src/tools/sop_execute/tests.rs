//! SOP 执行工具测试模块
//!
//! 本模块包含对 `SopExecuteTool` 的单元测试，用于验证 SOP（标准操作程序）执行工具的各项功能。
//!
//! # 测试覆盖范围
//!
//! - 自动执行模式测试
//! - 监督执行模式测试
//! - 未知 SOP 处理测试
//! - 参数验证测试（缺少必需参数）
//! - 带载荷执行测试
//! - 工具元数据测试（名称和参数 schema）
//!
//! # 依赖
//!
//! - `SopConfig`: SOP 配置结构
//! - `SopEngine`: SOP 执行引擎
//! - `SopExecuteTool`: SOP 执行工具实现

use super::super::*;
use crate::app::agent::config::SopConfig;
use crate::app::agent::sop::engine::SopEngine;
use crate::app::agent::sop::types::*;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// 创建测试用的 SOP 对象
///
/// 生成一个标准化的测试 SOP，包含两个步骤，用于测试各种执行场景。
///
/// # 参数
///
/// - `name`: SOP 的名称标识符
/// - `mode`: SOP 的执行模式（自动或监督）
///
/// # 返回
///
/// 返回一个配置好的 `Sop` 对象，具有以下特征：
/// - 版本号为 "1.0.0"
/// - 优先级为普通（Normal）
/// - 手动触发方式
/// - 包含两个测试步骤
/// - 无冷却时间，最大并发数为 1
///
/// # 示例
///
/// ```ignore
/// let sop = test_sop("my-test", SopExecutionMode::Auto);
/// assert_eq!(sop.name, "my-test");
/// ```
fn test_sop(name: &str, mode: SopExecutionMode) -> Sop {
    Sop {
        name: name.into(),
        description: format!("Test SOP: {name}"),
        version: "1.0.0".into(),
        priority: SopPriority::Normal,
        execution_mode: mode,
        triggers: vec![SopTrigger::Manual],
        // 定义两个测试步骤，模拟真实的 SOP 流程
        steps: vec![
            SopStep {
                number: 1,
                title: "Step one".into(),
                body: "Do step one".into(),
                suggested_tools: vec!["shell".into()],
                requires_confirmation: false,
            },
            SopStep {
                number: 2,
                title: "Step two".into(),
                body: "Do step two".into(),
                suggested_tools: vec![],
                requires_confirmation: false,
            },
        ],
        cooldown_secs: 0,
        max_concurrent: 1,
        location: None,
    }
}

/// 创建带有指定 SOP 列表的测试引擎
///
/// 构建一个 `SopEngine` 实例，并使用提供的 SOP 列表进行初始化，
/// 返回一个线程安全的共享引用，用于并发测试场景。
///
/// # 参数
///
/// - `sops`: 要加载到引擎中的 SOP 列表
///
/// # 返回
///
/// 返回一个 `Arc<Mutex<SopEngine>>`，允许在多线程环境中安全访问引擎。
///
/// # 示例
///
/// ```ignore
/// let engine = engine_with_sops(vec![test_sop("test", SopExecutionMode::Auto)]);
/// // 可以在多个测试中安全共享此引擎
/// ```
fn engine_with_sops(sops: Vec<Sop>) -> Arc<Mutex<SopEngine>> {
    let mut engine = SopEngine::new(SopConfig::default());
    // 使用测试专用方法设置 SOP 列表，避免依赖外部配置
    engine.set_sops_for_test(sops);
    Arc::new(Mutex::new(engine))
}

/// 测试自动执行模式的 SOP
///
/// 验证当 SOP 配置为自动执行模式时，工具能够成功执行 SOP 的所有步骤，
/// 而无需人工干预。
///
/// # 测试步骤
///
/// 1. 创建一个自动执行模式的 SOP
/// 2. 使用该 SOP 初始化引擎和工具
/// 3. 执行工具，传入 SOP 名称
/// 4. 验证执行成功且输出包含预期的运行 ID 和步骤信息
///
/// # 断言
///
/// - 执行结果为成功
/// - 输出包含运行 ID（"run-" 前缀）
/// - 输出包含第一个步骤的标题
#[tokio::test]
async fn execute_auto_sop() {
    // 准备：创建自动执行模式的测试 SOP
    let engine = engine_with_sops(vec![test_sop("test-sop", SopExecutionMode::Auto)]);
    let tool = SopExecuteTool::new(engine);

    // 执行：调用工具执行 SOP
    let result = tool.execute(json!({"name": "test-sop"})).await.unwrap();

    // 验证：检查执行成功并包含预期内容
    assert!(result.success);
    assert!(result.output.contains("run-"));
    assert!(result.output.contains("Step one"));
}

/// 测试监督执行模式的 SOP
///
/// 验证当 SOP 配置为监督执行模式时，工具会等待人工审批后才能继续执行，
/// 确保关键操作有适当的控制点。
///
/// # 测试步骤
///
/// 1. 创建一个监督执行模式的 SOP
/// 2. 使用该 SOP 初始化引擎和工具
/// 3. 执行工具，传入 SOP 名称
/// 4. 验证执行成功但处于等待审批状态
///
/// # 断言
///
/// - 执行结果为成功（已启动）
/// - 输出包含"等待审批"的提示信息
#[tokio::test]
async fn execute_supervised_sop() {
    // 准备：创建监督执行模式的测试 SOP
    let engine = engine_with_sops(vec![test_sop("test-sop", SopExecutionMode::Supervised)]);
    let tool = SopExecuteTool::new(engine);

    // 执行：调用工具执行 SOP
    let result = tool.execute(json!({"name": "test-sop"})).await.unwrap();

    // 验证：检查执行成功且处于等待审批状态
    assert!(result.success);
    assert!(result.output.contains("waiting for approval"));
}

/// 测试执行不存在的 SOP
///
/// 验证当尝试执行一个不存在的 SOP 时，工具会正确处理错误，
/// 返回失败状态和明确的错误信息。
///
/// # 测试步骤
///
/// 1. 创建一个空的引擎（不包含任何 SOP）
/// 2. 尝试执行一个不存在的 SOP
/// 3. 验证执行失败并返回适当的错误信息
///
/// # 断言
///
/// - 执行结果为失败
/// - 错误信息包含"Failed to start SOP"提示
#[tokio::test]
async fn execute_unknown_sop() {
    // 准备：创建空引擎，不包含任何 SOP
    let engine = engine_with_sops(vec![]);
    let tool = SopExecuteTool::new(engine);

    // 执行：尝试执行不存在的 SOP
    let result = tool.execute(json!({"name": "nonexistent"})).await.unwrap();

    // 验证：检查执行失败并包含错误信息
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Failed to start SOP"));
}

/// 测试缺少必需参数的情况
///
/// 验证当执行请求缺少必需的 `name` 参数时，工具会返回错误，
/// 确保参数验证逻辑正确工作。
///
/// # 测试步骤
///
/// 1. 创建引擎和工具实例
/// 2. 尝试执行一个空的 JSON 对象（不包含 name 参数）
/// 3. 验证返回错误
///
/// # 断言
///
/// - 执行返回 `Err`，表示参数验证失败
#[tokio::test]
async fn execute_missing_name() {
    // 准备：创建工具实例
    let engine = engine_with_sops(vec![]);
    let tool = SopExecuteTool::new(engine);

    // 执行：尝试执行但不提供必需的 name 参数
    let result = tool.execute(json!({})).await;

    // 验证：检查返回错误
    assert!(result.is_err());
}

/// 测试带自定义载荷的 SOP 执行
///
/// 验证工具能够正确处理包含自定义 payload 参数的执行请求，
/// 并将载荷数据传递到 SOP 执行流程中。
///
/// # 测试步骤
///
/// 1. 创建一个自动执行模式的 SOP
/// 2. 执行工具时传入 name 和 payload 参数
/// 3. 验证执行成功且输出包含载荷数据
///
/// # 断言
///
/// - 执行结果为成功
/// - 输出包含载荷中的数值（87.3）
#[tokio::test]
async fn execute_with_payload() {
    // 准备：创建自动执行模式的测试 SOP
    let engine = engine_with_sops(vec![test_sop("test-sop", SopExecutionMode::Auto)]);
    let tool = SopExecuteTool::new(engine);

    // 执行：调用工具并传入自定义 payload
    let result =
        tool.execute(json!({"name": "test-sop", "payload": "{\"value\": 87.3}"})).await.unwrap();

    // 验证：检查执行成功且输出包含载荷数据
    assert!(result.success);
    assert!(result.output.contains("87.3"));
}

/// 测试工具的元数据（名称和参数 schema）
///
/// 验证 `SopExecuteTool` 的基础元数据配置正确，包括：
/// - 工具名称应为 "sop_execute"
/// - 参数 schema 应正确定义必需参数列表
///
/// # 测试步骤
///
/// 1. 创建工具实例
/// 2. 验证工具名称
/// 3. 验证参数 schema 中的 required 字段为数组类型
///
/// # 断言
///
/// - 工具名称等于 "sop_execute"
/// - 参数 schema 中的 required 字段是数组
#[test]
fn name_and_schema() {
    // 准备：创建工具实例
    let engine = engine_with_sops(vec![]);
    let tool = SopExecuteTool::new(engine);

    // 验证：检查工具名称
    assert_eq!(tool.name(), "sop_execute");
    assert!(tool.description().contains("手动按名称触发"));

    // 验证：检查参数 schema 格式正确
    assert!(tool.parameters_schema()["required"].is_array());
}

#[tokio::test]
async fn execute_sop_without_steps_reports_start_failure() {
    let mut sop = test_sop("empty-sop", SopExecutionMode::Auto);
    sop.steps.clear();
    let engine = engine_with_sops(vec![sop]);
    let tool = SopExecuteTool::new(engine);

    let result = tool.execute(json!({"name": "empty-sop"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap().contains("SOP 'empty-sop' has no steps defined"));
}
