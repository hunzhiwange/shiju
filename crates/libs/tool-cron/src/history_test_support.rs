//! CronRun 工具强制执行测试。
//!
//! 验证手动触发计划任务时的历史记录、缺失任务错误、只读阻止、监督审批和
//! 速率限制，确保计划任务不会绕过通用安全策略。

use crate::{CronRunTool, cron_run::ExecuteJobNow};
use tool_types::Tool;
use std::sync::Arc;
use agent_security::SecurityPolicy;
use agent_config::runtime::Config;
use agent_cron::{add_job, list_runs};
use agent_security::AutonomyLevel;
use serde_json::json;
use tempfile::TempDir;

async fn test_config(tmp: &TempDir) -> Arc<Config> {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    Arc::new(config)
}

fn test_security(cfg: &Config) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::from_config(&cfg.autonomy, &cfg.workspace_dir))
}

pub async fn force_runs_job_and_records_history(execute_job_now: ExecuteJobNow) {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let job = add_job(&cfg, "*/5 * * * *", "echo run-now").unwrap();
    let tool = CronRunTool::new(cfg.clone(), test_security(&cfg), execute_job_now);

    let result = tool.execute(json!({ "job_id": job.id })).await.unwrap();
    assert!(result.success, "{:?}", result.error);

    let runs = list_runs(&cfg, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
}

pub async fn shell_run_requires_approval_for_medium_risk(execute_job_now: ExecuteJobNow) {
    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Supervised;
    config.autonomy.require_approval_for_medium_risk = true;
    config.autonomy.allowed_commands = vec!["touch".into()];
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let job = add_job(&cfg, "*/5 * * * *", "touch cron-run-approval").unwrap();
    let tool = CronRunTool::new(cfg.clone(), test_security(&cfg), execute_job_now);

    let denied = tool.execute(json!({ "job_id": job.id })).await.unwrap();
    assert!(!denied.success);
    assert!(denied.error.unwrap_or_default().contains("explicit approval"));

    // 显式 approved 标志模拟审批完成后的二次调用，验证安全门槛仍由工具执行。
    let approved = tool.execute(json!({ "job_id": job.id, "approved": true })).await.unwrap();
    assert!(approved.success, "{:?}", approved.error);
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn cron_history_manual_run_is_visible_until_completion(execute_job_now: ExecuteJobNow) {
    use agent_cron::active_runs::snapshot;
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await.as_ref().clone();
    config.autonomy.allowed_commands.push("curl".into());
    config.autonomy.block_high_risk_commands = false;
    let (listener, command) = shell_barrier().await;
    let job = agent_cron::add_job(&config, "*/5 * * * *", &command).unwrap();
    let config = Arc::new(config);
    let tool = CronRunTool::new(config.clone(), test_security(&config), execute_job_now);
    let execution = tool.execute(json!({"job_id": job.id, "approved": true}));
    let mut execution = Box::pin(execution);
    let stream = tokio::select! {
        accepted = listener.accept() => accepted.unwrap().0,
        _ = &mut execution => panic!("execution ended before reaching the shell barrier"),
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => panic!("shell did not start"),
    };
    let active = snapshot(&config);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].job_id, job.id);
    assert!(agent_cron::list_runs(&config, &job.id, 10).unwrap().is_empty());
    release_shell(stream).await;
    assert!(execution.await.unwrap().success);
    assert!(snapshot(&config).is_empty());
    let runs = agent_cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].started_at, active[0].started_at);
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn cron_history_cancelled_manual_run_leaves_no_history(execute_job_now: ExecuteJobNow) {
    use agent_cron::active_runs::snapshot;
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await.as_ref().clone();
    config.autonomy.allowed_commands.push("curl".into());
    config.autonomy.block_high_risk_commands = false;
    let (listener, command) = shell_barrier().await;
    let job = add_job(&config, "*/5 * * * *", &command).unwrap();
    let config = Arc::new(config);
    let tool = CronRunTool::new(config.clone(), test_security(&config), execute_job_now);
    let mut execution = Box::pin(tool.execute(json!({"job_id": job.id, "approved": true})));
    let _stream = tokio::select! {
        accepted = listener.accept() => accepted.unwrap().0,
        _ = &mut execution => panic!("execution ended before the shell barrier"),
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => panic!("shell did not start"),
    };
    assert_eq!(snapshot(&config).len(), 1);
    drop(execution);
    assert!(snapshot(&config).is_empty());
    assert!(list_runs(&config, &job.id, 10).unwrap().is_empty());
}

// A local HTTP connection blocks the real shell command until the test releases it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn shell_barrier() -> (tokio::net::TcpListener, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let command = format!(
        "curl --noproxy '*' --silent --max-time 10 http://{}/",
        listener.local_addr().unwrap()
    );
    (listener, command)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn release_shell(mut stream: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut request = [0; 4096];
    stream.read(&mut request).await.unwrap();
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        .await
        .unwrap();
}
