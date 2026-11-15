//! Cron 命令处理测试模块
//!
//! 本模块包含对 cron 任务命令处理器的单元测试，覆盖任务的更新、
//! 时区处理、字段保留以及错误场景等核心功能。
//!
//! ## 主要测试场景
//!
//! - **命令更新**：验证通过命令处理器更新任务的命令字段
//! - **表达式更新**：验证更新 cron 表达式的同时保留其他字段
//! - **时区处理**：验证时区字段的独立更新和表达式更新时时区的保留
//! - **字段保留**：验证更新部分字段时其他字段保持不变
//! - **错误处理**：验证无效输入（无更新字段、不存在的任务）的错误返回
//! - **安全检查**：验证命令执行的安全策略

use agent_config::runtime::Config;
use crate::CronJobPatch;
use crate::{
    CronCommands, CronJob, Schedule, add_shell_job, get_job, handle_command, list_jobs, pause_job,
    remove_job, resume_job, update_job,
};
use agent_security::SecurityPolicy;
use anyhow::Result;
use tempfile::TempDir;

/// 创建测试用的配置对象
///
/// 初始化一个临时目录作为工作空间，用于隔离测试环境。
/// 每个测试用例应使用独立的临时目录，避免状态污染。
///
/// # 参数
///
/// - `tmp`: 临时目录引用，用于创建测试工作空间
///
/// # 返回值
///
/// 返回配置好的 `Config` 实例，包含：
/// - `workspace_dir`: 临时目录下的工作空间路径
/// - `config_path`: 临时目录下的配置文件路径
fn test_config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    // 确保工作空间目录存在
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

/// 创建一个 shell 类型的 cron 任务用于测试
///
/// 封装 `add_shell_job` 调用，简化测试代码中的任务创建流程。
///
/// # 参数
///
/// - `config`: 配置引用，指定任务存储位置
/// - `expr`: cron 表达式（如 "*/5 * * * *"）
/// - `tz`: 可选的时区字符串（如 "America/Los_Angeles"）
/// - `cmd`: 要执行的 shell 命令
///
/// # 返回值
///
/// 返回创建成功的 `CronJob` 实例
///
/// # Panic
///
/// 如果任务创建失败，测试将 panic
fn make_job(config: &Config, expr: &str, tz: Option<&str>, cmd: &str) -> CronJob {
    add_shell_job(config, None, Schedule::Cron { expr: expr.into(), tz: tz.map(Into::into) }, cmd)
        .unwrap()
}

/// 通过命令处理器执行任务更新操作
///
/// 封装 `handle_command` 调用，将更新参数打包为 `CronCommands::Update` 枚举。
///
/// # 参数
///
/// - `config`: 配置引用
/// - `id`: 要更新的任务 ID
/// - `expression`: 可选的新 cron 表达式
/// - `tz`: 可选的新时区
/// - `command`: 可选的新命令
/// - `name`: 可选的新名称
///
/// # 返回值
///
/// 返回 `Result<()>`，成功为 `Ok(())`，失败包含错误信息
fn run_update(
    config: &Config,
    id: &str,
    expression: Option<&str>,
    tz: Option<&str>,
    command: Option<&str>,
    name: Option<&str>,
) -> Result<()> {
    handle_command(
        CronCommands::Update {
            id: id.into(),
            expression: expression.map(Into::into),
            tz: tz.map(Into::into),
            command: command.map(Into::into),
            name: name.map(Into::into),
        },
        config,
    )
}

/// 测试通过命令处理器更新任务的命令字段
///
/// 验证场景：
/// 1. 创建一个原始任务
/// 2. 通过 `handle_command` 更新命令
/// 3. 验证命令已更新且 ID 保持不变
#[test]
fn update_changes_command_via_handler() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    // 创建初始任务，命令为 "echo original"
    let job = make_job(&config, "*/5 * * * *", None, "echo original");

    // 更新命令为 "echo updated"
    run_update(&config, &job.id, None, None, Some("echo updated"), None).unwrap();

    // 验证更新后的任务
    let updated = get_job(&config, &job.id).unwrap();
    assert_eq!(updated.command, "echo updated");
    assert_eq!(updated.id, job.id);
}

/// 测试通过命令处理器更新任务的表达式字段
///
/// 验证场景：
/// 1. 创建一个使用默认表达式的任务
/// 2. 更新表达式为新的 cron 表达式
/// 3. 验证表达式已正确更新
#[test]
fn update_changes_expression_via_handler() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = make_job(&config, "*/5 * * * *", None, "echo test");

    // 将表达式从 "*/5 * * * *" 更新为 "0 9 * * *"
    run_update(&config, &job.id, Some("0 9 * * *"), None, None, None).unwrap();

    let updated = get_job(&config, &job.id).unwrap();
    assert_eq!(updated.expression, "0 9 * * *");
}

