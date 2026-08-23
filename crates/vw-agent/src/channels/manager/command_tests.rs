use super::*;
use crate::app::agent::config::SystemConfigStore;
use crate::global_database::GlobalDatabase;
use std::path::Path;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

fn telegram_config(allowed_users: Vec<String>) -> crate::app::agent::config::TelegramConfig {
    crate::app::agent::config::TelegramConfig {
        bot_token: "token".to_string(),
        allowed_users,
        stream_mode: crate::app::agent::config::StreamMode::Off,
        draft_update_interval_ms: 1000,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    }
}

#[tokio::test]
async fn handle_command_rejects_runtime_owned_commands() {
    let config = Config::default();

    let start = handle_command(ChannelCommands::Start, &config).await.expect_err("start");
    assert!(start.to_string().contains("main.rs"));

    let doctor = handle_command(ChannelCommands::Doctor, &config).await.expect_err("doctor");
    assert!(doctor.to_string().contains("main.rs"));
}

#[tokio::test]
async fn handle_command_list_accepts_default_config() {
    handle_command(ChannelCommands::List, &Config::default())
        .await
        .expect("list should only print configured channels");
}

#[tokio::test]
async fn handle_command_list_marks_configured_channel() {
    let mut config = Config::default();
    config.channels_config.telegram = Some(telegram_config(vec!["@alice".to_string()]));

    handle_command(ChannelCommands::List, &config)
        .await
        .expect("list should report both configured and unconfigured channels");
}

#[tokio::test]
async fn handle_command_add_and_remove_report_manual_configuration() {
    let config = Config::default();

    let add = handle_command(
        ChannelCommands::Add { channel_type: "unknown".to_string(), config: "{}".to_string() },
        &config,
    )
    .await
    .expect_err("add");
    assert!(add.to_string().contains("unknown"));

    let remove = handle_command(ChannelCommands::Remove { name: "telegram".to_string() }, &config)
        .await
        .expect_err("remove");
    assert!(remove.to_string().contains("telegram"));
}

#[tokio::test]
async fn handle_command_bind_telegram_rejects_empty_identity() {
    let config = Config::default();

    let err =
        handle_command(ChannelCommands::BindTelegram { identity: " @ ".to_string() }, &config)
            .await
            .expect_err("empty identity");

    assert!(err.to_string().contains("cannot be empty"));
}

struct HomeEnvGuard(Option<std::ffi::OsString>);

impl HomeEnvGuard {
    fn set(home: &Path) -> Self {
        let original = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", home) };
        Self(original)
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(original) = &self.0 {
                std::env::set_var("HOME", original);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn handle_command_bind_telegram_persists_new_identity() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home_guard = HomeEnvGuard::set(tmp.path());
    let mut config = Config::default();
    config.config_path = tmp.path().join("vibewindow.json");
    config.channels_config.telegram = Some(telegram_config(vec!["@alice".to_string()]));

    handle_command(ChannelCommands::BindTelegram { identity: " @bob ".to_string() }, &config)
        .await
        .expect("bind telegram identity");

    let payload = SystemConfigStore::new(GlobalDatabase::for_home(tmp.path()).expect("database"))
        .expect("config store")
        .get(&vw_api_types::id::TenantScope::default())
        .expect("read config")
        .expect("stored config");
    let persisted: Config = serde_json::from_str(&payload).expect("parse config");
    assert_eq!(
        persisted.channels_config.telegram.expect("telegram config").allowed_users,
        vec!["@alice", "bob"]
    );
}
// 本文件验证渠道管理命令分派、配置展示与 Telegram 身份绑定。
