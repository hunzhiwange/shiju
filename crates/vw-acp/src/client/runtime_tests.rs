use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol as acp;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::types::AcpAgentConfig;

use super::{AcpClient, ActorRuntime, DEFAULT_ACTOR_IDLE_TIMEOUT, InternalEvent, ProcessHandles};

#[derive(Clone, Default)]
struct TestClient;

#[async_trait(?Send)]
impl acp::Client for TestClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, _args: acp::SessionNotification) -> acp::Result<()> {
        Ok(())
    }
}

fn client(command: &str, args: &[&str]) -> AcpClient {
    AcpClient::new(
        "test-agent",
        AcpAgentConfig {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            env: HashMap::new(),
        },
    )
    .with_actor_idle_timeout(DEFAULT_ACTOR_IDLE_TIMEOUT)
}

fn connection() -> acp::ClientSideConnection {
    let (client_out, _agent_in) = tokio::io::duplex(1024);
    let (_agent_out, client_in) = tokio::io::duplex(1024);
    let (conn, _io) = acp::ClientSideConnection::new(
        TestClient,
        client_out.compat(),
        client_in.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    conn
}

fn pending_io_task() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        std::future::pending::<()>().await;
    })
}

async fn finished_io_task() -> tokio::task::JoinHandle<()> {
    let task = tokio::spawn(async {});
    tokio::task::yield_now().await;
    task
}

fn running_runtime(
    client: &AcpClient,
    cwd: impl Into<PathBuf>,
    io_task: tokio::task::JoinHandle<()>,
) -> ActorRuntime {
    let ProcessHandles { child, stderr_task } =
        client.spawn_child().expect("child process should spawn");
    let (_event_tx, event_rx) = mpsc::unbounded_channel::<InternalEvent>();
    let (_io_closed_tx, io_closed_rx) = oneshot::channel();

    ActorRuntime {
        cwd: cwd.into(),
        child,
        stderr_task,
        expected_session_id: Arc::new(super::Mutex::new(None)),
        event_rx,
        conn: connection(),
        io_closed_rx,
        io_task,
    }
}

#[cfg(unix)]
#[tokio::test]
async fn actor_runtime_restart_reason_reuses_running_runtime_for_same_cwd() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let cwd = Path::new("/tmp/vw-acp-runtime-same-cwd");
            let client = client("sh", &["-c", "sleep 30"]);
            let mut runtime = running_runtime(&client, cwd, pending_io_task());

            let reason = client.actor_runtime_restart_reason(&mut runtime, cwd);

            assert!(reason.is_none());
            client.shutdown_actor_runtime(runtime, Some("test_cleanup"), false).await;
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn actor_runtime_restart_reason_reports_cwd_change_first() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let client = client("sh", &["-c", "sleep 30"]);
            let mut runtime =
                running_runtime(&client, "/tmp/vw-acp-runtime-old", pending_io_task());

            let reason = client
                .actor_runtime_restart_reason(&mut runtime, Path::new("/tmp/vw-acp-runtime-new"));

            assert_eq!(reason, Some("cwd_changed"));
            client.shutdown_actor_runtime(runtime, Some("test_cleanup"), false).await;
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn actor_runtime_restart_reason_reports_closed_io_before_child_state() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let cwd = Path::new("/tmp/vw-acp-runtime-io-closed");
            let client = client("sh", &["-c", "sleep 30"]);
            let mut runtime = running_runtime(&client, cwd, finished_io_task().await);

            let reason = client.actor_runtime_restart_reason(&mut runtime, cwd);

            assert_eq!(reason, Some("io_closed"));
            client.shutdown_actor_runtime(runtime, Some("test_cleanup"), false).await;
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn actor_runtime_restart_reason_reports_agent_exit() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let cwd = Path::new("/tmp/vw-acp-runtime-agent-exit");
            let client = client("sh", &["-c", "exit 0"]);
            let mut runtime = running_runtime(&client, cwd, pending_io_task());
            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;

            let reason = client.actor_runtime_restart_reason(&mut runtime, cwd);

            assert_eq!(reason, Some("agent_exited"));
            client.shutdown_actor_runtime(runtime, Some("test_cleanup"), false).await;
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn shutdown_actor_runtime_records_exit_and_clears_reusable_session() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let client = client("sh", &["-c", "sleep 30"]);
            client.store_reusable_session(Some("session-1".to_string()));
            let runtime =
                running_runtime(&client, "/tmp/vw-acp-runtime-shutdown", pending_io_task());

            client.shutdown_actor_runtime(runtime, Some("explicit_shutdown"), true).await;
            let snapshot = client.get_agent_lifecycle_snapshot();
            let exit = snapshot.last_exit.expect("shutdown should record an exit");

            assert!(snapshot.pid.is_none());
            assert_eq!(exit.reason.as_deref(), Some("explicit_shutdown"));
            assert!(exit.unexpected_during_prompt);
            assert!(!client.has_reusable_session("session-1"));
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn prepare_actor_runtime_reuses_healthy_runtime_for_same_cwd() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let cwd = Path::new("/tmp/vw-acp-runtime-reuse");
            let client = client("sh", &["-c", "sleep 30"]);
            let mut runtime = Some(running_runtime(&client, cwd, pending_io_task()));
            let original_pid = runtime.as_ref().and_then(|runtime| runtime.child.id());

            client
                .prepare_actor_runtime(&mut runtime, cwd.to_path_buf())
                .await
                .expect("healthy runtime should be reused");

            assert_eq!(runtime.as_ref().and_then(|runtime| runtime.child.id()), original_pid);
            client
                .shutdown_actor_runtime(
                    runtime.take().expect("runtime should remain available"),
                    Some("test_cleanup"),
                    false,
                )
                .await;
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn prepare_actor_runtime_cleans_up_when_initialize_connection_fails() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let cwd = Path::new("/tmp/vw-acp-runtime-init-fails");
            let client = client("sh", &["-c", "exit 0"]);
            let mut runtime = None;

            let error = client
                .prepare_actor_runtime(&mut runtime, cwd.to_path_buf())
                .await
                .expect_err("exited agent should fail initialization");

            assert!(runtime.is_none());
            assert!(!error.to_string().is_empty());
        })
        .await;
}
