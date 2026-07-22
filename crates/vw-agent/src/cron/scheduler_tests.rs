use super::execute_job_now;
use crate::app::agent::config::Config;
use crate::app::agent::cron::{CronJob, DeliveryConfig, JobType, Schedule, SessionTarget};
use chrono::Utc;

fn shell_job(command: &str) -> CronJob {
    CronJob {
        id: "execute-now-security".into(),
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
        next_run: Utc::now(),
        last_run: None,
        last_status: None,
        last_output: None,
    }
}

#[tokio::test]
async fn execute_job_now_preserves_security_rejection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut config = Config {
        workspace_dir: temp.path().join("workspace"),
        config_path: temp.path().join("vibewindow.json"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).expect("create workspace");
    config.autonomy.allowed_commands = vec!["echo".into()];
    let job = shell_job("curl https://example.invalid");

    let (success, output) = execute_job_now(&config, &job).await;

    assert!(!success);
    assert!(output.contains("blocked by security policy"));
    assert!(output.contains("command not allowed"));
}
