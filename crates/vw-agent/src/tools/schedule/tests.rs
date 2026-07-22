//! # ScheduleTool 集成测试模块
//!
//! 本模块包含 `ScheduleTool` 的完整集成测试套件，验证计划任务工具的各项功能：
//!
//! - **基本功能**：创建、查询、列表、取消计划任务
//! - **别名支持**：`once`/`add`/`pause`/`resume` 等操作别名
//! - **安全策略**：只读模式、速率限制、命令白名单、审批机制
//! - **配置禁用**：当 cron 功能被禁用时的行为
//!
//! ## 测试覆盖范围
//!
//! | 功能区域 | 测试用例 |
//! |---------|---------|
//! | 工具元数据 | `tool_name_and_schema` |
//! | CRUD 操作 | `list_empty`, `create_get_and_cancel_roundtrip` |
//! | 操作别名 | `once_and_pause_resume_aliases_work` |
//! | 只读模式 | `readonly_blocks_mutating_actions` |
//! | 速率限制 | `rate_limit_blocks_create_action`, `rate_limit_blocks_cancel_and_keeps_job` |
//! | 错误处理 | `unknown_action_returns_failure`, `mutating_actions_fail_when_cron_disabled` |
//! | 命令安全 | `create_blocks_disallowed_command`, `medium_risk_create_requires_approval` |

use super::super::*;
use crate::Config;
use crate::app::agent::security::AutonomyLevel;
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

/// 创建测试环境的基础设施
///
/// 初始化一个隔离的临时目录，并配置 `ScheduleTool` 所需的运行时环境。
/// 返回一个元组，包含临时目录、配置对象和安全策略。
///
/// # 返回值
///
/// - `TempDir`: 临时目录句柄，目录会在句柄丢弃时自动清理
/// - `Config`: 包含工作空间路径的测试配置
/// - `Arc<SecurityPolicy>`: 基于配置生成的安全策略
///
/// # 环境设置
///
/// 1. 创建临时目录作为隔离环境
/// 2. 在临时目录下创建 `workspace` 子目录作为工作空间
/// 3. 设置配置文件路径为 `vibewindow.json`
/// 4. 根据配置的自主性级别创建安全策略
async fn test_setup() -> (TempDir, Config, Arc<SecurityPolicy>) {
    let tmp = TempDir::new().unwrap();
    let workspace_dir: PathBuf = tmp.path().join("workspace");
    let config_path: PathBuf = tmp.path().join("vibewindow.json");
    let config = Config { workspace_dir, config_path, ..Config::default() };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    (tmp, config, security)
}

/// 验证工具名称和参数 schema 的正确性
///
/// 测试 `ScheduleTool` 的基本元数据：
/// - 工具名称应为 `"schedule"`
/// - 参数 schema 应包含 `action` 属性定义
#[tokio::test]
async fn tool_name_and_schema() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);
    assert_eq!(tool.name(), "schedule");
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["action"].is_object());
}

/// 验证空任务列表的正确响应
///
/// 当没有任何计划任务时，`list` 操作应返回成功，
/// 且输出应包含 "No scheduled jobs" 提示信息。
#[tokio::test]
async fn list_empty() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    let result = tool.execute(json!({"action": "list"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("No scheduled jobs"));
}

/// 验证计划任务的完整生命周期：创建 → 查询 → 列表 → 取消
///
/// 测试 `ScheduleTool` 的核心 CRUD 操作：
/// 1. 使用 `create` 创建一个每 5 分钟执行的 cron 任务
/// 2. 使用 `list` 验证任务出现在列表中
/// 3. 从创建结果中解析任务 ID
/// 4. 使用 `get` 查询单个任务详情
/// 5. 使用 `cancel` 取消任务
///
/// # 验证点
///
/// - 创建结果包含 "Created recurring job" 消息
/// - 列表结果包含已创建任务的命令
/// - 查询结果返回正确的任务详情
/// - 取消操作成功完成
#[tokio::test]
async fn create_get_and_cancel_roundtrip() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    // 创建一个每 5 分钟执行的周期性任务
    let create = tool
        .execute(json!({
            "action": "create",
            "expression": "*/5 * * * *",
            "command": "echo hello"
        }))
        .await
        .unwrap();
    assert!(create.success);
    assert!(create.output.contains("Created recurring job"));

    // 验证任务出现在列表中
    let list = tool.execute(json!({"action": "list"})).await.unwrap();
    assert!(list.success);
    assert!(list.output.contains("echo hello"));

    // 从创建结果中提取任务 ID（格式为 "Created recurring job <id>"）
    let id = create.output.split_whitespace().nth(3).unwrap();

    // 使用 ID 查询单个任务
    let get = tool.execute(json!({"action": "get", "id": id})).await.unwrap();
    assert!(get.success);
    assert!(get.output.contains("echo hello"));

    // 取消任务
    let cancel = tool.execute(json!({"action": "cancel", "id": id})).await.unwrap();
    assert!(cancel.success);
}

