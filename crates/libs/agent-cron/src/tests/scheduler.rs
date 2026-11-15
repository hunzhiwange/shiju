//! 调度领域既有测试；真实宿主集成留在 vw-agent。
use agent_config::runtime::Config;
use agent_security::{SecurityPolicy, ShellRedirectPolicy};
use crate::{CronJob, DeliveryConfig, JobType, Schedule, SessionTarget};
use crate::execution::*;
use crate::scheduler::{run_job_command, run_job_command_with_timeout, cron_full_access_config,
    cron_agent_prompt, is_high_frequency_agent_job, warn_if_high_frequency_agent_job,
    notify_schedule_changed, wait_for_schedule_scan, is_one_shot_auto_delete};
#[cfg(not(target_arch = "wasm32"))]
use crate::scheduler::{cron_gateway_options, cron_gateway_agent_request, format_gateway_agent_output};
use chrono::{Duration as ChronoDuration, Utc};
use tempfile::TempDir;
use tokio::time::Duration;

async fn test_config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    config
}

fn test_job(command: &str) -> CronJob {
    CronJob {
        id: "test-job".into(),
        expression: "* * * * *".into(),
        schedule: Schedule::Cron { expr: "* * * * *".into(), tz: None },
        command: command.into(),
        prompt: None,
        name: None,
        job_type: JobType::Shell,
        session_target: SessionTarget::Isolated,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: None,
        wake: false,
        fallbacks: Vec::new(),
        full_access: false,
        task_pool: false,
        enabled: true,
        delivery: DeliveryConfig::default(),
        delete_after_run: false,
        created_at: Utc::now(),
        starts_at: None,
        ends_at: None,
        next_run: Utc::now(),
        last_run: None,
        last_status: None,
        last_output: None,
    }
}

#[tokio::test]
async fn schedule_change_notification_interrupts_poll_wait() {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    interval.tick().await;

    notify_schedule_changed();

    tokio::time::timeout(Duration::from_millis(100), wait_for_schedule_scan(&mut interval))
        .await
        .expect("schedule change notification should wake scheduler scan wait");
}

/// 测试 Shell 命令成功执行
///
/// 验证基本的命令执行功能：
/// - 命令能够正常执行并返回成功状态
/// - 输出中包含预期的内容
/// - 状态码为 0（成功退出）
#[tokio::test]
async fn run_job_command_success() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let job = test_job("echo scheduler-ok");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(success);
    assert!(output.contains("scheduler-ok"));
    assert!(output.contains("status=exit status: 0"));
}

#[tokio::test]
async fn run_job_command_failure() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let job = test_job("ls definitely_missing_file_for_scheduler_test");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("definitely_missing_file_for_scheduler_test"));
    assert!(output.contains("status=exit status:"));
}

#[tokio::test]
async fn run_job_command_reports_spawn_error_for_missing_working_directory() {
    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        workspace_dir: tmp.path().join("missing-workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.allowed_commands = vec!["echo".into()];
    let job = test_job("echo should-not-start");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;

    assert!(!success);
    assert!(output.starts_with("spawn error:"));
}

/// 测试命令执行超时
///
/// 验证命令超时机制：
/// - 长时间运行的命令会被强制终止
/// - 返回超时错误标识
/// - 输出中包含超时消息
#[tokio::test]
async fn run_job_command_times_out() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["sleep".into()];
    let job = test_job("sleep 1");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) =
        run_job_command_with_timeout(&config, &security, &job, Duration::from_millis(50)).await;
    assert!(!success);
    assert!(output.contains("job timed out after"));
}

/// 测试阻止未授权命令执行
///
/// 验证命令白名单机制：
/// - 只有在 allowed_commands 列表中的命令才能执行
/// - 未授权命令会被安全策略阻止
/// - 输出中包含明确的阻止原因
#[tokio::test]
async fn run_job_command_blocks_disallowed_command() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["echo".into()];
    let job = test_job("curl https://evil.example");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("command not allowed"));
}

/// 测试阻止禁止路径参数
///
/// 验证路径安全检查：
/// - 系统关键路径（如 /etc/passwd）被禁止访问
/// - 安全策略会检测命令参数中的路径
/// - 返回明确的禁止路径错误
#[tokio::test]
async fn run_job_command_blocks_forbidden_path_argument() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["cat".into()];
    let job = test_job("cat /etc/passwd");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("forbidden path argument"));
    assert!(output.contains("/etc/passwd"));
}

