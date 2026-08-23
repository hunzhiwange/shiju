//! 验证创建、加载、恢复和配置会话时的状态同步与回退路径。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::types::AcpAgentConfig;

use super::*;

#[derive(Debug, PartialEq, Eq)]
enum RecordedCall {
    NewSession(PathBuf),
    LoadSession { session_id: String, cwd: PathBuf },
    ResumeSession { session_id: String, cwd: PathBuf },
    SetMode { session_id: String, mode_id: String },
    SetConfig { session_id: String, option_name: String, value_id: String },
    SetModel { session_id: String, model: String },
}

#[derive(Default)]
struct TestAgentState {
    calls: Vec<RecordedCall>,
    fail_resume: bool,
    fail_set_config: bool,
    fail_set_model: bool,
}

#[derive(Clone, Default)]
struct TestAgent {
    state: Arc<Mutex<TestAgentState>>,
}

impl TestAgent {
    fn with_fail_resume() -> Self {
        let agent = Self::default();
        agent.state.lock().fail_resume = true;
        agent
    }

    fn with_fail_set_config() -> Self {
        let agent = Self::default();
        agent.state.lock().fail_set_config = true;
        agent
    }

    fn with_fail_set_model() -> Self {
        let agent = Self::default();
        agent.state.lock().fail_set_model = true;
        agent
    }

    fn calls(&self) -> Vec<RecordedCall> {
        std::mem::take(&mut self.state.lock().calls)
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
        args: acp::NewSessionRequest,
    ) -> acp::Result<acp::NewSessionResponse> {
        self.state.lock().calls.push(RecordedCall::NewSession(args.cwd));
        Ok(acp::NewSessionResponse::new("created-session"))
    }

    async fn load_session(
        &self,
        args: acp::LoadSessionRequest,
    ) -> acp::Result<acp::LoadSessionResponse> {
        self.state.lock().calls.push(RecordedCall::LoadSession {
            session_id: args.session_id.0.to_string(),
            cwd: args.cwd,
        });
        Ok(acp::LoadSessionResponse::new())
    }

    async fn resume_session(
        &self,
        args: acp::ResumeSessionRequest,
    ) -> acp::Result<acp::ResumeSessionResponse> {
        let mut state = self.state.lock();
        state.calls.push(RecordedCall::ResumeSession {
            session_id: args.session_id.0.to_string(),
            cwd: args.cwd,
        });
        if state.fail_resume {
            return Err(acp::Error::method_not_found());
        }
        Ok(acp::ResumeSessionResponse::new())
    }

    async fn set_session_mode(
        &self,
        args: acp::SetSessionModeRequest,
    ) -> acp::Result<acp::SetSessionModeResponse> {
        self.state.lock().calls.push(RecordedCall::SetMode {
            session_id: args.session_id.0.to_string(),
            mode_id: args.mode_id.0.to_string(),
        });
        Ok(acp::SetSessionModeResponse::new())
    }

    async fn set_session_config_option(
        &self,
        args: acp::SetSessionConfigOptionRequest,
    ) -> acp::Result<acp::SetSessionConfigOptionResponse> {
        let mut state = self.state.lock();
        state.calls.push(RecordedCall::SetConfig {
            session_id: args.session_id.0.to_string(),
            option_name: args.config_id.0.to_string(),
            value_id: args.value.0.to_string(),
        });
        if state.fail_set_config {
            return Err(acp::Error::invalid_params());
        }
        Ok(acp::SetSessionConfigOptionResponse::new(Vec::new()))
    }

    async fn set_session_model(
        &self,
        args: acp::SetSessionModelRequest,
    ) -> acp::Result<acp::SetSessionModelResponse> {
        let mut state = self.state.lock();
        state.calls.push(RecordedCall::SetModel {
            session_id: args.session_id.0.to_string(),
            model: args.model_id.0.to_string(),
        });
        if state.fail_set_model {
            return Err(acp::Error::invalid_params());
        }
        Ok(acp::SetSessionModelResponse::new())
    }

