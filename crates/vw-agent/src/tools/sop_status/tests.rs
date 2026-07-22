//! SOP 状态查询工具测试模块
//!
//! 本模块包含对 `SopStatusTool` 的全面测试用例，验证以下功能：
//! - 查询活跃 SOP 运行状态
//! - 按 SOP 名称过滤运行记录
//! - 查询特定运行 ID 的状态
//! - 指标收集与展示（全局/按 SOP）
//!
//! ## 测试覆盖
//!
//! | 场景 | 测试函数 |
//! |------|----------|
//! | 无活跃运行 | `status_no_runs` |
//! | 存在活跃运行 | `status_with_active_run` |
//! | 查询特定运行 | `status_specific_run` |
//! | 查询不存在的运行 | `status_unknown_run` |
//! | 按 SOP 名称过滤 | `status_filter_by_sop_name` |
//! | 全局指标展示 | `status_with_metrics_global` |
//! | 按 SOP 指标展示 | `status_with_metrics_per_sop` |
//! | 无收集器时的指标请求 | `status_metrics_without_collector` |
//! | 默认不显示指标 | `status_metrics_not_shown_by_default` |
//! | 工具名称与参数 Schema | `name_and_schema` |

use super::super::*;
use crate::app::agent::config::SopConfig;
use crate::app::agent::sop::SopMetricsCollector;
use crate::app::agent::sop::engine::SopEngine;
use crate::app::agent::sop::types::*;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// 创建用于测试的 SOP 配置实例
///
/// 生成一个最小可用的 SOP 对象，包含单个手动触发步骤。
/// 所有 SOP 使用默认配置值，适合快速创建测试 fixture。
///
/// # 参数
///
/// - `name`: SOP 名称标识符
///
/// # 返回
///
/// 返回配置好的 `Sop` 实例，包含：
/// - 单个步骤（Step one）
/// - 手动触发器
/// - 优先级 Normal
/// - 自动执行模式
fn test_sop(name: &str) -> Sop {
    Sop {
        name: name.into(),
        description: format!("Test SOP: {name}"),
        version: "1.0.0".into(),
        priority: SopPriority::Normal,
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
        max_concurrent: 2,
        location: None,
    }
}

/// 创建带有预设 SOP 列表的引擎实例
///
/// 使用默认配置初始化 `SopEngine`，并通过测试接口注入指定的 SOP 列表。
/// 返回线程安全的共享引用，支持多任务并发测试。
///
/// # 参数
///
/// - `sops`: 要加载到引擎中的 SOP 列表
///
/// # 返回
///
/// 返回 `Arc<Mutex<SopEngine>>`，可在测试中安全共享和修改
fn engine_with_sops(sops: Vec<Sop>) -> Arc<Mutex<SopEngine>> {
    let mut engine = SopEngine::new(SopConfig::default());
    engine.set_sops_for_test(sops);
    Arc::new(Mutex::new(engine))
}

/// 创建手动触发类型的 SOP 事件
///
/// 生成一个标准的手动触发事件 fixture，用于测试 SOP 运行启动。
/// 事件不携带 topic 和 payload，时间戳固定为测试用值。
///
/// # 返回
///
/// 返回 `SopEvent` 实例，source 为 `Manual`
fn manual_event() -> SopEvent {
    SopEvent {
        source: SopTriggerSource::Manual,
        topic: None,
        payload: None,
        timestamp: "2026-02-19T12:00:00Z".into(),
    }
}

/// 测试：查询无活跃运行时的状态输出
///
/// 验证当引擎中没有任何活跃 SOP 运行时，
/// `SopStatusTool` 返回成功状态并提示"No active runs"。
#[tokio::test]
async fn status_no_runs() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let tool = SopStatusTool::new(engine);
    let result = tool.execute(json!({})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("No active runs"));
}

/// 测试：查询存在活跃运行时的状态输出
///
/// 验证当引擎中有活跃 SOP 运行时，
/// 输出中正确显示运行数量和运行 ID。
#[tokio::test]
async fn status_with_active_run() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let run_id = {
        let mut e = engine.lock().unwrap();
        e.start_run("s1", manual_event()).unwrap();
        e.active_runs().keys().next().unwrap().clone()
    };
    let tool = SopStatusTool::new(engine);
    let result = tool.execute(json!({})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Active runs (1)"));
    assert!(result.output.contains(&run_id));
}

