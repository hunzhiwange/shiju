//! 验证客户端 Actor 循环的命令分派、运行时清理与生命周期记录。

use std::collections::HashMap;
#[cfg(unix)]
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(unix)]
use std::time::{Duration as StdDuration, Instant};

use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

use crate::{
    AcpAgentConfig, AcpError, AgentLifecycleExit, PermissionStats, PromptRequest, PromptResult,
    SessionInfo,
};

use super::super::{AcpClient, AcpClientActorHandle, ActorCommand, ChildExitSummary};

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

#[cfg(unix)]
static UNIQUE_TEST_ID: AtomicU64 = AtomicU64::new(1);

#[test]
fn command_family_detection_uses_basename_and_arguments() {
    assert!(client("/usr/local/bin/gemini", &["--experimental-acp"]).is_gemini_acp_command());
    assert!(!client("gemini", &["chat"]).is_gemini_acp_command());

    assert!(client("claude-agent-acp", &[]).is_claude_acp_command());
    assert!(
        client("npx", &["@agentclientprotocol/claude-agent-acp@^0.26.0"]).is_claude_acp_command()
    );
}

#[test]
fn store_reusable_session_replaces_and_clears_session_id() {
    let client = client("agent", &[]);

    client.store_reusable_session(Some("session-1".to_string()));
    assert_eq!(client.actor_state.lock().reusable_session_id, Some("session-1".to_string()));

    client.store_reusable_session(None);
    assert!(client.actor_state.lock().reusable_session_id.is_none());
}

#[test]
fn record_actor_start_and_exit_update_lifecycle_snapshot() {
    let client = client("agent", &[]);

    client.record_actor_start(Some(10));
    let started = client.get_agent_lifecycle_snapshot();

    assert_eq!(started.pid, Some(10));
    assert!(started.started_at.is_some());
    assert!(started.last_exit.is_none());

    client.record_actor_exit(
        ChildExitSummary { exit_code: Some(2), signal: Some("TERM".to_string()) },
        Some("test_reason"),
        true,
    );
    let exited = client.get_agent_lifecycle_snapshot();
    let exited_at = exited.last_exit.as_ref().and_then(|exit| exit.exited_at.clone());

    assert!(exited.pid.is_none());
    assert_eq!(
        exited.last_exit,
        Some(AgentLifecycleExit {
            exit_code: Some(2),
            signal: Some("TERM".to_string()),
            exited_at,
            reason: Some("test_reason".to_string()),
            unexpected_during_prompt: true,
        })
    );
}

#[test]
fn run_actor_thread_reports_startup_and_exits_when_channel_closes() {
    let client = client("agent", &[]);
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (startup_tx, startup_rx) = oneshot::channel();
    drop(command_tx);

    client.run_actor_thread(command_rx, startup_tx);

    let runtime =
        tokio::runtime::Builder::new_current_thread().enable_all().build().expect("test runtime");
    let startup = runtime.block_on(startup_rx).expect("startup response");
    assert!(startup.is_ok());
}

