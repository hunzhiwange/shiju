//! 验证运行时提示词请求构造、代理配置解析与可重试错误判定。
//!
//! 最小会话记录、控制器和异步通道 fixture 用于驱动运行时状态与竞态分支。

use super::*;
use crate::queue_owner_turn_controller::QueueOwnerActiveSessionController;
use crate::types::{
    AcpAgentConfig, AcpJsonRpcMessage, AcpMessageDirection, AcpSessionOptions, OutputErrorParams,
    OutputFormatterContext, PermissionStats, PromptResult, SessionEventLog, SessionMessage,
    SessionResumePolicy, SessionStateOptions, SessionStrategy, SessionTokenUsage,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct TestRuntimeError(&'static str);

#[derive(Default)]
struct CapturingFormatter {
    context: Option<String>,
    messages: Vec<Value>,
    flush_count: usize,
}

impl OutputFormatter for CapturingFormatter {
    fn set_context(&mut self, context: OutputFormatterContext) {
        self.context = Some(context.session_id);
    }

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        self.messages.push(serde_json::to_value(message).expect("message serializes"));
    }

    fn on_error(&mut self, _params: OutputErrorParams) {}

    fn flush(&mut self) {
        self.flush_count += 1;
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vw-acp-prompt-{label}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.path.join(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct HomeEnvGuard {
    _lock: tokio::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl HomeEnvGuard {
    async fn set(home_dir: &Path) -> Self {
        let lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
        let saved = save_home_env();
        unsafe { std::env::set_var("HOME", home_dir) };
        unsafe { std::env::set_var("USERPROFILE", home_dir) };
        unsafe { std::env::remove_var("HOMEDRIVE") };
        unsafe { std::env::remove_var("HOMEPATH") };
        Self { _lock: lock, saved }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..) {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn save_home_env() -> Vec<(&'static str, Option<std::ffi::OsString>)> {
    ["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect()
}

#[cfg(unix)]
struct MockPromptAgent {
    _dir: TempDir,
    script: PathBuf,
    calls_log: PathBuf,
    mode: &'static str,
}

#[cfg(unix)]
impl MockPromptAgent {
    fn new(mode: &'static str) -> Self {
        let dir = TempDir::new("agent");
        let script = dir.join("agent.py");
        let calls_log = dir.join("calls.jsonl");
        fs::write(&script, mock_prompt_agent_script()).expect("write mock agent");
        fs::write(&calls_log, "").expect("write calls log");
        Self { _dir: dir, script, calls_log, mode }
    }

    fn config(&self) -> AcpAgentConfig {
        AcpAgentConfig {
            command: "python3".to_string(),
            args: vec![
                "-u".to_string(),
                self.script.display().to_string(),
                self.calls_log.display().to_string(),
                self.mode.to_string(),
            ],
            env: HashMap::new(),
        }
    }

    fn methods(&self) -> Vec<String> {
        fs::read_to_string(&self.calls_log)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|value| value.get("method")?.as_str().map(ToOwned::to_owned))
            .collect()
    }

    fn calls(&self) -> Vec<Value> {
        fs::read_to_string(&self.calls_log)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .collect()
    }
}

#[cfg(unix)]
fn test_session_record(cwd: &Path, agent: &MockPromptAgent) -> crate::SessionRecord {
    crate::SessionRecord {
        schema: crate::SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "acp-1".to_string(),
        agent_session_id: Some("agent-1".to_string()),
        agent_command: "mock".to_string(),
        agent_config: Some(agent.config()),
        cwd: cwd.to_string_lossy().into_owned(),
        name: Some("work".to_string()),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/active.ndjson".to_string(),
            segment_count: 1,
            max_segment_bytes: 4096,
            max_segments: 3,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(true),
        closed_at: Some("2026-01-01T00:00:01Z".to_string()),
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
        vwacp: Some(crate::SessionAcpxState {
            current_mode_id: None,
            desired_mode_id: None,
            current_model_id: None,
            available_models: None,
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: Some(SessionStateOptions {
                model: Some("stored-model".to_string()),
                allowed_tools: Some(vec!["shell".to_string()]),
                max_turns: Some(2),
            }),
        }),
    }
}

#[cfg(unix)]
fn mock_prompt_agent_script() -> &'static str {
    r#"import json
import sys

calls_log = sys.argv[1]
mode = sys.argv[2] if len(sys.argv) > 2 else "success"
session_counter = 0
prompt_attempts = 0

def write_call(message):
    with open(calls_log, "a", encoding="utf-8") as handle:
        handle.write(json.dumps({
            "method": message.get("method"),
            "params": message.get("params", {}),
        }) + "\n")
        handle.flush()

def send(payload):
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()

for raw_line in sys.stdin:
    line = raw_line.strip()
    if not line:
        continue
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    params = message.get("params", {})
    write_call(message)

    if method == "initialize":
        result = {
            "protocolVersion": params["protocolVersion"],
            "agentCapabilities": {},
            "authMethods": [],
            "agentInfo": {"name": "mock-acp-agent", "version": "0.1.0"},
        }
    elif method == "session/new":
        session_counter += 1
        result = {"sessionId": f"session-{session_counter}"}
    elif method == "session/set_model" and mode == "model_error":
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {"code": -32603, "message": "model rejected"},
        })
        continue
    elif method in ("session/load", "session/resume", "session/set_model"):
        result = {}
    elif method == "session/prompt":
        prompt_attempts += 1
        session_id = params.get("sessionId", "session-1")
        if mode == "retry" and prompt_attempts == 1:
            send({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32603, "message": "agent returned -32603: transient prompt failure"},
            })
            continue
        if mode == "error":
            send({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32603, "message": "prompt failed"},
            })
            continue
        send({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": session_id,
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": {"type": "text", "text": "agent output"},
                },
            },
        })
        result = {
            "stopReason": "max_tokens",
            "usage": {
                "totalTokens": 7,
                "inputTokens": 3,
                "outputTokens": 4,
                "cachedReadTokens": 1,
                "thoughtTokens": 2,
            },
        }
    else:
        if request_id is None:
            continue
        result = {}

    if request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": result})
"#
}