/// 测试阻止选项赋值形式的禁止路径参数
///
/// 验证路径安全检查能够识别 --option=value 形式的路径参数：
/// - 检测长选项赋值中的路径（如 --file=/etc/passwd）
/// - 阻止通过选项绕过路径安全检查的尝试
#[tokio::test]
async fn run_job_command_blocks_forbidden_option_assignment_path_argument() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["grep".into()];
    let job = test_job("grep --file=/etc/passwd root ./src");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("forbidden path argument"));
    assert!(output.contains("/etc/passwd"));
}

/// 测试阻止短选项紧连形式的禁止路径参数
///
/// 验证路径安全检查能够识别 -ovalue 形式的路径参数：
/// - 检测短选项紧连值中的路径（如 -f/etc/passwd）
/// - 阻止通过短选项格式绕过路径检查的尝试
#[tokio::test]
async fn run_job_command_blocks_forbidden_short_option_attached_path_argument() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["grep".into()];
    let job = test_job("grep -f/etc/passwd root ./src");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("forbidden path argument"));
    assert!(output.contains("/etc/passwd"));
}

/// 测试阻止波浪号用户路径参数
///
/// 验证路径安全检查能够识别用户主目录路径：
/// - 检测 ~user 形式的路径（如 ~root/.ssh/id_rsa）
/// - 防止通过波浪号语法访问其他用户的文件
#[tokio::test]
async fn run_job_command_blocks_tilde_user_path_argument() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.workspace_only = true;
    config.autonomy.allowed_commands = vec!["cat".into()];
    let job = test_job("cat ~root/.ssh/id_rsa");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("forbidden path argument"));
    assert!(output.contains("~root/.ssh/id_rsa"));
}

/// 测试阻止输入重定向绕过
///
/// 验证 shell 重定向安全策略：
/// - 输入重定向（<）被视为 shell 特性而被阻止
/// - 防止通过重定向语法绕过路径安全检查
#[tokio::test]
async fn run_job_command_blocks_input_redirection_path_bypass() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["cat".into()];
    let job = test_job("cat </etc/passwd");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("command not allowed"));
}

/// 测试 Strip 重定向策略标准化常见 stderr 重定向
///
/// 验证 ShellRedirectPolicy::Strip 策略：
/// - 该策略会移除命令中的重定向语法
/// - 移除后命令仍能正常执行
/// - 常见的 2>&1 重定向会被正确处理
#[tokio::test]
async fn run_job_command_strip_policy_normalizes_common_stderr_redirects() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["echo".into()];
    config.autonomy.shell_redirect_policy = ShellRedirectPolicy::Strip;
    let job = test_job("echo scheduler-strip 2>&1");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(success);
    assert!(output.contains("scheduler-strip"));
}

#[tokio::test]
async fn run_job_command_allow_policy_executes_file_redirect() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["printf".into()];
    config.autonomy.allow_unsafe_shell_patterns = true;
    config.autonomy.shell_redirect_policy = ShellRedirectPolicy::Allow;
    let output_path = config.workspace_dir.join("cron-allow-output.txt");
    let job = test_job("printf redirected > cron-allow-output.txt");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;

    assert!(success, "{output}");
    assert_eq!(tokio::fs::read_to_string(output_path).await.unwrap(), "redirected");
}

/// 测试只读模式阻止命令执行
///
/// 验证 AutonomyLevel::ReadOnly 安全级别：
/// - 在只读模式下，所有命令执行都会被阻止
/// - 返回明确的只读模式错误信息
#[tokio::test]
async fn run_job_command_blocks_readonly_mode() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.level = agent_security::AutonomyLevel::ReadOnly;
    let job = test_job("echo should-not-run");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("read-only"));
}

/// 测试速率限制阻止命令执行
///
/// 验证每小时最大操作数限制：
/// - 当 max_actions_per_hour 为 0 时，命令执行被阻止
/// - 返回速率限制超出的错误信息
#[tokio::test]
async fn run_job_command_blocks_rate_limited() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.max_actions_per_hour = 0;
    let job = test_job("echo should-not-run");
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_job_command(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("rate limit exceeded"));
}

#[tokio::test]
async fn full_access_agent_job_uses_trusted_runtime_config() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.always_ask = vec!["shell".into()];
    config.autonomy.non_cli_excluded_tools = vec!["shell".into()];
    let run_config = cron_full_access_config(&config);

    assert_eq!(run_config.autonomy.level, agent_security::AutonomyLevel::Full);
    assert!(!run_config.autonomy.workspace_only);
    assert_eq!(run_config.autonomy.allowed_commands, vec!["*"]);
    assert_eq!(run_config.autonomy.allowed_roots, vec!["/"]);
    assert!(run_config.autonomy.forbidden_paths.is_empty());
    assert_eq!(run_config.autonomy.max_actions_per_hour, u32::MAX);
    assert!(!run_config.autonomy.require_approval_for_medium_risk);
    assert!(!run_config.autonomy.block_high_risk_commands);
    assert_eq!(run_config.autonomy.auto_approve, vec!["*"]);
    assert!(run_config.autonomy.always_ask.is_empty());
    assert!(run_config.autonomy.non_cli_excluded_tools.is_empty());
    assert!(run_config.autonomy.allow_unsafe_shell_patterns);
}

