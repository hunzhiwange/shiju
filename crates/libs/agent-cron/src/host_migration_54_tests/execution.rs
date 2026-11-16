use super::*;
#[tokio::test]
async fn run_job_command_uses_job_project_path_as_working_directory() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands = vec!["pwd".into()];
    let project_dir = tmp.path().join("project-dir");
    tokio::fs::create_dir_all(&project_dir).await.unwrap();
    let mut job = test_job("pwd");
    job.project_path = Some(project_dir.to_string_lossy().to_string());
    let security = SecurityPolicy::from_config(&config.autonomy, &project_dir);

    let (success, output) = execute_job_with_retry(&config, &security, &job).await;
    assert!(success);
    assert!(output.contains(&project_dir.to_string_lossy().to_string()));
}

/// 测试 Shell 命令执行失败
///
/// 验证命令执行失败时的处理：
/// - 命令失败时返回 false
/// - 输出中包含错误信息
/// - 状态码表示失败（非零）

/// 测试任务重试机制在首次失败后恢复
///
/// 验证 scheduler_retries 配置：
/// - 任务在失败后会被重试
/// - 重试时使用退避延迟（provider_backoff_ms）
/// - 成功的重试会被正确识别并返回成功
#[tokio::test]
async fn execute_job_with_retry_recovers_after_first_failure() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.reliability.scheduler_retries = 1;
    config.reliability.provider_backoff_ms = 1;
    config.autonomy.allowed_commands = vec!["sh".into()];
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    // 创建一个脚本，首次执行失败，第二次执行成功
    tokio::fs::write(
        config.workspace_dir.join("retry-once.sh"),
        "#!/bin/sh\nif [ -f retry-ok.flag ]; then\n  echo recovered\n  exit 0\nfi\ntouch retry-ok.flag\nexit 1\n",
    )
    .await
    .unwrap();
    let job = test_job("sh ./retry-once.sh");

    let (success, output) = execute_job_with_retry(&config, &security, &job).await;
    assert!(success);
    assert!(output.contains("recovered"));
}

/// 测试任务重试机制耗尽所有尝试
///
/// 验证当所有重试都失败时的处理：
/// - 重试次数耗尽后返回失败
/// - 输出中包含失败信息
#[tokio::test]
async fn execute_job_with_retry_exhausts_attempts() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.reliability.scheduler_retries = 1;
    config.reliability.provider_backoff_ms = 1;
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let job = test_job("ls always_missing_for_retry_test");

    let (success, output) = execute_job_with_retry(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("always_missing_for_retry_test"));
}

/// 测试 Agent 任务在只读模式下被阻止
///
/// 验证只读模式同样适用于 Agent 任务：
/// - AutonomyLevel::ReadOnly 会阻止 Agent 任务执行
/// - 返回明确的只读模式错误
#[tokio::test]
async fn wb_schedule_feedback_security_rejects_readonly_agent() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.level = agent_security::AutonomyLevel::ReadOnly;
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Say hello".into());
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("read-only"));
}

/// 测试 Agent 任务在速率限制下被阻止
///
/// 验证速率限制同样适用于 Agent 任务：
/// - max_actions_per_hour 为 0 时，Agent 任务被阻止
/// - 返回速率限制错误
#[tokio::test]
async fn run_agent_job_blocks_rate_limited() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.max_actions_per_hour = 0;
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Say hello".into());
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("rate limit exceeded"));
}

/// 测试 process_due_jobs 在空闲时仍标记组件健康
///
/// 验证调度器组件健康状态管理：
/// - 即使没有待处理任务，也会将组件标记为健康
/// - 清除之前存在的错误状态
/// - 记录健康检查时间戳
#[tokio::test]
async fn process_due_jobs_marks_component_ok_even_when_idle() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    let component = unique_component("scheduler-idle");

    // 先标记组件为错误状态
    agent_health::mark_component_error(&component, "pre-existing error");
    // 处理空任务列表
    process_due_jobs(&config, &security, Vec::new(), &component).await;

    let snapshot = agent_health::snapshot_json();
    let entry = &snapshot["components"][component.as_str()];
    assert_eq!(entry["status"], "ok");
    assert!(entry["last_ok"].as_str().is_some());
    assert!(entry["last_error"].is_null());
}

