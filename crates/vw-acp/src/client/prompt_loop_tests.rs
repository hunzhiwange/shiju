//! 验证 Actor 提示词循环的事件收集、取消竞争与错误收尾。
//!
//! 最小客户端、通道和运行时 fixture 用于显式驱动异步状态转换与资源收尾。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol as acp;
use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::types::{AcpAgentConfig, PromptEvent, PromptRequest, PromptUsage};

use super::{AcpClient, AcpError, ActorCommand, ActorRuntime, InternalEvent};

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

#[derive(Clone)]
struct TestAgent {
    state: Arc<TestAgentState>,
}

struct TestAgentState {
    new_session_id: String,
    fail_cancel: bool,
    prompt_started: tokio::sync::Notify,
    cancel_received: tokio::sync::Notify,
    prompt_response_rx:
        tokio::sync::Mutex<Option<oneshot::Receiver<acp::Result<acp::PromptResponse>>>>,
    prompt_session_ids: Mutex<Vec<String>>,
    cancelled_session_ids: Mutex<Vec<String>>,
}

impl TestAgent {
    fn new(
        new_session_id: &str,
        fail_cancel: bool,
    ) -> (Self, oneshot::Sender<acp::Result<acp::PromptResponse>>) {
        let (prompt_response_tx, prompt_response_rx) = oneshot::channel();
        (
            Self {
                state: Arc::new(TestAgentState {
                    new_session_id: new_session_id.to_string(),
                    fail_cancel,
                    prompt_started: tokio::sync::Notify::new(),
                    cancel_received: tokio::sync::Notify::new(),
                    prompt_response_rx: tokio::sync::Mutex::new(Some(prompt_response_rx)),
                    prompt_session_ids: Mutex::new(Vec::new()),
                    cancelled_session_ids: Mutex::new(Vec::new()),
                }),
            },
            prompt_response_tx,
        )
    }

    async fn wait_for_prompt(&self) {
        self.state.prompt_started.notified().await;
    }

    async fn wait_for_cancel(&self) {
        self.state.cancel_received.notified().await;
    }

    fn prompt_session_ids(&self) -> Vec<String> {
        self.state.prompt_session_ids.lock().clone()
    }

    fn cancelled_session_ids(&self) -> Vec<String> {
        self.state.cancelled_session_ids.lock().clone()
    }
}

#[async_trait(?Send)]
impl acp::Agent for TestAgent {
    async fn initialize(
        &self,
        args: acp::InitializeRequest,
    ) -> acp::Result<acp::InitializeResponse> {
        Ok(acp::InitializeResponse::new(args.protocol_version))
    }

    async fn authenticate(
        &self,
        _args: acp::AuthenticateRequest,
    ) -> acp::Result<acp::AuthenticateResponse> {
        Ok(acp::AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        _args: acp::NewSessionRequest,
    ) -> acp::Result<acp::NewSessionResponse> {
        Ok(acp::NewSessionResponse::new(self.state.new_session_id.clone()))
    }

    async fn prompt(&self, args: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
        self.state.prompt_session_ids.lock().push(args.session_id.0.to_string());
        self.state.prompt_started.notify_waiters();
        let prompt_response_rx = self
            .state
            .prompt_response_rx
            .lock()
            .await
            .take()
            .expect("prompt response receiver should be available");

        prompt_response_rx.await.expect("prompt response should be sent")
    }

    async fn cancel(&self, args: acp::CancelNotification) -> acp::Result<()> {
        self.state.cancelled_session_ids.lock().push(args.session_id.0.to_string());
        self.state.cancel_received.notify_one();
        if self.state.fail_cancel { Err(acp::Error::invalid_params()) } else { Ok(()) }
    }
}

fn client() -> AcpClient {
    AcpClient::new(
        "test-agent",
        AcpAgentConfig { command: "agent".to_string(), args: Vec::new(), env: HashMap::new() },
    )
}

fn runtime_for_connection(
    conn: acp::ClientSideConnection,
    event_rx: mpsc::UnboundedReceiver<InternalEvent>,
) -> ActorRuntime {
    let child = Command::new("true").spawn().expect("test child should spawn");
    let (_io_closed_tx, io_closed_rx) = oneshot::channel();
    let io_task = tokio::task::spawn(async {});

    ActorRuntime {
        cwd: PathBuf::from("."),
        child,
        stderr_task: None,
        expected_session_id: Arc::new(Mutex::new(None)),
        event_rx,
        conn,
        io_closed_rx,
        io_task,
    }
}

fn connection_pair(agent: TestAgent) -> acp::ClientSideConnection {
    let (client_out, agent_in) = tokio::io::duplex(4096);
    let (agent_out, client_in) = tokio::io::duplex(4096);

    let (client_conn, client_io_task) = acp::ClientSideConnection::new(
        TestClient,
        client_out.compat(),
        client_in.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    let (agent_conn, agent_io_task) =
        acp::AgentSideConnection::new(agent, agent_out.compat(), agent_in.compat(), |fut| {
            tokio::task::spawn_local(fut);
        });

    tokio::task::spawn_local(client_io_task);
    tokio::task::spawn_local(agent_io_task);
    drop(agent_conn);

    client_conn
}

async fn with_runtime<R>(
    agent: TestAgent,
    f: impl AsyncFnOnce(AcpClient, ActorRuntime, mpsc::UnboundedSender<InternalEvent>, TestAgent) -> R,
) -> R {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (internal_tx, internal_rx) = mpsc::unbounded_channel();
            let runtime = runtime_for_connection(connection_pair(agent.clone()), internal_rx);
            f(client(), runtime, internal_tx, agent).await
        })
        .await
}