#[tokio::test]
async fn actor_loop_dispatches_runtime_commands_and_close_response() {
    let client = client("", &[]);
    let _client_ref = client.clone();
    *client.permission_stats.lock() =
        PermissionStats { requested: 3, approved: 2, denied: 1, cancelled: 0 };
    client.store_reusable_session(Some("stale-session".to_string()));

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let cwd = PathBuf::from(".");

    let (create_tx, create_rx) = oneshot::channel();
    assert!(
        command_tx
            .send(ActorCommand::CreateSession { cwd: cwd.clone(), response_tx: create_tx })
            .is_ok()
    );

    let (load_tx, load_rx) = oneshot::channel();
    assert!(
        command_tx
            .send(ActorCommand::LoadSession {
                session_id: "load-session".to_string(),
                cwd: cwd.clone(),
                response_tx: load_tx,
            })
            .is_ok()
    );

    let (resume_tx, resume_rx) = oneshot::channel();
    assert!(
        command_tx
            .send(ActorCommand::ResumeSession {
                session_id: "resume-session".to_string(),
                cwd: cwd.clone(),
                response_tx: resume_tx,
            })
            .is_ok()
    );

    let (mode_tx, mode_rx) = oneshot::channel();
    assert!(
        command_tx
            .send(ActorCommand::SetSessionMode {
                session_id: "mode-session".to_string(),
                cwd: cwd.clone(),
                mode_id: "plan".to_string(),
                response_tx: mode_tx,
            })
            .is_ok()
    );

    let (config_tx, config_rx) = oneshot::channel();
    assert!(
        command_tx
            .send(ActorCommand::SetSessionConfigOption {
                session_id: "config-session".to_string(),
                cwd: cwd.clone(),
                option_name: "model".to_string(),
                value_id: "fast".to_string(),
                response_tx: config_tx,
            })
            .is_ok()
    );

    let (model_tx, model_rx) = oneshot::channel();
    assert!(
        command_tx
            .send(ActorCommand::SetSessionModel {
                session_id: "model-session".to_string(),
                cwd: cwd.clone(),
                model: "sonnet".to_string(),
                response_tx: model_tx,
            })
            .is_ok()
    );

    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let (prompt_tx, prompt_rx) = oneshot::channel();
    assert!(
        command_tx
            .send(ActorCommand::RunPrompt {
                request: PromptRequest::new(cwd, "hello"),
                event_tx,
                response_tx: prompt_tx,
            })
            .is_ok()
    );

    let (close_tx, close_rx) = oneshot::channel();
    assert!(command_tx.send(ActorCommand::Close { response_tx: close_tx }).is_ok());

    client.actor_loop(command_rx).await;

    assert_empty_command(create_rx.await.expect("create response"));
    assert_empty_command(load_rx.await.expect("load response"));
    assert_empty_command(resume_rx.await.expect("resume response"));
    assert_empty_command(mode_rx.await.expect("mode response"));
    assert_empty_command(config_rx.await.expect("config response"));
    assert_empty_command(model_rx.await.expect("model response"));
    assert_empty_command(prompt_rx.await.expect("prompt response"));
    close_rx.await.expect("close response");
    assert_eq!(*client.permission_stats.lock(), PermissionStats::default());
    assert!(client.actor_state.lock().reusable_session_id.is_none());
}

#[tokio::test]
async fn actor_loop_clears_reusable_session_when_command_channel_closes() {
    let client = client("agent", &[]);
    client.store_reusable_session(Some("stale-session".to_string()));
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    drop(command_tx);

    client.actor_loop(command_rx).await;

    assert!(client.actor_state.lock().reusable_session_id.is_none());
}

#[tokio::test]
async fn actor_loop_close_without_runtime_clears_reusable_session_and_replies() {
    let client = client("agent", &[]);
    client.store_reusable_session(Some("stale-session".to_string()));
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (close_tx, close_rx) = oneshot::channel();

    command_tx.send(ActorCommand::Close { response_tx: close_tx }).expect("send close");

    client.actor_loop(command_rx).await;

    close_rx.await.expect("close response");
    assert!(client.actor_state.lock().reusable_session_id.is_none());
}

