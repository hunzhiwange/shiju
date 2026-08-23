use super::*;
use crate::app::agent::config::SystemConfigStore;
use crate::global_database::GlobalDatabase;
use std::path::Path;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
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

fn persisted_allowed_users(home: &Path) -> Vec<String> {
    let payload = SystemConfigStore::new(GlobalDatabase::for_home(home).expect("database"))
        .expect("config store")
        .get(&vw_api_types::id::TenantScope::default())
        .expect("read config")
        .expect("stored config");
    serde_json::from_str::<Config>(&payload)
        .expect("parse config")
        .channels_config
        .telegram
        .expect("telegram config")
        .allowed_users
}

#[test]
fn normalize_telegram_identity_trims_leading_at() {
    assert_eq!(normalize_telegram_identity("  @alice  "), "alice");
    assert_eq!(normalize_telegram_identity("bob"), "bob");
    assert_eq!(normalize_telegram_identity("  @@carol  "), "carol");
    assert_eq!(normalize_telegram_identity("   "), "");
}

#[test]
fn maybe_restart_managed_daemon_service_is_currently_noop() {
    assert!(!maybe_restart_managed_daemon_service().expect("restart check"));
}

#[test]
fn report_managed_daemon_restart_result_accepts_all_outcomes() {
    report_managed_daemon_restart_result(Ok(true));
    report_managed_daemon_restart_result(Ok(false));
    report_managed_daemon_restart_result(Err(anyhow::anyhow!("reload failed")));
}

#[tokio::test]
async fn bind_telegram_identity_rejects_empty_identity() {
    let config = Config::default();

    let err = bind_telegram_identity(&config, "  @  ").await.expect_err("empty identity fails");

    assert!(err.to_string().contains("cannot be empty"));
}

#[tokio::test]
async fn bind_telegram_identity_requires_configured_channel() {
    let config = Config::default();

    let err = bind_telegram_identity(&config, "@alice").await.expect_err("missing telegram config");

    assert!(err.to_string().contains("not configured"));
}

#[tokio::test]
async fn bind_telegram_identity_is_noop_when_identity_already_allowed() {
    let mut config = Config::default();
    config.channels_config.telegram = Some(telegram_config(vec!["@alice".to_string()]));

    bind_telegram_identity(&config, " alice ").await.expect("existing identity should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn bind_telegram_identity_persists_normalized_new_identity() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home_guard = HomeEnvGuard::set(tmp.path());
    let mut config = Config::default();
    config.config_path = tmp.path().join("vibewindow.json");
    config.channels_config.telegram = Some(telegram_config(vec!["@alice".to_string()]));

    bind_telegram_identity(&config, " @bob ").await.expect("new identity should be persisted");

    assert_eq!(persisted_allowed_users(tmp.path()), vec!["@alice", "bob"]);
}

#[tokio::test(flavor = "current_thread")]
async fn bind_telegram_identity_keeps_wildcard_and_persists_new_identity() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home_guard = HomeEnvGuard::set(tmp.path());
    let mut config = Config::default();
    config.config_path = tmp.path().join("vibewindow.json");
    config.channels_config.telegram =
        Some(telegram_config(vec!["*".to_string(), "@alice".to_string()]));

    bind_telegram_identity(&config, " @@carol ")
        .await
        .expect("wildcard allowlist still permits saving explicit identity");

    assert_eq!(persisted_allowed_users(tmp.path()), vec!["*", "@alice", "carol"]);
}
// 本文件验证 Telegram 身份规范化、绑定持久化与托管服务重启结果。
