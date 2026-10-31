//! 会话连接与加载流程的单元测试。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_client_protocol::{
    ModelInfo, SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOption,
    SessionModelState, SetSessionConfigOptionResponse,
};
use async_trait::async_trait;

use super::connect_load::{
    ConnectAndLoadClient, ConnectAndLoadClientSession, ConnectAndLoadSessionError,
    ConnectAndLoadSessionOptions, connect_and_load_session,
};
use super::lifecycle::{AgentLifecycleExit, AgentLifecycleSnapshot};
use crate::error::AcpError;
use crate::queue_owner_turn_controller::{QueueControlFuture, QueueOwnerActiveSessionController};
use crate::{
    AcpAgentConfig, AcpClient, SESSION_RECORD_SCHEMA, SessionAcpxState, SessionEventLog,
    SessionRecord, SessionResumePolicy, SessionStateOptions, SessionTokenUsage,
};

static UNIQUE_TEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct TestActiveController;

impl QueueOwnerActiveSessionController for TestActiveController {
    fn has_active_prompt(&self) -> bool {
        false
    }

    fn request_cancel_active_prompt(&self) -> QueueControlFuture<bool> {
        Box::pin(async move { Ok(false) })
    }

    fn set_session_mode(&self, _mode_id: String) -> QueueControlFuture<()> {
        Box::pin(async move { Ok(()) })
    }

    fn set_session_model(&self, _model_id: String) -> QueueControlFuture<()> {
        Box::pin(async move { Ok(()) })
    }

    fn set_session_config_option(
        &self,
        _config_id: String,
        _value: String,
    ) -> QueueControlFuture<SetSessionConfigOptionResponse> {
        Box::pin(async move { unreachable!() })
    }
}

#[derive(Debug, Default)]
struct MockClientState {
    start_calls: usize,
    create_calls: usize,
    load_calls: usize,
    set_mode_calls: Vec<String>,
    set_model_calls: Vec<String>,
    set_config_calls: Vec<(String, String)>,
    protocol_calls: Vec<String>,
}

struct MockClient {
    state: Arc<Mutex<MockClientState>>,
    reusable_session: bool,
    supports_load_session: bool,
    lifecycle_snapshot: AgentLifecycleSnapshot,
    create_session: ConnectAndLoadClientSession,
    load_session: Result<ConnectAndLoadClientSession, String>,
    start_error: Option<String>,
    create_session_error: Option<String>,
    load_delay_ms: Option<u64>,
    set_mode_error: Option<String>,
    set_model_error: Option<String>,
    config_options_after_model: Option<Vec<SessionConfigOption>>,
}

impl MockClient {
    fn new(
        create_session: ConnectAndLoadClientSession,
        load_session: Result<ConnectAndLoadClientSession, String>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(MockClientState::default())),
            reusable_session: false,
            supports_load_session: true,
            lifecycle_snapshot: AgentLifecycleSnapshot::default(),
            create_session,
            load_session,
            start_error: None,
            create_session_error: None,
            load_delay_ms: None,
            set_mode_error: None,
            set_model_error: None,
            config_options_after_model: None,
        }
    }
}