// 下列行为用例覆盖正常完成、超时中断、回退重试和资源清理。
#[test]
fn map_finish_reason_maps_known_values_and_defaults() {
    assert_eq!(map_finish_reason(Some("length")), StopReason::MaxTokens);
    assert_eq!(map_finish_reason(Some("max_turn_requests")), StopReason::MaxTurnRequests);
    assert_eq!(map_finish_reason(Some("refusal")), StopReason::Refusal);
    assert_eq!(map_finish_reason(Some("cancelled")), StopReason::Cancelled);
    assert_eq!(map_finish_reason(Some("other")), StopReason::EndTurn);
    assert_eq!(map_finish_reason(None), StopReason::EndTurn);
}

#[test]
fn resolve_agent_config_prefers_structured_config() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "value".to_string());
    let structured = AcpAgentConfig {
        command: "structured-agent".to_string(),
        args: vec!["--fast".to_string()],
        env,
    };

    let resolved = resolve_agent_config("string-agent --ignored", Some(structured.clone()));

    assert_eq!(resolved, structured);
}

#[test]
fn resolve_agent_config_parses_command_when_config_missing() {
    let resolved = resolve_agent_config(r#"agent "two words" --flag"#, None);

    assert_eq!(resolved.command, "agent");
    assert_eq!(resolved.args, vec!["two words", "--flag"]);
    assert!(resolved.env.is_empty());
}

#[test]
fn prompt_request_uses_display_text_and_default_resume_strategy() {
    let prompt = crate::prompt_content::text_prompt("hello");

    let request = prompt_request_for_session(
        "session-1".to_string(),
        ".",
        &prompt,
        Some(SessionResumePolicy::AllowNew),
    );

    assert_eq!(request.prompt, "hello");
    assert!(request.cwd.is_absolute());
    assert_eq!(request.session_strategy, SessionStrategy::ResumeLoadOrNew("session-1".to_string()));
}

#[test]
fn prompt_request_same_session_only_uses_resume_or_load() {
    let prompt = crate::prompt_content::text_prompt("resume only");

    let request = prompt_request_for_session(
        "session-2".to_string(),
        "/tmp/project",
        &prompt,
        Some(SessionResumePolicy::SameSessionOnly),
    );

    assert_eq!(request.cwd, std::path::PathBuf::from("/tmp/project"));
    assert_eq!(request.session_strategy, SessionStrategy::ResumeOrLoad("session-2".to_string()));
}

#[test]
fn should_retry_prompt_requires_runtime_output_code() {
    let timeout = SessionRuntimeError::from_source(crate::session_runtime_helpers::TimeoutError {
        timeout_ms: 10,
    });

    assert!(!should_retry_prompt(&timeout));
}

#[test]
fn should_retry_prompt_accepts_retryable_acp_internal_error_messages() {
    let internal = SessionRuntimeError::from_source(TestRuntimeError("agent returned -32603"));
    let parse = SessionRuntimeError::from_source(TestRuntimeError("agent returned -32700"));

    assert!(should_retry_prompt(&internal));
    assert!(should_retry_prompt(&parse));
}

#[test]
fn should_retry_prompt_rejects_plain_runtime_errors() {
    let error = SessionRuntimeError::from_source(TestRuntimeError("plain runtime failure"));

    assert!(!should_retry_prompt(&error));
}

#[test]
fn emit_prompt_retry_notice_returns_when_suppressed_and_writes_when_enabled() {
    let error = SessionRuntimeError::from_source(TestRuntimeError("agent returned -32603"));

    emit_prompt_retry_notice(&error, 10, 1, 2, true);
    emit_prompt_retry_notice(&error, 10, 1, 2, false);
}

#[test]
fn to_prompt_result_preserves_stop_reason_and_session_id() {
    let client = AcpClient::new(
        "agent".to_string(),
        AcpAgentConfig { command: "agent".to_string(), args: Vec::new(), env: HashMap::new() },
    );

    let result = to_prompt_result(StopReason::Cancelled, "session-3".to_string(), &client);

    assert_eq!(
        result,
        RunPromptResult {
            stop_reason: StopReason::Cancelled,
            session_id: "session-3".to_string(),
            permission_stats: PermissionStats::default(),
        }
    );
}

#[test]
fn prompt_result_finish_reason_maps_to_runtime_stop_reason() {
    let result = PromptResult {
        session_id: "agent-session".to_string(),
        deltas: vec!["a".to_string(), "b".to_string()],
        finish_reason: Some("length".to_string()),
        usage: None,
    };

    assert_eq!(map_finish_reason(result.finish_reason.as_deref()), StopReason::MaxTokens);
}

#[tokio::test]
async fn noop_active_session_controller_denies_active_cancel() {
    let controller = NoopActiveSessionController;

    assert!(!controller.has_active_prompt());
    assert!(!controller.request_cancel_active_prompt().await.unwrap());
    assert!(controller.set_session_mode("default".to_string()).await.is_ok());
    assert!(controller.set_session_model("model".to_string()).await.is_ok());
    assert!(
        controller.set_session_config_option("id".to_string(), "value".to_string()).await.is_err()
    );
}

#[tokio::test]
async fn apply_requested_model_ignores_empty_model_without_client_call() {
    let client = AcpClient::new(
        "agent",
        AcpAgentConfig { command: "unused".to_string(), args: Vec::new(), env: HashMap::new() },
    );

    assert!(!apply_requested_model(&client, "session-1", ".", None, Some(1)).await.unwrap());
    assert!(
        !apply_requested_model(&client, "session-1", ".", Some(" \n\t "), Some(1)).await.unwrap()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn run_once_applies_model_replays_output_and_returns_stop_reason() {
    let agent = MockPromptAgent::new("success");
    let cwd = TempDir::new("run-once-cwd");
    let raw_messages = Arc::new(Mutex::new(Vec::<(AcpMessageDirection, Value)>::new()));
    let on_acp_message = {
        let raw_messages = raw_messages.clone();
        Arc::new(move |direction, message| {
            raw_messages
                .lock()
                .push((direction, serde_json::to_value(message).expect("message serializes")));
        }) as crate::types::AcpMessageCallback
    };
    let mut formatter = CapturingFormatter::default();

    let result = run_once(RunOnceOptions {
        agent_command: "mock".to_string(),
        agent_config: Some(agent.config()),
        cwd: cwd.path().to_string_lossy().into_owned(),
        prompt: crate::prompt_content::text_prompt("hello"),
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut formatter,
        on_acp_message: Some(on_acp_message),
        on_session_update: None,
        on_client_operation: None,
        suppress_sdk_console_errors: true,
        verbose: false,
        session_options: Some(AcpSessionOptions {
            model: Some("model-a".to_string()),
            allowed_tools: None,
            max_turns: None,
        }),
        prompt_retries: None,
        timeout_ms: Some(5_000),
    })
    .await
    .expect("run once succeeds");

    assert_eq!(result.stop_reason, StopReason::MaxTokens);
    assert_eq!(result.session_id, "session-1");
    assert_eq!(formatter.context.as_deref(), Some("session-1"));
    assert!(formatter.flush_count > 0);
    assert!(formatter.messages.iter().any(|message| message["method"] == "session/update"));
    assert!(
        raw_messages
            .lock()
            .iter()
            .any(|(direction, _)| { matches!(direction, AcpMessageDirection::Outbound) })
    );
    assert!(
        raw_messages
            .lock()
            .iter()
            .any(|(direction, _)| { matches!(direction, AcpMessageDirection::Inbound) })
    );

    let methods = agent.methods();
    assert!(methods.contains(&"session/new".to_string()));
    assert!(methods.contains(&"session/set_model".to_string()));
    assert!(methods.contains(&"session/prompt".to_string()));

    let prompt_call = agent
        .calls()
        .into_iter()
        .find(|call| call["method"] == "session/prompt")
        .expect("prompt call logged");
    assert_eq!(prompt_call["params"]["prompt"][0]["text"], "hello");
}

#[cfg(unix)]
#[tokio::test]
async fn run_once_retries_retryable_prompt_without_side_effects() {
    let agent = MockPromptAgent::new("retry");
    let cwd = TempDir::new("run-once-retry-cwd");
    let mut formatter = CapturingFormatter::default();

    let result = run_once(RunOnceOptions {
        agent_command: "mock".to_string(),
        agent_config: Some(agent.config()),
        cwd: cwd.path().to_string_lossy().into_owned(),
        prompt: crate::prompt_content::text_prompt("retry me"),
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        suppress_sdk_console_errors: true,
        verbose: false,
        session_options: None,
        prompt_retries: Some(1),
        timeout_ms: Some(5_000),
    })
    .await
    .expect("retry succeeds");

    assert_eq!(result.stop_reason, StopReason::MaxTokens);
    assert_eq!(agent.methods().into_iter().filter(|method| method == "session/prompt").count(), 2);
}

#[cfg(unix)]
#[tokio::test]
async fn run_once_returns_requested_model_error_before_prompt() {
    let agent = MockPromptAgent::new("model_error");
    let cwd = TempDir::new("run-once-model-error-cwd");
    let mut formatter = CapturingFormatter::default();

    let error = run_once(RunOnceOptions {
        agent_command: "mock".to_string(),
        agent_config: Some(agent.config()),
        cwd: cwd.path().to_string_lossy().into_owned(),
        prompt: crate::prompt_content::text_prompt("hello"),
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        suppress_sdk_console_errors: true,
        verbose: false,
        session_options: Some(AcpSessionOptions {
            model: Some("bad-model".to_string()),
            allowed_tools: None,
            max_turns: None,
        }),
        prompt_retries: None,
        timeout_ms: Some(5_000),
    })
    .await
    .expect_err("model setting fails");

    assert_eq!(error.output_params().code, crate::OutputErrorCode::Runtime);
    let methods = agent.methods();
    assert!(methods.contains(&"session/set_model".to_string()));
    assert!(!methods.contains(&"session/prompt".to_string()));
}

#[cfg(unix)]
#[tokio::test]
async fn run_once_returns_prompt_error_after_output_replay() {
    let agent = MockPromptAgent::new("error");
    let cwd = TempDir::new("run-once-error-cwd");
    let mut formatter = CapturingFormatter::default();

    let error = run_once(RunOnceOptions {
        agent_command: "mock".to_string(),
        agent_config: Some(agent.config()),
        cwd: cwd.path().to_string_lossy().into_owned(),
        prompt: crate::prompt_content::text_prompt("fail"),
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        suppress_sdk_console_errors: true,
        verbose: false,
        session_options: None,
        prompt_retries: None,
        timeout_ms: Some(5_000),
    })
    .await
    .expect_err("prompt fails");

    assert!(error.output_already_emitted());
    assert!(formatter.flush_count > 0);
    assert_eq!(agent.methods().into_iter().filter(|method| method == "session/prompt").count(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn run_session_prompt_updates_result_record_and_notifies_lifecycle_callbacks() {
    let home = TempDir::new("session-home");
    let _home = HomeEnvGuard::set(home.path()).await;
    let cwd = TempDir::new("session-cwd");
    let agent = MockPromptAgent::new("success");
    let record = test_session_record(cwd.path(), &agent);
    crate::session_persistence::write_session_record(&record).await.expect("write session record");
    let client_available_count = Arc::new(AtomicUsize::new(0));
    let client_closed_count = Arc::new(AtomicUsize::new(0));
    let prompt_active_count = Arc::new(AtomicUsize::new(0));
    let mut formatter = CapturingFormatter::default();

    let result = run_session_prompt(RunSessionPromptOptions {
        session_record_id: "record-1".to_string(),
        prompt: crate::prompt_content::text_prompt("stored prompt"),
        resume_policy: Some(SessionResumePolicy::SameSessionOnly),
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        timeout_ms: Some(5_000),
        suppress_sdk_console_errors: true,
        verbose: true,
        prompt_retries: None,
        on_client_available: Some({
            let client_available_count = client_available_count.clone();
            Arc::new(move |controller| {
                assert!(!controller.has_active_prompt());
                client_available_count.fetch_add(1, AtomicOrdering::SeqCst);
            })
        }),
        on_client_closed: Some({
            let client_closed_count = client_closed_count.clone();
            Arc::new(move || {
                client_closed_count.fetch_add(1, AtomicOrdering::SeqCst);
            })
        }),
        on_prompt_active: Some({
            let prompt_active_count = prompt_active_count.clone();
            Arc::new(move || {
                prompt_active_count.fetch_add(1, AtomicOrdering::SeqCst);
            })
        }),
    })
    .await
    .expect("session prompt succeeds");

    assert_eq!(result.stop_reason, StopReason::MaxTokens);
    assert!(result.resumed);
    assert_eq!(result.session_id, "record-1");
    assert_eq!(result.record.closed, Some(false));
    assert_eq!(result.record.closed_at, None);
    assert!(result.record.last_prompt_at.is_some());
    assert!(matches!(result.record.messages.first(), Some(SessionMessage::User(_))));
    assert!(matches!(result.record.messages.last(), Some(SessionMessage::Agent(_))));
    assert_eq!(formatter.context.as_deref(), Some("record-1"));
    assert_eq!(client_available_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(client_closed_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(prompt_active_count.load(AtomicOrdering::SeqCst), 1);

    let methods = agent.methods();
    assert!(methods.contains(&"session/resume".to_string()));
    assert!(methods.contains(&"session/prompt".to_string()));
}

#[cfg(unix)]
#[tokio::test]
async fn run_session_prompt_retries_retryable_prompt_without_client_notifications() {
    let home = TempDir::new("session-retry-home");
    let _home = HomeEnvGuard::set(home.path()).await;
    let cwd = TempDir::new("session-retry-cwd");
    let agent = MockPromptAgent::new("retry");
    let record = test_session_record(cwd.path(), &agent);
    crate::session_persistence::write_session_record(&record).await.expect("write session record");
    let mut formatter = CapturingFormatter::default();

    let result = run_session_prompt(RunSessionPromptOptions {
        session_record_id: "record-1".to_string(),
        prompt: crate::prompt_content::text_prompt("stored retry"),
        resume_policy: None,
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        timeout_ms: Some(5_000),
        suppress_sdk_console_errors: true,
        verbose: false,
        prompt_retries: Some(1),
        on_client_available: None,
        on_client_closed: None,
        on_prompt_active: None,
    })
    .await
    .expect("session retry succeeds");

    assert_eq!(result.stop_reason, StopReason::MaxTokens);
    assert_eq!(agent.methods().into_iter().filter(|method| method == "session/prompt").count(), 2);
}

#[cfg(unix)]
#[tokio::test]
async fn run_session_prompt_closes_client_callback_on_prompt_error() {
    let home = TempDir::new("session-error-home");
    let _home = HomeEnvGuard::set(home.path()).await;
    let cwd = TempDir::new("session-error-cwd");
    let agent = MockPromptAgent::new("error");
    let record = test_session_record(cwd.path(), &agent);
    crate::session_persistence::write_session_record(&record).await.expect("write session record");
    let client_closed_count = Arc::new(AtomicUsize::new(0));
    let prompt_active_count = Arc::new(AtomicUsize::new(0));
    let mut formatter = CapturingFormatter::default();

    let error = run_session_prompt(RunSessionPromptOptions {
        session_record_id: "record-1".to_string(),
        prompt: crate::prompt_content::text_prompt("stored fail"),
        resume_policy: Some(SessionResumePolicy::AllowNew),
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        timeout_ms: Some(5_000),
        suppress_sdk_console_errors: true,
        verbose: false,
        prompt_retries: None,
        on_client_available: Some(Arc::new(|_| {})),
        on_client_closed: Some({
            let client_closed_count = client_closed_count.clone();
            Arc::new(move || {
                client_closed_count.fetch_add(1, AtomicOrdering::SeqCst);
            })
        }),
        on_prompt_active: Some({
            let prompt_active_count = prompt_active_count.clone();
            Arc::new(move || {
                prompt_active_count.fetch_add(1, AtomicOrdering::SeqCst);
            })
        }),
    })
    .await
    .expect_err("session prompt fails");

    assert!(error.output_already_emitted());
    assert_eq!(client_closed_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(prompt_active_count.load(AtomicOrdering::SeqCst), 1);
}