#[tokio::test]
async fn actor_loop_close_ignores_dropped_response_receiver() {
    let client = client("agent", &[]);
    client.store_reusable_session(Some("stale-session".to_string()));
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (close_tx, close_rx) = oneshot::channel();
    drop(close_rx);

    command_tx.send(ActorCommand::Close { response_tx: close_tx }).expect("send close");

    client.actor_loop(command_rx).await;

    assert!(client.actor_state.lock().reusable_session_id.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn actor_loop_dispatches_successful_session_commands_until_close() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let fixture = MockActorAgentFixture::new();
            let client = fixture.client(Duration::from_secs(60));
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let actor_client = client.clone();
            let actor_task =
                tokio::task::spawn_local(async move { actor_client.actor_loop(command_rx).await });

            let created =
                send_create(&command_tx, fixture.cwd.clone()).await.expect("create should succeed");
            assert_eq!(created.session_id, "session-1");
            assert!(client.has_reusable_session("session-1"));

            let loaded = send_load(&command_tx, "loaded", fixture.cwd.clone())
                .await
                .expect("load should succeed");
            assert_eq!(loaded.session_id, "loaded");
            assert!(client.has_reusable_session("loaded"));

            let resumed = send_resume(&command_tx, "resumed", fixture.cwd.clone())
                .await
                .expect("resume should succeed");
            assert_eq!(resumed.session_id, "resumed");
            assert!(client.has_reusable_session("resumed"));

            send_mode(&command_tx, "resumed", fixture.cwd.clone(), "plan")
                .await
                .expect("mode should succeed");
            send_config(&command_tx, "resumed", fixture.cwd.clone(), "effort", "high")
                .await
                .expect("config should succeed");
            send_model(&command_tx, "resumed", fixture.cwd.clone(), "sonnet")
                .await
                .expect("model should succeed");

            let (event_tx, _event_rx) = mpsc::unbounded_channel();
            let prompt = send_prompt(&command_tx, fixture.cwd.clone(), event_tx)
                .await
                .expect("prompt should succeed");
            assert_eq!(prompt.session_id, "session-2");
            assert_eq!(prompt.finish_reason.as_deref(), Some("stop"));
            assert!(client.has_reusable_session("session-2"));

            let (close_tx, close_rx) = oneshot::channel();
            command_tx.send(ActorCommand::Close { response_tx: close_tx }).expect("send close");
            close_rx.await.expect("close response");
            actor_task.await.expect("actor task should finish");

            assert!(client.actor_state.lock().reusable_session_id.is_none());
            assert_eq!(
                fixture.logged_methods(),
                vec![
                    "initialize",
                    "session/new",
                    "session/load",
                    "session/resume",
                    "session/resume",
                    "session/set_mode",
                    "session/resume",
                    "session/set_config_option",
                    "session/resume",
                    "session/set_model",
                    "session/new",
                    "session/prompt",
                ]
            );
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn actor_loop_close_with_active_runtime_records_client_close_reason() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let fixture = MockActorAgentFixture::new();
            let client = fixture.client(Duration::from_secs(60));
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let actor_client = client.clone();
            let actor_task =
                tokio::task::spawn_local(async move { actor_client.actor_loop(command_rx).await });

            send_create(&command_tx, fixture.cwd.clone())
                .await
                .expect("create should start runtime");

            let (close_tx, close_rx) = oneshot::channel();
            command_tx.send(ActorCommand::Close { response_tx: close_tx }).expect("send close");
            close_rx.await.expect("close response");
            actor_task.await.expect("actor task should finish");

            let snapshot = client.get_agent_lifecycle_snapshot();
            let exit = snapshot.last_exit.expect("client close should record exit");
            assert!(snapshot.pid.is_none());
            assert_eq!(exit.reason.as_deref(), Some("client_close"));
            assert!(client.actor_state.lock().reusable_session_id.is_none());
        })
        .await;
}

#[tokio::test]
async fn actor_loop_exits_when_only_internal_sender_keeps_channel_open() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let client = client("agent", &[]);
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            client.actor_state.lock().handle =
                Some(AcpClientActorHandle { command_tx, thread: None });
            let weak_state = Arc::downgrade(&client.actor_state);
            let actor_client = client.clone();
            let actor_task =
                tokio::task::spawn_local(async move { actor_client.actor_loop(command_rx).await });

            drop(client);

            timeout(Duration::from_secs(1), actor_task)
                .await
                .expect("actor should exit without external client handles")
                .expect("actor task should not panic");
            assert!(weak_state.upgrade().is_none());
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn actor_loop_idle_timeout_shutdown_clears_runtime_and_accepts_close() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let fixture = MockActorAgentFixture::new();
            let client = fixture.client(Duration::from_millis(20));
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let actor_client = client.clone();
            let actor_task =
                tokio::task::spawn_local(async move { actor_client.actor_loop(command_rx).await });

            send_create(&command_tx, fixture.cwd.clone())
                .await
                .expect("create should succeed before idle timeout");

            let snapshot =
                wait_for_idle_shutdown(&client).await.expect("idle timeout should record shutdown");
            let exit = snapshot.last_exit.expect("idle shutdown exit");
            assert!(snapshot.pid.is_none());
            assert_eq!(exit.reason.as_deref(), Some("idle_timeout"));
            assert!(client.actor_state.lock().reusable_session_id.is_none());

            let (close_tx, close_rx) = oneshot::channel();
            command_tx.send(ActorCommand::Close { response_tx: close_tx }).expect("send close");
            close_rx.await.expect("close response");
            actor_task.await.expect("actor task should finish");
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn actor_loop_command_channel_closed_shutdowns_active_runtime() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let fixture = MockActorAgentFixture::new();
            let client = fixture.client(Duration::from_secs(60));
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let actor_client = client.clone();
            let actor_task =
                tokio::task::spawn_local(async move { actor_client.actor_loop(command_rx).await });

            send_create(&command_tx, fixture.cwd.clone())
                .await
                .expect("create should succeed before channel closes");
            drop(command_tx);
            actor_task.await.expect("actor task should finish");

            let snapshot = client.get_agent_lifecycle_snapshot();
            let exit = snapshot.last_exit.expect("channel closed exit");
            assert!(snapshot.pid.is_none());
            assert_eq!(exit.reason.as_deref(), Some("actor_channel_closed"));
            assert!(client.actor_state.lock().reusable_session_id.is_none());
        })
        .await;
}

