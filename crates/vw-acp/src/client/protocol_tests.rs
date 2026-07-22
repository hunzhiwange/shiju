use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex as StdMutex, MutexGuard};
use std::time::Duration;

use agent_client_protocol as acp;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::json;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::types::{AcpAgentConfig, AcpSessionOptions, SessionStrategy};

use super::{AcpClient, AcpError};

static ENV_TEST_LOCK: LazyLock<StdMutex<()>> = LazyLock::new(|| StdMutex::new(()));

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

#[derive(Default)]
struct AgentState {
    initialize_requests: Vec<acp::InitializeRequest>,
    new_session_requests: Vec<acp::NewSessionRequest>,
    load_session_requests: Vec<acp::LoadSessionRequest>,
    resume_session_requests: Vec<acp::ResumeSessionRequest>,
    initialize_error: bool,
    initialize_delay: Option<Duration>,
    auth_methods: Vec<acp::AuthMethod>,
    new_session_error: bool,
    new_session_delay: Option<Duration>,
    new_session_ids: VecDeque<String>,
    new_session_models: Option<acp::SessionModelState>,
    new_session_config_options: Option<Vec<acp::SessionConfigOption>>,
    load_results: VecDeque<bool>,
    resume_results: VecDeque<bool>,
}

#[derive(Clone, Default)]
struct TestAgent {
    state: Arc<Mutex<AgentState>>,
}

impl TestAgent {
    fn new() -> Self {
        Self::default()
    }

    fn failing_initialize() -> Self {
        let agent = Self::new();
        agent.state.lock().initialize_error = true;
        agent
    }

    fn delayed_initialize(delay: Duration) -> Self {
        let agent = Self::new();
        agent.state.lock().initialize_delay = Some(delay);
        agent
    }

    fn failing_new_session() -> Self {
        let agent = Self::new();
        agent.state.lock().new_session_error = true;
        agent
    }

    fn delayed_new_session(delay: Duration) -> Self {
        let agent = Self::new();
        agent.state.lock().new_session_delay = Some(delay);
        agent
    }

    fn with_new_session_ids(ids: impl IntoIterator<Item = &'static str>) -> Self {
        let agent = Self::new();
        agent.state.lock().new_session_ids = ids.into_iter().map(str::to_string).collect();
        agent
    }

    fn with_new_session_capabilities(
        models: acp::SessionModelState,
        config_options: Vec<acp::SessionConfigOption>,
    ) -> Self {
        let agent = Self::with_new_session_ids(["capable-session"]);
        let mut state = agent.state.lock();
        state.new_session_models = Some(models);
        state.new_session_config_options = Some(config_options);
        drop(state);
        agent
    }

    fn with_session_results(
        resume_results: impl IntoIterator<Item = bool>,
        load_results: impl IntoIterator<Item = bool>,
        new_session_ids: impl IntoIterator<Item = &'static str>,
    ) -> Self {
        let agent = Self::with_new_session_ids(new_session_ids);
        {
            let mut state = agent.state.lock();
            state.resume_results = resume_results.into_iter().collect();
            state.load_results = load_results.into_iter().collect();
        }
        agent
    }
}

#[async_trait(?Send)]
impl acp::Agent for TestAgent {
    async fn initialize(
        &self,
        args: acp::InitializeRequest,
    ) -> acp::Result<acp::InitializeResponse> {
        let (delay, should_fail, auth_methods) = {
            let mut state = self.state.lock();
            state.initialize_requests.push(args.clone());
            (state.initialize_delay, state.initialize_error, state.auth_methods.clone())
        };
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }
        if should_fail {
            return Err(acp::Error::invalid_params());
        }
        Ok(acp::InitializeResponse::new(args.protocol_version).auth_methods(auth_methods))
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
        let (delay, should_fail, session_id, models, config_options) = {
            let mut state = self.state.lock();
            state.new_session_requests.push(args);
            let session_id =
                state.new_session_ids.pop_front().unwrap_or_else(|| "new-session".to_string());
            (
                state.new_session_delay,
                state.new_session_error,
                session_id,
                state.new_session_models.clone(),
                state.new_session_config_options.clone(),
            )
        };
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }
        if should_fail {
            return Err(acp::Error::invalid_params());
        }
        Ok(acp::NewSessionResponse::new(session_id).models(models).config_options(config_options))
    }

    async fn load_session(
        &self,
        args: acp::LoadSessionRequest,
    ) -> acp::Result<acp::LoadSessionResponse> {
        let should_succeed = {
            let mut state = self.state.lock();
            state.load_session_requests.push(args);
            state.load_results.pop_front().unwrap_or(true)
        };
        if !should_succeed {
            return Err(acp::Error::invalid_params());
        }
        Ok(acp::LoadSessionResponse::new())
    }

    async fn resume_session(
        &self,
        args: acp::ResumeSessionRequest,
    ) -> acp::Result<acp::ResumeSessionResponse> {
        let should_succeed = {
            let mut state = self.state.lock();
            state.resume_session_requests.push(args);
            state.resume_results.pop_front().unwrap_or(true)
        };
        if !should_succeed {
            return Err(acp::Error::invalid_params());
        }
        Ok(acp::ResumeSessionResponse::new())
    }

    async fn prompt(&self, _args: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn cancel(&self, _args: acp::CancelNotification) -> acp::Result<()> {
        Ok(())
    }
}

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let lock = ENV_TEST_LOCK.lock().expect("env test lock should acquire");
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original, _lock: lock }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
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
}

