//! SOP 推进步骤工具测试模块
//!
//! 本模块包含 `SopAdvanceTool` 的单元测试用例，验证 SOP（标准操作流程）执行过程中的步骤推进功能。
//! 主要测试场景包括：
//! - 推进到下一步骤
//! - 完成整个 SOP 流程
//! - 步骤执行失败处理
//! - 无效状态处理
//! - 未知运行实例处理
//! - 审计日志记录（成功/失败场景）
//!
//! # 依赖关系
//! - `SopEngine`：SOP 执行引擎，负责管理和推进 SOP 运行实例
//! - `SopAdvanceTool`：工具封装，提供统一的执行接口
//! - `Memory`：内存后端，用于审计日志存储

use super::super::*;
use crate::app::agent::config::SopConfig;
use crate::app::agent::memory::Memory;
use crate::app::agent::sop::engine::SopEngine;
use crate::app::agent::sop::types::*;
use crate::app::agent::sop::{SopAuditLogger, SopMetricsCollector};
use serde_json::json;
use std::sync::{Arc, Mutex};

/// 构造测试用 SOP 实例
///
/// 创建一个包含两个步骤的标准操作流程，用于测试推进功能。
///
/// # 返回值
///
/// 返回配置好的 `Sop` 实例，包含：
/// - 名称："test-sop"
/// - 版本："1.0.0"
/// - 优先级：Normal
/// - 执行模式：Auto
/// - 触发方式：Manual（手动触发）
/// - 两个测试步骤
fn test_sop() -> Sop {
    Sop {
        name: "test-sop".into(),
        description: "Test SOP".into(),
        version: "1.0.0".into(),
        priority: SopPriority::Normal,
        execution_mode: SopExecutionMode::Auto,
        triggers: vec![SopTrigger::Manual],
        steps: vec![
            SopStep {
                number: 1,
                title: "Step one".into(),
                body: "Do step one".into(),
                suggested_tools: vec![],
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

/// 创建带有活跃运行实例的 SOP 引擎
///
/// 初始化一个 SOP 引擎，注册测试 SOP，并启动一个新的运行实例。
///
/// # 返回值
///
/// 返回元组 `(Arc<Mutex<SopEngine>>, String)`：
/// - 第一个元素：包装在 `Arc<Mutex>` 中的 SOP 引擎实例，支持多线程共享访问
/// - 第二个元素：刚启动的运行实例 ID
///
/// # 用途
///
/// 该辅助函数为需要测试推进功能的测试用例提供预配置的引擎环境，
/// 避免每个测试用例重复初始化代码。
fn engine_with_active_run() -> (Arc<Mutex<SopEngine>>, String) {
    let mut engine = SopEngine::new(SopConfig::default());
    engine.set_sops_for_test(vec![test_sop()]);
    let event = SopEvent {
        source: SopTriggerSource::Manual,
        topic: None,
        payload: None,
        timestamp: "2026-02-19T12:00:00Z".into(),
    };
    engine.start_run("test-sop", event).unwrap();
    let run_id = engine.active_runs().keys().next().expect("expected active run").clone();
    (Arc::new(Mutex::new(engine)), run_id)
}

/// 测试推进到下一步骤
///
/// 验证当前步骤标记为完成后，工具能正确推进到下一个步骤。
///
/// # 测试流程
///
/// 1. 创建带有活跃运行实例的引擎
/// 2. 使用 `SopAdvanceTool` 执行推进操作，状态为 "completed"
/// 3. 验证返回结果为成功
/// 4. 验证输出包含 "Next step" 和下一步骤标题
#[tokio::test]
async fn advance_to_next_step() {
    let (engine, run_id) = engine_with_active_run();
    let tool = SopAdvanceTool::new(engine);
    let result = tool
        .execute(json!({
            "run_id": run_id,
            "status": "completed",
            "output": "Step 1 done successfully"
        }))
        .await
        .unwrap();
    assert!(result.success);
    assert!(result.output.contains("Next step"));
    assert!(result.output.contains("Step two"));
}

/// 测试推进到流程完成
///
/// 验证当所有步骤都完成后，工具能正确标记整个 SOP 流程为完成状态。
///
/// # 测试流程
///
/// 1. 创建带有活跃运行实例的引擎
/// 2. 完成第一步（步骤 1）
/// 3. 完成第二步（步骤 2，即最后一步）
/// 4. 验证最终结果为成功
/// 5. 验证输出包含 "completed successfully"，表示整个流程已完成
#[tokio::test]
async fn advance_to_completion() {
    let (engine, run_id) = engine_with_active_run();
    let tool = SopAdvanceTool::new(engine.clone());

    // 完成步骤 1
    tool.execute(json!({
        "run_id": run_id,
        "status": "completed",
        "output": "Step 1 done"
    }))
    .await
    .unwrap();

    // 完成步骤 2
    let result = tool
        .execute(json!({
            "run_id": run_id,
            "status": "completed",
            "output": "Step 2 done"
        }))
        .await
        .unwrap();
    assert!(result.success);
    assert!(result.output.contains("completed successfully"));
}

/// 测试步骤执行失败推进
///
/// 验证当前步骤标记为失败时，工具能正确处理并记录失败信息。
///
/// # 测试流程
///
/// 1. 创建带有活跃运行实例的引擎
/// 2. 使用 `SopAdvanceTool` 执行推进操作，状态为 "failed"
/// 3. 验证工具执行成功（`success` 为 true），因为工具本身完成了任务
/// 4. 验证输出包含 "failed" 标识和具体的失败原因
///
/// # 注意事项
///
/// 这里的 `success` 表示工具调用成功，而非 SOP 步骤执行成功。
/// 步骤的失败状态通过输出内容体现。
#[tokio::test]
async fn advance_with_failure() {
    let (engine, run_id) = engine_with_active_run();
    let tool = SopAdvanceTool::new(engine);
    let result = tool
        .execute(json!({
            "run_id": run_id,
            "status": "failed",
            "output": "Valve stuck open"
        }))
        .await
        .unwrap();
    assert!(result.success); // 工具执行成功，但 SOP 步骤失败
    assert!(result.output.contains("failed"));
    assert!(result.output.contains("Valve stuck open"));
}

/// 测试无效状态推进
///
/// 验证当传入无效的状态值时，工具能正确返回错误信息。
///
/// # 测试流程
///
/// 1. 创建带有活跃运行实例的引擎
/// 2. 使用 `SopAdvanceTool` 执行推进操作，状态为 "invalid"（无效值）
/// 3. 验证返回结果为失败（`success` 为 false）
/// 4. 验证错误信息包含 "Invalid status"
///
/// # 预期行为
///
/// 工具应验证状态参数，只接受 "completed"、"failed" 等有效值。
#[tokio::test]
async fn advance_invalid_status() {
    let (engine, run_id) = engine_with_active_run();
    let tool = SopAdvanceTool::new(engine);
    let result = tool
        .execute(json!({
            "run_id": run_id,
            "status": "invalid",
            "output": "whatever"
        }))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Invalid status"));
}

/// 测试未知运行实例推进
///
/// 验证当尝试推进不存在的运行实例时，工具返回错误。
///
/// # 测试流程
///
/// 1. 创建空的 SOP 引擎（无任何运行实例）
/// 2. 使用 `SopAdvanceTool` 尝试推进一个不存在的运行实例
/// 3. 验证返回 `Err`，表示操作失败
///
/// # 预期行为
///
/// 工具应检查运行实例是否存在，不存在时返回错误而非静默失败。
#[tokio::test]
async fn advance_unknown_run() {
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));
    let tool = SopAdvanceTool::new(engine);
    let result = tool
        .execute(json!({
            "run_id": "nonexistent",
            "status": "completed",
            "output": "done"
        }))
        .await;
    assert!(result.is_err());
}

/// 测试工具名称和参数 schema
///
/// 验证 `SopAdvanceTool` 的基本属性：
/// - 工具名称应为 "sop_advance"
/// - 参数 schema 应包含必要的属性定义
///
/// # 测试流程
///
/// 1. 创建 `SopAdvanceTool` 实例
/// 2. 验证 `name()` 返回 "sop_advance"
/// 3. 验证参数 schema 中包含 `run_id` 属性
/// 4. 验证 `status` 属性包含枚举值定义
#[test]
fn name_and_schema() {
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));
    let tool = SopAdvanceTool::new(engine);
    assert_eq!(tool.name(), "sop_advance");
    assert!(tool.description().contains("前进到下一步"));
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["run_id"].is_object());
    assert!(schema["properties"]["status"]["enum"].is_array());
}

