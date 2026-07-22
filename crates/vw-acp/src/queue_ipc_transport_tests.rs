use super::*;

use std::error::Error as StdError;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::types::OutputErrorOrigin;

static NEXT_SOCKET_ID: AtomicUsize = AtomicUsize::new(1);

fn owner_record(socket_path: PathBuf) -> QueueOwnerRecord {
    QueueOwnerRecord {
        pid: 123,
        session_id: "session-1".to_string(),
        socket_path,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        heartbeat_at: "2026-01-01T00:00:00Z".to_string(),
        owner_generation: 1,
        queue_depth: 0,
    }
}

fn temp_socket_path(_name: &str) -> PathBuf {
    let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("vqt-{}-{id}.sock", std::process::id()))
}

#[cfg(unix)]
struct TempSocket {
    path: PathBuf,
}

#[cfg(unix)]
impl TempSocket {
    fn new(name: &str) -> Self {
        let path = temp_socket_path(name);
        let _ = std::fs::remove_file(&path);
        Self { path }
    }
}

#[cfg(unix)]
impl Drop for TempSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn socket_connection_timeout_error_formats_timeout_message() {
    let error = socket_connection_timeout_error(Path::new("/tmp/owner.sock"), 1_500);

    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert_eq!(error.to_string(), "Connection to /tmp/owner.sock timed out after 1500ms");
}

#[test]
fn should_retry_queue_connect_classifies_retryable_errors() {
    assert!(should_retry_queue_connect(&io::Error::new(ErrorKind::NotFound, "missing socket")));
    assert!(should_retry_queue_connect(&io::Error::new(
        ErrorKind::ConnectionRefused,
        "owner not listening"
    )));
    assert!(!should_retry_queue_connect(&io::Error::new(
        ErrorKind::PermissionDenied,
        "permission denied"
    )));

    #[cfg(windows)]
    {
        assert!(should_retry_queue_connect(&io::Error::from_raw_os_error(231)));
        assert!(should_retry_queue_connect(&io::Error::from_raw_os_error(233)));
    }
}

#[test]
fn to_queue_connection_error_marks_retryable_queue_origin() {
    let error = to_queue_connection_error(io::Error::new(ErrorKind::ConnectionRefused, "closed"));

    assert_eq!(error.message(), "closed");
    assert_eq!(error.origin(), Some(OutputErrorOrigin::Queue));
    assert_eq!(error.retryable(), Some(true));
    assert_eq!(StdError::source(&error).map(ToString::to_string).as_deref(), Some("closed"));
}

#[test]
fn to_queue_connection_error_marks_non_retryable_queue_origin() {
    let error = to_queue_connection_error(io::Error::new(ErrorKind::PermissionDenied, "denied"));

    assert_eq!(error.message(), "denied");
    assert_eq!(error.origin(), Some(OutputErrorOrigin::Queue));
    assert_eq!(error.retryable(), Some(false));
}

#[tokio::test]
async fn connect_to_socket_rejects_zero_timeout() {
    let error = match connect_to_socket(Path::new("owner.sock"), 0).await {
        Ok(_) => panic!("zero timeout should fail before connecting"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert_eq!(error.to_string(), "Connection to owner.sock timed out after 0ms");
}

#[tokio::test]
async fn connect_to_queue_owner_returns_none_after_retryable_failure() {
    let owner = owner_record(temp_socket_path("missing-owner"));

    let connection = connect_to_queue_owner(&owner, Some(0)).await.expect("retryable failure");

    assert!(connection.is_none());
}

#[tokio::test]
async fn connect_to_queue_owner_retries_then_returns_none() {
    // 用 >=2 的尝试次数走重试分支（attempt + 1 < attempts），最终所有尝试都因
    // socket 不存在而可重试，返回 None 而非错误。
    let owner = owner_record(temp_socket_path("retry-missing-owner"));

    let connection =
        connect_to_queue_owner(&owner, Some(3)).await.expect("retryable failures stay ok");

    assert!(connection.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn connect_to_queue_owner_retries_until_owner_appears() {
    // 第一次尝试时 socket 还不存在（可重试失败），随后绑定监听器，重试应当成功。
    let socket = TempSocket::new("retry-live-owner");
    let owner = owner_record(socket.path.clone());

    let bind_path = socket.path.clone();
    let binder = tokio::spawn(async move {
        wait_ms(QUEUE_CONNECT_RETRY_MS).await;
        tokio::net::UnixListener::bind(&bind_path).expect("bind owner socket")
    });

    let connection = connect_to_queue_owner(&owner, Some(40)).await.expect("connect after retries");
    let _listener = binder.await.expect("listener task");

    assert!(connection.is_some());
}

#[cfg(unix)]
#[tokio::test]
async fn connect_to_queue_owner_returns_live_unix_connection() {
    let socket = TempSocket::new("live-owner");
    let _listener = tokio::net::UnixListener::bind(&socket.path).expect("bind owner socket");
    let owner = owner_record(socket.path.clone());

    let connection = connect_to_queue_owner(&owner, Some(1)).await.expect("connect owner");

    assert!(connection.is_some());
}

#[cfg(unix)]
#[tokio::test]
async fn connect_to_queue_owner_uses_default_attempts_on_first_success() {
    // max_attempts 为 None 时走默认尝试次数分支；活的 socket 让首次尝试即成功，
    // 避免真正耗尽 40 次重试。
    let socket = TempSocket::new("default-attempts-owner");
    let _listener = tokio::net::UnixListener::bind(&socket.path).expect("bind owner socket");
    let owner = owner_record(socket.path.clone());

    let connection = connect_to_queue_owner(&owner, None).await.expect("connect owner");

    assert!(connection.is_some());
}

#[cfg(unix)]
#[tokio::test]
async fn connect_to_queue_owner_returns_error_for_non_retryable_failure() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let owner = owner_record(PathBuf::from(OsString::from_vec(b"bad\0socket".to_vec())));

    let error = match connect_to_queue_owner(&owner, Some(1)).await {
        Ok(_) => panic!("non retryable failure should return an error"),
        Err(error) => error,
    };

    assert_eq!(error.origin(), Some(OutputErrorOrigin::Queue));
    assert_eq!(error.retryable(), Some(false));
}