/// 测试通过命令处理器更新任务的名称字段
///
/// 验证场景：
/// 1. 创建一个无名称的任务
/// 2. 更新任务名称
/// 3. 验证名称已正确设置
#[test]
fn update_changes_name_via_handler() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = make_job(&config, "*/5 * * * *", None, "echo test");

    // 设置任务名称为 "new-name"
    run_update(&config, &job.id, None, None, None, Some("new-name")).unwrap();

    let updated = get_job(&config, &job.id).unwrap();
    assert_eq!(updated.name.as_deref(), Some("new-name"));
}

/// 测试单独更新时区字段
///
/// 验证场景：
/// 1. 创建一个无时区的任务
/// 2. 单独更新时区字段
/// 3. 验证时区已设置且表达式保持不变
#[test]
fn update_tz_alone_sets_timezone() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = make_job(&config, "*/5 * * * *", None, "echo test");

    // 设置时区为 "America/Los_Angeles"
    run_update(&config, &job.id, None, Some("America/Los_Angeles"), None, None).unwrap();

    let updated = get_job(&config, &job.id).unwrap();
    // 验证完整的 schedule 结构，确保表达式和时区都正确
    assert_eq!(
        updated.schedule,
        Schedule::Cron { expr: "*/5 * * * *".into(), tz: Some("America/Los_Angeles".into()) }
    );
}

/// 测试更新表达式时保留已存在的时区设置
///
/// 验证场景：
/// 1. 创建一个带时区的任务
/// 2. 更新表达式但不提供新的时区
/// 3. 验证原时区被保留
#[test]
fn update_expression_preserves_existing_tz() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    // 创建一个带时区的任务
    let job = make_job(&config, "*/5 * * * *", Some("America/Los_Angeles"), "echo test");

    // 更新表达式，不提供时区参数
    run_update(&config, &job.id, Some("0 9 * * *"), None, None, None).unwrap();

    let updated = get_job(&config, &job.id).unwrap();
    // 验证时区 "America/Los_Angeles" 被保留
    assert_eq!(
        updated.schedule,
        Schedule::Cron { expr: "0 9 * * *".into(), tz: Some("America/Los_Angeles".into()) }
    );
}

/// 测试更新部分字段时其他字段保持不变
///
/// 验证场景：
/// 1. 创建一个带名称的任务
/// 2. 仅更新命令字段
/// 3. 验证名称和表达式字段未被修改
#[test]
fn update_preserves_unchanged_fields() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    // 创建一个带名称的任务
    let job = add_shell_job(
        &config,
        Some("original-name".into()),
        Schedule::Cron { expr: "*/5 * * * *".into(), tz: None },
        "echo original",
    )
    .unwrap();

    // 仅更新命令字段
    run_update(&config, &job.id, None, None, Some("echo changed"), None).unwrap();

    let updated = get_job(&config, &job.id).unwrap();
    // 验证命令已更新
    assert_eq!(updated.command, "echo changed");
    // 验证名称被保留
    assert_eq!(updated.name.as_deref(), Some("original-name"));
    // 验证表达式被保留
    assert_eq!(updated.expression, "*/5 * * * *");
}

/// 测试更新操作不提供任何字段时返回错误
///
/// 验证场景：
/// 1. 创建一个任务
/// 2. 调用更新但不提供任何要更新的字段
/// 3. 验证返回错误且错误信息包含提示
#[test]
fn update_no_flags_fails() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = make_job(&config, "*/5 * * * *", None, "echo test");

    // 不提供任何更新字段，应返回错误
    let result = run_update(&config, &job.id, None, None, None, None);
    assert!(result.is_err());
    // 验证错误信息包含 "At least one of" 提示
    assert!(result.unwrap_err().to_string().contains("At least one of"));
}

/// 测试更新不存在的任务时返回错误
///
/// 验证场景：
/// 1. 不创建任何任务
/// 2. 尝试更新一个不存在的任务 ID
/// 3. 验证返回错误
#[test]
fn update_nonexistent_job_fails() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 尝试更新不存在的任务
    let result = run_update(&config, "nonexistent-id", None, None, Some("echo test"), None);
    assert!(result.is_err());
}

/// 测试安全策略允许安全命令
///
/// 验证场景：
/// 1. 从配置创建安全策略
/// 2. 验证简单的 echo 命令被允许执行
///
/// 此测试确保安全策略正确初始化并能正确判断命令安全性。
#[test]
fn update_security_allows_safe_command() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 从配置创建安全策略
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);
    // 验证 "echo safe" 命令被安全策略允许
    assert!(security.is_command_allowed("echo safe"));
}