fn connection_pair(agent: TestAgent) -> (acp::ClientSideConnection, TestAgent) {
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
    let (agent_conn, agent_io_task) = acp::AgentSideConnection::new(
        agent.clone(),
        agent_out.compat(),
        agent_in.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );

    tokio::task::spawn_local(client_io_task);
    tokio::task::spawn_local(agent_io_task);
    drop(agent_conn);

    (client_conn, agent)
}

async fn with_connection<R>(
    agent: TestAgent,
    f: impl AsyncFnOnce(acp::ClientSideConnection, TestAgent) -> R,
) -> R {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (conn, agent) = connection_pair(agent);
            f(conn, agent).await
        })
        .await
}

#[tokio::test]
async fn initialize_connection_sends_client_capabilities_and_info() {
    with_connection(TestAgent::new(), async |conn, agent| {
        client("agent", &[])
            .with_client_info("client-name", "1.2.3")
            .initialize_connection(&conn)
            .await
            .expect("initialize should succeed");

        let state = agent.state.lock();
        let request = state.initialize_requests.first().expect("initialize request should record");
        assert_eq!(request.protocol_version, acp::ProtocolVersion::V1);
        assert!(request.client_capabilities.fs.read_text_file);
        assert!(request.client_capabilities.fs.write_text_file);
        assert!(request.client_capabilities.terminal);
        let client_info = request.client_info.as_ref().expect("client info should be sent");
        assert_eq!(client_info.name, "client-name");
        assert_eq!(client_info.version, "1.2.3");
        assert_eq!(client_info.title.as_deref(), Some("VibeWindow ACP Client"));
    })
    .await;
}

#[tokio::test]
async fn initialize_connection_maps_agent_errors() {
    with_connection(TestAgent::failing_initialize(), async |conn, _agent| {
        let err = client("agent", &[])
            .initialize_connection(&conn)
            .await
            .expect_err("initialize error should map");

        assert!(matches!(err, AcpError::Initialize(_)));
    })
    .await;
}

#[tokio::test]
async fn initialize_connection_maps_gemini_agent_errors_after_timeout_wrapper() {
    with_connection(TestAgent::failing_initialize(), async |conn, _agent| {
        let err = client("gemini", &["--acp"])
            .initialize_connection(&conn)
            .await
            .expect_err("gemini initialize error should map");

        assert!(matches!(err, AcpError::Initialize(_)));
    })
    .await;
}

#[tokio::test]
async fn initialize_connection_times_out_for_gemini_acp_command() {
    let _timeout = EnvGuard::set("VWACP_GEMINI_ACP_STARTUP_TIMEOUT_MS", "1");

    with_connection(
        TestAgent::delayed_initialize(Duration::from_millis(50)),
        async |conn, _agent| {
            let err = client("gemini", &["--experimental-acp"])
                .initialize_connection(&conn)
                .await
                .expect_err("gemini initialize should time out");

            assert!(
                matches!(err, AcpError::GeminiStartupTimeout(message) if message.contains("gemini"))
            );
        },
    )
    .await;
}

#[tokio::test]
async fn new_session_id_sends_mcp_servers_and_session_meta() {
    let mcp_servers =
        vec![acp::McpServer::Http(acp::McpServerHttp::new("project", "https://example.test/mcp"))];
    let session_options = AcpSessionOptions {
        model: Some("sonnet".to_string()),
        allowed_tools: Some(vec![" shell ".to_string(), "".to_string(), "edit".to_string()]),
        max_turns: Some(4),
    };
    let cwd = Path::new("/tmp/vw-acp-protocol-new");

    with_connection(TestAgent::with_new_session_ids(["created-id"]), async |conn, agent| {
        let session_id = client("agent", &[])
            .with_mcp_servers(mcp_servers.clone())
            .with_session_options(Some(session_options))
            .new_session_id(&conn, cwd)
            .await
            .expect("session should be created");

        assert_eq!(session_id, "created-id");
        let state = agent.state.lock();
        let request = state.new_session_requests.first().expect("new session request");
        assert_eq!(request.cwd, cwd);
        assert_eq!(request.mcp_servers, mcp_servers);
        assert_eq!(
            request.meta.as_ref().and_then(|meta| meta.get("claudeCode")),
            Some(&json!({
                "options": {
                    "allowedTools": ["shell", "edit"],
                    "maxTurns": 4,
                    "model": "sonnet"
                }
            }))
        );
    })
    .await;
}

