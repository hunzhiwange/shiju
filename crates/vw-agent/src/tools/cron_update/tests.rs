//! # Cron 任务更新工具测试模块
//!
//! 本模块包含针对 `CronUpdateTool` 的集成测试用例，验证 cron 任务更新功能的各种安全约束和行为。
//!
//! ## 测试覆盖范围
//!
//! - 启用/禁用标志更新
//! - 不允许命令的阻止机制
//! - 只读模式下的变更阻止
//! - 中等风险操作需要显式审批
//! - 速率限制下的操作阻止
//!
//! ## 依赖关系
//!
//! - 依赖 `CronUpdateTool` 实现
//! - 依赖 `SecurityPolicy` 安全策略配置
//! - 依赖 `Config` 全局配置

use super::super::*;
use crate::app::agent::config::Config;
use crate::app::agent::cron::{add_job, get_job};
use crate::app::agent::security::AutonomyLevel;
use serde_json::json;
use tempfile::TempDir;

/// 创建用于测试的配置实例
///
/// 初始化一个临时的测试配置，包含工作空间目录和配置文件路径。
/// 该函数会自动创建所需的工作空间目录结构。
///
/// # 参数
///
/// * `tmp` - 临时目录引用，用于隔离测试环境
///
/// # 返回值
///
/// 返回一个线程安全的 `Config` 实例（`Arc<Config>`）
///
/// # 示例
///
/// ```ignore
/// let tmp = TempDir::new().unwrap();
/// let config = test_config(&tmp).await;
/// ```
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    // 构建测试配置，使用临时目录作为工作空间
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    // 确保工作空间目录存在
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    Arc::new(config)
}

/// 创建用于测试的安全策略实例
///
/// 基于给定配置生成对应的安全策略，用于控制工具的执行权限。
///
/// # 参数
///
/// * `cfg` - 配置引用，用于提取自治级别和工作空间信息
///
/// # 返回值
///
/// 返回一个线程安全的 `SecurityPolicy` 实例（`Arc<SecurityPolicy>`）
fn test_security(cfg: &Config) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::from_config(&cfg.autonomy, &cfg.workspace_dir))
}

/// 测试更新 cron 任务的启用标志
///
/// 验证 `CronUpdateTool` 能够正确更新 cron 任务的 `enabled` 字段。
/// 这是最低风险的更新操作，应该在默认安全策略下成功执行。
///
/// # 测试流程
///
/// 1. 创建临时测试环境
/// 2. 添加一个初始 cron 任务
/// 3. 使用工具将 `enabled` 标志设置为 `false`
/// 4. 验证操作成功且输出包含正确的更新值
#[tokio::test]
async fn updates_enabled_flag() {
    // 创建临时测试目录
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    // 添加一个每5分钟执行的测试任务
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronUpdateTool::new(cfg.clone(), test_security(&cfg));

    // 执行更新操作，将任务禁用
    let result = tool
        .execute(json!({
            "job_id": job.id,
            "patch": { "enabled": false }
        }))
        .await
        .unwrap();

    // 验证操作成功
    assert!(result.success, "{:?}", result.error);
    // 验证输出中包含正确的 enabled 值
    assert!(result.output.contains("\"enabled\": false"));
}

/// 测试阻止不允许的命令更新
///
/// 验证当尝试将 cron 任务的命令更改为不在允许列表中的命令时，
/// 安全策略会阻止该操作。
///
/// # 测试流程
///
/// 1. 创建配置，仅允许 `echo` 命令
/// 2. 添加一个初始任务
/// 3. 尝试将命令更新为 `curl`（不在允许列表中）
/// 4. 验证操作被阻止并返回错误信息
///
/// # 安全考虑
///
/// 此测试确保即使任务已存在，也不能将其命令更改为危险的或未授权的命令。
#[tokio::test]
async fn blocks_disallowed_command_updates() {
    let tmp = TempDir::new().unwrap();
    // 创建仅允许 echo 命令的限制性配置
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.allowed_commands = vec!["echo".into()];
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    let cfg = Arc::new(config);
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronUpdateTool::new(cfg.clone(), test_security(&cfg));

    // 尝试将命令更改为不允许的 curl 命令
    let result = tool
        .execute(json!({
            "job_id": job.id,
            "patch": { "command": "curl https://example.com" }
        }))
        .await
        .unwrap();
    // 验证操作被阻止
    assert!(!result.success);
    // 验证错误信息包含 "not allowed" 提示
    assert!(result.error.unwrap_or_default().contains("not allowed"));
}