/// 测试：查询特定运行 ID 的详细状态
///
/// 验证通过 `run_id` 参数查询特定运行时，
/// 输出中包含运行 ID 和当前状态（running）。
#[tokio::test]
async fn status_specific_run() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let run_id = {
        let mut e = engine.lock().unwrap();
        e.start_run("s1", manual_event()).unwrap();
        e.active_runs().keys().next().unwrap().clone()
    };
    let tool = SopStatusTool::new(engine);
    let result = tool.execute(json!({"run_id": run_id})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains(&format!("Run: {run_id}")));
    assert!(result.output.contains("Status: running"));
}

/// 测试：查询不存在的运行 ID
///
/// 验证当查询不存在的 run_id 时，
/// 工具返回成功但输出提示"No run found"。
#[tokio::test]
async fn status_unknown_run() {
    let engine = engine_with_sops(vec![]);
    let tool = SopStatusTool::new(engine);
    let result = tool.execute(json!({"run_id": "nonexistent"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("No run found"));
}

/// 测试：按 SOP 名称过滤运行状态
///
/// 验证通过 `sop_name` 参数过滤时，
/// 仅显示匹配名称的 SOP 运行记录。
#[tokio::test]
async fn status_filter_by_sop_name() {
    let engine = engine_with_sops(vec![test_sop("s1"), test_sop("s2")]);
    {
        let mut e = engine.lock().unwrap();
        e.start_run("s1", manual_event()).unwrap();
        e.start_run("s2", manual_event()).unwrap();
    }
    let tool = SopStatusTool::new(engine);
    let result = tool.execute(json!({"sop_name": "s1"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("s1"));
    // s2 的运行不应出现在结果中
    assert!(!result.output.contains(" s2 "));
}

/// 测试：验证工具名称和参数 Schema
///
/// 验证 `SopStatusTool` 的基本属性：
/// - 工具名称为 "sop_status"
/// - 参数 Schema 包含 run_id、sop_name、include_metrics 三个属性
#[test]
fn name_and_schema() {
    let engine = engine_with_sops(vec![]);
    let tool = SopStatusTool::new(engine);
    assert_eq!(tool.name(), "sop_status");
    assert!(tool.description().contains("查询 SOP 执行状态"));
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["run_id"].is_object());
    assert!(schema["properties"]["sop_name"].is_object());
    assert!(schema["properties"]["include_metrics"].is_object());
    assert!(schema["properties"]["include_gate_status"].is_object());
}

/// 测试：全局指标展示
///
/// 验证请求全局指标（无 sop_name 过滤）时，
/// 输出包含 SOP 级别的聚合指标：
/// - runs_completed
/// - completion_rate
#[tokio::test]
async fn status_with_metrics_global() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let collector = Arc::new(SopMetricsCollector::new());
    // 在收集器中记录一个已完成的运行
    let run = SopRun {
        run_id: "r1".into(),
        sop_name: "s1".into(),
        trigger_event: manual_event(),
        status: SopRunStatus::Completed,
        current_step: 1,
        total_steps: 1,
        started_at: "2026-02-19T12:00:00Z".into(),
        completed_at: Some("2026-02-19T12:05:00Z".into()),
        step_results: vec![SopStepResult {
            step_number: 1,
            status: SopStepStatus::Completed,
            output: "done".into(),
            started_at: "2026-02-19T12:00:00Z".into(),
            completed_at: Some("2026-02-19T12:01:00Z".into()),
        }],
        waiting_since: None,
    };
    collector.record_run_complete(&run);

    let tool = SopStatusTool::new(engine).with_collector(collector);
    let result = tool.execute(json!({"include_metrics": true})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Metrics (sop):"));
    assert!(result.output.contains("runs_completed: 1"));
    assert!(result.output.contains("completion_rate: 1"));
}

/// 测试：按 SOP 名称的指标展示
///
/// 验证同时指定 `sop_name` 和 `include_metrics` 时，
/// 输出包含该特定 SOP 的详细指标：
/// - runs_failed
/// - completion_rate（失败时为 0）
#[tokio::test]
async fn status_with_metrics_per_sop() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let collector = Arc::new(SopMetricsCollector::new());
    let run = SopRun {
        run_id: "r1".into(),
        sop_name: "s1".into(),
        trigger_event: manual_event(),
        status: SopRunStatus::Failed,
        current_step: 1,
        total_steps: 2,
        started_at: "2026-02-19T12:00:00Z".into(),
        completed_at: Some("2026-02-19T12:05:00Z".into()),
        step_results: vec![SopStepResult {
            step_number: 1,
            status: SopStepStatus::Failed,
            output: "fail".into(),
            started_at: "2026-02-19T12:00:00Z".into(),
            completed_at: Some("2026-02-19T12:01:00Z".into()),
        }],
        waiting_since: None,
    };
    collector.record_run_complete(&run);

    let tool = SopStatusTool::new(engine).with_collector(collector);
    let result = tool.execute(json!({"sop_name": "s1", "include_metrics": true})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Metrics (sop.s1):"));
    assert!(result.output.contains("runs_failed: 1"));
    assert!(result.output.contains("completion_rate: 0"));
}

/// 测试：无收集器时请求指标
///
/// 验证当工具未配置指标收集器时，
/// 请求指标会返回成功但提示指标"not available"。
#[tokio::test]
async fn status_metrics_without_collector() {
    let engine = engine_with_sops(vec![]);
    let tool = SopStatusTool::new(engine);
    let result = tool.execute(json!({"include_metrics": true})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("not available"));
}

/// 测试：默认不显示指标
///
/// 验证不指定 `include_metrics` 参数时，
/// 即使配置了指标收集器，输出也不包含指标信息。
#[tokio::test]
async fn status_metrics_not_shown_by_default() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let collector = Arc::new(SopMetricsCollector::new());
    let tool = SopStatusTool::new(engine).with_collector(collector);
    let result = tool.execute(json!({})).await.unwrap();
    assert!(result.success);
    assert!(!result.output.contains("Metrics"));
}

#[tokio::test]
async fn status_includes_gate_status_notice_when_requested() {
    let engine = engine_with_sops(vec![]);
    let result =
        SopStatusTool::new(engine).execute(json!({"include_gate_status": true})).await.unwrap();

    assert!(result.output.contains("Gate Status: not available"));
    assert!(result.output.contains("ampersona-gates feature not enabled"));
}

#[tokio::test]
async fn status_reports_poisoned_engine_lock() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let poison_target = engine.clone();
    let _ = std::thread::spawn(move || {
        let _guard = poison_target.lock().unwrap();
        panic!("poison sop engine for status test");
    })
    .join();

    let error = SopStatusTool::new(engine)
        .execute(json!({}))
        .await
        .expect_err("poisoned lock should surface as an error");

    assert!(error.to_string().contains("Engine lock poisoned"));
}

#[test]
fn metric_value_formatter_handles_non_number_values() {
    assert_eq!(super::format_metric_value(&json!("n/a")), "\"n/a\"");
    assert_eq!(super::format_metric_value(&json!(true)), "true");
}

#[tokio::test]
async fn status_specific_completed_run_includes_completion_and_step_result() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    let run_id = {
        let mut engine = engine.lock().unwrap();
        engine.start_run("s1", manual_event()).unwrap();
        let run_id = engine.active_runs().keys().next().unwrap().clone();
        engine
            .advance_step(
                &run_id,
                SopStepResult {
                    step_number: 1,
                    status: SopStepStatus::Completed,
                    output: "inspection complete".into(),
                    started_at: "2026-02-19T12:00:00Z".into(),
                    completed_at: Some("2026-02-19T12:01:00Z".into()),
                },
            )
            .unwrap();
        run_id
    };

    let result = SopStatusTool::new(engine).execute(json!({"run_id": run_id})).await.unwrap();

    assert!(result.output.contains("Completed:"));
    assert!(result.output.contains("Step results:"));
    assert!(result.output.contains("Step 1: completed — inspection complete"));
}

#[tokio::test]
async fn status_lists_finished_runs_when_no_runs_are_active() {
    let engine = engine_with_sops(vec![test_sop("s1")]);
    {
        let mut engine = engine.lock().unwrap();
        engine.start_run("s1", manual_event()).unwrap();
        let run_id = engine.active_runs().keys().next().unwrap().clone();
        engine.cancel_run(&run_id).unwrap();
    }

    let result = SopStatusTool::new(engine).execute(json!({})).await.unwrap();

    assert!(result.output.contains("No active runs."));
    assert!(result.output.contains("Finished runs (1):"));
    assert!(result.output.contains("s1 [cancelled]"));
}
