//! 会话运行时的超时、中断与等待辅助函数。

use std::future::Future;
use std::time::Duration;

#[cfg(test)]
#[path = "session_runtime_helpers_tests.rs"]
mod session_runtime_helpers_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Timed out after {timeout_ms}ms")]
pub struct TimeoutError {
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Interrupted")]
pub struct InterruptedError;

#[derive(Debug, thiserror::Error)]
pub enum WithInterruptError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error(transparent)]
    Inner(E),
    #[error(transparent)]
    Interrupted(#[from] InterruptedError),
    #[error("failed to install signal handler: {0}")]
    SignalHandler(#[source] std::io::Error),
}

pub async fn with_timeout<T, F>(future: F, timeout_ms: Option<u64>) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    let Some(timeout_ms) = timeout_ms.filter(|timeout_ms| *timeout_ms > 0) else {
        return Ok(future.await);
    };

    tokio::time::timeout(Duration::from_millis(timeout_ms), future)
        .await
        .map_err(|_| TimeoutError { timeout_ms })
}

#[cfg(unix)]
async fn wait_for_interrupt_signal() -> Result<(), std::io::Error> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sighup = signal(SignalKind::hangup())?;

    tokio::select! {
        _ = sigint.recv() => Ok(()),
        _ = sigterm.recv() => Ok(()),
        _ = sighup.recv() => Ok(()),
    }
}

#[cfg(not(unix))]
async fn wait_for_interrupt_signal() -> Result<(), std::io::Error> {
    tokio::signal::ctrl_c().await
}

pub async fn with_interrupt<T, E, Run, RunFuture, OnInterrupt, OnInterruptFuture>(
    run: Run,
    on_interrupt: OnInterrupt,
) -> Result<T, WithInterruptError<E>>
where
    E: std::error::Error + Send + Sync + 'static,
    Run: FnOnce() -> RunFuture,
    RunFuture: Future<Output = Result<T, E>>,
    OnInterrupt: Fn() -> OnInterruptFuture,
    OnInterruptFuture: Future<Output = ()>,
{
    let run_future = run();
    tokio::pin!(run_future);

    let interrupt_future = wait_for_interrupt_signal();
    tokio::pin!(interrupt_future);

    tokio::select! {
        result = &mut run_future => result.map_err(WithInterruptError::Inner),
        signal_result = &mut interrupt_future => {
            signal_result.map_err(WithInterruptError::SignalHandler)?;
            on_interrupt().await;
            Err(InterruptedError.into())
        }
    }
}