#[test]
fn with_collector_configures_metrics_collection() {
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));
    let collector = Arc::new(SopMetricsCollector::new());
    let tool = SopAdvanceTool::new(engine).with_collector(collector);

    assert!(tool.collector.is_some());
}

#[tokio::test]
async fn advance_with_skipped_status_moves_to_next_step() {
    let (engine, run_id) = engine_with_active_run();
    let tool = SopAdvanceTool::new(engine);

    let result = tool
        .execute(json!({
            "run_id": run_id,
            "status": "skipped",
            "output": "Not applicable"
        }))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.output.contains("Step two"));
}

#[tokio::test]
async fn advance_requires_each_declared_parameter() {
    let (engine, run_id) = engine_with_active_run();
    let tool = SopAdvanceTool::new(engine);

    assert!(tool.execute(json!({"status": "completed", "output": "done"})).await.is_err());
    assert!(tool.execute(json!({"run_id": &run_id, "output": "done"})).await.is_err());
    assert!(tool.execute(json!({"run_id": &run_id, "status": "completed"})).await.is_err());
}

/// 测试推进错误时不写入步骤审计日志
///
/// 验证当推进操作失败时（如运行实例不存在），不会产生幽灵审计记录。
/// 这是防止数据污染的重要安全检查。
///
/// # 测试流程
///
/// 1. 创建空引擎和 SQLite 内存后端
/// 2. 配置审计日志记录器
/// 3. 尝试推进不存在的运行实例（预期失败）
/// 4. 验证推进操作返回错误
/// 5. 查询审计日志，验证没有任何记录被写入
///
/// # 预期行为
///
/// 失败的推进操作不应产生任何审计条目，确保审计记录的一致性。
#[tokio::test]
async fn advance_error_does_not_write_step_audit() {
    // 使用不存在的 run_id —— advance_step 将失败
    let engine = Arc::new(Mutex::new(SopEngine::new(SopConfig::default())));
    let tmp = tempfile::tempdir().unwrap();
    let mem_cfg = crate::app::agent::config::MemoryConfig {
        backend: "sqlite".into(),
        ..crate::app::agent::config::MemoryConfig::default()
    };
    let memory: Arc<dyn Memory> =
        Arc::from(crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());
    let audit = Arc::new(SopAuditLogger::new(memory.clone()));

    let tool = SopAdvanceTool::new(engine).with_audit(audit.clone());
    let result = tool
        .execute(json!({
            "run_id": "nonexistent",
            "status": "completed",
            "output": "done"
        }))
        .await;
    // 对不存在的运行执行 advance_step 返回 Err（anyhow）
    assert!(result.is_err());

    // 验证没有幽灵审计条目被写入
    let runs = audit.list_runs().await.unwrap();
    assert!(runs.is_empty(), "no audit entries should exist after advance error");
}

