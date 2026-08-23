//! 测试 Matrix 调度适配器的消息映射、身份校验与失败处理。

use anyhow::Result;
use serde_json::{Value, json};
#[cfg(feature = "channel-matrix")]
use std::ffi::OsString;

#[cfg(feature = "channel-matrix")]
use crate::app::agent::config::schema::{Config, MatrixConfig};

use super::{
    ConfiguredMatrixDispatchAdapter, FakeMatrixDispatchAdapter, MatrixDispatchAdapter,
    MatrixDispatchRequest, MatrixDispatchTarget, NoMatrixDispatchAdapter,
};

#[cfg(feature = "channel-matrix")]
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(feature = "channel-matrix")]
struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

#[cfg(feature = "channel-matrix")]
impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }
}

#[cfg(feature = "channel-matrix")]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

fn matrix_dispatch_request(
    channel: &str,
    target: &str,
    recipient_id: Option<&str>,
    body: &str,
    payload_json: Value,
) -> MatrixDispatchRequest {
    MatrixDispatchRequest {
        channel: channel.to_string(),
        target: target.to_string(),
        recipient_id: recipient_id.map(str::to_string),
        body: body.to_string(),
        payload_json,
    }
}

fn dispatch_request_named(name: &str) -> MatrixDispatchRequest {
    matrix_dispatch_request(
        "matrix",
        "room:!worker-room:example.test",
        Some("@worker:example.test"),
        &format!("@worker:example.test dispatch {name}"),
        json!({"kind": "dispatch", "task_id": name}),
    )
}

#[cfg(feature = "channel-matrix")]
fn matrix_dispatch_payload(event_version: u8) -> Value {
    json!({
        "event_version": event_version,
        "platform_id": "local",
        "company_id": "local",
        "project_id": "project-1",
        "task_id": "task-1",
        "run_id": "run-task-1-1000",
        "worker_id": "matrix-test",
        "worker_role": "tester",
        "room_id": "!worker-room:example.test",
        "matrix_user_id": "@worker:example.test",
        "model": "gpt-5-mini",
        "model_provider": "openai",
        "agent": "codex",
        "acp_agent": "claude",
        "spec_path": "shared/tasks/task-1/spec.md",
        "metadata_path": "shared/tasks/task-1/meta.json",
        "created_at_ms": 1_000
    })
}

#[cfg(feature = "channel-matrix")]
async fn write_matrix_config(config_dir: &std::path::Path, room_id: &str) {
    let mut config = Config::default();
    config.config_path = config_dir.join("vibewindow.json");
    config.workspace_dir = config_dir.join("workspace");
    config.channels_config.matrix_enabled = true;
    config.channels_config.matrix = Some(MatrixConfig {
        homeserver: "https://matrix.example.test".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: Some("@bot:example.test".to_string()),
        device_id: Some("DEVICE".to_string()),
        room_id: room_id.to_string(),
        allowed_users: vec!["@worker:example.test".to_string()],
        mention_only: false,
    });
    crate::app::agent::config::schema::save_config(&config)
        .await
        .expect("matrix config should be saved");
}

#[test]
fn matrix_dispatch_target_preserves_fields() {
    let target = MatrixDispatchTarget {
        room_id: "!worker-room:example.test".to_string(),
        recipient_id: "@worker:example.test".to_string(),
    };

    assert_eq!(target.room_id, "!worker-room:example.test");
    assert_eq!(target.recipient_id, "@worker:example.test");
}

#[test]
fn matrix_dispatch_target_supports_clone_equality_and_debug() {
    fn assert_eq_trait<T: Eq>() {}

    assert_eq_trait::<MatrixDispatchTarget>();

    let target = MatrixDispatchTarget {
        room_id: "!worker-room:example.test".to_string(),
        recipient_id: "@worker:example.test".to_string(),
    };
    let cloned = target.clone();
    let debug = format!("{target:?}");

    assert_eq!(cloned, target);
    assert!(debug.contains("MatrixDispatchTarget"));
    assert!(debug.contains("room_id"));
    assert!(debug.contains("recipient_id"));
}

#[test]
fn matrix_dispatch_target_equality_includes_every_field() {
    let target = MatrixDispatchTarget {
        room_id: "!worker-room:example.test".to_string(),
        recipient_id: "@worker:example.test".to_string(),
    };

    assert_ne!(
        MatrixDispatchTarget {
            room_id: "!other-room:example.test".to_string(),
            recipient_id: "@worker:example.test".to_string(),
        },
        target
    );
    assert_ne!(
        MatrixDispatchTarget {
            room_id: "!worker-room:example.test".to_string(),
            recipient_id: "@other-worker:example.test".to_string(),
        },
        target
    );
}