/// 测试 process_due_jobs 任务失败不影响组件健康状态
///
/// 验证组件健康状态与单个任务执行结果的独立性：
/// - 单个任务执行失败不会将整个调度器组件标记为不健康
/// - 这允许调度器继续处理后续任务
#[tokio::test]
async fn process_due_jobs_failure_does_not_mark_component_unhealthy() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let job = test_job("ls definitely_missing_file_for_scheduler_component_health_test");
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));
    let component = unique_component("scheduler-fail");

    // 先标记组件为健康状态
    agent_health::mark_component_ok(&component);
    // 处理会失败的任务
    process_due_jobs(&config, &security, vec![job], &component).await;

    let snapshot = agent_health::snapshot_json();
    let entry = &snapshot["components"][component.as_str()];
    assert_eq!(entry["status"], "ok");
}

/// 测试任务结果持久化并重新调度 Shell 任务
///
/// 验证 persist_job_result 函数的核心功能：
/// - 记录任务执行运行历史
/// - 更新任务的最后状态和输出
/// - 为周期性任务计算下次运行时间
#[tokio::test]
async fn persist_job_result_records_run_and_reschedules_shell_job() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let job = cron::add_job(&config, "*/5 * * * *", "echo ok").unwrap();
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(success);

    // 验证运行记录被保存
    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    // 验证任务状态被更新
    let updated = cron::get_job(&config, &job.id).unwrap();
    assert_eq!(updated.last_status.as_deref(), Some("ok"));
}

/// 测试一次性任务成功后被删除
///
/// 验证 delete_after_run 标志的功能：
/// - Agent 类型的一次性任务在成功执行后会被自动删除
/// - 使用 Schedule::At 指定一次性执行时间
#[tokio::test]
async fn persist_job_result_success_deletes_one_shot() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let at = Utc::now() + ChronoDuration::minutes(10);
    // 创建一个一次性 Agent 任务，delete_after_run 为 true
    let job = cron::add_agent_job(
        &config,
        Some("one-shot".into()),
        Schedule::At { at },
        "Hello",
        SessionTarget::Isolated,
        None,
        None,
        true,
    )
    .unwrap();
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(success);
    // 验证任务已被删除
    let lookup = cron::get_job(&config, &job.id);
    assert!(lookup.is_err());
}

/// 测试一次性任务失败后被禁用
///
/// 验证一次性任务失败时的处理：
/// - 任务失败后不会被删除，而是被禁用
/// - 最后状态被设置为 "error"
/// - 允许管理员检查失败原因后决定是否重新启用
#[tokio::test]
async fn persist_job_result_failure_disables_one_shot() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let at = Utc::now() + ChronoDuration::minutes(10);
    let job = cron::add_agent_job(
        &config,
        Some("one-shot".into()),
        Schedule::At { at },
        "Hello",
        SessionTarget::Isolated,
        None,
        None,
        true,
    )
    .unwrap();
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    let success = persist_job_result(&config, &job, false, "boom", started, finished).await;
    assert!(!success);
    // 验证任务被禁用但未删除
    let updated = cron::get_job(&config, &job.id).unwrap();
    assert!(!updated.enabled);
    assert_eq!(updated.last_status.as_deref(), Some("error"));
}

/// 测试一次性 Shell 任务成功后被删除
///
/// 验证 add_once_at 创建的任务在成功后被删除：
/// - Shell 类型的一次性任务同样支持自动删除
/// - delete_after_run 标志默认为 true
#[tokio::test]
async fn persist_job_result_success_deletes_one_shot_shell_job() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let at = Utc::now() + ChronoDuration::minutes(10);
    let job = cron::add_once_at(&config, at, "echo one-shot-shell").unwrap();
    assert!(job.delete_after_run);
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(success);
    // 验证任务已被删除
    let lookup = cron::get_job(&config, &job.id);
    assert!(lookup.is_err());
}

#[test]
fn task_pool_agents_trim_names_and_default_empty_main_agent() {
    let mut job = test_job("");
    job.agent = Some(" planner ".into());
    job.acp_agent = Some(" codex ".into());
    let agents = crate::execution::task_pool_agents(&job);
    assert_eq!(agents.agent, "planner");
    assert_eq!(agents.acp_agent.as_deref(), Some("codex"));
    job.agent = Some("  ".into());
    job.acp_agent = Some("  ".into());
    let agents = crate::execution::task_pool_agents(&job);
    assert_eq!(agents.agent, "main");
    assert_eq!(agents.acp_agent, None);
}
