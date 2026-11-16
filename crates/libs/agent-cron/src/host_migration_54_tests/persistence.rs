use super::*;

/// 测试一次性 Shell 任务失败后被禁用
///
/// 验证 Shell 类型一次性任务失败时的处理：
/// - 与 Agent 类型一致，失败后禁用而非删除
/// - 保留任务记录以便排障
#[tokio::test]
async fn persist_job_result_failure_disables_one_shot_shell_job() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let at = Utc::now() + ChronoDuration::minutes(10);
    let job = cron::add_once_at(&config, at, "echo one-shot-shell").unwrap();
    assert!(job.delete_after_run);
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    let success = persist_job_result(&config, &job, false, "boom", started, finished).await;
    assert!(!success);
    let updated = cron::get_job(&config, &job.id).unwrap();
    assert!(!updated.enabled);
    assert_eq!(updated.last_status.as_deref(), Some("error"));
}

/// 测试 At 调度类型且 delete_after_run=false 时不会被删除
///
/// 验证 Schedule::At 与 delete_after_run 的独立性：
/// - At 调度类型本身不会自动设置 delete_after_run
/// - 只有显式设置 delete_after_run=true 才会在成功后删除
/// - 允许创建定期执行但不自动删除的 At 类型任务
#[tokio::test]
async fn persist_job_result_at_schedule_without_delete_after_run_is_not_deleted() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let at = Utc::now() + ChronoDuration::minutes(10);
    // 创建 At 类型任务，但 delete_after_run 为 false
    let job = cron::add_agent_job(
        &config,
        Some("at-no-autodelete".into()),
        Schedule::At { at },
        "Hello",
        SessionTarget::Isolated,
        None,
        None,
        false,
    )
    .unwrap();
    assert!(!job.delete_after_run);

    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);
    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(success);

    // 任务仍然存在且保持启用
    let updated = cron::get_job(&config, &job.id).unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.last_status.as_deref(), Some("ok"));
}

#[tokio::test]
async fn execute_and_persist_job_disables_invalid_shell_job() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let job = cron::add_job(&config, "*/5 * * * *", "echo initial").unwrap();
    let job = cron::update_job(
        &config,
        &job.id,
        cron::CronJobPatch { command: Some("   ".into()), ..cron::CronJobPatch::default() },
    )
    .unwrap();
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (job_id, success, output) =
        execute_and_persist_job(&config, &security, &job, "scheduler-invalid").await;

    assert_eq!(job_id, job.id);
    assert!(!success);
    assert!(output.contains("shell command is empty"));

    let stored = cron::get_job(&config, &job.id).unwrap();
    assert!(!stored.enabled);
    assert_eq!(stored.last_status.as_deref(), Some("error"));

    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "error");
}

#[tokio::test]
async fn deliver_if_configured_requires_channel_and_target_for_announce() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("echo ok");

    job.delivery = DeliveryConfig {
        mode: "announce".into(),
        channel: None,
        to: Some("target".into()),
        best_effort: true,
    };
    let missing_channel = deliver_if_configured(&config, &job, "output").await.unwrap_err();
    assert!(missing_channel.to_string().contains("delivery.channel is required"));

    job.delivery = DeliveryConfig {
        mode: "announce".into(),
        channel: Some("telegram".into()),
        to: None,
        best_effort: true,
    };
    let missing_target = deliver_if_configured(&config, &job, "output").await.unwrap_err();
    assert!(missing_target.to_string().contains("delivery.to is required"));
}

#[tokio::test]
async fn run_agent_job_rejects_missing_delegate_agent_before_provider_call() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Say hello".into());
    job.agent = Some("unknown-agent".into());
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("delegate agent 'unknown-agent' not found"));
}

#[tokio::test]
async fn run_agent_job_rejects_disabled_delegate_agent_before_provider_call() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.agents.insert(
        "disabled-agent".into(),
        agent_config::runtime::agent::DelegateAgentConfig { enabled: false, ..agent_config::runtime::agent::DelegateAgentConfig::default() },
    );
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Say hello".into());
    job.agent = Some("disabled-agent".into());
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("delegate agent 'disabled-agent' is disabled"));
}

#[tokio::test]
async fn wb_schedule_window_blocks_expired_manual_run() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("echo window-command-executed");
    job.ends_at = Some(Utc::now() - ChronoDuration::seconds(1));
    let (success, output) = execute_job_now(&config, &job).await;
    assert!(!success);
    assert!(output.contains("window"), "{output}");
    assert!(!output.contains("window-command-executed"));
}

#[tokio::test]
async fn wb_schedule_window_blocks_future_manual_run() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("echo window-command-executed");
    job.starts_at = Some(Utc::now() + ChronoDuration::hours(1));
    let (success, output) = execute_job_now(&config, &job).await;
    assert!(!success);
    assert!(output.contains("window"), "{output}");
}

#[tokio::test]
async fn wb_schedule_window_queued_expiration_never_starts_shell() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.security.bash.enabled = true;
    config.autonomy.allowed_commands = vec!["touch".into()];
    let marker = config.workspace_dir.join("should-not-exist");
    let job = cron::add_job(&config, "0 * * * * *", "touch should-not-exist").unwrap();
    let (success, output) = execute_job_now(&config, &job).await;
    assert!(success, "{output}");
    assert!(marker.exists());
    std::fs::remove_file(&marker).unwrap();
    let mut queued = job.clone();
    queued.ends_at = Some(Utc::now() - ChronoDuration::seconds(1));
    // Persist the new window while retaining the old queued snapshot.
    cron::update_job(
        &config,
        &job.id,
        cron::CronJobPatch { ends_at: Some(queued.ends_at), ..Default::default() },
    )
    .unwrap();
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);
    let (_, success, output) =
        execute_and_persist_job(&config, &security, &job, "window-test").await;
    assert!(!success);
    assert!(output.contains("window"), "{output}");
    assert!(!marker.exists());
    assert!(!cron::get_job(&config, &job.id).unwrap().enabled);
    assert!(cron::list_runs(&config, &job.id, 10).unwrap().is_empty());
}

#[tokio::test]
async fn wb_schedule_acceptance_manual_cron_run_preserves_next_run() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let now = Utc::now();
    let end = now + ChronoDuration::days(30);
    for schedule in [
        Schedule::Every { every_ms: 60_000 },
        Schedule::At { at: now + ChronoDuration::hours(1) },
        Schedule::Cron { expr: "30 9 * * *".into(), tz: Some("Asia/Shanghai".into()) },
    ] {
        let job = cron::add_shell_job_with_window(
            &config,
            Some("立即运行验收".into()),
            schedule,
            "echo acceptance-manual",
            None,
            Some(end),
        )
        .unwrap();
        let (success, output) = execute_job_now(&config, &job).await;
        assert!(success, "{output}");
        assert!(output.contains("acceptance-manual"));
        let read = cron::get_job(&config, &job.id).unwrap();
        assert_eq!(read.schedule, job.schedule);
        assert_eq!(read.next_run, job.next_run);
        assert_eq!(read.ends_at, Some(end));
        assert!(read.enabled);
    }
}
