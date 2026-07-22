//! SOP 引擎测试模块
//!
//! 本模块提供 SOP（标准操作程序）引擎的全面测试覆盖，包括：
//! - 触发器匹配（手动、MQTT、Webhook、Cron）
//! - MQTT 主题通配符匹配（+ 和 #）
//! - 条件表达式过滤（基于 JSON 载荷）
//! - 运行生命周期管理（启动、推进、取消）
//! - 并发限制（单 SOP 限制和全局限制）
//! - 冷却时间机制
//! - 执行模式（自动、监督、逐步、基于优先级）
//! - 审批超时自动处理
//! - 已完成运行淘汰策略
//!
//! 测试按职责拆分到多个独立文件，避免单文件体积继续膨胀。

use super::*;

use crate::app::agent::sop::types::SopExecutionMode;

#[path = "tests/approval_and_timeouts.rs"]
mod approval_and_timeouts;
#[path = "tests/execution_modes.rs"]
mod execution_modes;
#[path = "tests/fixtures.rs"]
mod fixtures;
#[path = "tests/lifecycle.rs"]
mod lifecycle;
#[path = "tests/query_and_utils.rs"]
mod query_and_utils;
#[path = "tests/retention.rs"]
mod retention;
#[path = "tests/trigger_matching.rs"]
mod trigger_matching;

#[test]
fn reload_replaces_preloaded_sops_with_workspace_contents() {
    let mut engine = fixtures::engine_with_sops(vec![fixtures::test_sop(
        "preloaded",
        SopExecutionMode::Auto,
        SopPriority::Normal,
    )]);
    let workspace = tempfile::tempdir().unwrap();

    assert_eq!(engine.sops().len(), 1);
    engine.reload(workspace.path());

    assert!(engine.sops().is_empty());
    assert!(engine.active_runs().is_empty());
}