#[test]
fn full_access_agent_prompt_includes_automation_instruction() {
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.full_access = true;

    let prompt = cron_agent_prompt(&job, "daily", "写入报告");
    assert!(prompt.contains("我是自动化脚本，无法回答你的问题"));
    assert!(prompt.contains("写入报告"));
}

#[test]
fn standard_agent_prompt_uses_compact_cron_prefix() {
    let job = test_job("");

    let prompt = cron_agent_prompt(&job, "daily", "write report");

    assert_eq!(prompt, "[cron:test-job daily] write report");
}

#[test]
fn agent_model_candidates_keep_primary_and_unique_non_blank_fallbacks() {
    let candidates = agent_model_candidates(
        Some("primary".into()),
        &[" fallback-a ".into(), "primary".into(), " ".into(), "fallback-a".into()],
    );

    assert_eq!(candidates, vec![Some("primary".into()), Some("fallback-a".into())]);
}

#[test]
fn agent_model_candidates_use_default_route_when_primary_is_blank() {
    let candidates =
        agent_model_candidates(Some("   ".into()), &["fallback-a".into(), "fallback-b".into()]);

    assert_eq!(candidates, vec![None, Some("fallback-a".into()), Some("fallback-b".into())]);
}

#[test]
fn normalize_agent_job_success_output_trims_or_uses_default_message() {
    assert_eq!(normalize_agent_job_success_output("  completed  "), "completed");
    assert_eq!(normalize_agent_job_success_output("   "), "agent job executed");
}

