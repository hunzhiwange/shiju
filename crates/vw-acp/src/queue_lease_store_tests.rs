use super::*;

use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::queue_paths::{
    TEST_HOME_ENV_LOCK, queue_base_dir, queue_lock_file_path, queue_socket_base_dir,
    queue_socket_path,
};

static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(1);

struct TempHome {
    path: PathBuf,
}

impl TempHome {
    async fn new(name: &str) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir()
            .join(format!("vw-acp-queue-lease-{name}-{}-{id}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&path).await;
        tokio::fs::create_dir_all(&path).await.expect("create temporary home");
        Self { path }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
        if let Some(socket_dir) = queue_socket_base_dir(&self.path) {
            let _ = std::fs::remove_dir_all(socket_dir);
        }
    }
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn clear(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn owner_record(
    session_id: &str,
    pid: u32,
    socket_path: PathBuf,
    heartbeat_at: &str,
) -> QueueOwnerRecord {
    QueueOwnerRecord {
        pid,
        session_id: session_id.to_string(),
        socket_path,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        heartbeat_at: heartbeat_at.to_string(),
        owner_generation: 7,
        queue_depth: 42,
    }
}

async fn write_path(path: &Path, contents: impl AsRef<[u8]>) {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.expect("create parent directory");
    }
    tokio::fs::write(path, contents).await.expect("write test file");
}

async fn write_owner_record(home_dir: &Path, record: &QueueOwnerRecord) {
    let payload = serde_json::to_vec_pretty(record).expect("serialize owner record");
    write_path(&queue_lock_file_path(&record.session_id, home_dir), payload).await;
}

#[test]
fn parse_queue_owner_record_accepts_valid_record() {
    let expected =
        owner_record("session-valid", 123, PathBuf::from("owner.sock"), "2026-01-01T00:00:00Z");
    let raw = serde_json::to_string(&expected).expect("serialize owner record");

    assert_eq!(parse_queue_owner_record(&raw), Some(expected));
}

#[test]
fn parse_queue_owner_record_rejects_malformed_or_invalid_identity() {
    let valid = serde_json::json!({
        "pid": 123,
        "sessionId": "session-invalid",
        "socketPath": "owner.sock",
        "createdAt": "2026-01-01T00:00:00Z",
        "heartbeatAt": "2026-01-01T00:00:00Z",
        "ownerGeneration": 7,
        "queueDepth": 0
    });
    let mut zero_pid = valid.clone();
    zero_pid["pid"] = serde_json::json!(0);
    let mut zero_generation = valid;
    zero_generation["ownerGeneration"] = serde_json::json!(0);

    assert_eq!(parse_queue_owner_record("not-json"), None);
    assert_eq!(parse_queue_owner_record(&zero_pid.to_string()), None);
    assert_eq!(parse_queue_owner_record(&zero_generation.to_string()), None);
}

#[test]
fn queue_owner_record_serializes_with_camel_case_fields() {
    let record =
        owner_record("session-casing", 123, PathBuf::from("owner.sock"), "2026-01-01T00:00:00Z");
    let value = serde_json::to_value(record).expect("serialize owner record");

    assert_eq!(value["sessionId"], "session-casing");
    assert_eq!(value["ownerGeneration"], 7);
    assert_eq!(value["queueDepth"], 42);
    assert!(value.get("session_id").is_none());
}

#[test]
fn queue_owner_heartbeat_is_stale_for_invalid_or_expired_timestamp() {
    let invalid = owner_record("session-invalid", 123, PathBuf::new(), "invalid");
    let expired = owner_record("session-expired", 123, PathBuf::new(), "1970-01-01T00:00:00Z");

    assert!(is_queue_owner_heartbeat_stale(&invalid));
    assert!(is_queue_owner_heartbeat_stale(&expired));
}

#[test]
fn queue_owner_heartbeat_is_not_stale_for_future_timestamp() {
    let owner = owner_record("session-future", 123, PathBuf::new(), "2999-01-01T00:00:00Z");

    assert!(!is_queue_owner_heartbeat_stale(&owner));
}

#[test]
fn generated_owner_metadata_is_valid() {
    let timestamp = now_iso();

    assert!(OffsetDateTime::parse(&timestamp, &Rfc3339).is_ok());
    assert_ne!(create_owner_generation(), 0);
}

#[tokio::test]
async fn ensure_queue_dir_creates_lock_and_socket_directories() {
    let temp_home = TempHome::new("ensure-directories").await;

    ensure_queue_dir(&temp_home.path).await.expect("ensure queue directories");

    assert!(queue_base_dir(&temp_home.path).is_dir());
    if let Some(socket_dir) = queue_socket_base_dir(&temp_home.path) {
        assert!(socket_dir.is_dir());
    }
}

#[cfg(not(windows))]
#[tokio::test]
async fn remove_socket_file_removes_existing_file_and_tolerates_missing_file() {
    let temp_home = TempHome::new("remove-socket").await;
    let socket_path = temp_home.path.join("owner.sock");
    write_path(&socket_path, b"socket").await;

    remove_socket_file(&socket_path).await.expect("remove socket file");
    assert!(!socket_path.exists());

    remove_socket_file(&socket_path).await.expect("ignore missing socket file");
}

#[tokio::test]
async fn wait_for_process_exit_accepts_current_process_as_not_owned() {
    assert!(wait_for_process_exit(std::process::id(), std::time::Duration::ZERO).await);
}

#[tokio::test]
async fn cleanup_stale_queue_owner_removes_recorded_socket_and_lock() {
    let temp_home = TempHome::new("cleanup-recorded").await;
    let session_id = "session-cleanup-recorded";
    let socket_path = temp_home.path.join("recorded.sock");
    let record =
        owner_record(session_id, std::process::id(), socket_path.clone(), "2999-01-01T00:00:00Z");
    write_owner_record(&temp_home.path, &record).await;
    #[cfg(not(windows))]
    write_path(&socket_path, b"socket").await;

    cleanup_stale_queue_owner(session_id, Some(&record), &temp_home.path)
        .await
        .expect("cleanup stale owner");

    assert!(!queue_lock_file_path(session_id, &temp_home.path).exists());
    #[cfg(not(windows))]
    assert!(!socket_path.exists());
}

#[tokio::test]
async fn cleanup_stale_queue_owner_tolerates_missing_derived_paths() {
    let temp_home = TempHome::new("cleanup-missing").await;

    cleanup_stale_queue_owner("session-cleanup-missing", None, &temp_home.path)
        .await
        .expect("ignore missing owner paths");
}

#[tokio::test]
async fn read_queue_owner_record_handles_missing_valid_and_invalid_files() {
    let temp_home = TempHome::new("read-record").await;
    let session_id = "session-read-record";

    assert_eq!(read_queue_owner_record(session_id, &temp_home.path).await, None);

    let expected =
        owner_record(session_id, 123, temp_home.path.join("owner.sock"), "2026-01-01T00:00:00Z");
    write_owner_record(&temp_home.path, &expected).await;
    assert_eq!(read_queue_owner_record(session_id, &temp_home.path).await, Some(expected));

    write_path(&queue_lock_file_path(session_id, &temp_home.path), b"invalid").await;
    assert_eq!(read_queue_owner_record(session_id, &temp_home.path).await, None);
}

#[tokio::test]
async fn default_queue_owner_functions_reject_missing_home_directory() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    #[cfg(not(windows))]
    let _home = EnvVarGuard::clear("HOME");
    #[cfg(windows)]
    let _user_profile = EnvVarGuard::clear("USERPROFILE");
    #[cfg(windows)]
    let _home_drive = EnvVarGuard::clear("HOMEDRIVE");
    #[cfg(windows)]
    let _home_path = EnvVarGuard::clear("HOMEPATH");

    assert_eq!(read_default_queue_owner_record("session-missing-home").await, None);
    assert_eq!(
        read_default_queue_owner_status("session-missing-home")
            .await
            .expect_err("status requires home")
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        try_acquire_default_queue_owner_lease("session-missing-home")
            .await
            .expect_err("acquire requires home")
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        terminate_default_queue_owner_for_session("session-missing-home")
            .await
            .expect_err("terminate requires home")
            .kind(),
        ErrorKind::NotFound
    );
}

#[test]
fn is_process_alive_rejects_missing_zero_and_current_pid() {
    assert!(!is_process_alive(None));
    assert!(!is_process_alive(Some(0)));
    assert!(!is_process_alive(Some(std::process::id())));
}

#[tokio::test]
async fn terminate_process_rejects_current_process_without_signaling() {
    assert!(!terminate_process(std::process::id()).await);
}

#[tokio::test]
async fn ensure_owner_is_usable_cleans_owner_that_is_not_alive() {
    let temp_home = TempHome::new("ensure-unusable").await;
    let session_id = "session-unusable";
    let socket_path = temp_home.path.join("unusable.sock");
    let owner = owner_record(session_id, std::process::id(), socket_path.clone(), "invalid");
    write_owner_record(&temp_home.path, &owner).await;
    #[cfg(not(windows))]
    write_path(&socket_path, b"socket").await;

    assert!(
        !ensure_owner_is_usable(session_id, &owner, &temp_home.path).await.expect("inspect owner")
    );
    assert!(!queue_lock_file_path(session_id, &temp_home.path).exists());
    #[cfg(not(windows))]
    assert!(!socket_path.exists());
}

#[tokio::test]
async fn read_queue_owner_status_returns_none_when_record_is_missing() {
    let temp_home = TempHome::new("status-missing").await;

    assert_eq!(
        read_queue_owner_status("session-status-missing", &temp_home.path)
            .await
            .expect("read missing status"),
        None
    );
}

#[tokio::test]
async fn read_queue_owner_status_cleans_owner_that_is_not_alive() {
    let temp_home = TempHome::new("status-unusable").await;
    let session_id = "session-status-unusable";
    let owner = owner_record(
        session_id,
        std::process::id(),
        temp_home.path.join("unusable.sock"),
        "2999-01-01T00:00:00Z",
    );
    write_owner_record(&temp_home.path, &owner).await;

    assert_eq!(
        read_queue_owner_status(session_id, &temp_home.path).await.expect("read unusable status"),
        None
    );
    assert!(!queue_lock_file_path(session_id, &temp_home.path).exists());
}

#[tokio::test]
async fn try_acquire_queue_owner_lease_creates_record_and_removes_old_socket() {
    let temp_home = TempHome::new("acquire-new").await;
    let session_id = "session-acquire-new";
    let socket_path = queue_socket_path(session_id, &temp_home.path);
    #[cfg(not(windows))]
    write_path(&socket_path, b"old socket").await;

    let lease = try_acquire_queue_owner_lease_with_now(session_id, &temp_home.path, || {
        "2026-07-16T01:02:03Z".to_string()
    })
    .await
    .expect("acquire owner lease")
    .expect("lease should be acquired");

    assert_eq!(lease.session_id, session_id);
    assert_eq!(lease.lock_path, queue_lock_file_path(session_id, &temp_home.path));
    assert_eq!(lease.socket_path, socket_path);
    assert_eq!(lease.created_at, "2026-07-16T01:02:03Z");
    assert_ne!(lease.owner_generation, 0);
    #[cfg(not(windows))]
    assert!(!lease.socket_path.exists());

    let raw = tokio::fs::read_to_string(&lease.lock_path).await.expect("read lease file");
    let record = parse_queue_owner_record(&raw).expect("parse lease record");
    assert!(raw.ends_with('\n'));
    assert_eq!(record.pid, std::process::id());
    assert_eq!(record.session_id, session_id);
    assert_eq!(record.socket_path, lease.socket_path);
    assert_eq!(record.created_at, lease.created_at);
    assert_eq!(record.heartbeat_at, lease.created_at);
    assert_eq!(record.owner_generation, lease.owner_generation);
    assert_eq!(record.queue_depth, 0);
}

#[tokio::test]
async fn try_acquire_queue_owner_lease_cleans_invalid_existing_record() {
    let temp_home = TempHome::new("acquire-invalid").await;
    let session_id = "session-acquire-invalid";
    let lock_path = queue_lock_file_path(session_id, &temp_home.path);
    let socket_path = queue_socket_path(session_id, &temp_home.path);
    write_path(&lock_path, b"invalid").await;
    #[cfg(not(windows))]
    write_path(&socket_path, b"old socket").await;

    let lease = try_acquire_queue_owner_lease_with_now(session_id, &temp_home.path, || {
        "2026-07-16T01:02:03Z".to_string()
    })
    .await
    .expect("inspect existing lease");

    assert_eq!(lease, None);
    assert!(!lock_path.exists());
    #[cfg(not(windows))]
    assert!(!socket_path.exists());
}

#[tokio::test]
async fn try_acquire_queue_owner_lease_cleans_existing_dead_owner() {
    let temp_home = TempHome::new("acquire-dead").await;
    let session_id = "session-acquire-dead";
    let socket_path = temp_home.path.join("dead.sock");
    let owner =
        owner_record(session_id, std::process::id(), socket_path.clone(), "2999-01-01T00:00:00Z");
    write_owner_record(&temp_home.path, &owner).await;
    #[cfg(not(windows))]
    write_path(&socket_path, b"socket").await;

    let lease = try_acquire_queue_owner_lease_with_now(session_id, &temp_home.path, || {
        "2026-07-16T01:02:03Z".to_string()
    })
    .await
    .expect("inspect dead owner");

    assert_eq!(lease, None);
    assert!(!queue_lock_file_path(session_id, &temp_home.path).exists());
    #[cfg(not(windows))]
    assert!(!socket_path.exists());
}

#[tokio::test]
async fn try_acquire_queue_owner_lease_returns_error_when_queue_dir_is_file() {
    let temp_home = TempHome::new("acquire-dir-error").await;
    let base_dir = queue_base_dir(&temp_home.path);
    write_path(&base_dir, b"not a directory").await;

    assert!(
        try_acquire_queue_owner_lease_with_now(
            "session-acquire-dir-error",
            &temp_home.path,
            || "2026-07-16T01:02:03Z".to_string(),
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn try_acquire_queue_owner_lease_uses_generated_timestamp() {
    let temp_home = TempHome::new("acquire-generated-time").await;

    let lease = try_acquire_queue_owner_lease("session-generated-time", &temp_home.path)
        .await
        .expect("acquire owner lease")
        .expect("lease should be acquired");

    assert!(OffsetDateTime::parse(&lease.created_at, &Rfc3339).is_ok());
}

#[tokio::test]
async fn refresh_queue_owner_lease_persists_updated_heartbeat_and_depth() {
    let temp_home = TempHome::new("refresh").await;
    let lease = QueueOwnerLease {
        session_id: "session-refresh".to_string(),
        lock_path: queue_lock_file_path("session-refresh", &temp_home.path),
        socket_path: temp_home.path.join("refresh.sock"),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        owner_generation: 99,
    };
    write_path(&lease.lock_path, b"existing lease").await;

    refresh_queue_owner_lease_with_now(&lease, 12, || "2026-07-16T04:05:06Z".to_string())
        .await
        .expect("refresh owner lease");

    let raw = tokio::fs::read_to_string(&lease.lock_path).await.expect("read refreshed lease");
    let record = parse_queue_owner_record(&raw).expect("parse refreshed lease");
    assert!(raw.ends_with('\n'));
    assert_eq!(record.pid, std::process::id());
    assert_eq!(record.session_id, lease.session_id);
    assert_eq!(record.socket_path, lease.socket_path);
    assert_eq!(record.created_at, lease.created_at);
    assert_eq!(record.heartbeat_at, "2026-07-16T04:05:06Z");
    assert_eq!(record.owner_generation, lease.owner_generation);
    assert_eq!(record.queue_depth, 12);
}

#[tokio::test]
async fn refresh_queue_owner_lease_uses_generated_heartbeat() {
    let temp_home = TempHome::new("refresh-generated-time").await;
    let lease = QueueOwnerLease {
        session_id: "session-refresh-generated".to_string(),
        lock_path: queue_lock_file_path("session-refresh-generated", &temp_home.path),
        socket_path: temp_home.path.join("refresh-generated.sock"),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        owner_generation: 101,
    };
    write_path(&lease.lock_path, b"existing lease").await;

    refresh_queue_owner_lease(&lease, 3).await.expect("refresh owner lease");

    let record = read_queue_owner_record("session-refresh-generated", &temp_home.path)
        .await
        .expect("read refreshed record");
    assert!(OffsetDateTime::parse(&record.heartbeat_at, &Rfc3339).is_ok());
    assert_eq!(record.queue_depth, 3);
}

#[tokio::test]
async fn release_queue_owner_lease_removes_files_and_is_idempotent() {
    let temp_home = TempHome::new("release").await;
    let lease = QueueOwnerLease {
        session_id: "session-release".to_string(),
        lock_path: queue_lock_file_path("session-release", &temp_home.path),
        socket_path: temp_home.path.join("release.sock"),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        owner_generation: 7,
    };
    write_path(&lease.lock_path, b"lock").await;
    #[cfg(not(windows))]
    write_path(&lease.socket_path, b"socket").await;

    release_queue_owner_lease(&lease).await.expect("release owner lease");
    assert!(!lease.lock_path.exists());
    #[cfg(not(windows))]
    assert!(!lease.socket_path.exists());

    release_queue_owner_lease(&lease).await.expect("release missing owner lease");
}

#[tokio::test]
async fn terminate_queue_owner_for_session_tolerates_missing_record() {
    let temp_home = TempHome::new("terminate-missing").await;

    terminate_queue_owner_for_session("session-terminate-missing", &temp_home.path)
        .await
        .expect("ignore missing owner");
}

#[tokio::test]
async fn terminate_queue_owner_for_session_cleans_owner_that_is_not_alive() {
    let temp_home = TempHome::new("terminate-dead").await;
    let session_id = "session-terminate-dead";
    let socket_path = temp_home.path.join("terminate.sock");
    let owner =
        owner_record(session_id, std::process::id(), socket_path.clone(), "2999-01-01T00:00:00Z");
    write_owner_record(&temp_home.path, &owner).await;
    #[cfg(not(windows))]
    write_path(&socket_path, b"socket").await;

    terminate_queue_owner_for_session(session_id, &temp_home.path)
        .await
        .expect("cleanup dead owner");

    assert!(!queue_lock_file_path(session_id, &temp_home.path).exists());
    #[cfg(not(windows))]
    assert!(!socket_path.exists());
}

#[tokio::test]
async fn wait_ms_completes_for_zero_duration() {
    assert!(tokio::time::timeout(std::time::Duration::from_millis(50), wait_ms(0)).await.is_ok());
}