#[async_trait(?Send)]
impl ConnectAndLoadClient for MockClient {
    async fn start(&self) -> Result<(), AcpError> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).start_calls += 1;
        match self.start_error.as_ref() {
            Some(message) => Err(AcpError::Initialize(message.clone())),
            None => Ok(()),
        }
    }

    async fn create_session(&self, _cwd: &Path) -> Result<ConnectAndLoadClientSession, AcpError> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).create_calls += 1;
        match self.create_session_error.as_ref() {
            Some(message) => Err(AcpError::NewSession(message.clone())),
            None => Ok(self.create_session.clone()),
        }
    }

    async fn load_session_with_options(
        &self,
        _session_id: &str,
        _cwd: &Path,
    ) -> Result<ConnectAndLoadClientSession, AcpError> {
        if let Some(delay_ms) = self.load_delay_ms {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).load_calls += 1;
        match &self.load_session {
            Ok(session) => Ok(session.clone()),
            Err(message) => Err(AcpError::LoadSession(message.clone())),
        }
    }

    async fn set_session_mode(
        &self,
        _session_id: &str,
        _cwd: &Path,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .set_mode_calls
            .push(mode_id.to_string());
        match self.set_mode_error.as_ref() {
            Some(message) => Err(AcpError::SetSessionConfigOption(message.clone())),
            None => Ok(()),
        }
    }

    async fn set_session_model(
        &self,
        _session_id: &str,
        _cwd: &Path,
        model_id: &str,
    ) -> Result<(), AcpError> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .set_model_calls
            .push(model_id.to_string());
        match self.set_model_error.as_ref() {
            Some(message) => Err(AcpError::SetSessionModel(message.clone())),
            None => Ok(()),
        }
    }

    async fn set_session_config_option(
        &self,
        _session_id: &str,
        _cwd: &Path,
        config_id: &str,
        value: &str,
    ) -> Result<SetSessionConfigOptionResponse, AcpError> {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.set_config_calls.push((config_id.to_string(), value.to_string()));
        state.protocol_calls.push(format!("config:{config_id}:{value}"));
        let config_options = self
            .config_options_after_model
            .clone()
            .or_else(|| self.create_session.config_options.clone())
            .unwrap_or_default();
        Ok(SetSessionConfigOptionResponse::new(config_options))
    }

    fn has_reusable_session(&self, _session_id: &str) -> bool {
        self.reusable_session
    }

    fn supports_load_session(&self) -> bool {
        self.supports_load_session
    }

    fn get_agent_lifecycle_snapshot(&self) -> AgentLifecycleSnapshot {
        self.lifecycle_snapshot.clone()
    }
}

fn sample_record() -> SessionRecord {
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: Some("agent-1".to_string()),
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/project/active.jsonl".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 4,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(true),
        closed_at: Some("2026-01-02T00:00:00Z".to_string()),
        pid: None,
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: None,
        messages: Vec::new(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: Some(SessionAcpxState {
            current_mode_id: None,
            desired_mode_id: Some("focus".to_string()),
            current_model_id: Some("model-a".to_string()),
            available_models: None,
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: Some(SessionStateOptions {
                model: Some("model-b".to_string()),
                allowed_tools: None,
                max_turns: None,
            }),
        }),
    }
}

fn select_config(
    id: &str,
    current: &str,
    values: &[&str],
    category: Option<SessionConfigOptionCategory>,
) -> SessionConfigOption {
    SessionConfigOption::select(
        id.to_string(),
        id.to_string(),
        current.to_string(),
        values
            .iter()
            .map(|value| SessionConfigSelectOption::new((*value).to_string(), (*value).to_string()))
            .collect::<Vec<_>>(),
    )
    .category(category)
}

#[tokio::test]
async fn session_reasoning_effort_validates_and_replays_after_model() {
    let initial_options = vec![
        select_config(
            "model_choice",
            "model-a",
            &["model-a", "model-b"],
            Some(SessionConfigOptionCategory::Model),
        ),
        select_config(
            "effort",
            "medium",
            &["default", "low", "medium", "high"],
            Some(SessionConfigOptionCategory::ThoughtLevel),
        ),
        select_config("temperature", "0.1", &["0.1", "0.2"], None),
    ];
    let after_model = vec![
        select_config(
            "model_choice",
            "model-b",
            &["model-a", "model-b"],
            Some(SessionConfigOptionCategory::Model),
        ),
        select_config(
            "effort",
            "medium",
            &["default", "high"],
            Some(SessionConfigOptionCategory::ThoughtLevel),
        ),
        select_config("temperature", "0.1", &["0.1", "0.2"], None),
    ];
    let mut record = sample_record();
    record.pid = Some(1_999_999_999);
    record.vwacp.as_mut().expect("vwacp state").desired_config_options = Some(
        [
            ("reasoning_effort".to_string(), "high".to_string()),
            ("temperature".to_string(), "0.2".to_string()),
        ]
        .into_iter()
        .collect(),
    );
    let persisted = serde_json::to_string(&record).expect("serialize desired config");
    record = serde_json::from_str(&persisted).expect("deserialize desired config");

    let mut mock = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: Some(initial_options),
        },
        Err("session not found".to_string()),
    );
    mock.config_options_after_model = Some(after_model);
    let client = Arc::new(mock);

    connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect("model and reasoning preferences should replay");

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(
        state.protocol_calls,
        vec![
            "config:model_choice:model-b".to_string(),
            "config:effort:high".to_string(),
            "config:temperature:0.2".to_string(),
        ]
    );
    assert!(state.set_model_calls.is_empty(), "model config must precede legacy set_model");
    drop(state);

    let available_error = crate::session_mode_preference::resolve_config_value(
        record.vwacp.as_ref().and_then(|state| state.config_options.as_deref()),
        "reasoning_effort",
        "xhigh",
    )
    .expect_err("unsupported effort must be rejected");
    assert!(available_error.contains("available values: default, high"));
}

