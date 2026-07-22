//! cron 任务运行记录工具的集成测试模块
//!
//! 本模块包含针对 `CronRunsTool` 的测试用例，验证以下功能：
//! - 运行记录的查询与输出截断
//! - 缺少必需参数时的错误处理
//!
//! 测试使用临时目录隔离，确保每次测试环境独立且可重复。

use super::super::*;
use crate::app::agent::config::Config;
use crate::app::agent::cron::{add_job, record_run};
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;
use tempfile::TempDir;

/// 创建用于测试的配置实例
///
/// 在指定的临时目录下初始化测试配置，创建必要的工作空间目录结构。
/// 该函数确保测试环境隔离，避免测试间相互干扰。
///
/// # 参数
///
/// * `tmp` - 临时目录引用，用于存放测试期间的文件和配置
///
/// # 返回值
///
/// 返回一个原子引用计数的 `Config` 实例，配置中包含：
/// - `workspace_dir`: 临时目录下的 workspace 子目录
/// - `config_path`: 临时目录下的 vibewindow.json 配置文件路径
///
/// # Panics
///
/// 当无法创建工作空间目录时会触发 panic，这通常表示文件系统错误
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    // 确保工作空间目录存在，否则后续操作可能失败
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    Arc::new(config)
}

/// 测试运行记录列表的输出截断功能
///
/// 该测试验证当 cron 任务的输出过长时，`CronRunsTool` 能够正确地截断输出，
/// 并在截断处显示省略号（"..."），避免过长的输出影响可读性。
///
/// # 测试流程
///
/// 1. 创建临时目录和测试配置
/// 2. 添加一个每 5 分钟执行一次的 cron 任务
/// 3. 为该任务记录一次运行，输出长度为 1000 字符（触发截断阈值）
/// 4. 使用 `CronRunsTool` 查询该任务的运行记录
/// 5. 验证结果中包含省略号，表明输出已被截断
///
/// # 验证点
///
/// - 查询操作成功执行（success 为 true）
/// - 输出中包含 "..." 标记，表示长输出已被截断
#[tokio::test]
async fn lists_runs_with_truncation() {
    // 初始化临时测试环境
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;

    // 添加一个每 5 分钟执行一次的测试任务
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();

    // 构造超长输出（1000 字符），用于触发输出截断逻辑
    let long_output = "x".repeat(1000);
    let now = Utc::now();

    // 记录一次任务运行，包含开始时间、结束时间、状态码和超长输出
    record_run(
        &cfg,
        &job.id,
        now,
        now + ChronoDuration::milliseconds(1),
        "ok",
        Some(&long_output),
        1,
    )
    .unwrap();

    // 创建 CronRunsTool 实例并查询运行记录
    let tool = CronRunsTool::new(cfg.clone());
    let result = tool.execute(json!({ "job_id": job.id, "limit": 5 })).await.unwrap();

    // 验证：操作成功，且输出已正确截断（包含省略号）
    assert!(result.success);
    assert!(result.output.contains("..."));
}

/// 测试缺少 job_id 参数时的错误处理
///
/// 该测试验证当调用 `CronRunsTool` 时未提供必需的 `job_id` 参数，
/// 工具能够正确返回错误信息，而不是崩溃或产生未定义行为。
///
/// # 测试流程
///
/// 1. 创建临时目录和测试配置
/// 2. 创建 `CronRunsTool` 实例
/// 3. 使用空的 JSON 对象调用 execute 方法（不包含任何参数）
/// 4. 验证返回结果为失败状态，且包含明确的错误信息
///
/// # 验证点
///
/// - 操作失败（success 为 false）
/// - 错误信息中包含 "Missing 'job_id'" 提示，指明缺失的参数
///
/// # 错误边界
///
/// 此测试确保工具对无效输入具有健壮的错误处理能力，
/// 符合"快速失败"和"显式错误"的工程原则
#[tokio::test]
async fn errors_when_job_id_missing() {
    // 初始化临时测试环境
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;

    // 创建 CronRunsTool 实例
    let tool = CronRunsTool::new(cfg);

    // 执行时不提供 job_id 参数，期望返回明确的错误信息
    let result = tool.execute(json!({})).await.unwrap();

    // 验证：操作失败，且错误信息明确指出缺少 job_id 参数
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("Missing 'job_id'"));
}

#[test]
fn truncate_preserves_short_unicode_output() {
    assert_eq!(super::truncate("你好世界", 4), "你好世界");
}