/// 验证操作别名和暂停/恢复功能
///
/// 测试以下别名操作：
/// - `once`: 创建一次性延迟任务（等同于延迟执行的 `create`）
/// - `add`: 创建周期性任务（`create` 的别名）
/// - `pause`: 暂停任务执行
/// - `resume`: 恢复任务执行
///
/// # 验证点
///
/// - `once` 操作使用 `delay` 参数而非 cron 表达式
/// - `add` 操作等同于 `create`
/// - 暂停和恢复操作都能成功执行
#[tokio::test]
async fn once_and_pause_resume_aliases_work() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    // 创建一个 30 分钟后执行的一次性任务
    let once = tool
        .execute(json!({
            "action": "once",
            "delay": "30m",
            "command": "echo delayed"
        }))
        .await
        .unwrap();
    assert!(once.success);

    // 使用 add 别名创建周期性任务
    let add = tool
        .execute(json!({
            "action": "add",
            "expression": "*/10 * * * *",
            "command": "echo recurring"
        }))
        .await
        .unwrap();
    assert!(add.success);

    // 测试暂停和恢复操作
    let id = add.output.split_whitespace().nth(3).unwrap();
    let pause = tool.execute(json!({"action": "pause", "id": id})).await.unwrap();
    assert!(pause.success);

    let resume = tool.execute(json!({"action": "resume", "id": id})).await.unwrap();
    assert!(resume.success);
}

/// 验证只读模式下修改操作被阻止
///
/// 当自主性级别设置为 `ReadOnly` 时：
/// - `create` 等修改操作应被拒绝
/// - 错误信息应包含 "read-only" 提示
/// - `list` 等只读操作仍应正常工作
#[tokio::test]
async fn readonly_blocks_mutating_actions() {
    let tmp = TempDir::new().unwrap();
    // 配置只读模式
    let workspace_dir: PathBuf = tmp.path().join("workspace");
    let config_path: PathBuf = tmp.path().join("vibewindow.json");
    let config = Config {
        workspace_dir,
        config_path,
        autonomy: crate::app::agent::config::AutonomyConfig {
            level: AutonomyLevel::ReadOnly,
            ..Default::default()
        },
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));

    let tool = ScheduleTool::new(security, config);

    // 尝试创建任务应被阻止
    let blocked = tool
        .execute(json!({
            "action": "create",
            "expression": "* * * * *",
            "command": "echo blocked"
        }))
        .await
        .unwrap();
    assert!(!blocked.success);
    assert!(blocked.error.as_deref().unwrap().contains("read-only"));

    // 只读操作（list）仍应正常工作
    let list = tool.execute(json!({"action": "list"})).await.unwrap();
    assert!(list.success);
}