// 下列行为用例重点检查复用、取消、通道关闭和失败后的状态一致性。
#[tokio::test]
async fn run_actor_prompt_collects_events_and_updates_session_state() {
    let (agent, prompt_response_tx) = TestAgent::new("session-1", false);

    with_runtime(agent, async |client, mut runtime, internal_tx, agent| {
        internal_tx
            .send(InternalEvent::Delta("stale".to_string()))
            .expect("stale event should queue");
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let request = PromptRequest::new(PathBuf::from("."), "hello");

        let run_prompt = client.run_actor_prompt(&mut runtime, request, event_tx);
        let drive_prompt = async {
            agent.wait_for_prompt().await;
            internal_tx
                .send(InternalEvent::Delta("one".to_string()))
                .expect("first delta should send");
            internal_tx.send(InternalEvent::Delta(String::new())).expect("empty delta should send");
            internal_tx
                .send(InternalEvent::SessionChanged {
                    expected: "session-1".to_string(),
                    actual: "session-2".to_string(),
                })
                .expect("session change should send");
            internal_tx
                .send(InternalEvent::Delta("two".to_string()))
                .expect("second delta should send");

            prompt_response_tx
                .send(Ok(acp::PromptResponse::new(acp::StopReason::MaxTokens)
                    .usage(acp::Usage::new(15, 3, 4).cached_read_tokens(5).thought_tokens(6))))
                .expect("prompt response should send");
        };

        let (result, _) = tokio::join!(run_prompt, drive_prompt);
        let result = result.expect("prompt should complete");

        assert_eq!(agent.prompt_session_ids(), vec!["session-1"]);
        assert_eq!(result.session_id, "session-2");
        assert_eq!(result.deltas, vec!["one", "two"]);
        assert_eq!(result.finish_reason.as_deref(), Some("length"));
        assert_eq!(
            result.usage,
            Some(PromptUsage {
                input_tokens: 3,
                output_tokens: 4,
                cached_tokens: 5,
                reasoning_tokens: 6,
            })
        );
        assert_eq!(
            drain_prompt_events(&mut event_rx),
            vec![
                PromptEvent::TextDelta("one".to_string()),
                PromptEvent::SessionChanged {
                    expected: "session-1".to_string(),
                    actual: "session-2".to_string(),
                },
                PromptEvent::TextDelta("two".to_string()),
            ]
        );
        assert!(!client.has_active_prompt());
        assert_eq!(client.actor_state.lock().reusable_session_id.as_deref(), Some("session-2"));
        assert_eq!(runtime.expected_session_id.lock().as_deref(), Some("session-2"));
    })
    .await;
}