fn sample_models() -> SessionModelState {
    SessionModelState::new(
        "model-a",
        vec![ModelInfo::new("model-a", "Current Model"), ModelInfo::new("model-b", "Model B")],
    )
}

#[cfg(unix)]
struct AcpClientAdapterFixture {
    cwd: PathBuf,
    script_path: PathBuf,
    calls_log: PathBuf,
}

#[cfg(unix)]
impl AcpClientAdapterFixture {
    fn new() -> Self {
        let id = UNIQUE_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("vw-acp-connect-load-adapter-test-{id}"));
        let cwd = base.join("cwd");
        let script_path = base.join("mock_agent.py");
        let calls_log = base.join("calls.jsonl");
        fs::create_dir_all(&cwd).expect("fixture cwd should be created");
        fs::write(&script_path, acp_client_adapter_script()).expect("fixture script should write");
        fs::write(&calls_log, "").expect("fixture calls log should write");
        Self { cwd, script_path, calls_log }
    }

    fn client(&self) -> AcpClient {
        AcpClient::new(
            "mock",
            AcpAgentConfig {
                command: "python3".to_string(),
                args: vec![
                    "-u".to_string(),
                    self.script_path.display().to_string(),
                    self.calls_log.display().to_string(),
                ],
                env: HashMap::new(),
            },
        )
    }

    fn logged_methods(&self) -> Vec<String> {
        fs::read_to_string(&self.calls_log)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let value: serde_json::Value = serde_json::from_str(line).ok()?;
                value.get("method")?.as_str().map(ToString::to_string)
            })
            .collect()
    }
}

#[cfg(unix)]
impl Drop for AcpClientAdapterFixture {
    fn drop(&mut self) {
        if let Some(base) = self.script_path.parent().filter(|path| path.exists()) {
            let _ = fs::remove_dir_all(base);
        }
    }
}

#[cfg(unix)]
fn acp_client_adapter_script() -> &'static str {
    r#"import json
import sys

calls_log = sys.argv[1]
session_counter = 0

def write_call(message):
    with open(calls_log, "a", encoding="utf-8") as handle:
        handle.write(json.dumps({
            "method": message.get("method"),
            "params": message.get("params", {}),
        }) + "\n")
        handle.flush()

for raw_line in sys.stdin:
    line = raw_line.strip()
    if not line:
        continue
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    write_call(message)

    if method == "initialize":
        result = {
            "protocolVersion": message["params"]["protocolVersion"],
            "agentCapabilities": {},
            "authMethods": [],
            "agentInfo": {"name": "mock-acp-agent", "version": "0.1.0"},
        }
    elif method == "session/new":
        session_counter += 1
        result = {"sessionId": f"session-{session_counter}"}
    elif method in (
        "session/load",
        "session/resume",
        "session/set_mode",
        "session/set_model",
    ):
        result = {}
    elif method == "session/set_config_option":
        result = {"configOptions": []}
    elif method == "session/prompt":
        result = {"stopReason": "end_turn"}
    else:
        if request_id is None:
            continue
        result = {}

    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()
"#
}