    async fn prompt(&self, _args: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn cancel(&self, _args: acp::CancelNotification) -> acp::Result<()> {
        Ok(())
    }
}

struct RuntimeFixture {
    client: AcpClient,
    runtime: Option<ActorRuntime>,
}

impl RuntimeFixture {
    async fn new(agent: TestAgent, cwd: &Path) -> Self {
        let client = AcpClient::new(
            "test-agent",
            AcpAgentConfig {
                command: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 30".to_string()],
                env: HashMap::new(),
            },
        );
        let runtime = runtime_for(&client, agent.clone(), cwd).await;
        Self { client, runtime: Some(runtime) }
    }

    async fn shutdown(mut self) {
        if let Some(runtime) = self.runtime.take() {
            self.client.shutdown_actor_runtime(runtime, Some("test_done"), false).await;
        }
    }
}

async fn runtime_for(client: &AcpClient, agent: TestAgent, cwd: &Path) -> ActorRuntime {
    let ProcessHandles { child, stderr_task } =
        client.spawn_child().expect("test child should spawn");
    let (client_out, agent_in) = tokio::io::duplex(4096);
    let (agent_out, client_in) = tokio::io::duplex(4096);
    let expected_session_id = Arc::new(Mutex::new(None::<String>));
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let (conn, client_io_task) = acp::ClientSideConnection::new(
        client.build_event_client(cwd, expected_session_id.clone(), event_tx),
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

    let (io_closed_tx, io_closed_rx) = oneshot::channel();
    let io_task = tokio::task::spawn_local(async move {
        let _ = client_io_task.await;
        let _ = io_closed_tx.send(());
    });
    tokio::task::spawn_local(agent_io_task);
    drop(agent_conn);

    ActorRuntime {
        cwd: cwd.to_path_buf(),
        child,
        stderr_task,
        expected_session_id,
        event_rx,
        conn,
        io_closed_rx,
        io_task,
    }
}

async fn with_fixture<R>(agent: TestAgent, f: impl AsyncFnOnce(&mut RuntimeFixture) -> R) -> R {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let cwd = PathBuf::from("/tmp/vw-acp-session-control");
            let mut fixture = RuntimeFixture::new(agent, &cwd).await;
            let output = f(&mut fixture).await;
            fixture.shutdown().await;
            output
        })
        .await
}

fn expected_session_id(runtime: &Option<ActorRuntime>) -> Option<String> {
    runtime.as_ref().and_then(|runtime| runtime.expected_session_id.lock().clone())
}

#[tokio::test]
async fn actor_create_session_stores_created_session_for_reuse() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        let cwd = PathBuf::from("/tmp/vw-acp-session-control");

        let session = fixture
            .client
            .actor_create_session(&mut fixture.runtime, cwd.clone())
            .await
            .expect("session should be created");

        assert_eq!(session.session_id, "created-session");
        assert_eq!(expected_session_id(&fixture.runtime), Some("created-session".to_string()));
        assert!(fixture.client.has_reusable_session("created-session"));
        assert_eq!(agent.calls(), vec![RecordedCall::NewSession(cwd)]);
    })
    .await;
}

