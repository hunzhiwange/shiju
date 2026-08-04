//! CronAdd 工具的集成测试模块
//!
//! 本模块包含针对 `CronAddTool` 的全面测试用例，覆盖以下场景：
//! - 基本的定时任务添加功能
//! - 安全策略限制（命令白名单、只读模式）
//! - 速率限制机制
//! - 中等风险命令的审批流程
//! - 输入参数验证（无效调度配置、缺失必填字段）
//!
//! # 测试架构
//!
//! 每个测试用例都遵循以下模式：
//! 1. 创建临时目录和工作空间
//! 2. 配置安全策略和自主级别
//! 3. 创建 `CronAddTool` 实例
//! 4. 执行测试命令并验证结果
//!
//! # 安全边界测试
//!
//! 本模块重点测试了多个安全边界：
//! - 命令执行白名单验证
//! - 只读模式下禁止修改操作
//! - 速率限制防止滥用
//! - 中等风险操作需要显式批准

use super::super::*;
use crate::app::agent::config::Config;
use crate::app::agent::cron::{add_job, list_jobs};
use crate::app::agent::security::AutonomyLevel;
use serde_json::json;
use tempfile::TempDir;

/// 创建用于测试的配置对象
///
/// 初始化一个带有临时工作空间的配置，适合在隔离环境中运行测试。
/// 该函数会自动创建工作空间目录，确保测试环境的完整性。
///
/// # 参数
///
/// * `tmp` - 临时目录引用，用于创建测试工作空间
///
/// # 返回值
///
/// 返回一个 `Arc<Config>`，包含：
/// - `workspace_dir`: 在临时目录下创建的 workspace 子目录
/// - `config_path`: 在临时目录下的 vibewindow.json 配置文件路径
/// - 其他字段使用默认值
///
/// # 示例
///
/// ```ignore
/// let tmp = TempDir::new().unwrap();
/// let cfg = test_config(&tmp).await;
/// // cfg.workspace_dir 已创建并可使用
/// ```
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    Arc::new(config)
}

/// 根据配置创建安全策略
///
/// 基于提供的配置和自主级别设置，构建相应的安全策略对象。
/// 该策略将用于验证工具执行时的安全约束。
///
/// # 参数
///
/// * `cfg` - 配置对象引用，包含自主级别和工作空间路径等信息
///
/// # 返回值
///
/// 返回一个 `Arc<SecurityPolicy>`，用于：
/// - 验证命令是否在允许列表中
/// - 检查操作是否符合当前的自主级别
/// - 强制执行速率限制和其他安全约束
fn test_security(cfg: &Config) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::from_config(&cfg.autonomy, &cfg.workspace_dir))
}

/// 测试成功添加 shell 定时任务
///
/// 验证在默认配置下，可以成功添加一个执行 shell 命令的定时任务。
/// 测试使用标准的 cron 表达式（每5分钟执行一次）。
///
/// # 验证点
///
/// - 执行结果标记为成功
/// - 输出中包含 "next_run" 字段，表示已计算出下次执行时间
#[tokio::test]
async fn adds_shell_job() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    // 执行添加定时任务的命令：每5分钟执行一次 "echo ok"
    let result = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "shell",
            "command": "echo ok"
        }))
        .await
        .unwrap();

    assert!(result.success, "{:?}", result.error);
    assert!(result.output.contains("next_run"));
}

/// 测试阻止不在白名单中的 shell 命令
///
/// 验证当自主级别为 Supervised 且命令不在允许列表中时，
/// 系统应拒绝执行该命令。这是防止恶意命令执行的关键安全检查。
///
/// # 场景设置
///
/// - 自主级别：Supervised（需要监督）
/// - 允许命令列表：仅包含 "echo"
/// - 尝试执行的命令：curl（不在白名单中）
///
/// # 验证点
///
/// - 执行结果标记为失败
/// - 错误信息包含 "not allowed"
#[tokio::test]
async fn blocks_disallowed_shell_command() {
    let tmp = TempDir::new().unwrap();

    // 配置：仅允许 echo 命令，设置为监督模式
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.allowed_commands = vec!["echo".into()];
    config.autonomy.level = AutonomyLevel::Supervised;
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    let cfg = Arc::new(config);
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    // 尝试执行 curl 命令（不在白名单中）
    let result = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "shell",
            "command": "curl https://example.com"
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("not allowed"));
}

/// 测试只读模式下阻止修改操作
///
/// 验证当自主级别设置为 ReadOnly 时，系统应阻止所有添加定时任务的操作，
/// 即使该命令本身是安全的。这确保了只读模式的有效性。
///
/// # 场景设置
///
/// - 自主级别：ReadOnly（只读模式）
/// - 尝试执行的命令：echo ok（本身是安全的）
///
/// # 验证点
///
/// - 执行结果标记为失败
/// - 错误信息包含 "read-only" 或 "not allowed"
#[tokio::test]
async fn blocks_mutation_in_read_only_mode() {
    let tmp = TempDir::new().unwrap();

    // 配置：设置为只读模式
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::ReadOnly;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    // 尝试在只读模式下添加定时任务
    let result = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "shell",
            "command": "echo ok"
        }))
        .await
        .unwrap();

    assert!(!result.success);
    let error = result.error.unwrap_or_default();
    assert!(error.contains("read-only") || error.contains("not allowed"));
}

