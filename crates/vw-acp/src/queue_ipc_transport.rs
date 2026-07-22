//! 队列 IPC 连接建立、重试与传输封装。

use std::cmp::max;
use std::io;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use tokio::time::timeout;

use crate::errors::{AcpxErrorOptions, QueueConnectionError};
use crate::perf_metrics::measure_perf;
use crate::queue_lease_store::{QueueOwnerRecord, wait_ms};
use crate::types::OutputErrorOrigin;

const QUEUE_CONNECT_ATTEMPTS: usize = 40;
pub const QUEUE_CONNECT_RETRY_MS: u64 = 50;
pub const SOCKET_CONNECTION_TIMEOUT_MS: u64 = 5_000;

#[cfg(unix)]
pub type QueueOwnerConnection = tokio::net::UnixStream;

#[cfg(windows)]
pub type QueueOwnerConnection = tokio::fs::File;

fn socket_connection_timeout_error(socket_path: &Path, timeout_ms: u64) -> io::Error {
    io::Error::new(
        ErrorKind::TimedOut,
        format!("Connection to {} timed out after {timeout_ms}ms", socket_path.display()),
    )
}

fn should_retry_queue_connect(error: &io::Error) -> bool {
    if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::ConnectionRefused) {
        return true;
    }

    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(231 | 233))
    }

    #[cfg(not(windows))]
    {
        false
    }
}

fn to_queue_connection_error(error: io::Error) -> QueueConnectionError {
    let retryable = should_retry_queue_connect(&error);
    QueueConnectionError::new(
        error.to_string(),
        AcpxErrorOptions {
            source: Some(Box::new(error)),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(retryable),
            ..AcpxErrorOptions::default()
        },
    )
}

#[cfg(unix)]
async fn connect_to_socket(
    socket_path: &Path,
    timeout_ms: u64,
) -> io::Result<QueueOwnerConnection> {
    if timeout_ms == 0 {
        return Err(socket_connection_timeout_error(socket_path, timeout_ms));
    }

    timeout(Duration::from_millis(timeout_ms), tokio::net::UnixStream::connect(socket_path))
        .await
        .map_err(|_| socket_connection_timeout_error(socket_path, timeout_ms))?
}

#[cfg(windows)]
async fn connect_to_socket(
    socket_path: &Path,
    timeout_ms: u64,
) -> io::Result<QueueOwnerConnection> {
    if timeout_ms == 0 {
        return Err(socket_connection_timeout_error(socket_path, timeout_ms));
    }

    timeout(
        Duration::from_millis(timeout_ms),
        tokio::fs::OpenOptions::new().read(true).write(true).open(socket_path),
    )
    .await
    .map_err(|_| socket_connection_timeout_error(socket_path, timeout_ms))?
}

pub async fn connect_to_queue_owner(
    owner: &QueueOwnerRecord,
    max_attempts: Option<usize>,
) -> Result<Option<QueueOwnerConnection>, QueueConnectionError> {
    let attempts = max(1, max_attempts.unwrap_or(QUEUE_CONNECT_ATTEMPTS));
    let mut attempt = 0;

    loop {
        match measure_perf("queue.connect", || async {
            connect_to_socket(&owner.socket_path, SOCKET_CONNECTION_TIMEOUT_MS).await
        })
        .await
        {
            Ok(connection) => return Ok(Some(connection)),
            Err(error) if should_retry_queue_connect(&error) && attempt + 1 < attempts => {
                attempt += 1;
                wait_ms(QUEUE_CONNECT_RETRY_MS).await;
            }
            Err(error) if should_retry_queue_connect(&error) => return Ok(None),
            Err(error) => return Err(to_queue_connection_error(error)),
        }
    }
}

#[cfg(test)]
#[path = "queue_ipc_transport_tests.rs"]
mod queue_ipc_transport_tests;
