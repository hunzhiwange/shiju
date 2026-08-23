//! 验证服务端代理能力声明、后端会话绑定与历史所有权边界。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use agent_client_protocol::{
    Agent, InitializeRequest, LoadSessionRequest, NewSessionRequest, PromptRequest, ProtocolVersion,
};
use async_trait::async_trait;

use super::{
    BackendError, BackendHistoryEntry, BackendRole, BackendSession, SessionBackend, VibeWindowAgent,
};

#[derive(Default)]
pub(super) struct FakeBackend {
    sessions: Mutex<HashMap<String, BackendSession>>,
}

impl FakeBackend {
    fn insert(&self, session: BackendSession) {
        self.sessions.lock().expect("fake backend lock").insert(session.id.clone(), session);
    }
}

#[async_trait]
impl SessionBackend for FakeBackend {
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        let id = format!("native-{}", self.sessions.lock().expect("fake backend lock").len() + 1);
        let session = BackendSession { id: id.clone(), cwd, history: Vec::new() };
        self.insert(session.clone());
        Ok(session)
    }

    async fn load_session(
        &self,
        session_id: &str,
        _cwd: PathBuf,
    ) -> Result<BackendSession, BackendError> {
        self.sessions
            .lock()
            .expect("fake backend lock")
            .get(session_id)
            .cloned()
            .ok_or_else(|| BackendError::NotFound(session_id.to_string()))
    }
}

async fn initialize(agent: &VibeWindowAgent<FakeBackend>) {
    Agent::initialize(agent, InitializeRequest::new(ProtocolVersion::V1))
        .await
        .expect("initialize");
}

#[tokio::test]
async fn initialize_advertises_only_implemented_v1_capabilities() {
    let agent = VibeWindowAgent::new(FakeBackend::default());

    let response = Agent::initialize(&agent, InitializeRequest::new(ProtocolVersion::V1))
        .await
        .expect("initialize");

    assert_eq!(response.protocol_version, ProtocolVersion::V1);
    assert!(response.agent_capabilities.load_session);
    assert!(response.auth_methods.is_empty());
    assert!(response.agent_capabilities.session_capabilities.list.is_none());
    assert!(!response.agent_capabilities.prompt_capabilities.image);
    assert!(!response.agent_capabilities.prompt_capabilities.audio);
    assert!(!response.agent_capabilities.prompt_capabilities.embedded_context);
}

#[tokio::test]
async fn new_session_binds_backend_session_and_cwd() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let canonical = std::fs::canonicalize(root.path()).expect("canonical cwd");
    let agent = VibeWindowAgent::new(FakeBackend::default());
    initialize(&agent).await;

    let response =
        Agent::new_session(&agent, NewSessionRequest::new(root.path())).await.expect("new session");
    let binding = agent.session_binding(response.session_id.0.as_ref()).expect("binding");

    assert_eq!(response.session_id.0.as_ref(), binding.backend_session_id());
    assert_eq!(binding.cwd(), canonical);
    assert!(binding.history().is_empty());
}

#[tokio::test]
async fn load_session_replays_only_owned_history() {
    let owned_root = tempfile::tempdir().expect("owned cwd");
    let other_root = tempfile::tempdir().expect("other cwd");
    let backend = FakeBackend::default();
    backend.insert(BackendSession {
        id: "owned".to_string(),
        cwd: std::fs::canonicalize(owned_root.path()).expect("owned canonical cwd"),
        history: vec![BackendHistoryEntry {
            role: BackendRole::Assistant,
            text: "previous answer".to_string(),
        }],
    });
    let agent = VibeWindowAgent::new(backend);
    initialize(&agent).await;

    Agent::load_session(&agent, LoadSessionRequest::new("owned", owned_root.path()))
        .await
        .expect("owned session");
    let binding = agent.session_binding("owned").expect("owned binding");
    assert_eq!(binding.history()[0].text, "previous answer");

    let error = Agent::load_session(&agent, LoadSessionRequest::new("owned", other_root.path()))
        .await
        .expect_err("foreign cwd must be rejected");
    assert!(error.data.is_some());
    assert_eq!(agent.session_binding("owned").expect("original binding").cwd(), binding.cwd());
}

#[tokio::test]
async fn unsupported_methods_are_not_advertised() {
    let agent = VibeWindowAgent::new(FakeBackend::default());
    let before_initialize =
        Agent::new_session(&agent, NewSessionRequest::new(".")).await.expect_err("not initialized");
    assert!(before_initialize.data.is_some());

    initialize(&agent).await;
    let unsupported = Agent::prompt(&agent, PromptRequest::new("missing", Vec::new()))
        .await
        .expect_err("prompt is implemented in the next batch");
    assert_eq!(unsupported.message, "Method not found");
}
