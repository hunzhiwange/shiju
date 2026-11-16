//! 定时任务真实宿主集成测试（纯领域测试位于 agent-cron）。
//!
//! 本模块提供了调度器核心功能的单元测试和集成测试，包括：
//! - Shell 命令执行与安全策略验证
//! - Agent 任务执行与权限检查
//! - 任务重试机制
//! - 任务结果持久化
//! - 投递配置处理
//! - 组件健康状态标记
//!
//! 所有测试都遵循 VibeWindow 的安全默认策略，确保定时任务在受控环境下运行。

use crate::app::agent::config::Config;
use crate::app::agent::cron;
use crate::app::agent::cron::scheduler::{
    deliver_announcement, deliver_if_configured, execute_and_persist_job, persist_job_result,
    process_due_jobs, run_agent_job,
};
use crate::app::agent::cron::{CronJob, DeliveryConfig, JobType, Schedule, SessionTarget};
use crate::app::agent::security::SecurityPolicy;
use chrono::{Duration as ChronoDuration, Utc};
use std::sync::Arc;
use std::sync::OnceLock;
use tempfile::TempDir;
use tokio::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use config_agent::DelegateAgentConfig;
#[cfg(not(target_arch = "wasm32"))]
use config_channel_types::MattermostConfig;

/// 获取测试环境全局锁
///
/// 用于确保涉及环境变量修改的测试不会并发执行，避免测试间的状态干扰。
/// 这是一个静态的 Tokio 互斥锁，在首次调用时初始化。
///
/// # 返回值
///
/// 返回互斥锁的守卫，在守卫生命周期内持有锁
async fn env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(())).lock().await
}

/// 环境变量守卫
///
/// 用于在测试中临时操作环境变量，并在测试结束后自动恢复原始状态。
/// 这确保了测试的隔离性和可重复性。
///
/// # 字段说明
///
/// - `key`: 环境变量名称
/// - `original`: 原始环境变量值（如果存在）
struct EnvGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvGuard {
    /// 创建一个环境变量守卫并清除指定变量
    ///
    /// 该方法会保存原始值并在守卫被丢弃时恢复。
    ///
    /// # 参数
    ///
    /// - `key`: 要清除的环境变量名称
    ///
    /// # 返回值
    ///
    /// 返回一个 `EnvGuard` 实例，它会记住原始值以便恢复
    ///
    /// # 安全性
    ///
    /// 使用 `unsafe` 块是因为在多线程环境中修改环境变量不是线程安全的，
    /// 但在测试场景中配合 `env_lock()` 使用是可接受的。
    fn unset(key: &'static str) -> Self {
        let original = std::env::var(key).ok();
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    /// 析构时恢复环境变量原始状态
    ///
    /// 如果原始值存在则恢复，否则清除该环境变量。
    fn drop(&mut self) {
        match self.original.as_ref() {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}

/// 创建测试用的配置对象
///
/// 为每个测试生成独立的临时工作空间，确保测试之间完全隔离。
///
/// # 参数
///
/// - `tmp`: 临时目录引用
///
/// # 返回值
///
/// 返回配置好的 `Config` 实例
async fn test_config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    config
}

/// 创建测试用的定时任务对象
///
/// 生成一个标准的测试任务，每分钟执行一次指定的命令。
///
/// # 参数
///
/// - `command`: 要执行的 shell 命令
///
/// # 返回值
///
/// 返回配置好的 `CronJob` 实例
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


/// 测试 Agent 任务在没有 API 密钥时返回错误
///
/// 验证 Agent 类型任务对 API 密钥的依赖：
/// - 没有配置任何 API 密钥时，Agent 任务会失败
/// - 使用环境变量守卫确保测试隔离
#[tokio::test]
async fn run_agent_job_returns_error_without_provider_key() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let _env = env_lock().await;
    let _generic = EnvGuard::unset("VIBEWINDOW_API_KEY");
    let _fallback = EnvGuard::unset("API_KEY");
    let _openrouter = EnvGuard::unset("OPENROUTER_API_KEY");
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Say hello".into());
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("agent job failed:"));
}

#[tokio::test]
async fn full_access_agent_job_bypasses_readonly_security_gate() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.level = crate::app::agent::security::AutonomyLevel::ReadOnly;
    let _env = env_lock().await;
    let _generic = EnvGuard::unset("VIBEWINDOW_API_KEY");
    let _fallback = EnvGuard::unset("API_KEY");
    let _openrouter = EnvGuard::unset("OPENROUTER_API_KEY");
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Say hello".into());
    job.full_access = true;
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;
    assert!(!success);
    assert!(output.contains("agent job failed:"));
    assert!(!output.contains("read-only"));
}