/// 测试推进成功时写入步骤审计日志
///
/// 验证当步骤推进成功时，审计日志正确记录步骤执行信息。
///
/// # 测试流程
///
/// 1. 创建带有活跃运行实例的引擎和 SQLite 内存后端
/// 2. 配置审计日志记录器
/// 3. 执行成功的推进操作（完成步骤 1）
/// 4. 验证操作返回成功
/// 5. 查询内存后端，验证存在以 "sop_step_" 开头的审计条目
///
/// # 预期行为
///
/// 成功的推进操作应产生对应的审计记录，包含步骤执行详情。
#[tokio::test]
async fn advance_success_writes_step_audit() {
    let (engine, run_id) = engine_with_active_run();
    let tmp = tempfile::tempdir().unwrap();
    let mem_cfg = crate::app::agent::config::MemoryConfig {
        backend: "sqlite".into(),
        ..crate::app::agent::config::MemoryConfig::default()
    };
    let memory: Arc<dyn Memory> =
        Arc::from(crate::app::agent::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());
    let audit = Arc::new(SopAuditLogger::new(memory.clone()));

    let tool = SopAdvanceTool::new(engine).with_audit(audit.clone());
    let result = tool
        .execute(json!({
            "run_id": run_id,
            "status": "completed",
            "output": "Step 1 done"
        }))
        .await
        .unwrap();
    assert!(result.success);

    // 验证步骤审计已写入
    let entries = memory
        .list(Some(&crate::app::agent::memory::traits::MemoryCategory::Custom("sop".into())), None)
        .await
        .unwrap();
    let step_keys: Vec<_> = entries.iter().filter(|e| e.key.starts_with("sop_step_")).collect();
    assert!(!step_keys.is_empty(), "step audit should be written on success");
}