#[test]
fn high_frequency_agent_warning_handles_shell_every_cron_and_at_schedules() {
    let shell = test_job("echo ok");
    assert!(!is_high_frequency_agent_job(&shell));
    warn_if_high_frequency_agent_job(&shell);

    let mut every = test_job("");
    every.job_type = JobType::Agent;
    every.prompt = Some("run often".into());
    every.schedule = Schedule::Every { every_ms: 60_000 };
    assert!(is_high_frequency_agent_job(&every));
    warn_if_high_frequency_agent_job(&every);

    let mut cron = every.clone();
    cron.schedule = Schedule::Cron { expr: "* * * * *".into(), tz: None };
    assert!(is_high_frequency_agent_job(&cron));
    warn_if_high_frequency_agent_job(&cron);

    cron.schedule = Schedule::Cron { expr: "*/5 * * * *".into(), tz: None };
    assert!(!is_high_frequency_agent_job(&cron));

    let mut at = every;
    at.schedule = Schedule::At { at: Utc::now() + ChronoDuration::minutes(10) };
    assert!(!is_high_frequency_agent_job(&at));
    warn_if_high_frequency_agent_job(&at);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_gateway_options_close_acp_client_after_request() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.full_access = true;

    let options = cron_gateway_options(&config, &job, Some("codex"));

    assert_eq!(options["acp_non_interactive_permissions"], "deny");
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(
        options.get("acp_close_client_after_request").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-all")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn wb_schedule_feedback_acp_restricted_options_keep_deny() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.full_access = false;

    let options = cron_gateway_options(&config, &job, Some("codex"));

    assert_eq!(options["acp_non_interactive_permissions"], "deny");
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-reads")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_gateway_options_omits_acp_lifecycle_options_without_acp_agent() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("");
    job.job_type = JobType::Agent;

    let options = cron_gateway_options(&config, &job, None);

    assert!(!options.contains_key("acp_close_client_after_request"));
    assert!(!options.contains_key("acp_agent"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_gateway_options_for_full_access_without_acp_only_enables_full_access() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.full_access = true;

    let options = cron_gateway_options(&config, &job, None);

    assert_eq!(options.len(), 1);
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_gateway_agent_request_includes_session_prompt_agent_model_and_acp_options() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("");
    job.id = "nightly".into();
    job.job_type = JobType::Agent;
    job.agent = Some("  reviewer  ".into());
    job.acp_agent = Some(" codex ".into());

    let (session_id, request) =
        cron_gateway_agent_request(&config, &job, "do the thing", Some("model-a".into()));

    assert_eq!(session_id, "cron-nightly");
    assert_eq!(request.session_id.as_ref().map(AsRef::as_ref), Some("cron-nightly"));
    assert_eq!(request.agent.as_deref(), Some("reviewer"));
    assert_eq!(request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(request.model.as_deref(), Some("model-a"));
    assert_eq!(request.messages[0]["role"], "user");
    assert_eq!(request.messages[0]["content"], "do the thing");
    let options = request.options.as_ref().and_then(serde_json::Value::as_object).unwrap();
    assert_eq!(options.get("acp_agent").and_then(serde_json::Value::as_str), Some("codex"));
    assert_eq!(
        options.get("acp_force_new_session").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        options.get("cwd").and_then(serde_json::Value::as_str),
        Some(config.workspace_dir.to_string_lossy().as_ref())
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn format_gateway_agent_output_trims_output_and_appends_available_message_ids() {
    let with_message =
        session_port::chat_stream::GatewayChatStreamCollected {
            output: "  done  ".into(),
            message_id: Some("assistant-1".into()),
            parent_message_id: Some("parent-ignored".into()),
        };
    assert_eq!(
        format_gateway_agent_output("cron-job-1", &with_message),
        "done\n[session:cron-job-1 message:assistant-1]"
    );

    let with_parent =
        session_port::chat_stream::GatewayChatStreamCollected {
            output: "parent done".into(),
            message_id: None,
            parent_message_id: Some("parent-1".into()),
        };
    assert_eq!(
        format_gateway_agent_output("cron-job-1", &with_parent),
        "parent done\n[session:cron-job-1 parent:parent-1]"
    );

    let without_ids =
        session_port::chat_stream::GatewayChatStreamCollected {
            output: "   ".into(),
            message_id: None,
            parent_message_id: None,
        };
    assert_eq!(
        format_gateway_agent_output("cron-job-1", &without_ids),
        "agent job executed\n[session:cron-job-1]"
    );
}

#[test]
fn one_shot_auto_delete_requires_delete_flag_and_at_schedule() {
    let mut job = test_job("echo ok");
    job.delete_after_run = true;
    assert!(!is_one_shot_auto_delete(&job));

    job.schedule = Schedule::At { at: Utc::now() + ChronoDuration::minutes(10) };
    assert!(is_one_shot_auto_delete(&job));

    job.delete_after_run = false;
    assert!(!is_one_shot_auto_delete(&job));
}

#[test]
fn effective_job_config_uses_project_path_when_present() {
    let tmp = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    let mut job = test_job("echo project");
    let project_dir = tmp.path().join("project-a");
    job.project_path = Some(project_dir.to_string_lossy().to_string());

    let run_config = effective_job_config(&config, &job);
    assert_eq!(run_config.workspace_dir, project_dir);

    job.project_path = Some("   ".into());
    let fallback = effective_job_config(&config, &job);
    assert_eq!(fallback.workspace_dir, config.workspace_dir);
}

#[test]
fn effective_job_type_promotes_prompt_only_shell_jobs_to_agent() {
    let mut job = test_job("");
    job.prompt = Some("  summarize the day  ".into());
    assert_eq!(effective_job_type(&job), JobType::Agent);

    job.prompt = Some("   ".into());
    assert_eq!(effective_job_type(&job), JobType::Shell);

    job.command = "echo shell".into();
    job.prompt = Some("summarize".into());
    assert_eq!(effective_job_type(&job), JobType::Shell);
}

#[test]
fn invalid_job_payload_reports_empty_shell_and_agent_jobs() {
    let mut shell = test_job("   ");
    let shell_error = invalid_job_payload(&shell).unwrap();
    assert!(shell_error.contains("shell command is empty"));

    shell.job_type = JobType::Agent;
    shell.prompt = Some("  ".into());
    let agent_error = invalid_job_payload(&shell).unwrap();
    assert!(agent_error.contains("agent prompt is empty"));
}

#[test]
fn invalid_job_payload_accepts_valid_shell_agent_and_legacy_prompt_jobs() {
    let shell = test_job("echo ok");
    assert!(invalid_job_payload(&shell).is_none());

    let mut agent = test_job("");
    agent.job_type = JobType::Agent;
    agent.prompt = Some("summarize".into());
    assert!(invalid_job_payload(&agent).is_none());

    let mut legacy_prompt = test_job("   ");
    legacy_prompt.prompt = Some("summarize".into());
    assert_eq!(effective_job_type(&legacy_prompt), JobType::Agent);
    assert!(invalid_job_payload(&legacy_prompt).is_none());
}
