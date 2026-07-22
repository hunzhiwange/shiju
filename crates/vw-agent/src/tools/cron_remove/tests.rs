//! cron_remove 工具测试模块
//!
//! 本模块提供 CronRemoveTool 的集成测试用例，验证定时任务删除功能在各种场景下的行为：
//! - 正常删除存在的定时任务
//! - 参数缺失时的错误处理
//! - 只读模式下的权限控制
//! - 频率限制场景下的保护机制
//!
//! # 测试覆盖
//!
//! | 场景 | 测试函数 | 预期行为 |
//! |------|----------|----------|
//! | 正常删除 | removes_existing_job | 成功删除，返回 success=true |
//! | 参数缺失 | errors_when_job_id_missing | 返回错误，提示缺少 job_id |
//! | 只读模式 | blocks_remove_in_read_only_mode | 返回错误，拒绝删除操作 |
//! | 频率限制 | blocks_remove_when_rate_limited | 返回错误，任务保留未删除 |

use super::super::*;
use crate::app::agent::config::Config;
use crate::app::agent::cron::{add_job, list_jobs};
use crate::app::agent::security::AutonomyLevel;
use serde_json::json;
use tempfile::TempDir;

/// 创建用于测试的配置实例
///
/// # 参数
///
/// - `tmp`: 临时目录引用，用于隔离测试环境
///
/// # 返回值
///
/// 返回包装在 `Arc` 中的 `Config` 实例，包含：
/// - 独立的 workspace 目录
/// - 独立的配置文件路径
/// - 其他字段使用默认值
///
/// # 副作用
///
/// 此函数会创建 workspace 目录结构
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    // 确保 workspace 目录存在
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    Arc::new(config)
}

/// 创建用于测试的安全策略实例
///
/// # 参数
///
/// - `cfg`: 配置引用，用于提取自主等级和工作空间路径
///
/// # 返回值
///
/// 返回包装在 `Arc` 中的 `SecurityPolicy` 实例
fn test_security(cfg: &Config) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::from_config(&cfg.autonomy, &cfg.workspace_dir))
}

#[tokio::test]
async fn tool_metadata_declares_cron_remove_contract() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronRemoveTool::new(cfg.clone(), test_security(&cfg));

    assert_eq!(tool.name(), "cron_remove");
    assert_eq!(tool.description(), "根据 ID 移除定时任务");
    assert_eq!(
        tool.parameters_schema(),
        json!({
            "type": "object",
            "properties": {
                "job_id": { "type": "string" }
            },
            "required": ["job_id"]
        })
    );
}

/// 测试：成功删除已存在的定时任务
///
/// # 测试流程
///
/// 1. 创建临时测试环境
/// 2. 添加一个定时任务（每 5 分钟执行 echo ok）
/// 3. 使用 CronRemoveTool 删除该任务
/// 4. 验证删除成功且任务列表为空
///
/// # 断言
///
/// - `result.success` 应为 true
/// - 任务列表应变为空
#[tokio::test]
async fn removes_existing_job() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    // 预先创建一个定时任务
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronRemoveTool::new(cfg.clone(), test_security(&cfg));

    // 执行删除操作
    let result = tool.execute(json!({"job_id": job.id})).await.unwrap();
    assert!(result.success);
    // 验证任务已被删除，列表应为空
    assert!(list_jobs(&cfg).unwrap().is_empty());
}

/// 测试：缺少 job_id 参数时返回错误
///
/// # 测试流程
///
/// 1. 创建临时测试环境
/// 2. 构造 CronRemoveTool 实例
/// 3. 传入空 JSON 对象（无 job_id 参数）
/// 4. 验证返回错误信息包含缺失参数提示
///
/// # 断言
///
/// - `result.success` 应为 false
/// - 错误信息应包含 "Missing 'job_id'"
#[tokio::test]
async fn errors_when_job_id_missing() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronRemoveTool::new(cfg.clone(), test_security(&cfg));

    // 传入缺少 job_id 的参数
    let result = tool.execute(json!({})).await.unwrap();
    assert!(!result.success);
    // 验证错误信息提示缺少必要参数
    assert!(result.error.unwrap_or_default().contains("Missing 'job_id'"));
}

/// 测试：只读模式下阻止删除操作
///
/// # 测试流程
///
/// 1. 创建临时测试环境
/// 2. 将自主等级设置为 ReadOnly
/// 3. 预先创建一个定时任务
/// 4. 尝试使用 CronRemoveTool 删除任务
/// 5. 验证操作被拒绝
///
/// # 断言
///
/// - `result.success` 应为 false
/// - 错误信息应包含 "read-only"
#[tokio::test]
async fn blocks_remove_in_read_only_mode() {
    let tmp = TempDir::new().unwrap();
    // 构造只读模式的配置
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::ReadOnly;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    // 预先创建一个定时任务
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronRemoveTool::new(cfg.clone(), test_security(&cfg));

    // 尝试在只读模式下删除任务
    let result = tool.execute(json!({"job_id": job.id})).await.unwrap();
    assert!(!result.success);
    // 验证错误信息提示只读模式限制
    assert!(result.error.unwrap_or_default().contains("read-only"));
}

/// 测试：频率限制场景下阻止删除操作并保留任务
///
/// # 测试流程
///
/// 1. 创建临时测试环境
/// 2. 将自主等级设置为 Full，但每小时最大操作数设为 0
/// 3. 预先创建一个定时任务
/// 4. 尝试使用 CronRemoveTool 删除任务
/// 5. 验证操作被拒绝且任务仍然存在
///
/// # 断言
///
/// - `result.success` 应为 false
/// - 错误信息应包含 "Rate limit exceeded"
/// - 任务列表长度应仍为 1（任务未被删除）
#[tokio::test]
async fn blocks_remove_when_rate_limited() {
    let tmp = TempDir::new().unwrap();
    // 构造频率限制配置：每小时最多 0 次操作
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Full;
    config.autonomy.max_actions_per_hour = 0;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    // 预先创建一个定时任务
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronRemoveTool::new(cfg.clone(), test_security(&cfg));

    // 尝试在频率限制下删除任务
    let result = tool.execute(json!({"job_id": job.id})).await.unwrap();
    assert!(!result.success);
    // 验证错误信息提示频率限制
    assert!(result.error.unwrap_or_default().contains("Rate limit exceeded"));
    // 验证任务仍然存在，未被删除
    assert_eq!(list_jobs(&cfg).unwrap().len(), 1);
}

#[tokio::test]
async fn disabled_cron_rejects_remove_before_mutation() {
    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.cron.enabled = false;
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    let cfg = Arc::new(config);
    let tool = CronRemoveTool::new(cfg.clone(), test_security(&cfg));

    let result = tool.execute(json!({"job_id": "job-1"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("cron is disabled by config (cron.enabled=false)"));
}