#[cfg(unix)]
#[tokio::test]
async fn actor_loop_io_closed_shutdowns_active_runtime_and_accepts_close() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let fixture = MockActorAgentFixture::exiting_after_new();
            let client = fixture.client(Duration::from_secs(60));
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let actor_client = client.clone();
            let actor_task =
                tokio::task::spawn_local(async move { actor_client.actor_loop(command_rx).await });

            send_create(&command_tx, fixture.cwd.clone())
                .await
                .expect("create should succeed before io closes");

            let snapshot =
                wait_for_shutdown_reason(&client, "io_closed").await.expect("io should close");
            let exit = snapshot.last_exit.expect("io closed exit");
            assert!(snapshot.pid.is_none());
            assert_eq!(exit.reason.as_deref(), Some("io_closed"));
            assert!(client.actor_state.lock().reusable_session_id.is_none());

            let (close_tx, close_rx) = oneshot::channel();
            command_tx.send(ActorCommand::Close { response_tx: close_tx }).expect("send close");
            close_rx.await.expect("close response");
            actor_task.await.expect("actor task should finish");
        })
        .await;
}

#[cfg(unix)]
async fn send_create(
    command_tx: &mpsc::UnboundedSender<ActorCommand>,
    cwd: PathBuf,
) -> Result<SessionInfo, AcpError> {
    let (response_tx, response_rx) = oneshot::channel();
    command_tx.send(ActorCommand::CreateSession { cwd, response_tx }).expect("send create");
    response_rx.await.expect("create response")
}

#[cfg(unix)]
async fn send_load(
    command_tx: &mpsc::UnboundedSender<ActorCommand>,
    session_id: &str,
    cwd: PathBuf,
) -> Result<SessionInfo, AcpError> {
    let (response_tx, response_rx) = oneshot::channel();
    command_tx
        .send(ActorCommand::LoadSession { session_id: session_id.to_string(), cwd, response_tx })
        .expect("send load");
    response_rx.await.expect("load response")
}

#[cfg(unix)]
async fn send_resume(
    command_tx: &mpsc::UnboundedSender<ActorCommand>,
    session_id: &str,
    cwd: PathBuf,
) -> Result<SessionInfo, AcpError> {
    let (response_tx, response_rx) = oneshot::channel();
    command_tx
        .send(ActorCommand::ResumeSession { session_id: session_id.to_string(), cwd, response_tx })
        .expect("send resume");
    response_rx.await.expect("resume response")
}

#[cfg(unix)]
async fn send_mode(
    command_tx: &mpsc::UnboundedSender<ActorCommand>,
    session_id: &str,
    cwd: PathBuf,
    mode_id: &str,
) -> Result<(), AcpError> {
    let (response_tx, response_rx) = oneshot::channel();
    command_tx
        .send(ActorCommand::SetSessionMode {
            session_id: session_id.to_string(),
            cwd,
            mode_id: mode_id.to_string(),
            response_tx,
        })
        .expect("send mode");
    response_rx.await.expect("mode response")
}

#[cfg(unix)]
async fn send_config(
    command_tx: &mpsc::UnboundedSender<ActorCommand>,
    session_id: &str,
    cwd: PathBuf,
    option_name: &str,
    value_id: &str,
) -> Result<agent_client_protocol::SetSessionConfigOptionResponse, AcpError> {
    let (response_tx, response_rx) = oneshot::channel();
    command_tx
        .send(ActorCommand::SetSessionConfigOption {
            session_id: session_id.to_string(),
            cwd,
            option_name: option_name.to_string(),
            value_id: value_id.to_string(),
            response_tx,
        })
        .expect("send config");
    response_rx.await.expect("config response")
}

