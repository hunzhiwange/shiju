//! 验证服务端提示词映射、内容能力限制、并发保护与定向取消。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    Agent, CancelNotification, ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest,
    ProtocolVersion, ResourceLink, SessionNotification, SessionUpdate, StopReason, TextContent,
};
use async_trait::async_trait;

use super::{
    BackendCancelHandle, BackendError, BackendEvent, BackendPrompt, BackendSession,
    BackendTermination, BackendToolStatus, PromptContent, SessionBackend, SessionUpdateSink,
    VibeWindowAgent,
};

#[derive(Default)]
struct RecordingSink {
    notifications: Arc<Mutex<Vec<SessionNotification>>>,
}

#[async_trait(?Send)]
impl SessionUpdateSink for RecordingSink {
    async fn send(&self, notification: SessionNotification) -> agent_client_protocol::Result<()> {
        self.notifications.lock().expect("notification lock").push(notification);
        Ok(())
    }
}

struct ScriptedCancel;

impl BackendCancelHandle for ScriptedCancel {
    fn cancel(&self) -> bool {
        true
    }
}

#[derive(Default)]
struct PromptBackend {
    prompts: Arc<Mutex<Vec<Vec<PromptContent>>>>,
    events: Mutex<HashMap<String, Vec<BackendEvent>>>,
    hold_open: bool,
    active: Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<BackendEvent>>>>,
    cancel_counts: Arc<Mutex<HashMap<String, usize>>>,
    next_session: AtomicUsize,
}

struct ChannelCancel {
    session_id: String,
    active: Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<BackendEvent>>>>,
    cancel_counts: Arc<Mutex<HashMap<String, usize>>>,
    cancelled: AtomicBool,
}

impl BackendCancelHandle for ChannelCancel {
    fn cancel(&self) -> bool {
        if self.cancelled.swap(true, Ordering::AcqRel) {
            return false;
        }
        *self
            .cancel_counts
            .lock()
            .expect("cancel count lock")
            .entry(self.session_id.clone())
            .or_default() += 1;
        if let Some(tx) = self.active.lock().expect("active lock").remove(&self.session_id) {
            let _ = tx.send(BackendEvent::Terminal(BackendTermination::Cancelled));
        }
        true
    }
}

#[async_trait]
impl SessionBackend for PromptBackend {
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        let index = self.next_session.fetch_add(1, Ordering::Relaxed) + 1;
        Ok(BackendSession { id: format!("session-{index}"), cwd, history: Vec::new() })
    }

    async fn load_session(
        &self,
        session_id: &str,
        cwd: PathBuf,
    ) -> Result<BackendSession, BackendError> {
        Ok(BackendSession { id: session_id.to_string(), cwd, history: Vec::new() })
    }

    async fn start_prompt(
        &self,
        session_id: &str,
        prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        self.prompts.lock().expect("prompt lock").push(prompt);
        let scripted =
            self.events.lock().expect("event lock").remove(session_id).unwrap_or_default();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for event in scripted {
            tx.send(event).expect("script receiver");
        }
        if self.hold_open {
            self.active.lock().expect("active lock").insert(session_id.to_string(), tx);
            Ok(BackendPrompt::new(
                rx,
                Arc::new(ChannelCancel {
                    session_id: session_id.to_string(),
                    active: Arc::clone(&self.active),
                    cancel_counts: Arc::clone(&self.cancel_counts),
                    cancelled: AtomicBool::new(false),
                }),
            ))
        } else {
            drop(tx);
            Ok(BackendPrompt::new(rx, Arc::new(ScriptedCancel)))
        }
    }
}

async fn prepared_agent(
    backend: PromptBackend,
    sink: RecordingSink,
    cwd: &std::path::Path,
) -> VibeWindowAgent<PromptBackend> {
    let agent = VibeWindowAgent::with_update_sink(backend, sink);
    Agent::initialize(&agent, InitializeRequest::new(ProtocolVersion::V1))
        .await
        .expect("initialize");
    Agent::new_session(&agent, NewSessionRequest::new(cwd)).await.expect("new session");
    agent
}

