//! ACP actor 生命周期状态记录。
//!
//! 本模块只维护 actor 句柄、可复用会话 id 和代理进程生命周期快照，避免这些
//! 状态更新散落在 actor 调度逻辑中。

use super::*;
use crate::session_runtime::AgentLifecycleExit;

impl AcpClient {
    pub(super) fn invalidate_actor(&self) {
        let mut state = self.actor_state.lock();
        state.handle = None;
        state.reusable_session_id = None;
        state.lifecycle.pid = None;
    }

    /// 判断指定会话是否正被当前 actor 运行时复用。
    pub fn has_reusable_session(&self, session_id: &str) -> bool {
        self.actor_state
            .lock()
            .reusable_session_id
            .as_deref()
            .is_some_and(|active_session_id| active_session_id == session_id)
    }

    /// 获取当前代理进程生命周期快照。
    pub fn get_agent_lifecycle_snapshot(&self) -> AgentLifecycleSnapshot {
        self.actor_state.lock().lifecycle.clone()
    }

    pub(super) fn record_actor_start(&self, pid: Option<u32>) {
        let mut state = self.actor_state.lock();
        state.lifecycle.pid = pid;
        state.lifecycle.started_at = Some(iso_now());
        state.lifecycle.last_exit = None;
    }

    pub(super) fn record_actor_exit(
        &self,
        exit: ChildExitSummary,
        reason: Option<&str>,
        unexpected_during_prompt: bool,
    ) {
        let mut state = self.actor_state.lock();
        state.lifecycle.pid = None;
        state.lifecycle.last_exit = Some(AgentLifecycleExit {
            exit_code: exit.exit_code,
            signal: exit.signal,
            exited_at: Some(iso_now()),
            reason: reason.map(ToOwned::to_owned),
            unexpected_during_prompt,
        });
    }

    pub(super) fn store_reusable_session(&self, session_id: Option<String>) {
        self.actor_state.lock().reusable_session_id = session_id;
    }
}

/// Sanitized outcome of an explicit initialize-only connection probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDiagnostic {
    StartupFailed,
    HandshakeFailed,
    AuthenticationRequired,
    Connected,
    TimedOut,
    Cancelled,
}

struct DiagnosticClient;

#[async_trait::async_trait(?Send)]
impl acp::Client for DiagnosticClient {
    async fn request_permission(
        &self,
        _: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }
    async fn session_notification(&self, _: acp::SessionNotification) -> acp::Result<()> {
        Ok(())
    }
}

impl AcpClient {
    /// Probe initialization without authentication, sessions or client capabilities.
    /// Cancellation requests cleanup; dropping the waiter also requests cleanup.
    pub async fn diagnose_connection(
        &self,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> ConnectionDiagnostic {
        let config = self.config.clone();
        let (tx, rx) = oneshot::channel();
        let spawned = thread::Builder::new().name("acp-diagnostic".into()).spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
                let _ = tx.send(ConnectionDiagnostic::StartupFailed);
                return;
            };
            runtime.block_on(tokio::task::LocalSet::new().run_until(async move {
                let client = AcpClient::new("connection-diagnostic", config);
                let mut tx = tx;
                if cancellation.is_cancelled() || tx.is_closed() {
                    let _ = tx.send(ConnectionDiagnostic::Cancelled);
                    return;
                }
                let Ok(ProcessHandles { mut child, stderr_task }) = client.spawn_child_with_stderr(true) else {
                    let _ = tx.send(ConnectionDiagnostic::StartupFailed);
                    return;
                };
                // Diagnostic spawning directs stderr to the null device.
                debug_assert!(stderr_task.is_none());
                let targets = process_cleanup_targets(child.id());
                let outgoing = child.stdin.take().expect("piped stdin").compat_write();
                let incoming = child.stdout.take().expect("piped stdout").compat();
                let (conn, io) = acp::ClientSideConnection::new(DiagnosticClient, outgoing, incoming, |future| { tokio::task::spawn_local(future); });
                let io_task = tokio::task::spawn_local(async move { let _ = io.await; });
                let result = tokio::select! {
                    biased;
                    _ = cancellation.cancelled() => ConnectionDiagnostic::Cancelled,
                    _ = tx.closed() => ConnectionDiagnostic::Cancelled,
                    response = timeout(Duration::from_secs(10), conn.initialize(acp::InitializeRequest::new(acp::ProtocolVersion::V1))) => match response {
                        Err(_) => ConnectionDiagnostic::TimedOut,
                        Ok(Err(_)) => ConnectionDiagnostic::HandshakeFailed,
                        Ok(Ok(response)) if !response.auth_methods.is_empty() => ConnectionDiagnostic::AuthenticationRequired,
                        Ok(Ok(_)) => ConnectionDiagnostic::Connected,
                    }
                };
                drop(conn);
                io_task.abort();
                let _ = io_task.await;
                let _ = client.finalize_child_with_cleanup_targets(child, None, targets).await;
                let _ = tx.send(result);
            }));
        });
        if spawned.is_err() {
            return ConnectionDiagnostic::StartupFailed;
        }
        rx.await.unwrap_or(ConnectionDiagnostic::StartupFailed)
    }
}