/// 测试速率限制阻止频繁的添加操作
///
/// 验证当设置速率限制为 0 时，系统应完全阻止添加定时任务的操作。
/// 这是防止系统被滥用的关键机制。
///
/// # 场景设置
///
/// - 自主级别：Full（完全自主）
/// - 每小时最大操作数：0（完全禁止）
///
/// # 验证点
///
/// - 执行结果标记为失败
/// - 错误信息包含 "Rate limit exceeded"
/// - 任务列表为空，确认任务未被添加
#[tokio::test]
async fn blocks_add_when_rate_limited() {
    let tmp = TempDir::new().unwrap();

    // 配置：完全自主但设置速率限制为 0
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Full;
    config.autonomy.max_actions_per_hour = 0;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    let result = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "shell",
            "command": "echo ok"
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("Rate limit exceeded"));
    assert!(list_jobs(&cfg).unwrap().is_empty());
}

/// 测试中等风险命令需要显式批准
///
/// 验证在监督模式下，某些中等风险的命令（如 touch）需要显式的批准标志
/// 才能添加到定时任务中。这提供了额外的安全层。
///
/// # 场景设置
///
/// - 自主级别：Supervised（监督模式）
/// - 允许命令列表：包含 "touch"
/// - 测试命令：touch（中等风险操作）
///
/// # 测试流程
///
/// 1. 首次尝试：不提供 approved 标志，应被拒绝
/// 2. 二次尝试：提供 approved: true，应成功添加
///
/// # 验证点
///
/// - 未批准时：执行失败，错误信息包含 "explicit approval"
/// - 已批准时：执行成功
#[tokio::test]
async fn medium_risk_shell_command_requires_approval() {
    let tmp = TempDir::new().unwrap();

    // 配置：允许 touch 命令，设置为监督模式
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.allowed_commands = vec!["touch".into()];
    config.autonomy.level = AutonomyLevel::Supervised;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    // 第一次尝试：不提供批准标志，应被拒绝
    let denied = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "shell",
            "command": "touch cron-approval-test"
        }))
        .await
        .unwrap();
    assert!(!denied.success);
    assert!(denied.error.unwrap_or_default().contains("explicit approval"));

    // 第二次尝试：提供显式批准，应成功
    let approved = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "shell",
            "command": "touch cron-approval-test",
            "approved": true
        }))
        .await
        .unwrap();
    assert!(approved.success, "{:?}", approved.error);
}

/// 测试拒绝无效的调度配置
///
/// 验证系统会拒绝无效的调度参数，例如设置 every_ms 为 0。
/// 这确保了定时任务配置的合理性和有效性。
///
/// # 测试场景
///
/// - 使用 "every" 类型的调度
/// - 设置 every_ms 为 0（无效值）
///
/// # 验证点
///
/// - 执行结果标记为失败
/// - 错误信息包含 "every_ms must be > 0"
#[tokio::test]
async fn rejects_invalid_schedule() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    // 尝试使用无效的调度参数（every_ms = 0）
    let result = tool
        .execute(json!({
            "schedule": { "kind": "every", "every_ms": 0 },
            "job_type": "shell",
            "command": "echo nope"
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("every_ms must be > 0"));
}

/// 测试 agent 类型任务必须提供 prompt 字段
///
/// 验证当任务类型为 "agent" 时，必须提供 prompt 字段。
/// agent 类型的任务需要明确的提示词来指导代理行为。
///
/// # 测试场景
///
/// - 任务类型：agent
/// - 不提供 prompt 字段
///
/// # 验证点
///
/// - 执行结果标记为失败
/// - 错误信息包含 "Missing 'prompt'"
#[tokio::test]
async fn agent_job_requires_prompt() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    // 尝试添加 agent 类型任务但不提供 prompt
    let result = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "agent"
        }))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("Missing 'prompt'"));
}

#[tokio::test]
async fn agent_job_inherits_execution_project_and_full_access() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let project_path = tmp.path().join("current-project");
    std::fs::create_dir_all(&project_path).unwrap();
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg))
        .with_execution_defaults(&project_path, true);

    let result = tool
        .execute(json!({
            "schedule": { "kind": "every", "every_ms": 60_000 },
            "job_type": "agent",
            "prompt": "analyze the current project"
        }))
        .await
        .unwrap();

    assert!(result.success, "{:?}", result.error);
    let jobs = list_jobs(&cfg).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0].project_path.as_deref(),
        Some(project_path.to_string_lossy().as_ref())
    );
    assert!(jobs[0].full_access);
}

#[tokio::test]
async fn agent_job_explicit_execution_options_override_defaults() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let default_project = tmp.path().join("default-project");
    let explicit_project = tmp.path().join("explicit-project");
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg))
        .with_execution_defaults(&default_project, true);

    let result = tool
        .execute(json!({
            "schedule": { "kind": "every", "every_ms": 60_000 },
            "job_type": "agent",
            "prompt": "analyze another project",
            "project_path": explicit_project.to_string_lossy(),
            "full_access": false
        }))
        .await
        .unwrap();

    assert!(result.success, "{:?}", result.error);
    let jobs = list_jobs(&cfg).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0].project_path.as_deref(),
        Some(explicit_project.to_string_lossy().as_ref())
    );
    assert!(!jobs[0].full_access);
}

#[tokio::test]
async fn rejects_unknown_job_type() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

    let result = tool
        .execute(json!({
            "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
            "job_type": "workflow",
            "command": "echo ok"
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Invalid job_type: workflow"));
    assert!(list_jobs(&cfg).unwrap().is_empty());
}