#[test]
fn matrix_dispatch_request_preserves_fields() {
    let request = matrix_dispatch_request(
        "matrix",
        "room:!worker-room:example.test",
        Some("@worker:example.test"),
        "@worker:example.test New task [task-1]: ship it",
        json!({"kind": "dispatch", "task_id": "task-1"}),
    );

    assert_eq!(request.channel, "matrix");
    assert_eq!(request.target, "room:!worker-room:example.test");
    assert_eq!(request.recipient_id.as_deref(), Some("@worker:example.test"));
    assert_eq!(request.body, "@worker:example.test New task [task-1]: ship it");
    assert_eq!(request.payload_json, json!({"kind": "dispatch", "task_id": "task-1"}));
}

#[test]
fn matrix_dispatch_request_supports_clone_equality_and_debug() {
    let request = dispatch_request_named("task-1");
    let cloned = request.clone();
    let debug = format!("{request:?}");

    assert_eq!(cloned, request);
    assert!(debug.contains("MatrixDispatchRequest"));
    assert!(debug.contains("channel"));
    assert!(debug.contains("target"));
    assert!(debug.contains("recipient_id"));
    assert!(debug.contains("body"));
    assert!(debug.contains("payload_json"));
}

#[test]
fn matrix_dispatch_request_equality_includes_every_field() {
    let request = matrix_dispatch_request(
        "matrix",
        "room:!worker-room:example.test",
        Some("@worker:example.test"),
        "@worker:example.test New task [task-1]: ship it",
        json!({"kind": "dispatch", "task_id": "task-1"}),
    );

    assert_ne!(
        matrix_dispatch_request(
            "email",
            "room:!worker-room:example.test",
            Some("@worker:example.test"),
            "@worker:example.test New task [task-1]: ship it",
            json!({"kind": "dispatch", "task_id": "task-1"}),
        ),
        request
    );
    assert_ne!(
        matrix_dispatch_request(
            "matrix",
            "room:!other-room:example.test",
            Some("@worker:example.test"),
            "@worker:example.test New task [task-1]: ship it",
            json!({"kind": "dispatch", "task_id": "task-1"}),
        ),
        request
    );
    assert_ne!(
        matrix_dispatch_request(
            "matrix",
            "room:!worker-room:example.test",
            None,
            "@worker:example.test New task [task-1]: ship it",
            json!({"kind": "dispatch", "task_id": "task-1"}),
        ),
        request
    );
    assert_ne!(
        matrix_dispatch_request(
            "matrix",
            "room:!worker-room:example.test",
            Some("@worker:example.test"),
            "@worker:example.test New task [task-2]: ship it",
            json!({"kind": "dispatch", "task_id": "task-1"}),
        ),
        request
    );
    assert_ne!(
        matrix_dispatch_request(
            "matrix",
            "room:!worker-room:example.test",
            Some("@worker:example.test"),
            "@worker:example.test New task [task-1]: ship it",
            json!({"kind": "dispatch", "task_id": "task-2"}),
        ),
        request
    );
}

#[tokio::test]
async fn no_matrix_dispatch_adapter_returns_explicit_configuration_error() {
    let adapter = NoMatrixDispatchAdapter;
    let error = adapter
        .send_message(dispatch_request_named("task-1"))
        .await
        .expect_err("NoMatrixDispatchAdapter should reject dispatch");

    assert_eq!(
        error.to_string(),
        "Matrix outbox dispatch is not configured; configure a Matrix dispatch adapter to deliver Matrix outbox messages"
    );
}

#[test]
fn no_matrix_dispatch_adapter_supports_default_clone_copy_and_debug() {
    fn assert_traits<T: Clone + Copy + Default>() {}

    assert_traits::<NoMatrixDispatchAdapter>();

    let adapter = NoMatrixDispatchAdapter;
    let copied = adapter;

    assert_eq!(format!("{copied:?}"), "NoMatrixDispatchAdapter");
}

#[test]
fn configured_matrix_dispatch_adapter_supports_default_clone_copy_and_debug() {
    fn assert_traits<T: Clone + Copy + Default>() {}

    assert_traits::<ConfiguredMatrixDispatchAdapter>();

    let adapter = ConfiguredMatrixDispatchAdapter;
    let copied = adapter;

    assert_eq!(format!("{copied:?}"), "ConfiguredMatrixDispatchAdapter");
}

#[cfg(not(feature = "channel-matrix"))]
#[tokio::test]
async fn configured_matrix_dispatch_adapter_requires_channel_matrix_feature() {
    let adapter = ConfiguredMatrixDispatchAdapter;
    let error = adapter
        .send_message(dispatch_request_named("task-1"))
        .await
        .expect_err("configured Matrix adapter should require channel-matrix feature");

    assert_eq!(error.to_string(), "Matrix outbox dispatch requires the channel-matrix feature");
}

