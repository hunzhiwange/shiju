use super::*;

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::queue_lease_store::QueueOwnerRecord;
use crate::queue_paths::{queue_base_dir, queue_lock_file_path, queue_socket_base_dir};

static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(1);

struct HomeEnvGuard {
    previous_home: Option<std::ffi::OsString>,
    previous_userprofile: Option<std::ffi::OsString>,
    previous_homedrive: Option<std::ffi::OsString>,
    previous_homepath: Option<std::ffi::OsString>,
}

impl HomeEnvGuard {
    fn set(home_dir: &Path) -> Self {
        let previous_home = std::env::var_os("HOME");
        let previous_userprofile = std::env::var_os("USERPROFILE");
        let previous_homedrive = std::env::var_os("HOMEDRIVE");
        let previous_homepath = std::env::var_os("HOMEPATH");
        unsafe { std::env::set_var("HOME", home_dir) };
        unsafe { std::env::set_var("USERPROFILE", home_dir) };
        unsafe { std::env::remove_var("HOMEDRIVE") };
        unsafe { std::env::remove_var("HOMEPATH") };
        Self { previous_home, previous_userprofile, previous_homedrive, previous_homepath }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        restore_env("HOME", self.previous_home.take());
        restore_env("USERPROFILE", self.previous_userprofile.take());
        restore_env("HOMEDRIVE", self.previous_homedrive.take());
        restore_env("HOMEPATH", self.previous_homepath.take());
    }
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => unsafe { std::env::set_var(key, value) },
        None => unsafe { std::env::remove_var(key) },
    }
}

struct TempHome {
    path: PathBuf,
}

impl TempHome {
    async fn new(name: &str) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir()
            .join(format!("vw-acp-queue-health-{name}-{}-{id}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&path).await;
        tokio::fs::create_dir_all(&path).await.expect("create temp home");
        Self { path }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
        #[cfg(not(windows))]
        if let Some(socket_dir) = queue_socket_base_dir(&self.path) {
            let _ = std::fs::remove_dir_all(socket_dir);
        }
    }
}

#[cfg(not(windows))]
struct TempSocket {
    path: PathBuf,
}

#[cfg(not(windows))]
impl TempSocket {
    fn new(name: &str) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("vwqh-{name}-{}-{id}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }
}

#[cfg(not(windows))]
impl Drop for TempSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

struct LiveProcess {
    child: Child,
}

impl LiveProcess {
    fn spawn() -> Self {
        #[cfg(windows)]
        let child = Command::new("cmd")
            .args(["/C", "timeout /T 5 /NOBREAK > NUL"])
            .spawn()
            .expect("spawn live process");

        #[cfg(not(windows))]
        let child = Command::new("sleep").arg("5").spawn().expect("spawn live process");

        Self { child }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for LiveProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn now_iso() -> String {
    OffsetDateTime::now_utc().format(&Rfc3339).expect("format timestamp")
}

async fn write_owner_record(home_dir: &Path, record: &QueueOwnerRecord) {
    tokio::fs::create_dir_all(queue_base_dir(home_dir)).await.expect("create queue dir");
    if let Some(socket_dir) = queue_socket_base_dir(home_dir) {
        tokio::fs::create_dir_all(socket_dir).await.expect("create socket dir");
    }
    let payload = serde_json::to_vec_pretty(record).expect("serialize owner record");
    tokio::fs::write(queue_lock_file_path(&record.session_id, home_dir), payload)
        .await
        .expect("write owner record");
}

fn owner_record(session_id: &str, pid: u32, socket_path: PathBuf) -> QueueOwnerRecord {
    let timestamp = now_iso();
    QueueOwnerRecord {
        pid,
        session_id: session_id.to_string(),
        socket_path,
        created_at: timestamp.clone(),
        heartbeat_at: timestamp,
        owner_generation: 7,
        queue_depth: 42,
    }
}

fn assert_empty_health(health: QueueOwnerHealth, session_id: &str) {
    assert_eq!(health.session_id, session_id);
    assert!(!health.has_lease);
    assert!(!health.healthy);
    assert!(!health.socket_reachable);
    assert!(!health.pid_alive);
    assert_eq!(health.pid, None);
    assert_eq!(health.socket_path, None);
    assert_eq!(health.owner_generation, None);
    assert_eq!(health.queue_depth, None);
}

#[tokio::test]
async fn probe_queue_owner_health_reports_empty_when_lease_is_missing() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("missing-lease").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);

    let health = probe_queue_owner_health("session-missing").await;

    assert_empty_health(health, "session-missing");
}

#[tokio::test]
async fn probe_queue_owner_health_reports_empty_when_status_is_unusable() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("unusable-status").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);
    let session_id = "session-unusable";
    let record = owner_record(session_id, std::process::id(), temp_home.path.join("missing.sock"));
    write_owner_record(&temp_home.path, &record).await;

    let health = probe_queue_owner_health(session_id).await;

    assert_empty_health(health, session_id);
}

#[tokio::test]
async fn probe_queue_owner_health_reports_unhealthy_when_socket_is_unreachable() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("unreachable-socket").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);
    let process = LiveProcess::spawn();
    let session_id = "session-unreachable";
    let socket_path = temp_home.path.join("missing.sock");
    let record = owner_record(session_id, process.pid(), socket_path.clone());
    write_owner_record(&temp_home.path, &record).await;

    let health = probe_queue_owner_health(session_id).await;

    assert_eq!(
        health,
        QueueOwnerHealth {
            session_id: session_id.to_string(),
            has_lease: true,
            healthy: false,
            socket_reachable: false,
            pid_alive: true,
            pid: Some(process.pid()),
            socket_path: Some(socket_path),
            owner_generation: Some(7),
            queue_depth: Some(42),
        }
    );
}

#[cfg(not(windows))]
#[tokio::test]
async fn probe_queue_owner_health_reports_healthy_when_socket_is_reachable() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("reachable-socket").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);
    let process = LiveProcess::spawn();
    let session_id = "session-reachable";
    let socket = TempSocket::new("reachable");
    let _listener = tokio::net::UnixListener::bind(&socket.path).expect("bind owner socket");
    let record = owner_record(session_id, process.pid(), socket.path.clone());
    write_owner_record(&temp_home.path, &record).await;

    let health = probe_queue_owner_health(session_id).await;

    assert_eq!(
        health,
        QueueOwnerHealth {
            session_id: session_id.to_string(),
            has_lease: true,
            healthy: true,
            socket_reachable: true,
            pid_alive: true,
            pid: Some(process.pid()),
            socket_path: Some(socket.path.clone()),
            owner_generation: Some(7),
            queue_depth: Some(42),
        }
    );
}