#[cfg(unix)]
#[tokio::test]
async fn acp_client_adapter_reuses_loaded_session_for_preference_changes() {
    let fixture = AcpClientAdapterFixture::new();
    let client = fixture.client();

    ConnectAndLoadClient::start(&client).await.expect("client should start");
    let created = ConnectAndLoadClient::create_session(&client, fixture.cwd.as_path())
        .await
        .expect("session should be created");
    let loaded =
        ConnectAndLoadClient::load_session_with_options(&client, "existing", fixture.cwd.as_path())
            .await
            .expect("session should load");

    ConnectAndLoadClient::set_session_mode(&client, "existing", fixture.cwd.as_path(), "focus")
        .await
        .expect("mode should set");
    ConnectAndLoadClient::set_session_model(&client, "existing", fixture.cwd.as_path(), "model-b")
        .await
        .expect("model should set");

    assert_eq!(created.session_id, "session-1");
    assert_eq!(loaded.session_id, "existing");
    assert!(ConnectAndLoadClient::has_reusable_session(&client, "existing"));
    assert!(ConnectAndLoadClient::supports_load_session(&client));
    assert!(ConnectAndLoadClient::get_agent_lifecycle_snapshot(&client).pid.is_some());
    client.close().await.expect("client should close");
    assert_eq!(
        fixture.logged_methods(),
        vec![
            "initialize".to_string(),
            "session/new".to_string(),
            "session/load".to_string(),
            "session/set_mode".to_string(),
            "session/set_model".to_string()
        ]
    );
    let calls = fs::read_to_string(&fixture.calls_log).expect("protocol calls");
    let calls = calls
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(calls[3]["params"], serde_json::json!({"sessionId":"existing", "modeId":"focus"}));
    assert_eq!(
        calls[4]["params"],
        serde_json::json!({"sessionId":"existing", "modelId":"model-b"})
    );
}

#[cfg(unix)]
#[tokio::test]
async fn acp_client_connect_and_load_adapter_delegates_config_option() {
    let fixture = AcpClientAdapterFixture::new();
    let client = fixture.client();

    ConnectAndLoadClient::start(&client).await.expect("client should start");
    let created = ConnectAndLoadClient::create_session(&client, fixture.cwd.as_path())
        .await
        .expect("session should be created");
    let response = ConnectAndLoadClient::set_session_config_option(
        &client,
        &created.session_id,
        fixture.cwd.as_path(),
        "effort",
        "high",
    )
    .await
    .expect("config option should set");

    assert!(response.config_options.is_empty());
    client.close().await.expect("client should close");
    assert!(fixture.logged_methods().iter().any(|method| method == "session/set_config_option"));
}

#[tokio::test]
async fn connect_and_load_session_falls_back_to_new_session_and_replays_preferences() {
    let mut record = sample_record();
    record.pid = Some(1_999_999_999);
    let client = Arc::new(MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("session not found".to_string()),
    ));

    let result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: true,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect("fallback to new session should succeed");

    assert_eq!(result.session_id, "session-2");
    assert_eq!(result.agent_session_id.as_deref(), Some("agent-2"));
    assert!(!result.resumed);
    assert_eq!(result.load_error.as_deref(), Some("acp load_session failed: session not found"));
    assert_eq!(record.acp_session_id, "session-2");
    assert_eq!(record.agent_session_id.as_deref(), Some("agent-2"));
    assert_eq!(record.closed, Some(false));
    assert_eq!(record.closed_at, None);
    assert_eq!(
        record.vwacp.as_ref().and_then(|state| state.current_model_id.as_deref()),
        Some("model-b")
    );
    assert_eq!(
        record.vwacp.as_ref().and_then(|state| state.available_models.as_ref()),
        Some(&vec!["model-a".to_string(), "model-b".to_string()])
    );

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 1);
    assert_eq!(state.create_calls, 1);
    assert_eq!(state.set_mode_calls, vec!["focus".to_string()]);
    assert_eq!(state.set_model_calls, vec!["model-b".to_string()]);
}