#[tokio::test]
async fn actor_load_and_resume_session_update_runtime_state() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        let cwd = PathBuf::from("/tmp/vw-acp-session-control");

        let loaded = fixture
            .client
            .actor_load_session(&mut fixture.runtime, "load-id".to_string(), cwd.clone())
            .await
            .expect("session should load");
        assert_eq!(loaded.session_id, "load-id");
        assert_eq!(expected_session_id(&fixture.runtime), Some("load-id".to_string()));
        assert!(fixture.client.has_reusable_session("load-id"));

        let resumed = fixture
            .client
            .actor_resume_session(&mut fixture.runtime, "resume-id".to_string(), cwd.clone())
            .await
            .expect("session should resume");
        assert_eq!(resumed.session_id, "resume-id");
        assert_eq!(expected_session_id(&fixture.runtime), Some("resume-id".to_string()));
        assert!(fixture.client.has_reusable_session("resume-id"));

        assert_eq!(
            agent.calls(),
            vec![
                RecordedCall::LoadSession { session_id: "load-id".to_string(), cwd: cwd.clone() },
                RecordedCall::ResumeSession { session_id: "resume-id".to_string(), cwd },
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn actor_set_session_mode_resolves_existing_session_before_setting_mode() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        let cwd = PathBuf::from("/tmp/vw-acp-session-control");

        fixture
            .client
            .actor_set_session_mode(
                &mut fixture.runtime,
                "mode-session".to_string(),
                cwd.clone(),
                "plan".to_string(),
            )
            .await
            .expect("mode should be set");

        assert_eq!(expected_session_id(&fixture.runtime), Some("mode-session".to_string()));
        assert!(fixture.client.has_reusable_session("mode-session"));
        assert_eq!(
            agent.calls(),
            vec![
                RecordedCall::ResumeSession { session_id: "mode-session".to_string(), cwd },
                RecordedCall::SetMode {
                    session_id: "mode-session".to_string(),
                    mode_id: "plan".to_string(),
                },
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn actor_set_config_option_falls_back_to_load_when_resume_fails() {
    let agent = TestAgent::with_fail_resume();
    with_fixture(agent.clone(), async |fixture| {
        let cwd = PathBuf::from("/tmp/vw-acp-session-control");

        let response = fixture
            .client
            .actor_set_session_config_option(
                &mut fixture.runtime,
                "config-session".to_string(),
                cwd.clone(),
                "effort".to_string(),
                "high".to_string(),
            )
            .await
            .expect("config option should be set through load fallback");

        assert!(response.config_options.is_empty());
        assert_eq!(expected_session_id(&fixture.runtime), Some("config-session".to_string()));
        assert!(fixture.client.has_reusable_session("config-session"));
        assert_eq!(
            agent.calls(),
            vec![
                RecordedCall::ResumeSession {
                    session_id: "config-session".to_string(),
                    cwd: cwd.clone(),
                },
                RecordedCall::LoadSession { session_id: "config-session".to_string(), cwd },
                RecordedCall::SetConfig {
                    session_id: "config-session".to_string(),
                    option_name: "effort".to_string(),
                    value_id: "high".to_string(),
                },
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn actor_set_config_option_wraps_agent_error_with_context() {
    let agent = TestAgent::with_fail_set_config();
    with_fixture(agent, async |fixture| {
        let err = fixture
            .client
            .actor_set_session_config_option(
                &mut fixture.runtime,
                "config-session".to_string(),
                PathBuf::from("/tmp/vw-acp-session-control"),
                "effort".to_string(),
                "high".to_string(),
            )
            .await
            .expect_err("config option error should be wrapped");
        let message = err.to_string();

        assert!(matches!(err, AcpError::SetSessionConfigOption(_)));
        assert!(message.contains("session/set_config_option"));
        assert!(message.contains(r#"for "effort"="high""#));
    })
    .await;
}

#[tokio::test]
async fn actor_set_session_model_wraps_agent_error_with_context() {
    let agent = TestAgent::with_fail_set_model();
    with_fixture(agent, async |fixture| {
        let err = fixture
            .client
            .actor_set_session_model(
                &mut fixture.runtime,
                "model-session".to_string(),
                PathBuf::from("/tmp/vw-acp-session-control"),
                "bad-model".to_string(),
            )
            .await
            .expect_err("model error should be wrapped");
        let message = err.to_string();

        assert!(matches!(err, AcpError::SetSessionModel(_)));
        assert!(message.contains("session/set_model"));
        assert!(message.contains(r#"for model "bad-model""#));
    })
    .await;
}

#[tokio::test]
async fn actor_set_session_model_stores_session_for_reuse() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        let cwd = PathBuf::from("/tmp/vw-acp-session-control");

        fixture
            .client
            .actor_set_session_model(
                &mut fixture.runtime,
                "model-session".to_string(),
                cwd.clone(),
                "test-model".to_string(),
            )
            .await
            .expect("model should be set");

        assert_eq!(expected_session_id(&fixture.runtime), Some("model-session".to_string()));
        assert!(fixture.client.has_reusable_session("model-session"));
        assert_eq!(
            agent.calls(),
            vec![
                RecordedCall::ResumeSession { session_id: "model-session".to_string(), cwd },
                RecordedCall::SetModel {
                    session_id: "model-session".to_string(),
                    model: "test-model".to_string(),
                },
            ]
        );
    })
    .await;
}
