//! CronListTool 单元测试模块
//!
//! 本模块包含对 `CronListTool` 的测试用例，验证定时任务列表工具的行为：
//! - 在没有定时任务时返回空列表
//! - 当定时任务功能被禁用时返回错误
//!
//! # 测试场景
//!
//! 1. **空列表场景**：工作区中没有任何定时任务时，工具应返回空的 JSON 数组
//! 2. **禁用场景**：配置中禁用 cron 功能时，工具应拒绝执行并返回错误消息

use super::super::*;
use crate::app::agent::config::Config;
use serde_json::json;
use tempfile::TempDir;

/// 创建用于测试的配置对象
///
/// 该函数初始化一个包含临时工作区和配置路径的 Config 实例，
/// 并在工作区中创建必要的目录结构。
///
/// # 参数
///
/// * `tmp` - 临时目录引用，用于隔离测试环境
///
/// # 返回值
///
/// 返回一个 `Arc<Config>`，包含指向临时目录的配置
///
/// # 示例
///
/// ```ignore
/// let tmp = TempDir::new().unwrap();
/// let cfg = test_config(&tmp).await;
/// // cfg 可用于创建工具实例
/// ```
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    // 使用临时目录路径创建配置
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    // 确保工作区目录存在
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    Arc::new(config)
}

/// 测试：无定时任务时返回空列表
///
/// # 测试步骤
///
/// 1. 创建临时目录和测试配置
/// 2. 实例化 CronListTool
/// 3. 执行不带任何参数的查询
/// 4. 验证返回结果成功且输出为空 JSON 数组
///
/// # 预期结果
///
/// - `result.success` 为 `true`
/// - `result.output` 为 `"[]"`（空数组）
#[tokio::test]
async fn returns_empty_list_when_no_jobs() {
    // 准备：创建临时测试环境
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronListTool::new(cfg);

    // 执行：调用工具查询定时任务列表
    let result = tool.execute(json!({})).await.unwrap();

    // 断言：应该成功返回空列表
    assert!(result.success);
    assert_eq!(result.output.trim(), "[]");
}

/// 测试：定时任务功能禁用时返回错误
///
/// # 测试步骤
///
/// 1. 创建临时目录和测试配置
/// 2. 将配置中的 `cron.enabled` 设置为 `false`
/// 3. 实例化 CronListTool
/// 4. 执行查询操作
/// 5. 验证返回失败状态且包含适当的错误消息
///
/// # 预期结果
///
/// - `result.success` 为 `false`
/// - `result.error` 包含 "cron is disabled" 字样
#[tokio::test]
async fn errors_when_cron_disabled() {
    // 准备：创建临时测试环境并禁用 cron
    let tmp = TempDir::new().unwrap();
    let mut cfg = (*test_config(&tmp).await).clone();
    cfg.cron.enabled = false;
    let tool = CronListTool::new(Arc::new(cfg));

    // 执行：尝试调用工具
    let result = tool.execute(json!({})).await.unwrap();

    // 断言：应该失败并返回禁用错误
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("cron is disabled"));
}

#[tokio::test]
async fn wb_schedule_tool_list_preserves_calendar_type_timezone_and_window() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let security = Arc::new(SecurityPolicy::from_config(&cfg.autonomy, &cfg.workspace_dir));
    let add = CronAddTool::new(cfg.clone(), security.clone());
    let update = CronUpdateTool::new(cfg.clone(), security);
    let list = CronListTool::new(cfg);
    assert_eq!((add.name(), update.name(), list.name()), ("cron_add", "cron_update", "cron_list"));
    let result = add.execute(json!({"schedule":{"kind":"biweekly","anchor_date":"2096-01-02","weekdays":[1,3,5],"hour":9,"minute":0,"timezone":"Asia/Shanghai"},"prompt":"summarize","starts_at":"2096-01-01T00:00:00Z","ends_at":"2097-01-01T00:00:00Z"})).await.unwrap();
    assert!(result.success, "{:?}", result.error);
    let result = list.execute(json!({})).await.unwrap();
    assert!(result.success, "{:?}", result.error);
    let jobs: serde_json::Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(jobs.as_array().unwrap().len(), 1);
    assert_eq!(jobs[0]["schedule"]["kind"], "biweekly");
    assert_eq!(jobs[0]["schedule"]["timezone"], "Asia/Shanghai");
    assert_eq!(jobs[0]["starts_at"], "2096-01-01T00:00:00Z");
    assert_eq!(jobs[0]["ends_at"], "2097-01-01T00:00:00Z");
}
