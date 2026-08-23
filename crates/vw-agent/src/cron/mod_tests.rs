// 覆盖定时任务类型、调度表达式及运行结果的公共契约。

use super::*;
use crate::app::agent::config::Config;
use tempfile::TempDir;

fn test_config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

fn add_cron_job(config: &Config) -> CronJob {
    add_shell_job(
        config,
        None,
        Schedule::Cron { expr: "*/5 * * * *".into(), tz: None },
        "echo original",
    )
    .unwrap()
}

// 固定时间与临时存储夹具用于隔离真实时钟和持久化状态，保证调度断言可重复。
#[test]
fn list_prints_agent_prompt_and_recorded_status() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_agent_job(
        &config,
        Some("agent-job".into()),
        Schedule::Every { every_ms: 60_000 },
        "summarize",
        SessionTarget::Main,
        None,
        None,
        false,
    )
    .unwrap();

    record_last_run(
        &config,
        &job.id,
        chrono::Utc::now() - chrono::Duration::minutes(1),
        false,
        "failed",
    )
    .unwrap();

    handle_command(CronCommands::List, &config).unwrap();
}

#[test]
fn handle_pause_and_resume_toggle_enabled() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_cron_job(&config);

    handle_command(CronCommands::Pause { id: job.id.clone() }, &config).unwrap();
    assert!(!get_job(&config, &job.id).unwrap().enabled);

    handle_command(CronCommands::Resume { id: job.id.clone() }, &config).unwrap();
    assert!(get_job(&config, &job.id).unwrap().enabled);
}

#[test]
fn add_at_rejects_invalid_timestamp() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    let err = handle_command(
        CronCommands::AddAt { at: "not-a-date".into(), command: "echo at".into() },
        &config,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("Invalid RFC3339 timestamp"));
}

#[test]
fn update_rejects_disallowed_command() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp);
    config.autonomy.allowed_commands = vec!["echo".into()];
    let job = add_cron_job(&config);

    let err = handle_command(
        CronCommands::Update {
            id: job.id.clone(),
            expression: None,
            tz: None,
            command: Some("date".into()),
            name: None,
        },
        &config,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("Command blocked by security policy"));
    assert_eq!(get_job(&config, &job.id).unwrap().command, "echo original");
}

#[test]
fn once_rejects_delay_without_number() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    let err = handle_command(
        CronCommands::Once { delay: "minutes".into(), command: "echo once".into() },
        &config,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("cannot parse integer"));
}