#[tokio::test]
async fn new_session_info_preserves_advertised_models_and_config_options() {
    let models = acp::SessionModelState::new(
        "model-a",
        vec![acp::ModelInfo::new("model-a", "Model A"), acp::ModelInfo::new("model-b", "Model B")],
    );
    let effort = acp::SessionConfigOption::select(
        "effort",
        "Effort",
        "medium",
        vec![
            acp::SessionConfigSelectOption::new("medium", "Medium"),
            acp::SessionConfigSelectOption::new("high", "High"),
        ],
    )
    .category(acp::SessionConfigOptionCategory::ThoughtLevel);
    let agent = TestAgent::with_new_session_capabilities(models.clone(), vec![effort.clone()]);

    with_connection(agent, async |conn, _agent| {
        let info = client("agent", &[])
            .new_session_info(&conn, Path::new("/tmp/vw-acp-protocol-capabilities"))
            .await
            .expect("new session should retain capabilities");

        assert_eq!(info.session_id, "capable-session");
        assert_eq!(info.models, Some(models));
        assert_eq!(info.config_options, Some(vec![effort]));
    })
    .await;
}

#[tokio::test]
async fn new_session_id_maps_agent_errors() {
    with_connection(TestAgent::failing_new_session(), async |conn, _agent| {
        let err = client("agent", &[])
            .new_session_id(&conn, Path::new("/tmp/vw-acp-protocol-error"))
            .await
            .expect_err("new session error should map");

        assert!(matches!(err, AcpError::NewSession(_)));
    })
    .await;
}

#[tokio::test]
async fn new_session_id_maps_claude_agent_errors_after_timeout_wrapper() {
    with_connection(TestAgent::failing_new_session(), async |conn, _agent| {
        let err = client("npx", &["@agentclientprotocol/claude-agent-acp@^0.26.0"])
            .new_session_id(&conn, Path::new("/tmp/vw-acp-protocol-claude-error"))
            .await
            .expect_err("claude new session error should map");

        assert!(matches!(err, AcpError::NewSession(_)));
    })
    .await;
}

#[tokio::test]
async fn new_session_id_times_out_for_claude_acp_command() {
    let _timeout = EnvGuard::set("VWACP_CLAUDE_ACP_SESSION_CREATE_TIMEOUT_MS", "1");

    with_connection(TestAgent::delayed_new_session(Duration::from_millis(50)), async |conn, _agent| {
        let err = client("/opt/bin/claude-agent-acp", &[])
            .new_session_id(&conn, Path::new("/tmp/vw-acp-protocol-claude-timeout"))
            .await
            .expect_err("claude new session should time out");

        assert!(
            matches!(err, AcpError::ClaudeSessionCreateTimeout(message) if message.contains("Claude ACP session creation timed out"))
        );
    })
    .await;
}

#[tokio::test]
async fn load_and_resume_session_ids_forward_requests_and_map_errors() {
    let mcp_servers =
        vec![acp::McpServer::Http(acp::McpServerHttp::new("project", "https://example.test/mcp"))];
    let cwd = Path::new("/tmp/vw-acp-protocol-existing");

    with_connection(
        TestAgent::with_session_results([true, false], [true, false], []),
        async |conn, agent| {
            let acp_client = client("agent", &[]).with_mcp_servers(mcp_servers.clone());

            let loaded = acp_client
                .load_session_id(&conn, cwd, "load-id".to_string())
                .await
                .expect("load should succeed");
            assert_eq!(loaded, "load-id");

            let resumed = acp_client
                .resume_session_id(&conn, cwd, "resume-id".to_string())
                .await
                .expect("resume should succeed");
            assert_eq!(resumed, "resume-id");

            let load_err = acp_client
                .load_session_id(&conn, cwd, "bad-load".to_string())
                .await
                .expect_err("load should map agent error");
            assert!(matches!(load_err, AcpError::LoadSession(_)));

            let resume_err = acp_client
                .resume_session_id(&conn, cwd, "bad-resume".to_string())
                .await
                .expect_err("resume should map agent error");
            assert!(matches!(resume_err, AcpError::ResumeSession(_)));

            let state = agent.state.lock();
            assert_eq!(state.load_session_requests[0].session_id.0.as_ref(), "load-id");
            assert_eq!(state.load_session_requests[0].cwd, cwd);
            assert_eq!(state.load_session_requests[0].mcp_servers, mcp_servers);
            assert_eq!(state.resume_session_requests[0].session_id.0.as_ref(), "resume-id");
            assert_eq!(state.resume_session_requests[0].cwd, cwd);
            assert_eq!(state.resume_session_requests[0].mcp_servers, mcp_servers);
        },
    )
    .await;
}