#[tokio::test]
async fn connect_and_load_session_loads_existing_session_and_updates_record_state() {
    let mut record = sample_record();
    record.pid = Some(running_child_process_id());
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "ignored-loaded-session".to_string(),
            agent_session_id: Some("agent-loaded".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Ok(ConnectAndLoadClientSession {
            session_id: "ignored-loaded-session".to_string(),
            agent_session_id: Some("agent-loaded".to_string()),
            models: Some(sample_models()),
            config_options: None,
        }),
    );
    client.lifecycle_snapshot = AgentLifecycleSnapshot {
        pid: Some(1234),
        started_at: Some("2026-01-03T00:00:00Z".to_string()),
        last_exit: Some(AgentLifecycleExit {
            exit_code: Some(0),
            signal: None,
            exited_at: Some("2026-01-03T00:01:00Z".to_string()),
            reason: Some("clean-exit".to_string()),
            unexpected_during_prompt: false,
        }),
    };
    let client = Arc::new(client);
    let client_available_calls = Arc::new(Mutex::new(0usize));
    let connected_records = Arc::new(Mutex::new(Vec::new()));
    let resolved_session_ids = Arc::new(Mutex::new(Vec::new()));

    let result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: None,
        timeout_ms: Some(5_000),
        verbose: true,
        active_controller: Arc::new(TestActiveController),
        on_client_available: Some({
            let client_available_calls = client_available_calls.clone();
            Arc::new(move |_controller| {
                *client_available_calls.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) +=
                    1;
            })
        }),
        on_connected_record: Some({
            let connected_records = connected_records.clone();
            Arc::new(move |record| {
                connected_records
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(record.acp_session_id.clone());
            })
        }),
        on_session_id_resolved: Some({
            let resolved_session_ids = resolved_session_ids.clone();
            Arc::new(move |session_id| {
                resolved_session_ids
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(session_id.to_string());
            })
        }),
    })
    .await
    .expect("load session should succeed");

    assert_eq!(result.session_id, "session-1");
    assert_eq!(result.agent_session_id.as_deref(), Some("agent-loaded"));
    assert!(result.resumed);
    assert_eq!(result.load_error, None);
    assert_eq!(record.acp_session_id, "session-1");
    assert_eq!(record.agent_session_id.as_deref(), Some("agent-loaded"));
    assert_eq!(record.closed, Some(false));
    assert_eq!(record.closed_at, None);
    assert_eq!(record.pid, Some(1234));
    assert_eq!(record.agent_started_at.as_deref(), Some("2026-01-03T00:00:00Z"));
    assert_eq!(record.last_agent_exit_code, Some(0));
    assert_eq!(record.last_agent_disconnect_reason.as_deref(), Some("clean-exit"));
    assert_eq!(
        record.vwacp.as_ref().and_then(|state| state.current_model_id.as_deref()),
        Some("model-a")
    );
    assert_eq!(
        record.vwacp.as_ref().and_then(|state| state.available_models.as_ref()),
        Some(&vec!["model-a".to_string(), "model-b".to_string()])
    );
    assert_eq!(*client_available_calls.lock().unwrap_or_else(|poisoned| poisoned.into_inner()), 1);
    assert_eq!(
        *connected_records.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
        vec!["session-1".to_string()]
    );
    assert_eq!(
        *resolved_session_ids.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
        vec!["session-1".to_string()]
    );

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 1);
    assert_eq!(state.create_calls, 0);
    assert!(state.set_mode_calls.is_empty());
    assert!(state.set_model_calls.is_empty());
}

fn running_child_process_id() -> u32 {
    let mut child = Command::new("sleep").arg("1").spawn().expect("sleep process should start");
    let pid = child.id();
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    pid
}

#[tokio::test]
async fn connect_and_load_session_reuses_loaded_session_without_starting_client() {
    let mut record = sample_record();
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("load should not be called".to_string()),
    );
    client.reusable_session = true;
    let client = Arc::new(client);

    let result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::SameSessionOnly),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect("reusable session should be accepted");

    assert_eq!(result.session_id, "session-1");
    assert_eq!(result.agent_session_id.as_deref(), Some("agent-1"));
    assert!(result.resumed);
    assert_eq!(result.load_error, None);

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 0);
    assert_eq!(state.load_calls, 0);
    assert_eq!(state.create_calls, 0);
}

#[tokio::test]
async fn connect_and_load_session_creates_new_session_when_load_is_unsupported() {
    let mut record = sample_record();
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: None,
            models: None,
            config_options: None,
        },
        Err("load should not be called".to_string()),
    );
    client.supports_load_session = false;
    let client = Arc::new(client);

    let result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect("unsupported load should create a fresh session when allowed");

    assert_eq!(result.session_id, "session-2");
    assert_eq!(result.agent_session_id.as_deref(), Some("agent-1"));
    assert!(!result.resumed);
    assert_eq!(record.acp_session_id, "session-2");
    assert_eq!(record.agent_session_id.as_deref(), Some("agent-1"));

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 0);
    assert_eq!(state.create_calls, 1);
}