#[tokio::test]
async fn run_actor_prompt_maps_prompt_errors_and_clears_active_prompt() {
    let (agent, prompt_response_tx) = TestAgent::new("session-error", false);

    with_runtime(agent, async |client, mut runtime, _internal_tx, agent| {
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let run_prompt = client.run_actor_prompt(
            &mut runtime,
            PromptRequest::new(PathBuf::from("."), "hello"),
            event_tx,
        );
        let drive_prompt = async {
            agent.wait_for_prompt().await;
            prompt_response_tx
                .send(Err(acp::Error::invalid_params()))
                .expect("prompt error should send");
        };

        let (result, _) = tokio::join!(run_prompt, drive_prompt);

        assert!(matches!(result, Err(AcpError::Prompt(_))));
        assert!(!client.has_active_prompt());
        assert!(client.cancelling_session_ids.lock().is_empty());
    })
    .await;
}

#[tokio::test]
async fn run_actor_prompt_processes_internal_events_while_prompt_is_pending() {
    let (agent, prompt_response_tx) = TestAgent::new("session-pending-event", false);

    with_runtime(agent, async |client, mut runtime, internal_tx, agent| {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let run_prompt = client.run_actor_prompt(
            &mut runtime,
            PromptRequest::new(PathBuf::from("."), "hello"),
            event_tx,
        );
        let drive_prompt = async {
            agent.wait_for_prompt().await;
            internal_tx
                .send(InternalEvent::Delta("pending".to_string()))
                .expect("pending delta should send");
            let event = tokio::time::timeout(Duration::from_secs(1), event_rx.recv())
                .await
                .expect("pending delta should be forwarded")
                .expect("event channel should remain open");
            assert_eq!(event, PromptEvent::TextDelta("pending".to_string()));
            prompt_response_tx
                .send(Ok(acp::PromptResponse::new(acp::StopReason::EndTurn)))
                .expect("prompt response should send");
        };

        let (result, _) = tokio::join!(run_prompt, drive_prompt);
        let result = result.expect("prompt should complete");

        assert_eq!(result.deltas, vec!["pending"]);
        assert_eq!(result.finish_reason.as_deref(), Some("stop"));
        assert!(!client.has_active_prompt());
    })
    .await;
}

#[tokio::test]
async fn run_actor_prompt_ignores_false_and_closed_cancel_signals() {
    let (agent, prompt_response_tx) = TestAgent::new("session-false-cancel", false);

    with_runtime(agent, async |client, mut runtime, _internal_tx, agent| {
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let run_prompt = client.run_actor_prompt(
            &mut runtime,
            PromptRequest::new(PathBuf::from("."), "hello"),
            event_tx,
        );
        let drive_prompt = async {
            agent.wait_for_prompt().await;
            let active_prompt =
                client.active_prompt.lock().clone().expect("active prompt should be registered");
            active_prompt.cancel_tx.send(false).expect("false cancel signal should send");
            tokio::task::yield_now().await;
            client.clear_active_prompt(&active_prompt.session_id);
            drop(active_prompt);
            tokio::task::yield_now().await;
            prompt_response_tx
                .send(Ok(acp::PromptResponse::new(acp::StopReason::EndTurn)))
                .expect("prompt response should send");
        };

        let (result, _) = tokio::join!(run_prompt, drive_prompt);
        let result = result.expect("prompt should ignore non-cancelling signals");

        assert_eq!(result.finish_reason.as_deref(), Some("stop"));
        assert_eq!(agent.cancelled_session_ids(), Vec::<String>::new());
        assert!(!client.has_active_prompt());
    })
    .await;
}

#[tokio::test]
async fn run_actor_prompt_sends_cancel_and_waits_for_cancelled_response() {
    let (agent, prompt_response_tx) = TestAgent::new("session-cancel", false);

    with_runtime(agent, async |client, mut runtime, _internal_tx, agent| {
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let run_prompt = client.run_actor_prompt(
            &mut runtime,
            PromptRequest::new(PathBuf::from("."), "hello"),
            event_tx,
        );
        let cancel_prompt = async {
            agent.wait_for_prompt().await;
            assert!(!client.cancel("other-session").await.expect("other cancel should not fail"));
            assert!(client.cancel("session-cancel").await.expect("cancel should send"));
            tokio::task::yield_now().await;
            prompt_response_tx
                .send(Ok(acp::PromptResponse::new(acp::StopReason::Cancelled)))
                .expect("cancelled prompt response should send");
        };

        let (result, _) = tokio::join!(run_prompt, cancel_prompt);
        let result = result.expect("cancelled prompt should still return response");

        assert_eq!(agent.cancelled_session_ids(), vec!["session-cancel"]);
        assert_eq!(result.finish_reason.as_deref(), Some("cancelled"));
        assert!(!client.has_active_prompt());
        assert!(client.cancelling_session_ids.lock().is_empty());
    })
    .await;
}