#[tokio::test]
async fn resolve_session_strategy_success_shortcuts_are_preserved() {
    let cwd = Path::new("/tmp/vw-acp-protocol-resolve-shortcuts");

    with_connection(
        TestAgent::with_session_results([true, false], [true, true], []),
        async |conn, _agent| {
            let acp_client = client("agent", &[]);
            let expected_session_id = Arc::new(Mutex::new(None));

            let cases = [
                (SessionStrategy::ResumeLoadOrNew("resume-short".to_string()), "resume-short"),
                (SessionStrategy::ResumeLoadOrNew("load-short".to_string()), "load-short"),
                (SessionStrategy::LoadOrNew("load-ok".to_string()), "load-ok"),
            ];

            for (strategy, expected) in cases {
                let resolved = acp_client
                    .resolve_session(&conn, cwd, &strategy, &expected_session_id)
                    .await
                    .expect("strategy shortcut should resolve");

                assert_eq!(resolved, expected);
                assert_eq!(expected_session_id.lock().as_deref(), Some(expected));
            }
        },
    )
    .await;
}

#[tokio::test]
async fn resolve_session_strategies_update_expected_session_id() {
    let cwd = Path::new("/tmp/vw-acp-protocol-resolve");

    with_connection(
        TestAgent::with_session_results(
            [true, true, false, false],
            [true, true, false, false],
            ["new-direct", "resume-load-new", "load-new"],
        ),
        async |conn, _agent| {
            let acp_client = client("agent", &[]);
            let expected_session_id = Arc::new(Mutex::new(None));

            let cases = [
                (SessionStrategy::New, "new-direct"),
                (SessionStrategy::Load("load-direct".to_string()), "load-direct"),
                (SessionStrategy::Resume("resume-direct".to_string()), "resume-direct"),
                (SessionStrategy::ResumeOrLoad("resume-ok".to_string()), "resume-ok"),
                (
                    SessionStrategy::ResumeOrLoad("load-after-resume".to_string()),
                    "load-after-resume",
                ),
                (
                    SessionStrategy::ResumeLoadOrNew("resume-load-new".to_string()),
                    "resume-load-new",
                ),
                (SessionStrategy::LoadOrNew("load-new".to_string()), "load-new"),
            ];

            for (strategy, expected) in cases {
                let resolved = acp_client
                    .resolve_session(&conn, cwd, &strategy, &expected_session_id)
                    .await
                    .expect("strategy should resolve");

                assert_eq!(resolved, expected);
                assert_eq!(expected_session_id.lock().as_deref(), Some(expected));
            }
        },
    )
    .await;
}

#[tokio::test]
async fn resolve_existing_session_resumes_or_falls_back_to_load() {
    let cwd = Path::new("/tmp/vw-acp-protocol-existing-resolve");

    with_connection(
        TestAgent::with_session_results([true, false], [true], []),
        async |conn, _agent| {
            let acp_client = client("agent", &[]);
            let expected_session_id = Arc::new(Mutex::new(None));

            acp_client
                .resolve_existing_session(
                    &conn,
                    cwd,
                    "resume-existing".to_string(),
                    &expected_session_id,
                )
                .await
                .expect("resume should resolve existing session");
            assert_eq!(expected_session_id.lock().as_deref(), Some("resume-existing"));

            acp_client
                .resolve_existing_session(
                    &conn,
                    cwd,
                    "load-existing".to_string(),
                    &expected_session_id,
                )
                .await
                .expect("load fallback should resolve existing session");
            assert_eq!(expected_session_id.lock().as_deref(), Some("load-existing"));
        },
    )
    .await;
}

#[test]
fn command_detection_uses_basename_and_acp_flags() {
    assert!(client("/opt/bin/gemini", &["--acp"]).is_gemini_acp_command());
    assert!(client("gemini", &["--experimental-acp"]).is_gemini_acp_command());
    assert!(!client("gemini", &["chat"]).is_gemini_acp_command());
    assert!(!client("other", &["--acp"]).is_gemini_acp_command());

    assert!(client("/opt/bin/claude-agent-acp", &[]).is_claude_acp_command());
    assert!(
        client("npx", &["@agentclientprotocol/claude-agent-acp@^0.26.0"]).is_claude_acp_command()
    );
    assert!(!client("claude", &["--print"]).is_claude_acp_command());
}