#[tokio::test]
async fn connect_and_load_session_requires_same_session_when_load_is_unsupported() {
    let mut record = sample_record();
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: None,
            models: None,
            config_options: None,
        },
        Err("load should not be called".to_string()),
    );
    client.supports_load_session = false;
    let client = Arc::new(client);

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::SameSessionOnly),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("same-session-only should reject unsupported load");

    assert!(matches!(error, ConnectAndLoadSessionError::SessionResumeRequired(_)));
    assert_eq!(record.acp_session_id, "session-1");

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 0);
    assert_eq!(state.create_calls, 0);
}

#[tokio::test]
async fn connect_and_load_session_returns_non_fallback_load_error() {
    let mut record = sample_record();
    let client = Arc::new(MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("permission denied".to_string()),
    ));

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("non-fallback load errors should be returned");

    assert!(matches!(error, ConnectAndLoadSessionError::Acp(AcpError::LoadSession(_))));
    assert_eq!(record.acp_session_id, "session-1");

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 1);
    assert_eq!(state.create_calls, 0);
}

#[tokio::test]
async fn connect_and_load_session_returns_start_error_without_loading() {
    let mut record = sample_record();
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Ok(ConnectAndLoadClientSession {
            session_id: "session-1".to_string(),
            agent_session_id: Some("agent-1".to_string()),
            models: Some(sample_models()),
            config_options: None,
        }),
    );
    client.start_error = Some("initialize failed".to_string());
    let client = Arc::new(client);

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("start error should be returned");

    assert!(matches!(error, ConnectAndLoadSessionError::Acp(AcpError::Initialize(_))));

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 0);
    assert_eq!(state.create_calls, 0);
}

#[tokio::test]
async fn connect_and_load_session_does_not_fallback_when_load_times_out() {
    let mut record = sample_record();
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("session not found".to_string()),
    );
    client.load_delay_ms = Some(50);
    let client = Arc::new(client);

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(1),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("load timeout should not fallback");

    assert!(matches!(error, ConnectAndLoadSessionError::Timeout(_)));

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 0);
    assert_eq!(state.create_calls, 0);
}

#[tokio::test]
async fn connect_and_load_session_returns_create_error_after_fallback() {
    let mut record = sample_record();
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("session not found".to_string()),
    );
    client.create_session_error = Some("create failed".to_string());
    let client = Arc::new(client);

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("create error should be returned after fallback");

    assert!(matches!(error, ConnectAndLoadSessionError::Acp(AcpError::NewSession(_))));
    assert_eq!(record.acp_session_id, "session-1");

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.start_calls, 1);
    assert_eq!(state.load_calls, 1);
    assert_eq!(state.create_calls, 1);
}

#[tokio::test]
async fn connect_and_load_session_falls_back_for_unsupported_load_error_codes() {
    for message in ["-32601 method not found", "-32602 invalid params"] {
        let mut record = sample_record();
        record.vwacp.as_mut().expect("vwacp state").desired_mode_id = None;
        let client = Arc::new(MockClient::new(
            ConnectAndLoadClientSession {
                session_id: "session-2".to_string(),
                agent_session_id: Some("agent-2".to_string()),
                models: Some(sample_models()),
                config_options: None,
            },
            Err(message.to_string()),
        ));

        let result = connect_and_load_session(ConnectAndLoadSessionOptions {
            client: client.clone(),
            record: &mut record,
            resume_policy: Some(SessionResumePolicy::AllowNew),
            timeout_ms: Some(5_000),
            verbose: false,
            active_controller: Arc::new(TestActiveController),
            on_client_available: None,
            on_connected_record: None,
            on_session_id_resolved: None,
        })
        .await
        .expect("unsupported load errors should fallback");

        assert_eq!(result.session_id, "session-2");
        assert!(!result.resumed);

        let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(state.load_calls, 1);
        assert_eq!(state.create_calls, 1);
    }
}

#[tokio::test]
async fn connect_and_load_session_falls_back_for_closed_query_without_agent_messages() {
    let mut record = sample_record();
    record.vwacp = None;
    let client = Arc::new(MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: None,
            models: None,
            config_options: None,
        },
        Err("query closed before response".to_string()),
    ));

    let result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect("closed query without agent messages should fallback");

    assert_eq!(result.session_id, "session-2");
    assert_eq!(record.agent_session_id.as_deref(), Some("agent-1"));

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.create_calls, 1);
    assert!(state.set_mode_calls.is_empty());
    assert!(state.set_model_calls.is_empty());
}