#[tokio::test]
async fn run_agent_job_delivers_task_pool_jobs_with_trimmed_agent_and_default_model() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.agents.insert(
        "planner".into(),
        DelegateAgentConfig { enabled: true, ..DelegateAgentConfig::default() },
    );
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Create the report".into());
    job.name = Some("Daily".into());
    job.agent = Some(" planner ".into());
    job.acp_agent = Some(" codex ".into());
    job.model = Some("   ".into());
    job.task_pool = true;
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;

    assert!(success);
    assert!(output.contains("agent job delivered to task pool:"));
    assert!(
        config
            .workspace_dir
            .join(config_runtime_paths::HOME_CONFIG_DIR_NAME)
            .join("tasks/_index.sqlite3")
            .is_file()
    );
}

#[tokio::test]
async fn run_agent_job_reports_task_pool_delivery_failure_without_mutating_security_result() {
    let tmp = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: tmp.path().join("workspace-file"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    tokio::fs::write(&config.workspace_dir, "not a directory").await.unwrap();
    let mut job = test_job("");
    job.job_type = JobType::Agent;
    job.prompt = Some("Create the report".into());
    job.task_pool = true;
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);

    let (success, output) = run_agent_job(&config, &security, &job).await;

    assert!(!success);
    assert!(output.contains("agent task pool delivery failed"));
}

/// 测试投递失败在非尽力模式下标记任务错误
///
/// 验证 DeliveryConfig.best_effort=false 的行为：
/// - 任务本身成功但投递失败时，整体标记为失败
/// - 任务保持启用状态以便重试
/// - 运行记录中状态为 "error"
#[tokio::test]
async fn wb_schedule_feedback_persist_job_result_delivery_failure_non_best_effort_marks_error() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    // 创建带有投递配置的任务，best_effort 为 false
    let job = cron::add_agent_job(
        &config,
        Some("announce-job".into()),
        Schedule::Cron { expr: "*/5 * * * *".into(), tz: None },
        "deliver this",
        SessionTarget::Isolated,
        None,
        Some(DeliveryConfig {
            mode: "announce".into(),
            channel: Some("telegram".into()),
            to: Some("123456".into()),
            best_effort: false,
        }),
        false,
    )
    .unwrap();
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    // 由于没有配置 Telegram，投递会失败，且 best_effort=false
    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(!success);

    let updated = cron::get_job(&config, &job.id).unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.last_status.as_deref(), Some("error"));

    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "error");
    assert!(runs[0].output.as_deref().unwrap().contains("telegram channel not configured"));
}

/// 测试投递失败在尽力模式下保持成功
///
/// 验证 DeliveryConfig.best_effort=true 的行为：
/// - 任务本身成功时，投递失败不影响整体结果
/// - 任务状态保持为成功
/// - 适用于非关键通知场景
#[tokio::test]
async fn wb_schedule_feedback_persist_job_result_delivery_failure_best_effort_keeps_success() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    // 创建带有投递配置的任务，best_effort 为 true
    let job = cron::add_agent_job(
        &config,
        Some("announce-job-best-effort".into()),
        Schedule::Cron { expr: "*/5 * * * *".into(), tz: None },
        "deliver this",
        SessionTarget::Isolated,
        None,
        Some(DeliveryConfig {
            mode: "announce".into(),
            channel: Some("telegram".into()),
            to: Some("123456".into()),
            best_effort: true,
        }),
        false,
    )
    .unwrap();
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(success);

    let updated = cron::get_job(&config, &job.id).unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.last_status.as_deref(), Some("ok"));

    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "ok");
    assert!(runs[0].output.as_deref().unwrap().contains("telegram channel not configured"));
}

/// 测试 deliver_if_configured 处理无配置和无效通道
///
/// 验证投递函数的健壮性：
/// - 无投递配置时返回 Ok
/// - 无效通道时返回错误
/// - 错误消息清晰指明原因
#[tokio::test]
async fn deliver_if_configured_handles_none_and_invalid_channel() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("echo ok");

    // 无投递配置时应该成功
    assert!(deliver_if_configured(&config, &job, "x").await.is_ok());

    // 配置无效通道时应该失败
    job.delivery = DeliveryConfig {
        mode: "announce".into(),
        channel: Some("invalid".into()),
        to: Some("target".into()),
        best_effort: true,
    };
    let err = deliver_if_configured(&config, &job, "x").await.unwrap_err();
    assert!(err.to_string().contains("unsupported delivery channel"));
}