#[tokio::test]
async fn prompt_maps_text_resource_links_and_backend_events() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let backend = PromptBackend::default();
    let captured_prompts = Arc::clone(&backend.prompts);
    backend.events.lock().expect("event lock").insert(
        "session-1".to_string(),
        vec![
            BackendEvent::Text("answer".to_string()),
            BackendEvent::ToolStart {
                call_id: "call-1".to_string(),
                title: "shell".to_string(),
                raw_input: serde_json::json!({"command": "pwd"}),
            },
            BackendEvent::ToolUpdate {
                call_id: "call-1".to_string(),
                status: BackendToolStatus::Running,
                content: Some("half".to_string()),
                raw_output: None,
            },
            BackendEvent::ToolComplete {
                call_id: "call-1".to_string(),
                status: BackendToolStatus::Completed,
                content: Some("done".to_string()),
                raw_output: Some(serde_json::json!({"ok": true})),
            },
            BackendEvent::Terminal(BackendTermination::EndTurn),
        ],
    );
    let notifications = Arc::new(Mutex::new(Vec::new()));
    let sink = RecordingSink { notifications: Arc::clone(&notifications) };
    let agent = prepared_agent(backend, sink, root.path()).await;
    let request = PromptRequest::new(
        "session-1",
        vec![
            ContentBlock::Text(TextContent::new("hello")),
            ContentBlock::ResourceLink(ResourceLink::new("spec", "file:///tmp/spec.md")),
        ],
    );

    let response = Agent::prompt(&agent, request).await.expect("prompt");

    assert_eq!(response.stop_reason, StopReason::EndTurn);
    let binding = agent.session_binding("session-1").expect("binding");
    assert_eq!(binding.cwd(), std::fs::canonicalize(root.path()).unwrap());
    let prompts = captured_prompts.lock().expect("captured prompt lock");
    let captured = &prompts[0];
    assert!(matches!(&captured[0], PromptContent::Text(text) if text == "hello"));
    assert!(matches!(
        &captured[1],
        PromptContent::ResourceLink { uri, .. } if uri == "file:///tmp/spec.md"
    ));
    let notifications = notifications.lock().expect("notification lock");
    assert!(matches!(notifications[0].update, SessionUpdate::AgentMessageChunk(_)));
    assert!(matches!(notifications[1].update, SessionUpdate::ToolCall(_)));
    assert!(matches!(notifications[2].update, SessionUpdate::ToolCallUpdate(_)));
    assert!(matches!(notifications[3].update, SessionUpdate::ToolCallUpdate(_)));
}

#[tokio::test]
async fn prompt_rejects_unadvertised_content() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let agent =
        prepared_agent(PromptBackend::default(), RecordingSink::default(), root.path()).await;
    let request = PromptRequest::new(
        "session-1",
        vec![ContentBlock::Image(agent_client_protocol::ImageContent::new("base64", "image/png"))],
    );

    let error = Agent::prompt(&agent, request).await.expect_err("image must be rejected");
    assert!(error.data.is_some());
}

#[tokio::test]
async fn same_session_rejects_concurrent_prompt() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let backend = PromptBackend { hold_open: true, ..PromptBackend::default() };
    let active = Arc::clone(&backend.active);
    let agent = prepared_agent(backend, RecordingSink::default(), root.path()).await;
    let first = Agent::prompt(&agent, PromptRequest::new("session-1", vec!["first".into()]));
    let control = async {
        while !active.lock().expect("active lock").contains_key("session-1") {
            tokio::task::yield_now().await;
        }
        let error = Agent::prompt(&agent, PromptRequest::new("session-1", vec!["again".into()]))
            .await
            .expect_err("same-session prompt must be exclusive");
        Agent::cancel(&agent, CancelNotification::new("session-1")).await.expect("cancel");
        error
    };
    let (first, error) = tokio::join!(first, control);
    assert!(error.data.is_some());
    assert_eq!(first.expect("first prompt").stop_reason, StopReason::Cancelled);
}

#[tokio::test]
async fn cancel_targets_only_active_session() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let backend = PromptBackend { hold_open: true, ..PromptBackend::default() };
    let active = Arc::clone(&backend.active);
    let cancel_counts = Arc::clone(&backend.cancel_counts);
    let agent = prepared_agent(backend, RecordingSink::default(), root.path()).await;
    Agent::new_session(&agent, NewSessionRequest::new(root.path())).await.expect("second session");
    let first = Agent::prompt(&agent, PromptRequest::new("session-1", vec!["one".into()]));
    let second = Agent::prompt(&agent, PromptRequest::new("session-2", vec!["two".into()]));
    let control = async {
        while active.lock().expect("active lock").len() != 2 {
            tokio::task::yield_now().await;
        }
        Agent::cancel(&agent, CancelNotification::new("unknown")).await.expect("unknown cancel");
        Agent::cancel(&agent, CancelNotification::new("session-1")).await.expect("first cancel");
        Agent::cancel(&agent, CancelNotification::new("session-1")).await.expect("repeat cancel");
        assert_eq!(cancel_counts.lock().expect("cancel count lock").get("session-1"), Some(&1));
        assert!(!cancel_counts.lock().expect("cancel count lock").contains_key("session-2"));
        Agent::cancel(&agent, CancelNotification::new("session-2")).await.expect("second cancel");
    };
    let (first, second, ()) = tokio::join!(first, second, control);
    assert_eq!(first.expect("first prompt").stop_reason, StopReason::Cancelled);
    assert_eq!(second.expect("second prompt").stop_reason, StopReason::Cancelled);
}