#[tokio::test]
async fn connect_and_load_session_falls_back_for_internal_error_without_agent_messages() {
    let mut record = sample_record();
    record.vwacp.as_mut().expect("vwacp state").desired_mode_id = None;
    let client = Arc::new(MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(SessionModelState::new(
                "model-b",
                vec![ModelInfo::new("model-b", "Model B")],
            )),
            config_options: None,
        },
        Err("-32603 internal error".to_string()),
    ));

    let result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect("internal error without agent messages should fallback");

    assert_eq!(result.session_id, "session-2");
    assert_eq!(
        record.vwacp.as_ref().and_then(|state| state.current_model_id.as_deref()),
        Some("model-b")
    );

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.create_calls, 1);
    assert!(state.set_model_calls.is_empty());
}

#[tokio::test]
async fn connect_and_load_session_does_not_fallback_for_internal_error_after_agent_messages() {
    let mut record = sample_record();
    record.messages.push(crate::SessionMessage::Agent(crate::SessionAgentMessage {
        content: vec![crate::SessionAgentContent::Text("answer".to_string())],
        tool_results: HashMap::new(),
        reasoning_details: None,
    }));
    let client = Arc::new(MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("-32603 internal error".to_string()),
    ));

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("internal error after agent messages should not fallback");

    assert!(matches!(error, ConnectAndLoadSessionError::Acp(AcpError::LoadSession(_))));

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.load_calls, 1);
    assert_eq!(state.create_calls, 0);
}

#[tokio::test]
async fn connect_and_load_session_falls_back_for_generic_not_found_message() {
    let mut record = sample_record();
    record.vwacp.as_mut().expect("vwacp state").desired_mode_id = None;
    let client = Arc::new(MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: None,
            config_options: None,
        },
        Err("workspace not found".to_string()),
    ));

    let result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect("generic not found load error should fallback");

    assert_eq!(result.session_id, "session-2");

    let state = client.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.create_calls, 1);
}

#[tokio::test]
async fn connect_and_load_session_requires_same_session_when_load_fails() {
    let mut record = sample_record();
    let client = Arc::new(MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("session not found".to_string()),
    ));

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client,
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::SameSessionOnly),
        timeout_ms: Some(5_000),
        verbose: false,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("same-session-only should reject fallback");

    assert!(matches!(error, ConnectAndLoadSessionError::SessionResumeRequired(_)));
    assert_eq!(record.acp_session_id, "session-1");
    assert_eq!(record.agent_session_id.as_deref(), Some("agent-1"));
}

#[tokio::test]
async fn connect_and_load_session_restores_original_ids_when_mode_replay_fails() {
    let mut record = sample_record();
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("session not found".to_string()),
    );
    client.set_mode_error = Some("mode replay failed".to_string());
    let client = Arc::new(client);

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client,
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: true,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("mode replay failure should be surfaced");

    assert!(matches!(error, ConnectAndLoadSessionError::SessionModeReplay(_)));
    assert_eq!(record.acp_session_id, "session-1");
    assert_eq!(record.agent_session_id.as_deref(), Some("agent-1"));
}

#[tokio::test]
async fn connect_and_load_session_restores_original_ids_when_model_replay_fails() {
    let mut record = sample_record();
    record.vwacp.as_mut().expect("vwacp state").desired_mode_id = None;
    let mut client = MockClient::new(
        ConnectAndLoadClientSession {
            session_id: "session-2".to_string(),
            agent_session_id: Some("agent-2".to_string()),
            models: Some(sample_models()),
            config_options: None,
        },
        Err("session not found".to_string()),
    );
    client.set_model_error = Some("model replay failed".to_string());
    let client = Arc::new(client);

    let error = connect_and_load_session(ConnectAndLoadSessionOptions {
        client,
        record: &mut record,
        resume_policy: Some(SessionResumePolicy::AllowNew),
        timeout_ms: Some(5_000),
        verbose: true,
        active_controller: Arc::new(TestActiveController),
        on_client_available: None,
        on_connected_record: None,
        on_session_id_resolved: None,
    })
    .await
    .expect_err("model replay failure should be surfaced");

    assert!(matches!(error, ConnectAndLoadSessionError::SessionModelReplay(_)));
    assert_eq!(record.acp_session_id, "session-1");
    assert_eq!(record.agent_session_id.as_deref(), Some("agent-1"));
}