/// 验证速率限制阻止 create 操作
///
/// 当配置 `max_actions_per_hour` 为 0 时：
/// - 任何 `create` 操作都会被速率限制阻止
/// - 错误信息应包含 "Rate limit exceeded"
/// - 任务不会被创建，列表保持为空
#[tokio::test]
async fn rate_limit_blocks_create_action() {
    let tmp = TempDir::new().unwrap();
    // 配置零速率限制（禁止所有操作）
    let workspace_dir: PathBuf = tmp.path().join("workspace");
    let config_path: PathBuf = tmp.path().join("vibewindow.json");
    let config = Config {
        workspace_dir,
        config_path,
        autonomy: crate::app::agent::config::AutonomyConfig {
            level: AutonomyLevel::Full,
            max_actions_per_hour: 0,
            ..Default::default()
        },
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    let tool = ScheduleTool::new(security, config);

    // 尝试创建任务应被速率限制阻止
    let blocked = tool
        .execute(json!({
            "action": "create",
            "expression": "*/5 * * * *",
            "command": "echo blocked-by-rate-limit"
        }))
        .await
        .unwrap();
    assert!(!blocked.success);
    assert!(blocked.error.as_deref().unwrap_or_default().contains("Rate limit exceeded"));

    // 验证任务未被创建
    let list = tool.execute(json!({"action": "list"})).await.unwrap();
    assert!(list.success);
    assert!(list.output.contains("No scheduled jobs"));
}

/// 验证速率限制阻止 cancel 操作但保留任务
///
/// 场景：`max_actions_per_hour` 设置为 1
/// 1. 第一次操作（create）成功，消耗配额
/// 2. 第二次操作（cancel）因超出配额被阻止
/// 3. 任务仍然存在，未被取消
///
/// # 验证点
///
/// - 速率限制阻止后续操作，但不会回滚已完成的操作
/// - 错误信息清晰说明速率限制原因
#[tokio::test]
async fn rate_limit_blocks_cancel_and_keeps_job() {
    let tmp = TempDir::new().unwrap();
    // 配置每小时仅允许 1 次操作
    let workspace_dir: PathBuf = tmp.path().join("workspace");
    let config_path: PathBuf = tmp.path().join("vibewindow.json");
    let config = Config {
        workspace_dir,
        config_path,
        autonomy: crate::app::agent::config::AutonomyConfig {
            level: AutonomyLevel::Full,
            max_actions_per_hour: 1,
            ..Default::default()
        },
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    let tool = ScheduleTool::new(security, config);

    // 第一次操作：成功创建任务（消耗配额）
    let create = tool
        .execute(json!({
            "action": "create",
            "expression": "*/5 * * * *",
            "command": "echo keep-me"
        }))
        .await
        .unwrap();
    assert!(create.success);
    let id = create.output.split_whitespace().nth(3).unwrap();

    // 第二次操作：因超出配额被阻止
    let cancel = tool.execute(json!({"action": "cancel", "id": id})).await.unwrap();
    assert!(!cancel.success);
    assert!(cancel.error.as_deref().unwrap_or_default().contains("Rate limit exceeded"));

    // 验证任务仍然存在（未被取消）
    let get = tool.execute(json!({"action": "get", "id": id})).await.unwrap();
    assert!(get.success);
    assert!(get.output.contains("echo keep-me"));
}

/// 验证未知操作返回失败
///
/// 当请求一个不存在的 action 时：
/// - 操作应失败
/// - 错误信息应包含 "Unknown action"
#[tokio::test]
async fn unknown_action_returns_failure() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    let result = tool.execute(json!({"action": "explode"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap().contains("Unknown action"));
}

/// 验证 cron 禁用时修改操作失败
///
/// 当 `config.cron.enabled` 为 false 时：
/// - 所有修改性操作（如 create）应被拒绝
/// - 错误信息应包含 "cron is disabled"
#[tokio::test]
async fn mutating_actions_fail_when_cron_disabled() {
    let tmp = TempDir::new().unwrap();
    // 配置禁用 cron 功能
    let workspace_dir: PathBuf = tmp.path().join("workspace");
    let config_path: PathBuf = tmp.path().join("vibewindow.json");
    let mut config = Config { workspace_dir, config_path, ..Config::default() };
    config.cron.enabled = false;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    let tool = ScheduleTool::new(security, config);

    // 尝试创建任务应被拒绝
    let create = tool
        .execute(json!({
            "action": "create",
            "expression": "*/5 * * * *",
            "command": "echo hello"
        }))
        .await
        .unwrap();

    assert!(!create.success);
    assert!(create.error.as_deref().unwrap_or_default().contains("cron is disabled"));
}

/// 验证命令白名单阻止不允许的命令
///
/// 在 Supervised 模式下：
/// - 只有 `allowed_commands` 列表中的命令被允许
/// - 尝试执行不在白名单中的命令（如 curl）应被拒绝
/// - 错误信息应包含 "not allowed"
#[tokio::test]
async fn create_blocks_disallowed_command() {
    let tmp = TempDir::new().unwrap();
    // 配置监督模式，仅允许 echo 命令
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Supervised;
    config.autonomy.allowed_commands = vec!["echo".into()];
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    let tool = ScheduleTool::new(security, config);

    // 尝试使用不在白名单中的命令
    let result = tool
        .execute(json!({
            "action": "create",
            "expression": "*/5 * * * *",
            "command": "curl https://example.com"
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or_default().contains("not allowed"));
}

/// 验证中等风险命令需要显式审批
///
/// 在 Supervised 模式下：
/// - 某些命令（如 touch）被归类为中等风险
/// - 中等风险命令需要 `approved: true` 参数才能执行
/// - 未提供审批时，错误信息应包含 "explicit approval"
#[tokio::test]
async fn medium_risk_create_requires_approval() {
    let tmp = TempDir::new().unwrap();
    // 配置监督模式，允许 touch 命令
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Supervised;
    config.autonomy.allowed_commands = vec!["touch".into()];
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    let tool = ScheduleTool::new(security, config);

    // 未提供审批：应被拒绝
    let denied = tool
        .execute(json!({
            "action": "create",
            "expression": "*/5 * * * *",
            "command": "touch schedule-policy-test"
        }))
        .await
        .unwrap();
    assert!(!denied.success);
    assert!(denied.error.as_deref().unwrap_or_default().contains("explicit approval"));

    // 提供审批：应成功
    let approved = tool
        .execute(json!({
            "action": "create",
            "expression": "*/5 * * * *",
            "command": "touch schedule-policy-test",
            "approved": true
        }))
        .await
        .unwrap();
    assert!(approved.success, "{:?}", approved.error);
}

#[tokio::test]
async fn execute_rejects_missing_action_and_required_ids() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    let missing_action = tool.execute(json!({})).await.expect_err("action is required");
    assert!(missing_action.to_string().contains("Missing 'action' parameter"));

    for action in ["get", "cancel", "pause", "resume"] {
        let error = tool
            .execute(json!({ "action": action }))
            .await
            .expect_err("mutating and lookup actions require an id");
        assert!(error.to_string().contains("Missing 'id' parameter"));
    }
}

#[tokio::test]
async fn create_like_actions_reject_invalid_parameter_combinations() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    let cases = [
        (json!({ "action": "create", "command": "echo missing schedule" }), "Exactly one of"),
        (
            json!({
                "action": "create",
                "expression": "* * * * *",
                "delay": "1m",
                "command": "echo conflicting"
            }),
            "Exactly one of",
        ),
        (
            json!({ "action": "add", "delay": "1m", "command": "echo invalid add" }),
            "requires 'expression'",
        ),
        (json!({ "action": "once", "command": "echo missing time" }), "requires exactly one"),
        (
            json!({
                "action": "once",
                "delay": "1m",
                "run_at": "2030-01-01T00:00:00Z",
                "command": "echo conflicting once"
            }),
            "either delay or run_at",
        ),
    ];

    for (input, expected_error) in cases {
        let result = tool.execute(input).await.expect("shape errors are tool results");
        assert!(!result.success);
        assert!(result.error.as_deref().is_some_and(|error| error.contains(expected_error)));
    }
}

#[tokio::test]
async fn create_rejects_blank_command_and_invalid_run_at() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    let blank_command = tool
        .execute(json!({
            "action": "create",
            "expression": "* * * * *",
            "command": "   "
        }))
        .await
        .expect_err("blank commands are invalid arguments");
    assert!(blank_command.to_string().contains("Missing or empty 'command' parameter"));

    let invalid_run_at = tool
        .execute(json!({
            "action": "create",
            "run_at": "not-a-timestamp",
            "command": "echo invalid timestamp"
        }))
        .await
        .expect_err("invalid RFC3339 timestamps are rejected");
    assert!(invalid_run_at.to_string().contains("Invalid run_at timestamp"));
}

#[tokio::test]
async fn missing_jobs_return_failures_for_lookup_and_mutations() {
    let (_tmp, config, security) = test_setup().await;
    let tool = ScheduleTool::new(security, config);

    for action in ["get", "cancel", "pause", "resume"] {
        let result = tool
            .execute(json!({ "action": action, "id": "missing-job" }))
            .await
            .expect("missing jobs are represented as tool failures");
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
