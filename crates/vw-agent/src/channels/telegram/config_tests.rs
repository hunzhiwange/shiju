// 本文件验证 Telegram 配置规范化、身份绑定与系统配置持久化。

use super::TelegramChannel;
use crate::app::agent::config::{Config, StreamMode, SystemConfigStore, TelegramConfig};
use crate::global_database::GlobalDatabase;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};
use std::path::Path;
use std::sync::MutexGuard;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Mutex, oneshot};

fn env_lock() -> MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

struct HomeEnvGuard {
    original: Option<std::ffi::OsString>,
}

impl HomeEnvGuard {
    fn set(path: &Path) -> Self {
        let original = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", path) };
        Self { original }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(original) = &self.original {
                std::env::set_var("HOME", original);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}

fn telegram_config(allowed_users: Vec<String>) -> TelegramConfig {
    TelegramConfig {
        bot_token: "telegram-token".to_string(),
        allowed_users,
        stream_mode: Default::default(),
        draft_update_interval_ms: 1000,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    }
}

async fn seed_system_config(home: &Path, mut config: Config) -> std::path::PathBuf {
    let config_dir = vw_config_types::paths::home_config_dir(home);
    let config_path = config_dir.join("vibewindow.json");
    config.config_path = config_path.clone();
    config.workspace_dir = config_dir.join("workspace");
    crate::app::agent::config::schema::save_config(&config)
        .await
        .expect("seed system config database");
    config_path
}

fn stored_system_config(home: &Path) -> String {
    SystemConfigStore::new(GlobalDatabase::for_home(home).expect("global database"))
        .expect("system config store")
        .get(&vw_api_types::id::TenantScope::default())
        .expect("read system config")
        .expect("stored system config")
}

struct TestServerState {
    requests: Mutex<usize>,
    responses: Mutex<VecDeque<(StatusCode, &'static str)>>,
}

struct TestServer {
    base_url: String,
    state: Arc<TestServerState>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    async fn spawn(responses: Vec<(StatusCode, &'static str)>) -> Self {
        let state = Arc::new(TestServerState {
            requests: Mutex::new(0),
            responses: Mutex::new(VecDeque::from(responses)),
        });
        let app = Router::new().route("/{*path}", get(record_request)).with_state(state.clone());
        let listener =
            tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve test server");
        });

        Self { base_url: format!("http://{addr}"), state, shutdown: Some(shutdown_tx) }
    }

    async fn request_count(&self) -> usize {
        *self.state.requests.lock().await
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn record_request(State(state): State<Arc<TestServerState>>) -> Response<Body> {
    *state.requests.lock().await += 1;
    let (status, body) = state
        .responses
        .lock()
        .await
        .pop_front()
        .unwrap_or((StatusCode::OK, r#"{"ok":true,"result":{"username":"cached_bot"}}"#));

    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("response should build")
}

#[test]
fn with_api_base_trims_trailing_slashes() {
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base("https://api.example.test///".to_string());

    assert_eq!(channel.api_url("getMe"), "https://api.example.test/bot123:ABC/getMe");
}

#[test]
// 身份规范化支持用户名与数字标识，但不混淆二者语义。
fn normalize_identity_accepts_usernames_and_numeric_ids() {
    assert_eq!(TelegramChannel::normalize_identity("@Alice"), "Alice");
    assert_eq!(TelegramChannel::normalize_identity(" 12345 "), "12345");
    assert_eq!(TelegramChannel::normalize_identity("@"), "");
}

#[test]
fn new_normalizes_allowed_users_and_defaults_runtime_state() {
    let channel = TelegramChannel::new(
        "token".to_string(),
        vec![" @Alice ".to_string(), "".to_string(), "@".to_string(), "42".to_string()],
        true,
    );

    assert_eq!(
        channel.allowed_users.read().expect("allowed users lock").clone(),
        vec!["Alice".to_string(), "42".to_string()]
    );
    assert!(channel.mention_only);
    assert_eq!(channel.stream_mode, StreamMode::Off);
    assert_eq!(channel.draft_update_interval_ms, 1000);
    assert!(channel.workspace_dir.is_none());
}

#[test]
fn streaming_and_workspace_builders_update_fields() {
    let workspace = std::path::PathBuf::from("/tmp/vw-telegram-workspace");

    let channel = TelegramChannel::new("token".to_string(), vec![], false)
        .with_workspace_dir(workspace.clone())
        .with_streaming(StreamMode::Partial, 250);

    assert_eq!(channel.workspace_dir, Some(workspace));
    assert_eq!(channel.stream_mode, StreamMode::Partial);
    assert_eq!(channel.draft_update_interval_ms, 250);
}

#[test]
fn group_reply_allowed_senders_are_trimmed_sorted_and_deduped() {
    let channel = TelegramChannel::new("token".to_string(), vec![], true)
        .with_group_reply_allowed_senders(vec![
            " 42 ".to_string(),
            "".to_string(),
            "*".to_string(),
            "42".to_string(),
            "7".to_string(),
        ]);

    assert_eq!(
        channel.group_reply_allowed_sender_ids,
        vec!["*".to_string(), "42".to_string(), "7".to_string()]
    );
    assert!(channel.is_group_sender_trigger_enabled(Some("42")));
    assert!(channel.is_group_sender_trigger_enabled(Some("anything")));
    assert!(!channel.is_group_sender_trigger_enabled(None));
    assert!(!channel.is_group_sender_trigger_enabled(Some("   ")));
}

#[test]
fn runtime_allowlist_adds_normalized_identity_once_and_ignores_empty() {
    let channel = TelegramChannel::new("token".to_string(), vec!["@alice".to_string()], false);

    channel.add_allowed_identity_runtime(" @bob ");
    channel.add_allowed_identity_runtime("bob");
    channel.add_allowed_identity_runtime("@");

    assert_eq!(
        channel.allowed_users.read().expect("allowed users lock").clone(),
        vec!["alice".to_string(), "bob".to_string()]
    );
}

#[test]
fn extract_bind_code_accepts_bot_suffix_and_rejects_other_commands() {
    assert_eq!(TelegramChannel::extract_bind_code("/bind ABC123"), Some("ABC123"));
    assert_eq!(TelegramChannel::extract_bind_code("/bind@vibebot XYZ789"), Some("XYZ789"));
    assert_eq!(TelegramChannel::extract_bind_code("/bind"), None);
    assert_eq!(TelegramChannel::extract_bind_code("/other ABC123"), None);
}

#[test]
fn pairing_code_active_is_true_for_empty_allowlist() {
    let empty = TelegramChannel::new("token".to_string(), vec![], false);
    let configured = TelegramChannel::new("token".to_string(), vec!["alice".to_string()], false);

    assert!(empty.pairing_code_active());
    assert!(!configured.pairing_code_active());
}

#[tokio::test(flavor = "current_thread")]
async fn load_config_without_env_reads_database_and_sets_runtime_paths() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let home = tempfile::tempdir().expect("temp home");
    let _home_guard = HomeEnvGuard::set(home.path());
    let mut config = Config::default();
    config.channels_config.telegram = Some(telegram_config(vec!["alice".to_string()]));
    let config_path = seed_system_config(home.path(), config).await;

    let loaded = TelegramChannel::load_config_without_env().await.expect("load config");

    assert_eq!(loaded.config_path, config_path);
    assert_eq!(
        loaded.workspace_dir,
        vw_config_types::paths::home_config_dir(home.path()).join("workspace")
    );
    assert_eq!(
        loaded.channels_config.telegram.expect("telegram config").allowed_users,
        vec!["alice".to_string()]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn load_config_without_env_initializes_default_and_reports_invalid_database_payload() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let home = tempfile::tempdir().expect("temp home");
    let _home_guard = HomeEnvGuard::set(home.path());

    let initialized = TelegramChannel::load_config_without_env().await.expect("default config");
    assert!(initialized.channels_config.telegram.is_none());

    let store =
        SystemConfigStore::new(GlobalDatabase::for_home(home.path()).expect("global database"))
            .expect("system config store");
    store.upsert(&vw_api_types::id::TenantScope::default(), "{").expect("write invalid payload");

    let parse = TelegramChannel::load_config_without_env().await.unwrap_err().to_string();
    assert!(parse.contains("Failed to parse tenant system config payload"));
}

#[tokio::test(flavor = "current_thread")]
async fn persist_allowed_identity_rejects_missing_telegram_section_and_empty_identity() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let home = tempfile::tempdir().expect("temp home");
    let _home_guard = HomeEnvGuard::set(home.path());
    seed_system_config(home.path(), Config::default()).await;
    let channel = TelegramChannel::new("token".to_string(), vec![], false);

    let missing = channel.persist_allowed_identity("alice").await.unwrap_err().to_string();
    assert!(missing.contains("Missing channels.telegram in the system config"), "{missing}");

    let mut config = Config::default();
    config.channels_config.telegram = Some(telegram_config(Vec::new()));
    seed_system_config(home.path(), config).await;

    let empty = channel.persist_allowed_identity(" @ ").await.unwrap_err().to_string();
    assert!(empty.contains("Cannot persist empty Telegram identity"));
}

#[tokio::test(flavor = "current_thread")]
async fn persist_allowed_identity_skips_save_when_identity_already_exists() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let home = tempfile::tempdir().expect("temp home");
    let _home_guard = HomeEnvGuard::set(home.path());
    let mut config = Config::default();
    config.channels_config.telegram = Some(telegram_config(vec!["alice".to_string()]));
    seed_system_config(home.path(), config).await;
    let before = stored_system_config(home.path());
    let channel = TelegramChannel::new("token".to_string(), vec![], false);

    channel.persist_allowed_identity(" @alice ").await.expect("existing identity is ok");

    let after = stored_system_config(home.path());
    assert_eq!(after, before);
}

#[tokio::test(flavor = "current_thread")]
async fn persist_allowed_identity_appends_normalized_identity() {
    let _async_env_guard = crate::test_support::global_async_env_lock().lock().await;
    let _env_guard = env_lock();
    let home = tempfile::tempdir().expect("temp home");
    let _home_guard = HomeEnvGuard::set(home.path());
    let mut config = Config::default();
    config.channels_config.telegram = Some(telegram_config(vec!["alice".to_string()]));
    seed_system_config(home.path(), config).await;
    let channel = TelegramChannel::new("token".to_string(), vec![], false);

    channel.persist_allowed_identity(" @bob ").await.expect("persist new identity");

    let persisted = TelegramChannel::load_config_without_env().await.expect("reload config");
    assert_eq!(
        persisted.channels_config.telegram.expect("telegram config").allowed_users,
        vec!["alice".to_string(), "bob".to_string()]
    );
}

#[tokio::test]
async fn fetch_bot_username_reads_get_me_response() {
    let server =
        TestServer::spawn(vec![(StatusCode::OK, r#"{"ok":true,"result":{"username":"vibebot"}}"#)])
            .await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    assert_eq!(channel.fetch_bot_username().await.expect("username"), "vibebot");
    assert_eq!(server.request_count().await, 1);
}

#[tokio::test]
async fn get_bot_username_caches_successful_fetch() {
    let server = TestServer::spawn(vec![(
        StatusCode::OK,
        r#"{"ok":true,"result":{"username":"cached_bot"}}"#,
    )])
    .await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    assert_eq!(channel.get_bot_username().await.as_deref(), Some("cached_bot"));
    assert_eq!(channel.get_bot_username().await.as_deref(), Some("cached_bot"));
    assert_eq!(server.request_count().await, 1);
}

#[tokio::test]
async fn fetch_bot_username_reports_status_and_missing_username() {
    let status_server = TestServer::spawn(vec![(StatusCode::BAD_GATEWAY, r#"{"ok":false}"#)]).await;
    let status_channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(status_server.base_url.clone());
    assert!(status_channel.fetch_bot_username().await.unwrap_err().to_string().contains("502"));

    let missing_server =
        TestServer::spawn(vec![(StatusCode::OK, r#"{"ok":true,"result":{}}"#)]).await;
    let missing_channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(missing_server.base_url.clone());
    assert!(
        missing_channel
            .fetch_bot_username()
            .await
            .unwrap_err()
            .to_string()
            .contains("Bot username not found")
    );
}

#[tokio::test]
async fn get_bot_username_returns_none_when_fetch_fails() {
    let server = TestServer::spawn(vec![(StatusCode::BAD_GATEWAY, r#"{"ok":false}"#)]).await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    assert_eq!(channel.get_bot_username().await, None);
    assert_eq!(server.request_count().await, 1);
    assert!(channel.bot_username.lock().is_none());
}