#[cfg(feature = "channel-matrix")]
#[tokio::test]
async fn configured_matrix_dispatch_adapter_requires_matrix_config_when_feature_enabled() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp config dir");
    let config_dir = temp.path().join("config");
    let _home = EnvGuard::set("HOME", temp.path());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    let adapter = ConfiguredMatrixDispatchAdapter;

    let error = adapter
        .send_message(dispatch_request_named("task-1"))
        .await
        .expect_err("configured Matrix adapter should require Matrix channel config");

    assert_eq!(error.to_string(), "Matrix outbox dispatch requires channels.matrix configuration");
}

#[cfg(feature = "channel-matrix")]
#[tokio::test]
async fn configured_matrix_dispatch_adapter_rejects_plain_target_without_fallback_room() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp config dir");
    let config_dir = temp.path().join("config");
    let _home = EnvGuard::set("HOME", temp.path());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    write_matrix_config(&config_dir, "  ").await;
    let adapter = ConfiguredMatrixDispatchAdapter;

    let error = adapter
        .send_message(matrix_dispatch_request(
            "matrix",
            "@worker:example.test",
            Some("@worker:example.test"),
            "@worker:example.test dispatch task-1",
            matrix_dispatch_payload(1),
        ))
        .await
        .expect_err("plain Matrix target should require configured fallback room");

    assert_eq!(error.to_string(), "Matrix outbox target must start with room:");
}

#[cfg(feature = "channel-matrix")]
#[tokio::test]
async fn configured_matrix_dispatch_adapter_rejects_empty_explicit_room_target() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp config dir");
    let config_dir = temp.path().join("config");
    let _home = EnvGuard::set("HOME", temp.path());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    write_matrix_config(&config_dir, "!fallback-room:example.test").await;
    let adapter = ConfiguredMatrixDispatchAdapter;

    let error = adapter
        .send_message(matrix_dispatch_request(
            "matrix",
            "room:  ",
            Some("@worker:example.test"),
            "@worker:example.test dispatch task-1",
            matrix_dispatch_payload(1),
        ))
        .await
        .expect_err("empty Matrix room target should be rejected");

    assert_eq!(error.to_string(), "Matrix outbox room target must not be empty");
}

#[cfg(feature = "channel-matrix")]
#[tokio::test]
async fn configured_matrix_dispatch_adapter_wraps_invalid_payload_parse_error() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp config dir");
    let config_dir = temp.path().join("config");
    let _home = EnvGuard::set("HOME", temp.path());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    write_matrix_config(&config_dir, "!fallback-room:example.test").await;
    let adapter = ConfiguredMatrixDispatchAdapter;

    let error = adapter
        .send_message(matrix_dispatch_request(
            "matrix",
            "room:!worker-room:example.test",
            Some("@worker:example.test"),
            "@worker:example.test dispatch task-1",
            json!({"unexpected": true}),
        ))
        .await
        .expect_err("invalid dispatch payload should be rejected before Matrix send");

    assert_eq!(error.to_string(), "parse Matrix scheduler dispatch event payload");
}

#[cfg(feature = "channel-matrix")]
#[tokio::test]
async fn configured_matrix_dispatch_adapter_wraps_invalid_dispatch_event_error() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp config dir");
    let config_dir = temp.path().join("config");
    let _home = EnvGuard::set("HOME", temp.path());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    write_matrix_config(&config_dir, "!fallback-room:example.test").await;
    let adapter = ConfiguredMatrixDispatchAdapter;

    let error = adapter
        .send_message(matrix_dispatch_request(
            "matrix",
            "room:!worker-room:example.test",
            Some("@worker:example.test"),
            "@worker:example.test dispatch task-1",
            matrix_dispatch_payload(2),
        ))
        .await
        .expect_err("invalid dispatch event should be rejected before Matrix send");

    assert_eq!(error.to_string(), "send Matrix scheduler dispatch event");
    assert!(format!("{error:#}").contains("event_version must be 1"));
}

#[tokio::test]
async fn fake_matrix_dispatch_adapter_records_successful_requests() -> Result<()> {
    let adapter = FakeMatrixDispatchAdapter::new();
    let request = dispatch_request_named("task-1");

    adapter.send_message(request.clone()).await?;

    assert_eq!(adapter.recorded_requests(), vec![request]);
    Ok(())
}