#[tokio::test]
async fn wb_schedule_feedback_rejects_each_unconfigured_known_channel() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let cases = [
        ("TELEGRAM", "telegram channel not configured"),
        ("discord", "discord channel not configured"),
        ("slack", "slack channel not configured"),
        ("mattermost", "mattermost channel not configured"),
        ("qq", "qq channel not configured"),
        ("email", "email channel not configured"),
    ];

    for (channel, expected) in cases {
        let err = deliver_announcement(&config, channel, "target", "output").await.unwrap_err();
        assert!(err.to_string().contains(expected), "unexpected error for {channel}: {err}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn deliver_announcement_sends_configured_mattermost_thread_message() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    let (base_url, request) = spawn_http_server("201 Created", r#"{"id":"post-1"}"#).await;
    config.channels_config.mattermost = Some(MattermostConfig {
        url: base_url,
        bot_token: "mattermost-token".into(),
        channel_id: Some("configured-channel".into()),
        allowed_users: vec!["*".into()],
        thread_replies: Some(false),
        mention_only: Some(true),
        group_reply: None,
    });

    deliver_announcement(&config, "MatterMost", "channel-a:root-b", "hello mattermost")
        .await
        .unwrap();
    let request = request.await.unwrap();

    assert!(request.starts_with("POST /api/v4/posts HTTP/1.1"));
    assert!(request.contains("authorization: Bearer mattermost-token"));
    assert!(request.contains(r#""channel_id":"channel-a""#));
    assert!(request.contains(r#""root_id":"root-b""#));
    assert!(request.contains(r#""message":"hello mattermost""#));
}

#[cfg(not(target_arch = "wasm32"))]
async fn spawn_http_server(status: &str, body: &str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_string();
    let body = body.to_string();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if http_request_is_complete(&request) {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&request).to_string()
    });

    (format!("http://{address}"), handle)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_request_is_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let body_start = header_end + 4;
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    request.len() >= body_start + content_length
}

#[tokio::test]
async fn deliver_if_configured_treats_announce_mode_case_insensitively() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("echo ok");
    job.delivery = DeliveryConfig {
        mode: "AnNoUnCe".into(),
        channel: Some("invalid".into()),
        to: Some("target".into()),
        best_effort: true,
    };

    let err = deliver_if_configured(&config, &job, "output").await.unwrap_err();
    assert!(err.to_string().contains("unsupported delivery channel"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_history_scheduled_run_is_visible_until_completion() {
    use crate::app::agent::cron::{
        active_runs::snapshot,
        active_runs_tests::{release_shell, shell_barrier},
    };
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands.push("curl".into());
    config.autonomy.block_high_risk_commands = false;
    let (listener, command) = shell_barrier().await;
    let delivery_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    config.channels_config.mattermost = Some(MattermostConfig {
        url: format!("http://{}", delivery_listener.local_addr().unwrap()),
        bot_token: "test-only".into(),
        channel_id: None,
        allowed_users: vec![],
        thread_replies: None,
        mention_only: None,
        group_reply: None,
    });
    let job = crate::app::agent::cron::add_job(&config, "*/5 * * * *", &command).unwrap();
    let job = cron::update_job(
        &config,
        &job.id,
        cron::CronJobPatch {
            delivery: Some(DeliveryConfig {
                mode: "announce".into(),
                channel: Some("mattermost".into()),
                to: Some("test-channel".into()),
                best_effort: false,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);
    let execution = execute_and_persist_job(&config, &security, &job, "active-run-test");
    let mut execution = Box::pin(execution);
    let stream = tokio::select! {
        accepted = listener.accept() => accepted.unwrap().0,
        _ = &mut execution => panic!("execution ended before reaching the shell barrier"),
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => panic!("shell did not start"),
    };
    let active = snapshot(&config);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].job_id, job.id);
    assert!(crate::app::agent::cron::list_runs(&config, &job.id, 10).unwrap().is_empty());
    release_shell(stream).await;
    let mut delivery_stream = tokio::select! {
        accepted = delivery_listener.accept() => accepted.unwrap().0,
        _ = &mut execution => panic!("execution ended before delivery"),
        _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("delivery did not start"),
    };
    assert_eq!(snapshot(&config), active);
    assert!(cron::list_runs(&config, &job.id, 10).unwrap().is_empty());
    let mut request = Vec::new();
    let mut buffer = [0; 1024];
    tokio::select! {
        _ = async {
            while !http_request_is_complete(&request) {
                let read = delivery_stream.read(&mut buffer).await.unwrap();
                assert_ne!(read, 0);
                request.extend_from_slice(&buffer[..read]);
            }
        } => {},
        _ = &mut execution => panic!("execution ended before delivery response"),
        _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("delivery request did not arrive"),
    }
    delivery_stream.write_all(b"HTTP/1.1 201 Created\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"id\":\"post-1\"}").await.unwrap();
    assert!(execution.await.1);
    assert!(snapshot(&config).is_empty());
    let runs = crate::app::agent::cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].started_at, active[0].started_at);
}

#[tokio::test]
async fn wb_schedule_feedback_unavailable_credentials_rejected() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.channels_config.unavailable_channels.insert("slack:default".into());
    let err = deliver_announcement(&config, "slack", "target", "report").await.unwrap_err();
    assert!(err.to_string().contains("credentials unavailable"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_wecom_delivery_none_makes_zero_webhook_requests() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let job =
        cron::add_shell_job(&config, None, Schedule::Every { every_ms: 60_000 }, "echo result")
            .unwrap();
    let mut job = job;
    job.delivery = DeliveryConfig {
        mode: "none".into(),
        channel: Some("wecom_webhook".into()),
        to: Some("default".into()),
        best_effort: false,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap()).parse().unwrap();
    crate::cron::delivery::TEST_WEBHOOK_ENDPOINT
        .scope(endpoint, async {
            deliver_if_configured(&config, &job, "result").await.unwrap();
        })
        .await;
    assert!(tokio::time::timeout(Duration::from_millis(100), listener.accept()).await.is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_wecom_delivery_manual_and_due_shell_trigger_share_route_and_receipts() {
    let tenant =
        vw_api_transport::TenantScope::new("cron-trigger-test", uuid::Uuid::new_v4().to_string())
            .unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let tmp = TempDir::new().unwrap();
        let mut config = test_config(&tmp).await;
        config.channels_config.wecom_webhook = Some(serde_json::from_value(serde_json::json!({"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture"})).unwrap());
        crate::config::schema::save_config_for_tenant(&config, &tenant).await.unwrap();
        let job = cron::add_shell_job(&config, None, Schedule::Every { every_ms:1 }, "echo delivery-result").unwrap();
        let job = cron::update_job(&config, &job.id, cron::CronJobPatch { delivery:Some(DeliveryConfig { mode:"announce".into(), channel:Some("wecom_webhook".into()), to:Some("default".into()), best_effort:false }), ..Default::default() }).unwrap();
        let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let captured = requests.clone();
        let app = axum::Router::new().route("/", axum::routing::post(move |axum::Json(body):axum::Json<serde_json::Value>| {
            captured.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                assert!(body["markdown"]["content"].as_str().unwrap().contains("delivery-result"));
                axum::Json(serde_json::json!({"errcode":0}))
            }
        }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/",listener.local_addr().unwrap()).parse().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener,app).await.unwrap(); });
        crate::cron::delivery::TEST_WEBHOOK_ENDPOINT.scope(endpoint, async {
            let (ok, output) = cron::scheduler::execute_job_now(&config,&job).await;
            assert!(ok, "{output}");
            assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 1);
            assert_eq!(cron::get_job(&config,&job.id).unwrap().next_run,job.next_run);
            let security = Arc::new(SecurityPolicy::from_configs(&config.autonomy,&config.security,&config.workspace_dir));
            process_due_jobs(&config,&security,vec![job.clone()],"cron-wecom-test").await;
            assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 2);
            let runs = cron::list_runs(&config,&job.id,10).unwrap();
            assert_eq!(runs.len(),1);
            assert_eq!(runs[0].status,"ok");
        }).await;
        server.abort(); let _ = server.await;
    }).await;
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn persist_job_result_delivery_failure_wecom_respects_best_effort_for_shell_and_agent() {
    let tenant = vw_api_transport::TenantScope::new(
        "cron-best-effort-test",
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp).await;
        crate::config::schema::save_config_for_tenant(&config, &tenant).await.unwrap();
        for job_type in [JobType::Shell, JobType::Agent] {
            for best_effort in [false, true] {
                let job = cron::add_shell_job(
                    &config,
                    None,
                    Schedule::Every { every_ms: 60_000 },
                    "echo result",
                )
                .unwrap();
                let mut job = job;
                job.job_type = job_type.clone();
                job.delivery = DeliveryConfig {
                    mode: "announce".into(),
                    channel: Some("wecom".into()),
                    to: Some("user".into()),
                    best_effort,
                };
                let now = Utc::now();
                assert_eq!(
                    persist_job_result(&config, &job, true, "result", now, now).await,
                    best_effort
                );
                let runs = cron::list_runs(&config, &job.id, 10).unwrap();
                assert_eq!(runs[0].status, if best_effort { "ok" } else { "error" });
                assert!(runs[0].output.as_deref().unwrap().contains("Bot ID"));
            }
        }
    })
    .await;
}
