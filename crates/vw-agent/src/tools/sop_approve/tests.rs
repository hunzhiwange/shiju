//! SOP 审批工具测试模块
//!
//! 本模块包含对 [`SopApproveTool`] 的单元测试，覆盖以下场景：
//! - 审批等待中的 SOP 运行实例
//! - 审批不存在运行实例的错误处理
//! - 缺少必需参数时的错误处理
//! - 工具名称和参数 schema 验证
//! - 审批成功时写入审计日志
//! - 审批失败时不写入审计日志

use super::super::*;
use crate::app::agent::config::SopConfig;
use crate::app::agent::memory::Memory;
use crate::app::agent::sop::SopAuditLogger;
use crate::app::agent::sop::engine::SopEngine;
use crate::app::agent::sop::types::*;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// 创建用于测试的标准 SOP 实例
///
/// 返回一个具有以下特征的测试用 SOP：
/// - 名称：`test-sop`
/// - 执行模式：监督模式（Supervised），需要人工审批
/// - 包含一个测试步骤
fn test_sop() -> Sop {
    Sop {
        name: "test-sop".into(),
        description: "Test SOP".into(),
        version: "1.0.0".into(),
        priority: SopPriority::Normal,
        // 使用监督模式，触发后需要等待审批
        execution_mode: SopExecutionMode::Supervised,
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

/// 创建包含活动运行实例的 SOP 引擎
///
/// 该辅助函数完成以下操作：
/// 1. 创建新的 SOP 引擎实例
/// 2. 注册测试用 SOP
/// 3. 启动一次运行（监督模式下进入等待审批状态）
/// 4. 返回引擎的线程安全引用和运行 ID
///
/// # 返回值
///
/// 返回元组 `(Arc<Mutex<SopEngine>>, String)`：
/// - 第一个元素是包装在互斥锁中的 SOP 引擎
/// - 第二个元素是新创建的运行实例 ID
fn engine_with_run() -> (Arc<Mutex<SopEngine>>, String) {
    let mut engine = SopEngine::new(SopConfig::default());
    engine.set_sops_for_test(vec![test_sop()]);

    // 构造手动触发事件
    let event = SopEvent {
        source: SopTriggerSource::Manual,
        topic: None,
        payload: None,
        timestamp: "2026-02-19T12:00:00Z".into(),
    };

    // 启动运行 —— 监督模式下会进入 WaitApproval 状态
    engine.start_run("test-sop", event).unwrap();

    // 获取刚创建的活动运行 ID
    let run_id = engine.active_runs().keys().next().expect("expected active run").clone();
    (Arc::new(Mutex::new(engine)), run_id)
}

/// 测试成功审批等待中的 SOP 运行
///
/// 验证当传入有效的运行 ID 时：
/// - 审批操作成功（`success` 为 true）
/// - 输出包含 "Approved" 关键字
/// - 输出包含当前步骤标题
#[tokio::test]
async fn approve_waiting_run() {
    let (engine, run_id) = engine_with_run();
    let tool = SopApproveTool::new(engine);
    let result = tool.execute(json!({"run_id": run_id})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("Approved"));
    assert!(result.output.contains("Step one"));
}

/// 测试审批不存在的运行实例
///
/// 验证当传入不存在的运行 ID 时：
/// - 操作失败（`success` 为 false）
/// - 返回包含 "Approval failed" 的错误信息
#[tokio::test]
async fn approve_nonexistent_run() {
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));
    let tool = SopApproveTool::new(engine);
    let result = tool.execute(json!({"run_id": "nonexistent"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap().contains("Approval failed"));
}

/// 测试缺少必需参数 run_id 时的错误处理
///
/// 验证当请求中缺少 `run_id` 字段时：
/// - 返回错误结果而非失败的成功响应
#[tokio::test]
async fn approve_missing_run_id() {
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));
    let tool = SopApproveTool::new(engine);
    let result = tool.execute(json!({})).await;

    assert!(result.is_err());
}

/// 测试工具名称和参数 schema
///
/// 验证：
/// - 工具名称为 "sop_approve"
/// - 参数 schema 中 `required` 字段为数组类型
#[test]
fn name_and_schema() {
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));
    let tool = SopApproveTool::new(engine);

    assert_eq!(tool.name(), "sop_approve");
    assert!(tool.description().contains("批准"));
    assert!(tool.parameters_schema()["required"].is_array());
}

/// 测试审批成功时写入审计日志
///
/// 验证当审批成功时：
/// - 审计日志被正确写入内存后端
/// - 日志条目的 key 以 "sop_approval_" 前缀存储
#[tokio::test]
async fn approve_writes_audit() {
    let (engine, run_id) = engine_with_run();

    // 创建临时目录用于 SQLite 内存后端
    let tmp = tempfile::tempdir().unwrap();
    let mem_cfg = crate::app::agent::config::MemoryConfig {
        backend: "sqlite".into(),
        ..crate::app::agent::config::MemoryConfig::default()
    };

    // 初始化内存后端和审计日志记录器
    let memory: Arc<dyn Memory> =
        Arc::from(crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());
    let audit = Arc::new(SopAuditLogger::new(memory.clone()));

    // 使用带审计功能的工具执行审批
    let tool = SopApproveTool::new(engine).with_audit(audit.clone());
    let result = tool.execute(json!({"run_id": &run_id})).await.unwrap();
    assert!(result.success);

    // 验证审批审计条目已写入（以 sop_approval_ 前缀存储）
    let entries = memory
        .list(Some(&crate::app::agent::memory::traits::MemoryCategory::Custom("sop".into())), None)
        .await
        .unwrap();
    let approval_keys: Vec<_> =
        entries.iter().filter(|e| e.key.starts_with("sop_approval_")).collect();
    assert!(!approval_keys.is_empty(), "approval audit should be written on approve");
}

/// 测试审批失败时不写入审计日志
///
/// 验证当审批失败（如运行不存在）时：
/// - 不会创建审计日志条目
/// - `get_run` 方法返回 None
#[tokio::test]
async fn approve_failure_does_not_write_audit() {
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));

    // 创建临时目录和内存后端
    let tmp = tempfile::tempdir().unwrap();
    let mem_cfg = crate::app::agent::config::MemoryConfig {
        backend: "sqlite".into(),
        ..crate::app::agent::config::MemoryConfig::default()
    };
    let memory: Arc<dyn Memory> =
        Arc::from(crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());
    let audit = Arc::new(SopAuditLogger::new(memory.clone()));

    // 执行失败的审批操作
    let tool = SopApproveTool::new(engine).with_audit(audit.clone());
    let result = tool.execute(json!({"run_id": "nonexistent"})).await.unwrap();
    assert!(!result.success);

    // 验证失败的审批不会写入审计日志
    let stored = audit.get_run("nonexistent").await.unwrap();
    assert!(stored.is_none(), "failed approve should not write audit");
}