#[cfg(unix)]
async fn send_model(
    command_tx: &mpsc::UnboundedSender<ActorCommand>,
    session_id: &str,
    cwd: PathBuf,
    model: &str,
) -> Result<(), AcpError> {
    let (response_tx, response_rx) = oneshot::channel();
    command_tx
        .send(ActorCommand::SetSessionModel {
            session_id: session_id.to_string(),
            cwd,
            model: model.to_string(),
            response_tx,
        })
        .expect("send model");
    response_rx.await.expect("model response")
}

#[cfg(unix)]
async fn send_prompt(
    command_tx: &mpsc::UnboundedSender<ActorCommand>,
    cwd: PathBuf,
    event_tx: mpsc::UnboundedSender<crate::PromptEvent>,
) -> Result<PromptResult, AcpError> {
    let (response_tx, response_rx) = oneshot::channel();
    command_tx
        .send(ActorCommand::RunPrompt {
            request: PromptRequest::new(cwd, "hello"),
            event_tx,
            response_tx,
        })
        .expect("send prompt");
    response_rx.await.expect("prompt response")
}

#[cfg(unix)]
async fn wait_for_idle_shutdown(client: &AcpClient) -> Option<crate::AgentLifecycleSnapshot> {
    wait_for_shutdown_reason(client, "idle_timeout").await
}

#[cfg(unix)]
async fn wait_for_shutdown_reason(
    client: &AcpClient,
    reason: &str,
) -> Option<crate::AgentLifecycleSnapshot> {
    let deadline = Instant::now() + StdDuration::from_secs(3);
    while Instant::now() < deadline {
        let snapshot = client.get_agent_lifecycle_snapshot();
        if snapshot.pid.is_none()
            && snapshot.last_exit.as_ref().and_then(|exit| exit.reason.as_deref()) == Some(reason)
        {
            return Some(snapshot);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    None
}

#[cfg(unix)]
struct MockActorAgentFixture {
    cwd: PathBuf,
    script_path: PathBuf,
    calls_log: PathBuf,
}

#[cfg(unix)]
impl MockActorAgentFixture {
    fn new() -> Self {
        Self::with_script(mock_actor_agent_script())
    }

    fn exiting_after_new() -> Self {
        Self::with_script(mock_actor_agent_exiting_after_new_script())
    }

    fn with_script(script: &str) -> Self {
        let id = UNIQUE_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("vw-acp-actor-test-{id}"));
        let cwd = base.join("cwd");
        let script_path = base.join("mock_agent.py");
        let calls_log = base.join("calls.jsonl");
        fs::create_dir_all(&cwd).expect("fixture cwd");
        fs::write(&script_path, script).expect("fixture script");
        fs::write(&calls_log, "").expect("fixture calls log");
        Self { cwd, script_path, calls_log }
    }

    fn client(&self, actor_idle_timeout: Duration) -> AcpClient {
        let args = vec![
            "-u".to_string(),
            self.script_path.display().to_string(),
            self.calls_log.display().to_string(),
        ];
        AcpClient::new(
            "mock",
            AcpAgentConfig { command: "python3".to_string(), args, env: HashMap::new() },
        )
        .with_actor_idle_timeout(actor_idle_timeout)
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
impl Drop for MockActorAgentFixture {
    fn drop(&mut self) {
        if let Some(base) = self.script_path.parent().filter(|path| path.exists()) {
            let _ = fs::remove_dir_all(base);
        }
    }
}

#[cfg(unix)]
fn mock_actor_agent_script() -> &'static str {
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
fn mock_actor_agent_exiting_after_new_script() -> &'static str {
    r#"import json
import sys

calls_log = sys.argv[1]

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
        result = {"sessionId": "session-1"}
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
        sys.stdout.flush()
        break
    else:
        if request_id is None:
            continue
        result = {}

    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()
"#
}

fn assert_empty_command<T>(result: Result<T, AcpError>) {
    assert!(matches!(result, Err(AcpError::EmptyCommand)));
}
