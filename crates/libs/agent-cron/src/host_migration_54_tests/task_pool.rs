use super::*;
use agent_config::runtime::agent::DelegateAgentConfig;
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
