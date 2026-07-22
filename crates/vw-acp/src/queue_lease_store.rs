//! 队列所有者租约、心跳与进程状态存储。
//!
//! 本模块负责把队列所有者的进程元数据落到磁盘，以便多个 CLI 进程可以就同一个
//! 会话协调后台所有者的生命周期。
//!
//! 这里主要处理锁文件、租约续期、进程活性检查和失效清理，
//! 是保证“同一会话只存在一个可用 owner”的关键基础设施。

use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::time::sleep;

use crate::queue_paths::{
    default_home_dir, queue_base_dir, queue_lock_file_path, queue_socket_base_dir,
    queue_socket_path,
};

const PROCESS_EXIT_GRACE_MS: u64 = 1_500;
const PROCESS_POLL_MS: u64 = 50;
const QUEUE_OWNER_STALE_HEARTBEAT_MS: i128 = 15_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueOwnerRecord {
    pub pid: u32,
    pub session_id: String,
    pub socket_path: PathBuf,
    pub created_at: String,
    pub heartbeat_at: String,
    pub owner_generation: u64,
    pub queue_depth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueOwnerLease {
    pub session_id: String,
    pub lock_path: PathBuf,
    pub socket_path: PathBuf,
    pub created_at: String,
    pub owner_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueOwnerStatus {
    pub pid: u32,
    pub socket_path: PathBuf,
    pub heartbeat_at: String,
    pub owner_generation: u64,
    pub queue_depth: u64,
    pub alive: bool,
    pub stale: bool,
}

fn parse_queue_owner_record(raw: &str) -> Option<QueueOwnerRecord> {
    let record = serde_json::from_str::<QueueOwnerRecord>(raw).ok()?;
    if record.pid == 0 || record.owner_generation == 0 {
        return None;
    }
    Some(record)
}

fn create_owner_generation() -> u64 {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    elapsed.as_secs().saturating_mul(1_000_000).saturating_add(u64::from(elapsed.subsec_micros()))
}

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn is_queue_owner_heartbeat_stale(owner: &QueueOwnerRecord) -> bool {
    let Ok(heartbeat) = OffsetDateTime::parse(&owner.heartbeat_at, &Rfc3339) else {
        return true;
    };

    let age_ms = (OffsetDateTime::now_utc() - heartbeat).whole_milliseconds();
    age_ms > QUEUE_OWNER_STALE_HEARTBEAT_MS
}

async fn ensure_queue_dir(home_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(queue_base_dir(home_dir)).await?;
    if let Some(socket_dir) = queue_socket_base_dir(home_dir) {
        fs::create_dir_all(socket_dir).await?;
    }
    Ok(())
}

async fn remove_socket_file(socket_path: &Path) -> io::Result<()> {
    if cfg!(windows) {
        return Ok(());
    }

    match fs::remove_file(socket_path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

async fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() <= deadline {
        if !is_process_alive(Some(pid)) {
            return true;
        }
        wait_ms(PROCESS_POLL_MS).await;
    }

    !is_process_alive(Some(pid))
}

async fn cleanup_stale_queue_owner(
    session_id: &str,
    owner: Option<&QueueOwnerRecord>,
    home_dir: &Path,
) -> io::Result<()> {
    let lock_path = queue_lock_file_path(session_id, home_dir);
    let socket_path = owner
        .map(|record| record.socket_path.clone())
        .unwrap_or_else(|| queue_socket_path(session_id, home_dir));

    let _ = remove_socket_file(&socket_path).await;

    match fs::remove_file(lock_path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

async fn read_queue_owner_record_from_path(lock_path: &Path) -> Option<QueueOwnerRecord> {
    let payload = fs::read_to_string(lock_path).await.ok()?;
    parse_queue_owner_record(&payload)
}

fn default_home_dir_required() -> io::Result<PathBuf> {
    default_home_dir()
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "home directory is unavailable"))
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: i32) -> bool {
    let result = unsafe { libc::kill(pid as i32, signal) };
    if result == 0 {
        return true;
    }

    let error = io::Error::last_os_error();
    matches!(error.raw_os_error(), Some(code) if code == libc::EPERM)
}

#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
fn tasklist_output(pid: u32) -> Option<String> {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(windows)]
fn run_taskkill(pid: u32, force: bool) -> bool {
    let mut command = Command::new("taskkill");
    command.args(["/PID", &pid.to_string()]);
    if force {
        command.arg("/F");
    }
    command.status().map(|status| status.success()).unwrap_or(false)
}

pub async fn read_queue_owner_record(
    session_id: &str,
    home_dir: impl AsRef<Path>,
) -> Option<QueueOwnerRecord> {
    read_queue_owner_record_from_path(&queue_lock_file_path(session_id, home_dir)).await
}

pub async fn read_default_queue_owner_record(session_id: &str) -> Option<QueueOwnerRecord> {
    let home_dir = default_home_dir()?;
    read_queue_owner_record(session_id, home_dir).await
}

pub fn is_process_alive(pid: Option<u32>) -> bool {
    let Some(pid) = pid else {
        return false;
    };

    if pid == 0 || pid == std::process::id() {
        return false;
    }

    #[cfg(unix)]
    {
        send_signal(pid, 0)
    }

    #[cfg(windows)]
    {
        tasklist_output(pid)
            .map(|output| {
                output.lines().any(|line| !line.trim().is_empty() && !line.starts_with("INFO:"))
            })
            .unwrap_or(false)
    }
}

pub async fn terminate_process(pid: u32) -> bool {
    if !is_process_alive(Some(pid)) {
        return false;
    }

    #[cfg(unix)]
    if !send_signal(pid, libc::SIGTERM) {
        return false;
    }

    #[cfg(windows)]
    if !run_taskkill(pid, false) {
        return false;
    }

    if wait_for_process_exit(pid, Duration::from_millis(PROCESS_EXIT_GRACE_MS)).await {
        return true;
    }

    #[cfg(unix)]
    if !send_signal(pid, libc::SIGKILL) {
        return false;
    }

    #[cfg(windows)]
    if !run_taskkill(pid, true) {
        return false;
    }

    let _ = wait_for_process_exit(pid, Duration::from_millis(PROCESS_EXIT_GRACE_MS)).await;
    true
}

pub async fn ensure_owner_is_usable(
    session_id: &str,
    owner: &QueueOwnerRecord,
    home_dir: impl AsRef<Path>,
) -> io::Result<bool> {
    let home_dir = home_dir.as_ref();
    let alive = is_process_alive(Some(owner.pid));
    let stale = is_queue_owner_heartbeat_stale(owner);
    if alive && !stale {
        return Ok(true);
    }

    if alive {
        let _ = terminate_process(owner.pid).await;
    }
    cleanup_stale_queue_owner(session_id, Some(owner), home_dir).await?;
    Ok(false)
}

pub async fn read_queue_owner_status(
    session_id: &str,
    home_dir: impl AsRef<Path>,
) -> io::Result<Option<QueueOwnerStatus>> {
    let home_dir = home_dir.as_ref();
    let Some(owner) = read_queue_owner_record(session_id, home_dir).await else {
        return Ok(None);
    };

    let alive = ensure_owner_is_usable(session_id, &owner, home_dir).await?;
    if !alive {
        return Ok(None);
    }

    Ok(Some(QueueOwnerStatus {
        pid: owner.pid,
        socket_path: owner.socket_path.clone(),
        heartbeat_at: owner.heartbeat_at.clone(),
        owner_generation: owner.owner_generation,
        queue_depth: owner.queue_depth,
        alive,
        stale: is_queue_owner_heartbeat_stale(&owner),
    }))
}

pub async fn read_default_queue_owner_status(
    session_id: &str,
) -> io::Result<Option<QueueOwnerStatus>> {
    let home_dir = default_home_dir_required()?;
    read_queue_owner_status(session_id, home_dir).await
}

pub async fn try_acquire_queue_owner_lease(
    session_id: &str,
    home_dir: impl AsRef<Path>,
) -> io::Result<Option<QueueOwnerLease>> {
    try_acquire_queue_owner_lease_with_now(session_id, home_dir, now_iso).await
}

pub async fn try_acquire_default_queue_owner_lease(
    session_id: &str,
) -> io::Result<Option<QueueOwnerLease>> {
    let home_dir = default_home_dir_required()?;
    try_acquire_queue_owner_lease(session_id, home_dir).await
}

pub async fn try_acquire_queue_owner_lease_with_now<F>(
    session_id: &str,
    home_dir: impl AsRef<Path>,
    now_iso_factory: F,
) -> io::Result<Option<QueueOwnerLease>>
where
    F: Fn() -> String,
{
    let home_dir = home_dir.as_ref();
    ensure_queue_dir(home_dir).await?;
    let lock_path = queue_lock_file_path(session_id, home_dir);
    let socket_path = queue_socket_path(session_id, home_dir);
    let created_at = now_iso_factory();
    let record = QueueOwnerRecord {
        pid: std::process::id(),
        session_id: session_id.to_string(),
        socket_path: socket_path.clone(),
        created_at: created_at.clone(),
        heartbeat_at: created_at.clone(),
        owner_generation: create_owner_generation(),
        queue_depth: 0,
    };
    let payload =
        serde_json::to_vec_pretty(&record).map_err(|error| io::Error::other(error.to_string()))?;

    match OpenOptions::new().create_new(true).write(true).open(&lock_path).await {
        Ok(mut file) => {
            use tokio::io::AsyncWriteExt;

            file.write_all(&payload).await?;
            file.write_all(b"\n").await?;
            let _ = remove_socket_file(&socket_path).await;
            Ok(Some(QueueOwnerLease {
                session_id: session_id.to_string(),
                lock_path,
                socket_path,
                created_at,
                owner_generation: record.owner_generation,
            }))
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let owner = read_queue_owner_record(session_id, home_dir).await;
            if owner.is_none() {
                cleanup_stale_queue_owner(session_id, None, home_dir).await?;
                return Ok(None);
            }

            let owner = owner.expect("owner checked as some");
            if !is_process_alive(Some(owner.pid)) || is_queue_owner_heartbeat_stale(&owner) {
                if is_process_alive(Some(owner.pid)) {
                    let _ = terminate_process(owner.pid).await;
                }
                cleanup_stale_queue_owner(session_id, Some(&owner), home_dir).await?;
            }
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub async fn refresh_queue_owner_lease(
    lease: &QueueOwnerLease,
    queue_depth: u64,
) -> io::Result<()> {
    refresh_queue_owner_lease_with_now(lease, queue_depth, now_iso).await
}

pub async fn refresh_queue_owner_lease_with_now<F>(
    lease: &QueueOwnerLease,
    queue_depth: u64,
    now_iso_factory: F,
) -> io::Result<()>
where
    F: Fn() -> String,
{
    let record = QueueOwnerRecord {
        pid: std::process::id(),
        session_id: lease.session_id.clone(),
        socket_path: lease.socket_path.clone(),
        created_at: lease.created_at.clone(),
        heartbeat_at: now_iso_factory(),
        owner_generation: lease.owner_generation,
        queue_depth,
    };
    let payload =
        serde_json::to_vec_pretty(&record).map_err(|error| io::Error::other(error.to_string()))?;
    let mut output = payload;
    output.push(b'\n');
    fs::write(&lease.lock_path, output).await
}

pub async fn release_queue_owner_lease(lease: &QueueOwnerLease) -> io::Result<()> {
    let _ = remove_socket_file(&lease.socket_path).await;

    match fs::remove_file(&lease.lock_path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub async fn terminate_queue_owner_for_session(
    session_id: &str,
    home_dir: impl AsRef<Path>,
) -> io::Result<()> {
    let home_dir = home_dir.as_ref();
    let Some(owner) = read_queue_owner_record(session_id, home_dir).await else {
        return Ok(());
    };

    if is_process_alive(Some(owner.pid)) {
        let _ = terminate_process(owner.pid).await;
    }

    cleanup_stale_queue_owner(session_id, Some(&owner), home_dir).await
}

pub async fn terminate_default_queue_owner_for_session(session_id: &str) -> io::Result<()> {
    let home_dir = default_home_dir_required()?;
    terminate_queue_owner_for_session(session_id, home_dir).await
}

pub async fn wait_ms(ms: u64) {
    sleep(Duration::from_millis(ms)).await;
}

#[cfg(test)]
#[path = "queue_lease_store_tests.rs"]
mod queue_lease_store_tests;
