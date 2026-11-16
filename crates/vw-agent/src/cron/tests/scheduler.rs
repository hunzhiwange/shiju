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
    deliver_if_configured, persist_job_result,
    process_due_jobs, run_agent_job,
};
use crate::app::agent::cron::{CronJob, DeliveryConfig, JobType, Schedule, SessionTarget};
use crate::app::agent::security::SecurityPolicy;
use chrono::Utc;
use std::sync::Arc;
use std::sync::OnceLock;
use tempfile::TempDir;
use tokio::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use tokio::{
    net::TcpListener,
};
use config_agent::DelegateAgentConfig;

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