/// 测试只读模式下阻止变更操作
///
/// 验证当自治级别设置为 `ReadOnly` 时，任何 cron 任务的修改操作都会被阻止。
/// 这是一种最高限制级别的安全模式。
///
/// # 测试流程
///
/// 1. 创建自治级别为 `ReadOnly` 的配置
/// 2. 添加一个初始任务
/// 3. 尝试更新任务的 `enabled` 标志
/// 4. 验证操作被阻止并返回包含 "read-only" 的错误信息
///
/// # 使用场景
///
/// 只读模式适用于审计、监控等不应产生副作用的场景。
#[tokio::test]
async fn blocks_mutation_in_read_only_mode() {
    let tmp = TempDir::new().unwrap();
    // 创建只读模式的配置
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::ReadOnly;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronUpdateTool::new(cfg.clone(), test_security(&cfg));

    // 尝试在只读模式下修改任务
    let result = tool
        .execute(json!({
            "job_id": job.id,
            "patch": { "enabled": false }
        }))
        .await
        .unwrap();
    // 验证操作被阻止
    assert!(!result.success);
    // 验证错误信息包含 "read-only" 提示
    assert!(result.error.unwrap_or_default().contains("read-only"));
}

/// 测试中等风险 shell 命令更新需要显式审批
///
/// 验证在 `Supervised`（监督）自治级别下，中等风险的命令更新
/// 需要显式的 `approved` 标志才能执行。
///
/// # 测试流程
///
/// 1. 创建自治级别为 `Supervised` 的配置，允许 `echo` 和 `touch` 命令
/// 2. 添加一个初始任务
/// 3. 尝试将命令更新为 `touch` 命令（在允许列表中但属于中等风险）
/// 4. 不带审批标志执行，验证操作被阻止
/// 5. 带审批标志执行，验证操作成功
///
/// # 安全模型
///
/// - 允许列表中的命令可能仍需要审批
/// - `approved: true` 标志用于确认用户已审查并授权该操作
#[tokio::test]
async fn medium_risk_shell_update_requires_approval() {
    let tmp = TempDir::new().unwrap();
    // 创建监督模式配置，允许特定命令
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Supervised;
    config.autonomy.allowed_commands = vec!["echo".into(), "touch".into()];
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronUpdateTool::new(cfg.clone(), test_security(&cfg));

    // 不带审批标志执行，应该被拒绝
    let denied = tool
        .execute(json!({
            "job_id": job.id,
            "patch": { "command": "touch cron-update-approval-test" }
        }))
        .await
        .unwrap();
    // 验证操作被阻止
    assert!(!denied.success);
    // 验证错误信息要求显式审批
    assert!(denied.error.unwrap_or_default().contains("explicit approval"));

    // 带审批标志执行，应该成功
    let approved = tool
        .execute(json!({
            "job_id": job.id,
            "patch": { "command": "touch cron-update-approval-test" },
            "approved": true
        }))
        .await
        .unwrap();
    // 验证操作成功
    assert!(approved.success, "{:?}", approved.error);
}

/// 测试速率限制下阻止更新操作
///
/// 验证当达到速率限制时，即使是低风险的操作也会被阻止。
/// 速率限制是防止滥用和资源耗尽的重要安全机制。
///
/// # 测试流程
///
/// 1. 创建自治级别为 `Full` 但每小时最大操作数设为 0 的配置
/// 2. 添加一个初始任务
/// 3. 尝试更新任务的 `enabled` 标志
/// 4. 验证操作因速率限制被阻止
/// 5. 验证原始任务状态未被修改
///
/// # 速率限制说明
///
/// - `max_actions_per_hour = 0` 表示禁用所有操作
/// - 速率限制适用于所有类型的操作，无论风险级别
/// - 被速率限制阻止的操作不会产生任何副作用
#[tokio::test]
async fn blocks_update_when_rate_limited() {
    let tmp = TempDir::new().unwrap();
    // 创建完全自治但速率限制为0的配置（禁止所有操作）
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Full;
    config.autonomy.max_actions_per_hour = 0;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let tool = CronUpdateTool::new(cfg.clone(), test_security(&cfg));

    // 尝试在速率限制下执行更新
    let result = tool
        .execute(json!({
            "job_id": job.id,
            "patch": { "enabled": false }
        }))
        .await
        .unwrap();
    // 验证操作因速率限制被阻止
    assert!(!result.success);
    // 验证错误信息包含速率限制提示
    assert!(result.error.unwrap_or_default().contains("Rate limit exceeded"));
    // 验证任务状态未被修改，仍然为启用状态
    assert!(get_job(&cfg, &job.id).unwrap().enabled);
}

#[tokio::test]
async fn disabled_cron_rejects_update_before_parsing_patch() {
    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.cron.enabled = false;
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    let cfg = Arc::new(config);
    let tool = CronUpdateTool::new(cfg.clone(), test_security(&cfg));

    let result = tool.execute(json!({"job_id": "job-1"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("cron is disabled by config (cron.enabled=false)"));
}
