//! ACP actor runtime 管理。
//!
//! 本模块负责按需创建、复用、重启和关闭承载 ACP 连接的 runtime。actor 主循环只
//! 持有 `Option<ActorRuntime>` 并在命令到来时调用这里的窄方法。

use super::*;

impl AcpClient {
    pub(super) fn actor_runtime_restart_reason(
        &self,
        runtime: &mut ActorRuntime,
        cwd: &Path,
    ) -> Option<&'static str> {
        if runtime.cwd != cwd {
            return Some("cwd_changed");
        }
        if runtime.io_task.is_finished() {
            return Some("io_closed");
        }
        match runtime.child.try_wait() {
            Ok(Some(_)) => Some("agent_exited"),
            Ok(None) => None,
            Err(err) => {
                tracing::warn!(
                    target: "vw_acp",
                    acp_agent = %self.agent_name,
                    error = %err,
                    "failed to query ACP agent process state before reusing runtime"
                );
                Some("child_wait_error")
            }
        }
    }

    pub(super) async fn prepare_actor_runtime(
        &self,
        runtime: &mut Option<ActorRuntime>,
        cwd: PathBuf,
    ) -> Result<(), AcpError> {
        let restart_reason = runtime
            .as_mut()
            .and_then(|active_runtime| self.actor_runtime_restart_reason(active_runtime, &cwd));

        if let Some(reason) = restart_reason {
            if let Some(active_runtime) = runtime.take() {
                self.shutdown_actor_runtime(active_runtime, Some(reason), false).await;
            }
        } else if runtime.is_some() {
            return Ok(());
        }

        *runtime = Some(self.spawn_actor_runtime(&cwd).await?);
        Ok(())
    }

    async fn spawn_actor_runtime(&self, cwd: &Path) -> Result<ActorRuntime, AcpError> {
        let ProcessHandles { mut child, stderr_task } = self.spawn_child()?;
        let pid = child.id();
        let outgoing = TappedWriter::new(
            child.stdin.take().ok_or(AcpError::MissingStdin)?,
            self.make_message_tap(AcpMessageDirection::Outbound),
        )
        .compat_write();
        let expected_session_id = Arc::new(Mutex::new(None::<String>));
        let (event_tx, event_rx) = mpsc::unbounded_channel::<InternalEvent>();
        let incoming = TappedReader::new(
            child.stdout.take().ok_or(AcpError::MissingStdout)?,
            self.make_message_tap(AcpMessageDirection::Inbound),
        )
        .compat();
        let (conn, handle_io) = acp::ClientSideConnection::new(
            self.build_event_client(cwd, expected_session_id.clone(), event_tx),
            outgoing,
            incoming,
            |fut| {
                tokio::task::spawn_local(fut);
            },
        );
        let (io_closed_tx, io_closed_rx) = oneshot::channel();
        let io_task = tokio::task::spawn_local(async move {
            let _ = handle_io.await;
            let _ = io_closed_tx.send(());
        });

        match self.initialize_connection(&conn).await {
            Ok(()) => {
                self.record_actor_start(pid);
                Ok(ActorRuntime {
                    cwd: cwd.to_path_buf(),
                    child,
                    stderr_task,
                    expected_session_id,
                    event_rx,
                    conn,
                    io_closed_rx,
                    io_task,
                })
            }
            Err(err) => {
                let cleanup_targets = process_cleanup_targets(child.id());
                self.shutdown_io_task(io_task).await;
                let finalized = self
                    .finalize_child_with_cleanup_targets(child, stderr_task, cleanup_targets)
                    .await;
                Err(enrich_acp_error_with_process_context(err, &finalized))
            }
        }
    }

    pub(super) async fn shutdown_actor_runtime(
        &self,
        runtime: ActorRuntime,
        reason: Option<&str>,
        unexpected_during_prompt: bool,
    ) -> FinalizedChild {
        let ActorRuntime {
            cwd: _,
            child,
            stderr_task,
            expected_session_id,
            event_rx,
            conn,
            io_closed_rx,
            io_task,
        } = runtime;
        let cleanup_targets = process_cleanup_targets(child.id());
        drop(conn);
        drop(event_rx);
        drop(expected_session_id);
        drop(io_closed_rx);
        self.shutdown_io_task(io_task).await;
        let finalized =
            self.finalize_child_with_cleanup_targets(child, stderr_task, cleanup_targets).await;
        self.record_actor_exit(finalized.summary.clone(), reason, unexpected_during_prompt);
        self.store_reusable_session(None);
        finalized
    }

    async fn shutdown_io_task(&self, io_task: tokio::task::JoinHandle<()>) {
        io_task.abort();
        let _ = io_task.await;
    }
}