#[tokio::test]
async fn active_prompt_services_model_control_without_blocking_cancellation() {
    let (agent, response) = TestAgent::new("active-session", false);
    with_runtime(agent, async |client, mut runtime, _events, agent| {
        let (commands, mut command_rx) = mpsc::unbounded_channel();
        let (events, _event_rx) = mpsc::unbounded_channel();
        let mut deferred = std::collections::VecDeque::new();
        let prompt = client.run_actor_prompt_with_controls(
            &mut runtime,
            PromptRequest::new(PathBuf::from("."), "hello"),
            events,
            Some(&mut command_rx),
            &mut deferred,
        );
        let control = async {
            agent.wait_for_prompt().await;
            let (reply, result) = oneshot::channel();
            commands
                .send(ActorCommand::SetSessionModel {
                    session_id: "active-session".into(),
                    cwd: PathBuf::from("."),
                    model: "model-b".into(),
                    response_tx: reply,
                })
                .unwrap();
            // 此 Agent 不支持切模，必须立即返回协议错误，不能排在活动提示后。
            let result =
                tokio::time::timeout(Duration::from_secs(1), result).await.unwrap().unwrap();
            assert!(result.is_err());
            assert!(client.has_active_prompt());
            assert!(client.cancel("active-session").await.unwrap());
            agent.wait_for_cancel().await;
            response.send(Ok(acp::PromptResponse::new(acp::StopReason::Cancelled))).unwrap();
        };
        let (result, ()) = tokio::join!(prompt, control);
        assert_eq!(result.unwrap().finish_reason.as_deref(), Some("cancelled"));
        assert!(deferred.is_empty());
    })
    .await;
}

#[tokio::test]
async fn run_actor_prompt_returns_cancelled_response_when_agent_rejects_cancel_notification() {
    let (agent, prompt_response_tx) = TestAgent::new("session-cancel-error", true);

    with_runtime(agent, async |client, mut runtime, _internal_tx, agent| {
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let run_prompt = client.run_actor_prompt(
            &mut runtime,
            PromptRequest::new(PathBuf::from("."), "hello"),
            event_tx,
        );
        let cancel_prompt = async {
            agent.wait_for_prompt().await;
            assert!(client.cancel("session-cancel-error").await.expect("cancel should signal"));
            tokio::task::yield_now().await;
            prompt_response_tx
                .send(Ok(acp::PromptResponse::new(acp::StopReason::Cancelled)))
                .expect("cancelled prompt response should send");
        };

        let (result, _) = tokio::join!(run_prompt, cancel_prompt);

        let result = result.expect("cancelled prompt response should win");
        assert_eq!(result.finish_reason.as_deref(), Some("cancelled"));
        assert_eq!(agent.cancelled_session_ids(), vec!["session-cancel-error"]);
        assert!(!client.has_active_prompt());
        assert!(client.cancelling_session_ids.lock().is_empty());
    })
    .await;
}

#[tokio::test]
async fn run_actor_prompt_returns_prompt_error_after_agent_rejects_cancel_notification() {
    let (agent, prompt_response_tx) = TestAgent::new("session-cancel-error", true);

    with_runtime(agent, async |client, mut runtime, _internal_tx, agent| {
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let run_prompt = client.run_actor_prompt(
            &mut runtime,
            PromptRequest::new(PathBuf::from("."), "hello"),
            event_tx,
        );
        let cancel_prompt = async {
            agent.wait_for_prompt().await;
            assert!(client.cancel("session-cancel-error").await.expect("cancel should signal"));
            agent.wait_for_cancel().await;
            prompt_response_tx
                .send(Err(acp::Error::invalid_params()))
                .expect("prompt error should send");
        };

        let (result, _) = tokio::join!(run_prompt, cancel_prompt);

        assert!(matches!(result, Err(AcpError::Prompt(_))));
        assert_eq!(agent.cancelled_session_ids(), vec!["session-cancel-error"]);
        assert!(!client.has_active_prompt());
        assert!(client.cancelling_session_ids.lock().is_empty());
    })
    .await;
}

fn drain_prompt_events(event_rx: &mut mpsc::UnboundedReceiver<PromptEvent>) -> Vec<PromptEvent> {
    let mut events = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        events.push(event);
    }
    events
}
