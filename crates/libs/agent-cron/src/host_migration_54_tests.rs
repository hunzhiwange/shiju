//! 调度、窗口与持久化领域测试；宿主端口不应被这些场景调用。
use agent_config::runtime::Config;
use agent_security::SecurityPolicy;
use crate as cron;
use crate::{CronJob, DeliveryConfig, JobType, Schedule, SessionTarget};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::sync::Arc;
use tempfile::TempDir;
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

/// 生成唯一的组件标识符
///
/// 用于在测试中创建具有唯一名称的组件，避免并发测试间的标识符冲突。
///
/// # 参数
///
/// - `prefix`: 组件名称前缀
///
/// # 返回值
///
/// 返回格式为 `{prefix}-{uuid}` 的唯一标识符
fn unique_component(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}


struct TestHost;
#[async_trait::async_trait]
impl crate::delivery::DeliveryHost for TestHost {
    #[cfg(not(target_arch = "wasm32"))]
    fn validate_webhook(&self, _: &str) -> anyhow::Result<()> { panic!("unexpected webhook validation") }
    #[cfg(not(target_arch = "wasm32"))]
    async fn latest_config(&self, _: &Config) -> anyhow::Result<Config> { panic!("unexpected configuration reload") }
    #[cfg(not(target_arch = "wasm32"))]
    async fn send_validated(&self, _: &Config, _: &str, _: &str, _: &str) -> anyhow::Result<()> { panic!("unexpected delivery") }
}
#[async_trait::async_trait]
impl crate::scheduler::SchedulerHost for TestHost {
    async fn deliver_agent_job_to_task_pool(&self, _: &Config, _: &CronJob, _: &str, _: String) -> anyhow::Result<String> { panic!("unexpected task pool call") }
    #[cfg(not(target_arch = "wasm32"))]
    async fn run_gateway_agent_job(&self, _: &Config, _: &CronJob, _: &str, _: Option<String>) -> anyhow::Result<String> { panic!("unexpected gateway call") }
    async fn deliver_announcement(&self, config: &Config, channel: &str, target: &str, output: &str) -> anyhow::Result<()> { crate::announcement::deliver_announcement(self, config, channel, target, output).await }
}
#[cfg(test)]
async fn execute_job_with_retry(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
) -> (bool, String) {
    crate::scheduler::execute_job_with_retry(&TestHost, config, security, job).await
}

#[cfg(test)]
async fn process_due_jobs(
    config: &Config,
    security: &Arc<SecurityPolicy>,
    jobs: Vec<CronJob>,
    component: &str,
) {
    crate::scheduler::process_due_jobs(&TestHost, config, security, jobs, component).await
}

#[cfg(test)]
async fn execute_and_persist_job(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
    component: &str,
) -> (String, bool, String) {
    crate::scheduler::execute_and_persist_job(&TestHost, config, security, job, component).await
}

#[cfg(test)]
async fn run_agent_job(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
) -> (bool, String) {
    crate::scheduler::run_agent_job(&TestHost, config, security, job).await
}

#[cfg(test)]
async fn persist_job_result(
    config: &Config,
    job: &CronJob,
    success: bool,
    output: &str,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
) -> bool {
    crate::scheduler::persist_job_result(&TestHost, config, job, success, output, started_at, finished_at).await
}

pub(super) async fn deliver_if_configured(
    config: &Config,
    job: &CronJob,
    output: &str,
) -> anyhow::Result<()> {
    crate::scheduler::deliver_if_configured(&TestHost, config, job, output).await
}

async fn execute_job_now(config: &Config, job: &CronJob) -> (bool, String) { crate::scheduler::execute_job_now(&TestHost, config, job).await }

#[path = "host_migration_54_tests/execution.rs"]
mod execution;
#[path = "host_migration_54_tests/persistence.rs"]
mod persistence;

#[path = "host_migration_54_tests/announcement.rs"]
mod announcement;
