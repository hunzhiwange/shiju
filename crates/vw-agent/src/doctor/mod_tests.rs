// 覆盖诊断流程的检查顺序、结构化结果与命令返回语义。

use super::{Severity, diagnose, run};
use crate::app::agent::config::{Config, TelegramConfig};
use chrono::Utc;
use tempfile::TempDir;

fn test_config(tmp: &TempDir) -> Config {
    let workspace_dir = tmp.path().join("workspace");
    let config_dir = tmp.path().join("config");
    let config_path = config_dir.join("vibewindow.json");
    std::fs::create_dir_all(&workspace_dir).unwrap();
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(&config_path, "{}").unwrap();

    let mut config = Config {
        workspace_dir,
        config_path,
        default_provider: Some("openai".into()),
        api_key: Some("configured".into()),
        default_model: Some("gpt-4.1".into()),
        ..Config::default()
    };
    config.gateway.port = 42617;
    config.channels_config.telegram = Some(TelegramConfig {
        bot_token: "token".into(),
        allowed_users: vec!["user".into()],
        stream_mode: Default::default(),
        draft_update_interval_ms: 1000,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });
    config
}

fn write_fresh_state(config: &Config) {
    let now = Utc::now().to_rfc3339();
    let path = crate::app::agent::daemon::state_file_path(config);
    std::fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "updated_at": now,
            "components": {
                "scheduler": {
                    "status": "ok",
                    "last_ok": now,
                },
                "channel:telegram": {
                    "status": "ok",
                    "last_ok": now,
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();
}

// 诊断断言同时检查严重级别与可操作提示，避免只有返回值正确而用户指引失真。
#[test]
fn diagnose_returns_structured_results_in_check_order() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    write_fresh_state(&config);

    let results = diagnose(&config);

    assert!(!results.is_empty());
    assert!(results.iter().any(|item| {
        item.severity == Severity::Ok
            && item.category == "config"
            && item.message == "provider \"openai\" is valid"
    }));
    assert!(results.iter().any(|item| {
        item.severity == Severity::Ok
            && item.category == "daemon"
            && item.message.contains("heartbeat fresh")
    }));

    let mut categories = Vec::new();
    for result in &results {
        if categories.last().is_none_or(|category| category != &result.category) {
            categories.push(result.category.as_str());
        }
    }
    assert_eq!(categories[..5], ["config", "workspace", "daemon", "environment", "cli-tools"]);
}

#[test]
fn run_returns_ok_for_successful_diagnostics() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    write_fresh_state(&config);

    let result = run(&config);

    assert!(result.is_ok());
}

#[test]
fn run_returns_ok_when_diagnostics_have_errors() {
    let tmp = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: tmp.path().join("missing-workspace"),
        config_path: tmp.path().join("missing-config").join("vibewindow.json"),
        ..Config::default()
    };

    let result = run(&config);

    assert!(result.is_ok());
}