#[tokio::test]
async fn fake_matrix_dispatch_adapter_records_failed_requests() {
    let adapter = FakeMatrixDispatchAdapter::new();
    let request = dispatch_request_named("task-1");
    adapter.fail_next("matrix send timed out");

    let error = adapter
        .send_message(request.clone())
        .await
        .expect_err("fake Matrix adapter should return queued failure");

    assert_eq!(error.to_string(), "matrix send timed out");
    assert_eq!(adapter.recorded_requests(), vec![request]);
}

#[tokio::test]
async fn fake_matrix_dispatch_adapter_uses_queued_outcomes_in_order() -> Result<()> {
    let adapter = FakeMatrixDispatchAdapter::default();
    let first = dispatch_request_named("task-1");
    let second = dispatch_request_named("task-2");
    let third = dispatch_request_named("task-3");
    adapter.succeed_next();
    adapter.fail_next("matrix rate limited");

    adapter.send_message(first.clone()).await?;
    let error = adapter
        .send_message(second.clone())
        .await
        .expect_err("fake Matrix adapter should return second queued outcome");
    adapter.send_message(third.clone()).await?;

    assert_eq!(error.to_string(), "matrix rate limited");
    assert_eq!(adapter.recorded_requests(), vec![first, second, third]);
    Ok(())
}

#[test]
fn fake_matrix_dispatch_adapter_supports_clone_and_debug() {
    let adapter = FakeMatrixDispatchAdapter::new();
    let cloned = adapter.clone();
    let debug = format!("{adapter:?}");

    assert!(debug.contains("FakeMatrixDispatchAdapter"));
    assert!(cloned.recorded_requests().is_empty());
}

#[tokio::test]
async fn fake_matrix_dispatch_adapter_clone_shares_requests_and_outcomes() -> Result<()> {
    let adapter = FakeMatrixDispatchAdapter::new();
    let cloned = adapter.clone();
    let first = dispatch_request_named("task-1");
    let second = dispatch_request_named("task-2");
    cloned.fail_next("queued by clone");

    let error = adapter
        .send_message(first.clone())
        .await
        .expect_err("queued clone failure should be visible to original");
    cloned.send_message(second.clone()).await?;

    assert_eq!(error.to_string(), "queued by clone");
    assert_eq!(adapter.recorded_requests(), vec![first.clone(), second.clone()]);
    assert_eq!(cloned.recorded_requests(), vec![first, second]);
    Ok(())
}

#[tokio::test]
async fn fake_matrix_dispatch_adapter_recorded_requests_returns_snapshot() -> Result<()> {
    let adapter = FakeMatrixDispatchAdapter::new();
    let first = dispatch_request_named("task-1");
    let second = dispatch_request_named("task-2");

    adapter.send_message(first.clone()).await?;
    let snapshot = adapter.recorded_requests();
    adapter.send_message(second).await?;

    assert_eq!(snapshot, vec![first]);
    assert_eq!(adapter.recorded_requests().len(), 2);
    Ok(())
}

#[cfg(feature = "channel-matrix")]
#[test]
fn matrix_dispatch_room_id_uses_explicit_room_target() -> Result<()> {
    assert_eq!(
        super::matrix_dispatch_room_id(
            "room:  !worker-room:example.test  ",
            "!fallback:example.test"
        )?,
        "!worker-room:example.test"
    );
    Ok(())
}

#[cfg(feature = "channel-matrix")]
#[test]
fn matrix_dispatch_room_id_accepts_explicit_room_without_fallback() -> Result<()> {
    assert_eq!(
        super::matrix_dispatch_room_id("room:!worker-room:example.test", "   ")?,
        "!worker-room:example.test"
    );
    Ok(())
}

#[cfg(feature = "channel-matrix")]
#[test]
fn matrix_dispatch_room_id_uses_configured_fallback_for_plain_target() -> Result<()> {
    assert_eq!(
        super::matrix_dispatch_room_id("@worker:example.test", "  !fallback-room:example.test  ")?,
        "!fallback-room:example.test"
    );
    Ok(())
}

#[cfg(feature = "channel-matrix")]
#[test]
fn matrix_dispatch_room_id_rejects_plain_target_without_fallback() {
    let error = super::matrix_dispatch_room_id("@worker:example.test", "  ")
        .expect_err("plain Matrix target should require configured fallback room");

    assert_eq!(error.to_string(), "Matrix outbox target must start with room:");
}

#[cfg(feature = "channel-matrix")]
#[test]
fn matrix_dispatch_room_id_rejects_empty_explicit_room() {
    let error = super::matrix_dispatch_room_id("room:  ", "!fallback-room:example.test")
        .expect_err("empty Matrix room target should be rejected");

    assert_eq!(error.to_string(), "Matrix outbox room target must not be empty");
}
